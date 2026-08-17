// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use avcore::timeline::{ShapeClip, TextClip};
use avcore::{ClipInstance, MediaAsset, TrackKind};
use eframe::egui;
use tracing::{debug, error, warn};

use super::App;

/// Playhead for a frozen clip after `elapsed_secs` of wall-clock playback since
/// `playhead_at_start`, clamped to the clip's own end (`clip_start_secs + clip_duration_secs`)
/// — pulled out of [`App::pump_preview_frame`] as a pure function so the math is unit
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

impl App {
    /// Drops the current pipeline/texture so the next Editor frame rebuilds overlay branches
    /// from their latest styling. Word-timing changes can replace a text branch's buffer live,
    /// but arbitrary text/shape property edits still use this conservative full rebuild.
    pub fn invalidate_preview_rendering(&mut self) {
        self.preview = None;
        self.preview_texture = None;
        self.preview_clip_id = None;
        self.preview_overlay_clip_ids.clear();
        self.preview_audio_clip_ids.clear();
        self.preview_text_clip_ids.clear();
        self.preview_shape_clip_ids.clear();
    }

    /// The clip covering the active sequence's timeline playhead, and the asset it plays from,
    /// if both resolve — `None` if the video track is missing/empty, nothing covers the
    /// playhead ([`avcore::timeline::Track::clip_at`]), or the clip's `asset_id` isn't in the
    /// media library. Always the *first* video track (background/track 0) — see
    /// [`App::current_preview_overlay_clips`] for the rest.
    fn current_preview_clip(&self) -> Option<(ClipInstance, MediaAsset)> {
        let project = self.active_project();
        let timeline = project.timeline();
        let track = timeline
            .tracks
            .iter()
            .find(|t| t.kind == TrackKind::Video && t.visible)?;
        let clip = track.clip_at(timeline.playhead_secs)?;
        let asset = project
            .media_library
            .iter()
            .find(|a| a.id == clip.asset_id)?;
        Some((clip.clone(), asset.clone()))
    }

    /// Every overlay-track (video track index 1+, in track order — matches
    /// [`avcore::render::resolve_timeline_segments_multi`]'s "track index = z-order"
    /// convention) clip covering the playhead, with the asset it plays from — a track with
    /// nothing at the playhead, or whose clip's asset doesn't resolve, is just skipped rather
    /// than aborting the whole list, same "degrade gracefully" shape
    /// [`App::current_preview_clip`] already has for the background track.
    fn current_preview_overlay_clips(&self) -> Vec<(ClipInstance, MediaAsset)> {
        let project = self.active_project();
        let timeline = project.timeline();
        timeline
            .tracks
            .iter()
            .filter(|t| t.kind == TrackKind::Video && t.visible)
            .skip(1)
            .filter_map(|t| {
                let clip = t.clip_at(timeline.playhead_secs)?;
                let asset = project
                    .media_library
                    .iter()
                    .find(|a| a.id == clip.asset_id)?;
                Some((clip.clone(), asset.clone()))
            })
            .collect()
    }

    /// Every audio-only track clip covering the playhead, in track order. Video clips with
    /// embedded audio are already represented by the background/overlay branch lists.
    fn current_preview_audio_clips(&self) -> Vec<(ClipInstance, MediaAsset)> {
        let project = self.active_project();
        let timeline = project.timeline();
        timeline
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Audio && track.visible)
            .filter_map(|track| {
                let clip = track.clip_at(timeline.playhead_secs)?;
                let asset = project
                    .media_library
                    .iter()
                    .find(|asset| asset.id == clip.asset_id && asset.has_audio)?;
                Some((clip.clone(), asset.clone()))
            })
            .collect()
    }

    /// Every [`TextClip`] covering the playhead, across every `TrackKind::Text` track — same
    /// "collect what's here, skip what isn't" shape [`App::current_preview_overlay_clips`] has,
    /// but there's no asset to resolve (text has no source file) and no track-index-as-z-order
    /// convention (there's no `overlay`/`background` distinction among text tracks — every one
    /// composites the same way, on top of every video branch, matching export's own
    /// post-processing-pass ordering).
    fn preview_text_clips_at(&self, position_secs: f64) -> Vec<TextClip> {
        let timeline = self.active_project().timeline();
        timeline
            .tracks
            .iter()
            .filter(|t| t.kind == TrackKind::Text)
            .filter_map(|t| {
                t.text_clips.iter().find(|c| {
                    position_secs >= c.start_secs && position_secs < c.start_secs + c.duration_secs
                })
            })
            .cloned()
            .collect()
    }

    fn current_preview_text_clips(&self) -> Vec<TextClip> {
        self.preview_text_clips_at(self.active_project().timeline().playhead_secs)
    }

    /// Pushes a replacement RGBA frame only when `position_secs` crosses into another timed
    /// word (or a gap between words). `Preview` owns the last active-word state and ignores
    /// same-word calls, so invoking this every UI frame during playback remains cheap.
    fn refresh_preview_text_highlights(&mut self, position_secs: f64) {
        if self.preview_text_clip_ids.is_empty() {
            return;
        }
        let clips = self.preview_text_clips_at(position_secs);
        let ids: Vec<u64> = clips.iter().map(|clip| clip.id).collect();
        if ids != self.preview_text_clip_ids {
            return;
        }
        let timed_clips: Vec<(&TextClip, f64)> = clips
            .iter()
            .map(|clip| (clip, position_secs - clip.start_secs))
            .collect();
        if let Some(preview) = self.preview.as_mut() {
            if let Err(error) = preview.update_text_overlays(&timed_clips) {
                warn!(%error, "failed to refresh preview word highlight");
            }
        }
    }

    /// Same role as [`App::current_preview_text_clips`], for [`ShapeClip`]s on
    /// `TrackKind::Shape` tracks.
    fn current_preview_shape_clips(&self) -> Vec<ShapeClip> {
        let timeline = self.active_project().timeline();
        timeline
            .tracks
            .iter()
            .filter(|t| t.kind == TrackKind::Shape)
            .filter_map(|t| {
                t.shape_clips.iter().find(|c| {
                    timeline.playhead_secs >= c.start_secs
                        && timeline.playhead_secs < c.start_secs + c.duration_secs
                })
            })
            .cloned()
            .collect()
    }

    /// The resolved source path (proxy preferred) for a clip/asset pair, or `None` if it
    /// doesn't exist on disk — the same "don't spin up a pipeline for a file that's known to be
    /// missing" check [`App::ensure_preview_loaded`] already applied to the background clip.
    fn preview_source_path(asset: &MediaAsset) -> Option<std::path::PathBuf> {
        let path = asset
            .proxy_path
            .clone()
            .unwrap_or_else(|| asset.source_path.clone());
        path.exists().then_some(path)
    }

    /// This clip's own timeline-to-source offset at `playhead_secs`, honoring `frozen` (always
    /// the held anchor frame) and `speed_factor` — the same math
    /// [`App::ensure_preview_loaded`]/[`App::seek_preview`] already apply to the background
    /// clip, pulled out so overlay branches can each compute their own independently (every
    /// branch has its own trim points and may have its own `frozen`/`speed_factor`).
    fn clip_seek_offset(clip: &ClipInstance, playhead_secs: f64) -> f64 {
        if clip.frozen {
            clip.source_in_secs
        } else {
            let speed = clip.speed_factor.max(0.01) as f64;
            clip.source_in_secs + (playhead_secs - clip.start_secs) * speed
        }
    }

    /// Reopens the preview pipeline whenever the clip covering the timeline playhead
    /// ([`App::current_preview_clip`]) differs from the one last opened for
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
        let current = self.current_preview_clip();
        let (overlays, audio_clips, text_clips, shape_clips) = if current.is_some() {
            (
                self.current_preview_overlay_clips(),
                self.current_preview_audio_clips(),
                self.current_preview_text_clips(),
                self.current_preview_shape_clips(),
            )
        } else {
            (Vec::new(), Vec::new(), Vec::new(), Vec::new())
        };
        let overlay_ids: Vec<u64> = overlays.iter().map(|(c, _)| c.id).collect();
        let audio_ids: Vec<u64> = audio_clips.iter().map(|(clip, _)| clip.id).collect();
        let text_ids: Vec<u64> = text_clips.iter().map(|c| c.id).collect();
        let shape_ids: Vec<u64> = shape_clips.iter().map(|c| c.id).collect();
        let current_clip_id = current.as_ref().map(|(c, _)| c.id);
        if current_clip_id == self.preview_clip_id
            && overlay_ids == self.preview_overlay_clip_ids
            && audio_ids == self.preview_audio_clip_ids
            && text_ids == self.preview_text_clip_ids
            && shape_ids == self.preview_shape_clip_ids
        {
            return;
        }
        self.preview = None;
        self.preview_texture = None;
        // Not just the background id — `preview_clip_present()` (and the Editor's "preview
        // unavailable" vs. plain placeholder choice) needs to tell "a clip is here but its
        // pipeline failed to open" apart from "there's nothing to preview at all", and an
        // unresolvable asset is the latter, not the former.
        self.preview_clip_id = current_clip_id;
        self.preview_overlay_clip_ids = overlay_ids;
        self.preview_audio_clip_ids = audio_ids;
        self.preview_text_clip_ids = text_ids;
        self.preview_shape_clip_ids = shape_ids;

        let Some((clip, asset)) = current else {
            self.preview_playing = false;
            return;
        };
        let Some(path) = Self::preview_source_path(&asset) else {
            // A moved/deleted source file would otherwise still spin up a whole GStreamer
            // pipeline just to watch it fail to open a file that isn't there.
            return;
        };
        let playhead = self.active_project().timeline().playhead_secs;

        let wants_composited = !(overlays.is_empty()
            && audio_clips.is_empty()
            && text_clips.is_empty()
            && shape_clips.is_empty());
        let overlay_paths: Vec<Option<std::path::PathBuf>> = overlays
            .iter()
            .map(|(_, a)| Self::preview_source_path(a))
            .collect();
        let audio_paths: Vec<Option<std::path::PathBuf>> = audio_clips
            .iter()
            .map(|(_, asset)| Self::preview_source_path(asset))
            .collect();
        // Every video overlay clip needs a resolvable path too — falling back to the plain
        // single-clip pipeline (background only, dropping any text/shape overlays as well)
        // rather than silently dropping just the unresolvable overlay would misrepresent which
        // clips are actually compositing.
        let composited = wants_composited
            && overlay_paths.iter().all(Option::is_some)
            && audio_paths.iter().all(Option::is_some);

        let opened = if composited {
            let overlay_refs: Vec<(&std::path::Path, &ClipInstance)> = overlay_paths
                .iter()
                .zip(&overlays)
                .map(|(p, (c, _))| (p.as_deref().expect("checked above"), c))
                .collect();
            let audio_refs: Vec<(&std::path::Path, &ClipInstance)> = audio_paths
                .iter()
                .zip(&audio_clips)
                .map(|(path, (clip, _))| (path.as_deref().expect("checked above"), clip))
                .collect();
            // The elapsed time since each text clip's own start — what
            // `render_text_clip_rgba` resolves its current highlighted word against.
            let text_refs: Vec<(&TextClip, f64)> = text_clips
                .iter()
                .map(|c| (c, playhead - c.start_secs))
                .collect();
            let shape_refs: Vec<&ShapeClip> = shape_clips.iter().collect();
            avcore::preview::Preview::open_composited(
                &path,
                Some(&clip),
                &overlay_refs,
                &audio_refs,
                &text_refs,
                &shape_refs,
            )
        } else {
            avcore::preview::Preview::open(&path, Some(&clip))
        };

        match opened {
            Ok(preview) => {
                debug!(
                    path = %path.display(),
                    clip_id = clip.id,
                    overlay_count = overlays.len(),
                    "preview pipeline opened"
                );
                if !composited {
                    if clip.frozen {
                        // Always show the held anchor frame (the frame at source_in_secs),
                        // never whatever offset the playhead happens to be at within this clip
                        // — matches export holding that same frame for the block's whole
                        // trimmed duration.
                        if let Err(e) = preview.seek(clip.source_in_secs) {
                            warn!(error = %e, "failed to seek newly opened frozen preview");
                        }
                        self.preview_frozen_since = self
                            .preview_playing
                            .then_some((std::time::Instant::now(), playhead));
                    } else {
                        let speed = clip.speed_factor.max(0.01) as f64;
                        let offset = Self::clip_seek_offset(&clip, playhead);
                        if let Err(e) = preview.seek_with_rate(offset, speed) {
                            warn!(error = %e, "failed to seek newly opened preview");
                        }
                        self.preview_frozen_since = None;
                    }
                } else {
                    let mut offsets = vec![Self::clip_seek_offset(&clip, playhead)];
                    offsets.extend(
                        overlays
                            .iter()
                            .map(|(c, _)| Self::clip_seek_offset(c, playhead)),
                    );
                    offsets.extend(
                        audio_clips
                            .iter()
                            .map(|(clip, _)| Self::clip_seek_offset(clip, playhead)),
                    );
                    let mut rates = vec![clip.speed_factor.max(0.01) as f64];
                    rates.extend(
                        overlays
                            .iter()
                            .map(|(c, _)| c.speed_factor.max(0.01) as f64),
                    );
                    rates.extend(
                        audio_clips
                            .iter()
                            .map(|(clip, _)| clip.speed_factor.max(0.01) as f64),
                    );
                    if let Err(e) = preview.seek_composited(&offsets, &rates) {
                        warn!(error = %e, "failed to seek newly opened composited preview");
                    }
                    // A frozen background clip's pipeline stays Paused regardless of rate (same
                    // as the single-clip path) — its own playhead advance is wall-clock-driven
                    // instead, uniformly across whatever overlays are compositing on top of it.
                    self.preview_frozen_since = clip
                        .frozen
                        .then(|| self.preview_playing)
                        .unwrap_or(false)
                        .then_some((std::time::Instant::now(), playhead));
                }
                if self.preview_playing && !clip.frozen {
                    if let Err(e) = preview.play() {
                        warn!(error = %e, "failed to resume preview playback across a cut");
                    }
                }
                self.preview = Some(preview);
            }
            Err(e) => error!(path = %path.display(), error = %e, "failed to open preview pipeline"),
        }
    }

    /// Toggles play/pause on the current preview pipeline. A no-op if nothing is selected or
    /// the pipeline failed to open. A frozen clip's underlying pipeline stays `Paused`
    /// regardless — only [`App::preview_frozen_since`] starts/stops, driving the playhead
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

    /// Enters or exits the Editor preview panel's fullscreen overlay. Entering resets
    /// [`App::fullscreen_controls_last_moved`] to `None` so the overlay's controls start
    /// visible rather than possibly already faded from a stale timestamp.
    pub fn toggle_fullscreen_preview(&mut self) {
        self.fullscreen_preview = !self.fullscreen_preview;
        self.fullscreen_controls_last_moved = None;
    }

    pub fn exit_fullscreen_preview(&mut self) {
        self.fullscreen_preview = false;
    }

    /// Marks the fullscreen overlay's controls as just-interacted-with — called whenever the
    /// pointer moves or is pressed while [`App::fullscreen_preview`] is active, resetting the
    /// fade-out timer so the controls stay visible for another
    /// [`crate::screens::editor::FULLSCREEN_CONTROLS_IDLE_SECS`].
    pub fn note_fullscreen_controls_activity(&mut self) {
        self.fullscreen_controls_last_moved = Some(std::time::Instant::now());
    }

    /// Opacity multiplier (`0.0..=1.0`) for the fullscreen overlay's controls, based on how
    /// long it's been since the last pointer activity — fully visible until
    /// [`crate::screens::editor::FULLSCREEN_CONTROLS_IDLE_SECS`] elapses, then a
    /// half-second linear fade to fully transparent. `None`-last-moved (just entered
    /// fullscreen) is treated as "just now", so controls start fully visible.
    pub fn fullscreen_controls_opacity(&self) -> f32 {
        const FADE_SECS: f32 = 0.5;
        let idle_secs = self
            .fullscreen_controls_last_moved
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        let over = idle_secs - crate::screens::editor::FULLSCREEN_CONTROLS_IDLE_SECS;
        if over <= 0.0 {
            1.0
        } else {
            (1.0 - over / FADE_SECS).clamp(0.0, 1.0)
        }
    }

    /// Seeks to `position_secs` (timeline-relative). Takes the fast path — seeking the
    /// already-open pipeline directly — when `position_secs` still falls within the clip it's
    /// currently loaded for; otherwise updates the timeline playhead and lets
    /// [`App::ensure_preview_loaded`] open the right clip's pipeline next frame. A no-op if
    /// nothing is selected or the pipeline failed to open.
    pub fn seek_preview(&mut self, position_secs: f64) {
        let same_clip = self
            .preview_clip_id
            .zip(self.current_preview_clip())
            .filter(|(loaded_id, (clip, _))| *loaded_id == clip.id)
            .map(|(_, (clip, _))| clip);

        // For a composited pipeline, the overlay set covering the playhead — video, text, and
        // shape alike — must also still match exactly what was opened for: a plain
        // background-id match isn't enough once there are overlay branches, since the fast
        // path below reuses the already-open pipeline's branches as-is rather than rebuilding
        // them.
        let timeline = self.active_project().timeline();
        let overlays_still_match = {
            let overlay_ids: Vec<u64> = timeline
                .tracks
                .iter()
                .filter(|t| t.kind == TrackKind::Video && t.visible)
                .skip(1)
                .filter_map(|t| t.clip_at(position_secs).map(|c| c.id))
                .collect();
            overlay_ids == self.preview_overlay_clip_ids
        };
        let audio_still_match = {
            let audio_ids: Vec<u64> = timeline
                .tracks
                .iter()
                .filter(|track| track.kind == TrackKind::Audio && track.visible)
                .filter_map(|track| track.clip_at(position_secs).map(|clip| clip.id))
                .collect();
            audio_ids == self.preview_audio_clip_ids
        };
        let text_still_match = {
            let text_ids: Vec<u64> = timeline
                .tracks
                .iter()
                .filter(|t| t.kind == TrackKind::Text)
                .filter_map(|t| {
                    t.text_clips
                        .iter()
                        .find(|c| {
                            position_secs >= c.start_secs
                                && position_secs < c.start_secs + c.duration_secs
                        })
                        .map(|c| c.id)
                })
                .collect();
            text_ids == self.preview_text_clip_ids
        };
        let shape_still_match = {
            let shape_ids: Vec<u64> = timeline
                .tracks
                .iter()
                .filter(|t| t.kind == TrackKind::Shape)
                .filter_map(|t| {
                    t.shape_clips
                        .iter()
                        .find(|c| {
                            position_secs >= c.start_secs
                                && position_secs < c.start_secs + c.duration_secs
                        })
                        .map(|c| c.id)
                })
                .collect();
            shape_ids == self.preview_shape_clip_ids
        };
        let is_composited = !self.preview_overlay_clip_ids.is_empty()
            || !self.preview_audio_clip_ids.is_empty()
            || !self.preview_text_clip_ids.is_empty()
            || !self.preview_shape_clip_ids.is_empty();
        let branches_still_match =
            overlays_still_match && audio_still_match && text_still_match && shape_still_match;

        match (&self.preview, same_clip.filter(|_| branches_still_match)) {
            (Some(preview), Some(clip)) if !is_composited => {
                let speed = clip.speed_factor.max(0.01) as f64;
                let offset = Self::clip_seek_offset(&clip, position_secs);
                if let Err(e) = preview.seek_with_rate(offset, speed) {
                    warn!(error = %e, "failed to seek preview");
                }
                self.active_project_mut().timeline_mut().playhead_secs = position_secs;
            }
            (Some(preview), Some(clip)) => {
                let timeline = self.active_project().timeline();
                let overlay_clips: Vec<&ClipInstance> = timeline
                    .tracks
                    .iter()
                    .filter(|t| t.kind == TrackKind::Video && t.visible)
                    .skip(1)
                    .filter_map(|t| t.clip_at(position_secs))
                    .collect();
                let audio_clips: Vec<&ClipInstance> = timeline
                    .tracks
                    .iter()
                    .filter(|track| track.kind == TrackKind::Audio && track.visible)
                    .filter_map(|track| track.clip_at(position_secs))
                    .collect();
                let mut offsets = vec![Self::clip_seek_offset(&clip, position_secs)];
                offsets.extend(
                    overlay_clips
                        .iter()
                        .map(|c| Self::clip_seek_offset(c, position_secs)),
                );
                offsets.extend(
                    audio_clips
                        .iter()
                        .map(|clip| Self::clip_seek_offset(clip, position_secs)),
                );
                let mut rates = vec![clip.speed_factor.max(0.01) as f64];
                rates.extend(
                    overlay_clips
                        .iter()
                        .map(|c| c.speed_factor.max(0.01) as f64),
                );
                rates.extend(
                    audio_clips
                        .iter()
                        .map(|clip| clip.speed_factor.max(0.01) as f64),
                );
                if let Err(e) = preview.seek_composited(&offsets, &rates) {
                    warn!(error = %e, "failed to seek composited preview");
                }
                self.active_project_mut().timeline_mut().playhead_secs = position_secs;
            }
            _ => {
                self.active_project_mut().timeline_mut().playhead_secs = position_secs;
            }
        }
        self.refresh_preview_text_highlights(position_secs);
    }

    /// Whether the clip at the timeline playhead has a live preview pipeline — `false` before
    /// any clip covers the playhead, before [`App::ensure_preview_loaded`] has run for it,
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
                let speed = clip.speed_factor.max(0.01) as f64;
                clip.start_secs + (position - clip.source_in_secs) / speed
            }
        };
        self.active_project_mut().timeline_mut().playhead_secs = new_playhead;
        self.refresh_preview_text_highlights(new_playhead);
    }
}
