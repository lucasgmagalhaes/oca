use avcore::{ClipInstance, MediaAsset, TrackKind};
use eframe::egui;
use tracing::{debug, error, warn};

use super::OcaApp;

/// Playhead for a frozen clip after `elapsed_secs` of wall-clock playback since
/// `playhead_at_start`, clamped to the clip's own end (`clip_start_secs + clip_duration_secs`)
/// — pulled out of [`OcaApp::pump_preview_frame`] as a pure function so the math is unit
/// testable without a live `Preview` pipeline (every other preview test in `app_test.rs` runs
/// against a nonexistent source path specifically to avoid needing one).
pub(crate) fn frozen_playhead(
    playhead_at_start: f64,
    elapsed_secs: f64,
    clip_start_secs: f64,
    clip_duration_secs: f64,
) -> f64 {
    (playhead_at_start + elapsed_secs).min(clip_start_secs + clip_duration_secs)
}

impl OcaApp {
    /// Id of the clip covering the active sequence's timeline playhead, if any. Cheaper than
    /// [`OcaApp::current_preview_clip`] — no clones — used by [`OcaApp::ensure_preview_loaded`]
    /// for the early-exit check.
    pub(super) fn current_preview_clip_id(&self) -> Option<u64> {
        let timeline = self.active_project().timeline();
        let track = timeline
            .tracks
            .iter()
            .find(|t| t.kind == TrackKind::Video)?;
        Some(track.clip_at(timeline.playhead_secs)?.id)
    }

    /// The clip covering the active sequence's timeline playhead, and the asset it plays from,
    /// if both resolve — `None` if the video track is missing/empty, nothing covers the
    /// playhead ([`avcore::timeline::Track::clip_at`]), or the clip's `asset_id` isn't in the
    /// media library.
    fn current_preview_clip(&self) -> Option<(ClipInstance, MediaAsset)> {
        let project = self.active_project();
        let timeline = project.timeline();
        let track = timeline
            .tracks
            .iter()
            .find(|t| t.kind == TrackKind::Video)?;
        let clip = track.clip_at(timeline.playhead_secs)?;
        let asset = project
            .media_library
            .iter()
            .find(|a| a.id == clip.asset_id)?;
        Some((clip.clone(), asset.clone()))
    }

    /// Reopens the preview pipeline whenever the clip covering the timeline playhead
    /// ([`OcaApp::current_preview_clip`]) differs from the one last opened for
    /// (`preview_clip_id`) — called once per frame from the Editor's preview panel, right
    /// before it reads any preview state, so the first paint after the playhead moves onto a
    /// different clip is what actually triggers `Preview::open`. Prefers the editing proxy if
    /// one exists (lighter to decode), otherwise the original source file. Leaves `preview` as
    /// `None` without an error dialog if `Preview::open` fails (e.g. a source file that's been
    /// moved or deleted since import) — the Editor screen shows a muted "preview unavailable"
    /// label instead, and won't retry until the playhead moves onto a different clip. Seeks
    /// into the newly opened clip at the playhead's own offset, and resumes playback
    /// (`preview_playing` was already `true`) so crossing a cut doesn't pause playback, just
    /// hitches while the new pipeline opens.
    pub fn ensure_preview_loaded(&mut self) {
        let current_clip_id = self.current_preview_clip_id();
        if current_clip_id == self.preview_clip_id {
            return;
        }
        let current = self.current_preview_clip();
        self.preview = None;
        self.preview_texture = None;
        self.preview_clip_id = current_clip_id;

        let Some((clip, asset)) = current else {
            self.preview_playing = false;
            return;
        };
        let path = asset
            .proxy_path
            .clone()
            .unwrap_or_else(|| asset.source_path.clone());
        // A moved/deleted source file would otherwise still spin up a whole GStreamer pipeline
        // just to watch it fail to open a file that isn't there.
        if !path.exists() {
            return;
        }

        match avcore::preview::Preview::open(&path, Some(&clip)) {
            Ok(preview) => {
                debug!(path = %path.display(), clip_id = clip.id, "preview pipeline opened");
                let playhead = self.active_project().timeline().playhead_secs;
                if clip.frozen {
                    // Always show the held anchor frame (the frame at source_in_secs), never
                    // whatever offset the playhead happens to be at within this clip — matches
                    // export holding that same frame for the block's whole trimmed duration.
                    if let Err(e) = preview.seek(clip.source_in_secs) {
                        warn!(error = %e, "failed to seek newly opened frozen preview");
                    }
                    self.preview_frozen_since = self
                        .preview_playing
                        .then_some((std::time::Instant::now(), playhead));
                } else {
                    let offset = clip.source_in_secs + (playhead - clip.start_secs);
                    if let Err(e) = preview.seek(offset) {
                        warn!(error = %e, "failed to seek newly opened preview");
                    }
                    self.preview_frozen_since = None;
                    if self.preview_playing {
                        if let Err(e) = preview.play() {
                            warn!(error = %e, "failed to resume preview playback across a cut");
                        }
                    }
                }
                self.preview = Some(preview);
            }
            Err(e) => error!(path = %path.display(), error = %e, "failed to open preview pipeline"),
        }
    }

    /// Toggles play/pause on the current preview pipeline. A no-op if nothing is selected or
    /// the pipeline failed to open. A frozen clip's underlying pipeline stays `Paused`
    /// regardless — only [`OcaApp::preview_frozen_since`] starts/stops, driving the playhead
    /// forward on its own since there's no advancing pipeline position to read for a held
    /// frame.
    pub fn toggle_preview_playback(&mut self) {
        let Some(preview) = &self.preview else {
            return;
        };
        let frozen = self.current_preview_clip().is_some_and(|(c, _)| c.frozen);
        let now_playing = !self.preview_playing;
        let result = if frozen {
            Ok(())
        } else if now_playing {
            preview.play()
        } else {
            preview.pause()
        };
        match result {
            Ok(()) => {
                self.preview_playing = now_playing;
                self.preview_frozen_since = (frozen && now_playing).then_some((
                    std::time::Instant::now(),
                    self.active_project().timeline().playhead_secs,
                ));
            }
            Err(e) => warn!(error = %e, "failed to toggle preview playback"),
        }
    }

    /// Seeks to `position_secs` (timeline-relative). Takes the fast path — seeking the
    /// already-open pipeline directly — when `position_secs` still falls within the clip it's
    /// currently loaded for; otherwise updates the timeline playhead and lets
    /// [`OcaApp::ensure_preview_loaded`] open the right clip's pipeline next frame. A no-op if
    /// nothing is selected or the pipeline failed to open.
    pub fn seek_preview(&mut self, position_secs: f64) {
        let same_clip = self
            .preview_clip_id
            .zip(self.current_preview_clip())
            .filter(|(loaded_id, (clip, _))| *loaded_id == clip.id)
            .map(|(_, (clip, _))| clip);

        match (&self.preview, same_clip) {
            (Some(preview), Some(clip)) => {
                let offset = clip.source_in_secs + (position_secs - clip.start_secs);
                if let Err(e) = preview.seek(offset) {
                    warn!(error = %e, "failed to seek preview");
                }
                self.active_project_mut().timeline_mut().playhead_secs = position_secs;
            }
            _ => {
                self.active_project_mut().timeline_mut().playhead_secs = position_secs;
            }
        }
    }

    /// Whether the clip at the timeline playhead has a live preview pipeline — `false` before
    /// any clip covers the playhead, before [`OcaApp::ensure_preview_loaded`] has run for it,
    /// and when it couldn't open one.
    pub fn preview_available(&self) -> bool {
        self.preview.is_some()
    }

    /// Whether a clip currently covers the timeline playhead, whether or not its preview
    /// pipeline could actually be opened — distinguishes "nothing to preview here" from
    /// "something's here but its preview failed to open" for the Editor's empty-state label.
    pub fn preview_clip_present(&self) -> bool {
        self.preview_clip_id.is_some()
    }

    /// Pulls the latest decoded video frame (if any) into `preview_texture`, and — while
    /// playing — mirrors the pipeline's position into the active project's timeline playhead,
    /// converting from the clip-relative position `Preview` reports back to timeline time.
    /// Once the clip currently loaded (`preview_clip_id`) plays past its own
    /// `source_out_secs`, advances the playhead to that clip's end instead — the next frame's
    /// `ensure_preview_loaded` call then naturally opens whatever clip (if any) covers that
    /// position, continuing playback across the cut. Called once per frame from
    /// [`eframe::App::ui`], before the screens draw.
    pub(super) fn pump_preview_frame(&mut self, ctx: &egui::Context) {
        let Some(preview) = &self.preview else {
            return;
        };

        if let Some(frame) = preview.current_frame() {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [frame.width as usize, frame.height as usize],
                &frame.rgba,
            );
            match &mut self.preview_texture {
                Some(texture) => texture.set(image, egui::TextureOptions::LINEAR),
                None => {
                    self.preview_texture =
                        Some(ctx.load_texture("preview", image, egui::TextureOptions::LINEAR));
                }
            }
        }

        if !self.preview_playing {
            return;
        }
        let Some(clip_id) = self.preview_clip_id else {
            return;
        };
        let Some(clip) = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .find_map(|t| t.clips.iter().find(|c| c.id == clip_id))
            .cloned()
        else {
            return;
        };

        let new_playhead = if clip.frozen {
            // The pipeline is paused on the held anchor frame (see ensure_preview_loaded), so
            // there's no Preview::position_secs to derive playback progress from — advance the
            // playhead by wall-clock time elapsed since playback started instead, same
            // real-time rate a normally-decoding clip's own position would advance at.
            let (started_at, playhead_at_start) = *self
                .preview_frozen_since
                .get_or_insert_with(|| (std::time::Instant::now(), clip.start_secs));
            let elapsed = started_at.elapsed().as_secs_f64();
            frozen_playhead(
                playhead_at_start,
                elapsed,
                clip.start_secs,
                clip.duration_secs(),
            )
        } else {
            let Some(position) = preview.position_secs() else {
                return;
            };
            if position >= clip.source_out_secs {
                clip.start_secs + clip.duration_secs()
            } else {
                clip.start_secs + (position - clip.source_in_secs)
            }
        };
        self.active_project_mut().timeline_mut().playhead_secs = new_playhead;
    }
}
