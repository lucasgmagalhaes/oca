use avcore::timeline::{ClipInstance, TextClip, Timeline, Track, TrackKind};

use super::OcaApp;

impl OcaApp {
    /// Appends `asset_id` to the timeline as a new, untrimmed clip — what double-clicking an
    /// asset in the media library panel does. Lands on the first track whose kind matches the
    /// asset (video onto video, audio onto audio), auto-creating one (`"V1"`/`"A1"`) if none
    /// exists yet, right after whatever's already there
    /// ([`avcore::timeline::Track::duration_secs`] — `0.0` for an empty/new track). A no-op if
    /// `asset_id` isn't in the active project's media library.
    pub fn add_asset_to_timeline(&mut self, asset_id: u64) {
        let Some((kind, duration_secs)) = self.asset_kind_and_duration(asset_id) else {
            return;
        };
        let timeline = self.active_project_mut().timeline_mut();
        let track_index = resolve_or_create_track(timeline, kind, None);
        let start_secs = timeline.tracks[track_index].duration_secs();
        let clip_id = next_clip_id(timeline);
        timeline.tracks[track_index]
            .clips
            .push(avcore::timeline::ClipInstance {
                id: clip_id,
                asset_id,
                start_secs,
                source_in_secs: 0.0,
                source_out_secs: duration_secs,
                composite_id: None,
                gain_db: 0.0,
                frozen: false,
                speed_factor: 1.0,
                crop_x: 0.0,
                crop_y: 0.0,
                crop_w: 1.0,
                crop_h: 1.0,
                mask_shape: avcore::timeline::MaskShape::None,
                mask_corner_radius: 0.0,
                flipped_h: false,
                color_filter: avcore::timeline::ColorFilter::None,
                vignette_intensity: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                sharpen: 0.0,
                chroma_key_enabled: false,
                chroma_key_color: [0, 255, 0],
                chroma_key_tolerance: 0.4,
                blur_intensity: 0.0,
                shake_intensity: 0.0,
                glitch_intensity: 0.0,
                pixelize_intensity: 0.0,
                transition_in: avcore::timeline::TransitionType::None,
                transition_duration_secs: 0.5,
                zoom_start: 1.0,
                zoom_end: 1.0,
            });
    }

    /// Inserts `asset_id` onto the timeline at `start_secs` — what dropping an asset dragged
    /// out of the media library onto the timeline strip does. Prefers `preferred_track_id` if
    /// it exists and matches the asset's kind (the track row the drop landed on); otherwise
    /// falls back to the same track-resolution as [`OcaApp::add_asset_to_timeline`] (first
    /// existing track of matching kind, auto-created if none exists). A no-op if `asset_id`
    /// isn't in the active project's media library or `start_secs` is negative.
    pub fn add_asset_to_timeline_at(
        &mut self,
        asset_id: u64,
        preferred_track_id: Option<u64>,
        start_secs: f64,
    ) {
        if start_secs < 0.0 {
            return;
        }
        let Some((kind, duration_secs)) = self.asset_kind_and_duration(asset_id) else {
            return;
        };
        let timeline = self.active_project_mut().timeline_mut();
        let track_index = resolve_or_create_track(timeline, kind, preferred_track_id);
        let clip_id = next_clip_id(timeline);
        timeline.tracks[track_index]
            .clips
            .push(avcore::timeline::ClipInstance {
                id: clip_id,
                asset_id,
                start_secs,
                source_in_secs: 0.0,
                source_out_secs: duration_secs,
                composite_id: None,
                gain_db: 0.0,
                frozen: false,
                speed_factor: 1.0,
                crop_x: 0.0,
                crop_y: 0.0,
                crop_w: 1.0,
                crop_h: 1.0,
                mask_shape: avcore::timeline::MaskShape::None,
                mask_corner_radius: 0.0,
                flipped_h: false,
                color_filter: avcore::timeline::ColorFilter::None,
                vignette_intensity: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                sharpen: 0.0,
                chroma_key_enabled: false,
                chroma_key_color: [0, 255, 0],
                chroma_key_tolerance: 0.4,
                blur_intensity: 0.0,
                shake_intensity: 0.0,
                glitch_intensity: 0.0,
                pixelize_intensity: 0.0,
                transition_in: avcore::timeline::TransitionType::None,
                transition_duration_secs: 0.5,
                zoom_start: 1.0,
                zoom_end: 1.0,
            });
    }

    /// Looks up `asset_id` in the active project's media library and returns its track kind
    /// and duration, or `None` if it isn't there.
    fn asset_kind_and_duration(&self, asset_id: u64) -> Option<(TrackKind, f64)> {
        let asset = self
            .active_project()
            .media_library
            .iter()
            .find(|a| a.id == asset_id)?;
        let kind = match asset.kind {
            avcore::MediaKind::Video => TrackKind::Video,
            avcore::MediaKind::Audio => TrackKind::Audio,
        };
        Some((kind, asset.duration_secs))
    }

    /// Splits whichever clip covers the timeline playhead, on every track that has one there,
    /// into two — what `Ctrl+B` and the toolbar's "Cortar / Split" button do. Cutting every
    /// track at once (rather than just a clicked clip) keeps V1/A1/A2 in sync, which is the
    /// point of a gameplay edit. A no-op on any track where nothing covers the playhead.
    pub fn split_at_playhead(&mut self) {
        let at_secs = self.active_project().timeline().playhead_secs;
        let mut next_id = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .map(|c| c.id)
            .max()
            .unwrap_or(0)
            + 1;

        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if track.split_clip_at(at_secs, next_id) {
                next_id += 1;
            }
        }
    }

    /// Removes `selected_clip_id` from whichever track has it and clears the selection — what
    /// pressing `Delete` on the timeline does. Leaves a gap rather than rippling later clips
    /// left, matching `split_at_playhead`'s equally simple non-ripple editing model. If the
    /// selected clip is a composite block member (`composite_id.is_some()`), every clip
    /// sharing that id is removed too — deleting one member deletes the whole block, per
    /// `request.md`'s Fase 3 "reutilizado ... como se fosse um clipe só" spec. A no-op if
    /// nothing is selected.
    pub fn delete_selected_clip(&mut self) {
        let Some(clip_id) = self.selected_clip_id else {
            return;
        };
        let timeline = self.active_project_mut().timeline_mut();
        let composite_id = timeline
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .find(|c| c.id == clip_id)
            .and_then(|c| c.composite_id);
        for track in &mut timeline.tracks {
            match composite_id {
                Some(group) => track.clips.retain(|c| c.composite_id != Some(group)),
                None => track.clips.retain(|c| c.id != clip_id),
            }
        }
        self.selected_clip_id = None;
    }

    /// Calls `f` with a mutable borrow of the selected clip, if any — the shared dispatch path
    /// for every `set_selected_clip_*` setter.
    pub(super) fn with_selected_clip_mut(
        &mut self,
        f: impl FnOnce(&mut avcore::timeline::ClipInstance),
    ) {
        let Some(clip_id) = self.selected_clip_id else {
            return;
        };
        if let Some(clip) = self.active_project_mut().timeline_mut().clip_mut(clip_id) {
            f(clip);
        }
    }

    /// Drags `clip_id`'s left edge to `new_start_secs` — what dragging the left handle on a
    /// timeline clip does. A no-op if the clip isn't found or the drag would violate
    /// [`avcore::timeline::ClipInstance::trim_start`]'s bounds (start/source-in going
    /// negative, or shrinking past [`OcaApp::MIN_TRIM_DURATION_SECS`]).
    pub fn trim_clip_start(&mut self, clip_id: u64, new_start_secs: f64) {
        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if let Some(clip) = track.clip_mut(clip_id) {
                clip.trim_start(new_start_secs.max(0.0), Self::MIN_TRIM_DURATION_SECS);
                return;
            }
        }
    }

    /// Drags `clip_id`'s right edge to `new_end_secs` — what dragging the right handle on a
    /// timeline clip does. Bounded above by the clip's source asset's own duration (looked up
    /// via the clip's `asset_id`), so a trim can't ask the source media for footage past its
    /// actual end; skipped if the asset can't be found (best effort rather than blocking the
    /// drag entirely).
    pub fn trim_clip_end(&mut self, clip_id: u64, new_end_secs: f64) {
        let asset_id = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .find(|c| c.id == clip_id)
            .map(|c| c.asset_id);
        let Some(asset_id) = asset_id else {
            return;
        };
        let max_source_out_secs = self
            .active_project()
            .media_library
            .iter()
            .find(|a| a.id == asset_id)
            .map(|a| a.duration_secs);

        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if let Some(clip) = track.clip_mut(clip_id) {
                clip.trim_end(
                    new_end_secs,
                    Self::MIN_TRIM_DURATION_SECS,
                    max_source_out_secs,
                );
                return;
            }
        }
    }

    /// Repositions `clip_id` to `new_start_secs` on its own track — what dragging a timeline
    /// clip's body does when it's dropped back on the same track it started on. A no-op if
    /// the clip isn't found or `new_start_secs` is negative.
    pub fn move_clip(&mut self, clip_id: u64, new_start_secs: f64) {
        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if track.move_clip(clip_id, new_start_secs) {
                return;
            }
        }
    }

    /// [`OcaApp::move_clip`], but if `clip_id` is a composite block member every other clip
    /// sharing its `composite_id` moves by the same delta, on the same track — what dragging a
    /// composite block's body does, so the whole block reads as one clip per `request.md`'s
    /// Fase 3 "blocos compostos" spec. A no-op if `clip_id` isn't found; falls back to a plain
    /// [`OcaApp::move_clip`] if it isn't a composite member.
    pub fn move_clip_with_group(&mut self, clip_id: u64, new_start_secs: f64) {
        let found = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .find_map(|t| {
                let dragged = t.clips.iter().find(|c| c.id == clip_id)?;
                Some((dragged.start_secs, dragged.composite_id, t))
            });
        let Some((old_start_secs, composite_id, track)) = found else {
            return;
        };
        let Some(group) = composite_id else {
            self.move_clip(clip_id, new_start_secs);
            return;
        };
        let delta = new_start_secs - old_start_secs;
        let updates: Vec<(u64, f64)> = track
            .clips
            .iter()
            .filter(|c| c.composite_id == Some(group))
            .map(|c| (c.id, c.start_secs + delta))
            .collect();
        for (id, start_secs) in updates {
            self.move_clip(id, start_secs);
        }
    }

    /// Moves `clip_id` onto `target_track_id` at `new_start_secs` — what dragging a timeline
    /// clip's body onto a *different* track does, once the editor has confirmed the drop
    /// target's row is a same-kind track. A no-op if the clip or target track aren't found,
    /// the kinds don't match, or `new_start_secs` is negative — see
    /// [`avcore::timeline::Timeline::move_clip_to_track`] for the exact rules.
    pub fn move_clip_to_track(
        &mut self,
        clip_id: u64,
        target_track_id: u64,
        new_start_secs: f64,
    ) {
        self.active_project_mut().timeline_mut().move_clip_to_track(
            clip_id,
            target_track_id,
            new_start_secs,
        );
    }

    /// Whether a clip is waiting in the clipboard for [`OcaApp::paste_clip_at_playhead`] — lets
    /// the timeline context menu grey out "Colar" instead of pasting nothing.
    pub fn has_clipboard_clip(&self) -> bool {
        self.clipboard_clip.is_some()
    }

    /// Copies `selected_clip_id` (and the track kind it's on) to [`OcaApp::clipboard_clip`] —
    /// what `Ctrl+C`/the timeline context menu's "Copiar" do. A no-op if nothing is selected.
    pub fn copy_selected_clip(&mut self) {
        let Some(clip_id) = self.selected_clip_id else {
            return;
        };
        let found = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .find_map(|t| {
                t.clips
                    .iter()
                    .find(|c| c.id == clip_id)
                    .map(|c| (c.clone(), t.kind))
            });
        if let Some(copied) = found {
            self.clipboard_clip = Some(copied);
        }
    }

    /// [`OcaApp::copy_selected_clip`] followed by [`OcaApp::delete_selected_clip`] — what
    /// `Ctrl+X`/the context menu's "Recortar" do.
    pub fn cut_selected_clip(&mut self) {
        self.copy_selected_clip();
        self.delete_selected_clip();
    }

    /// Pastes [`OcaApp::clipboard_clip`] as a new, freshly-id'd clip at the playhead's current
    /// position on the active sequence — what `Ctrl+V`/the context menu's "Colar" do. Lands on
    /// a matching-kind track the same way [`OcaApp::add_asset_to_timeline`] does (first
    /// existing track of that kind, auto-created if none exists); always the playhead, not
    /// wherever the context menu happened to be opened — a known simplification. A no-op if
    /// the clipboard is empty. Works across sequence tabs and even across projects, since
    /// `clipboard_clip` isn't scoped to either.
    pub fn paste_clip_at_playhead(&mut self) {
        let Some((copied, kind)) = self.clipboard_clip.clone() else {
            return;
        };
        let playhead_secs = self.active_project().timeline().playhead_secs;
        let timeline = self.active_project_mut().timeline_mut();
        let track_index = resolve_or_create_track(timeline, kind, None);
        let clip_id = next_clip_id(timeline);
        timeline.tracks[track_index]
            .clips
            .push(avcore::timeline::ClipInstance {
                id: clip_id,
                asset_id: copied.asset_id,
                start_secs: playhead_secs,
                source_in_secs: copied.source_in_secs,
                source_out_secs: copied.source_out_secs,
                // A pasted clip is always standalone, even if the copied original was a
                // composite member — copy/paste doesn't replicate group membership (a known
                // gap short of request.md's "reutilizado ... como se fosse um clipe só").
                composite_id: None,
                gain_db: copied.gain_db,
                frozen: copied.frozen,
                speed_factor: copied.speed_factor,
                crop_x: copied.crop_x,
                crop_y: copied.crop_y,
                crop_w: copied.crop_w,
                crop_h: copied.crop_h,
                mask_shape: copied.mask_shape,
                mask_corner_radius: copied.mask_corner_radius,
                flipped_h: copied.flipped_h,
                color_filter: copied.color_filter,
                vignette_intensity: copied.vignette_intensity,
                brightness: copied.brightness,
                contrast: copied.contrast,
                saturation: copied.saturation,
                sharpen: copied.sharpen,
                chroma_key_enabled: copied.chroma_key_enabled,
                chroma_key_color: copied.chroma_key_color,
                chroma_key_tolerance: copied.chroma_key_tolerance,
                blur_intensity: copied.blur_intensity,
                shake_intensity: copied.shake_intensity,
                glitch_intensity: copied.glitch_intensity,
                pixelize_intensity: copied.pixelize_intensity,
                transition_in: copied.transition_in,
                transition_duration_secs: copied.transition_duration_secs,
                zoom_start: copied.zoom_start,
                zoom_end: copied.zoom_end,
            });
    }

    /// Adds/removes `clip_id` from [`OcaApp::multi_selected_clip_ids`] — what `Ctrl+click`ing
    /// a timeline clip does, building up a set of candidates for
    /// [`OcaApp::merge_into_composite`].
    pub fn toggle_multi_select(&mut self, clip_id: u64) {
        if !self.multi_selected_clip_ids.remove(&clip_id) {
            self.multi_selected_clip_ids.insert(clip_id);
        }
    }

    /// Merges every clip in [`OcaApp::multi_selected_clip_ids`] into one composite block —
    /// what the toolbar's "Mesclar em bloco composto" button does (per `request.md`'s Fase 3
    /// "blocos compostos" spec). Assigns them all a fresh `composite_id` and clears the
    /// multi-selection. A no-op, leaving the multi-selection untouched so the user can fix
    /// their pick, if fewer than two ids were selected or they aren't all on the same track —
    /// composite blocks don't span tracks yet.
    pub fn merge_into_composite(&mut self) {
        if self.multi_selected_clip_ids.len() < 2 {
            return;
        }
        let ids = self.multi_selected_clip_ids.clone();
        let timeline = self.active_project_mut().timeline_mut();
        let Some(track) = timeline
            .tracks
            .iter_mut()
            .find(|t| ids.iter().all(|id| t.clips.iter().any(|c| c.id == *id)))
        else {
            return;
        };
        let group_id = track
            .clips
            .iter()
            .filter_map(|c| c.composite_id)
            .max()
            .unwrap_or(0)
            + 1;
        for clip in &mut track.clips {
            if ids.contains(&clip.id) {
                clip.composite_id = Some(group_id);
            }
        }
        self.multi_selected_clip_ids.clear();
    }

    /// Whether formatting is waiting in the clipboard for
    /// [`OcaApp::paste_selected_clip_formatting`] — lets the timeline context menu grey out
    /// "Colar formatação" otherwise.
    pub fn has_formatting_clipboard(&self) -> bool {
        self.formatting_clipboard.is_some()
    }

    /// Copies `selected_clip_id`'s gain/freeze/speed/crop/mask/flip/color-filter/vignette/
    /// color-adjustment settings to [`OcaApp::formatting_clipboard`] — what `Ctrl+Shift+C`/the
    /// context menu's "Copiar formatação" do. A no-op if nothing is selected.
    pub fn copy_selected_clip_formatting(&mut self) {
        let Some(clip) = self.selected_clip() else {
            return;
        };
        self.formatting_clipboard = Some(clip.formatting());
    }

    /// Applies [`OcaApp::formatting_clipboard`] onto `selected_clip_id`, without touching any
    /// other field (position, trim, composite membership) — what `Ctrl+Shift+V`/the context
    /// menu's "Colar formatação" do. A no-op if nothing is selected or the clipboard is empty.
    pub fn paste_selected_clip_formatting(&mut self) {
        let Some(formatting) = self.formatting_clipboard else {
            return;
        };
        self.with_selected_clip_mut(|clip| clip.apply_formatting(&formatting));
    }

    /// Minimum clip duration a drag-trim is allowed to shrink a clip to — small enough to feel
    /// unrestrictive, large enough that a clip can't accidentally get dragged down to
    /// (near-)zero length.
    const MIN_TRIM_DURATION_SECS: f64 = 0.1;

    /// Appends a new empty video track to the active sequence's timeline. The track's name is
    /// generated as `V<n>` where `n` is one past the count of existing video tracks — so a
    /// timeline with one video track "V1" gets a second track "V2". The new track is always
    /// visible and starts empty; the user drags clips onto it from the media library.
    pub fn add_video_track(&mut self) {
        let timeline = self.active_project_mut().timeline_mut();
        let video_count = timeline.tracks.iter().filter(|t| t.kind == TrackKind::Video).count();
        let track_id = timeline.tracks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let name = format!("V{}", video_count + 1);
        timeline.tracks.push(avcore::timeline::Track {
            id: track_id,
            name,
            kind: TrackKind::Video,
            clips: Vec::new(),
            text_clips: Vec::new(),
            visible: true,
        });
    }

    /// Toggles the `visible` flag on the track with `track_id` in the active sequence. A no-op
    /// if the track isn't found (shouldn't happen from the timeline UI, but safe regardless).
    pub fn toggle_track_visibility(&mut self, track_id: u64) {
        let timeline = self.active_project_mut().timeline_mut();
        if let Some(track) = timeline.tracks.iter_mut().find(|t| t.id == track_id) {
            track.visible = !track.visible;
        }
    }
}

/// Finds the track to place a new clip of `kind` on, for [`OcaApp::add_asset_to_timeline`] and
/// [`OcaApp::add_asset_to_timeline_at`]: `preferred_track_id` if it exists and matches `kind`,
/// else the first existing track of that kind, else a newly created `"V1"`/`"A1"` track
/// appended to `timeline.tracks`. Returns the resolved track's index.
pub(super) fn resolve_or_create_track(
    timeline: &mut avcore::timeline::Timeline,
    kind: TrackKind,
    preferred_track_id: Option<u64>,
) -> usize {
    if let Some(id) = preferred_track_id {
        if let Some(index) = timeline
            .tracks
            .iter()
            .position(|t| t.id == id && t.kind == kind)
        {
            return index;
        }
    }
    if let Some(index) = timeline.tracks.iter().position(|t| t.kind == kind) {
        return index;
    }
    let track_id = timeline.tracks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    let name = match kind {
        TrackKind::Video => "V1",
        TrackKind::Audio => "A1",
        TrackKind::Text => "T1",
    };
    timeline.tracks.push(avcore::timeline::Track {
        id: track_id,
        name: name.to_string(),
        kind,
        clips: Vec::new(),
        text_clips: Vec::new(),
        visible: true,
    });
    timeline.tracks.len() - 1
}

/// The next free clip id across every track in `timeline`, including text clips — one past the
/// current max, `1` if the timeline has no clips yet. Covers both [`ClipInstance`]s and
/// [`TextClip`]s so their ids are globally unique within a timeline.
pub(super) fn next_clip_id(timeline: &avcore::timeline::Timeline) -> u64 {
    let video_audio_max = timeline
        .tracks
        .iter()
        .flat_map(|t| &t.clips)
        .map(|c| c.id)
        .max()
        .unwrap_or(0);
    let text_max = timeline
        .tracks
        .iter()
        .flat_map(|t| &t.text_clips)
        .map(|c| c.id)
        .max()
        .unwrap_or(0);
    video_audio_max.max(text_max) + 1
}

impl OcaApp {
    /// Appends a new text track (`TrackKind::Text`) to the active sequence's timeline. The
    /// track is named using [`crate::i18n::Text::DefaultTextTrackName`]. A no-op if the
    /// project has no sequences.
    pub fn add_text_track(&mut self) {
        use crate::i18n::Text;
        let locale = self.locale;
        let track_id = {
            let timeline = self.active_project().timeline();
            timeline.tracks.iter().map(|t| t.id).max().unwrap_or(0) + 1
        };
        let name = Text::DefaultTextTrackName.tr(locale).to_string();
        self.active_project_mut()
            .timeline_mut()
            .tracks
            .push(avcore::timeline::Track {
                id: track_id,
                name,
                kind: TrackKind::Text,
                clips: Vec::new(),
                text_clips: Vec::new(),
                visible: true,
            });
    }

    /// Appends a new [`TextClip`] to the first text track in the active sequence, starting at
    /// the current playhead position and lasting 3 seconds. Selects it immediately so the
    /// properties panel shows its controls. A no-op if no text track exists yet.
    pub fn add_text_clip(&mut self) {
        let playhead_secs = self.active_project().timeline().playhead_secs;
        let track_id = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .find(|t| t.kind == TrackKind::Text)
            .map(|t| t.id);
        let Some(track_id) = track_id else { return };

        let clip_id = next_clip_id(self.active_project().timeline());
        let timeline = self.active_project_mut().timeline_mut();
        if let Some(track) = timeline.tracks.iter_mut().find(|t| t.id == track_id) {
            track.text_clips.push(TextClip {
                id: clip_id,
                start_secs: playhead_secs,
                duration_secs: 3.0,
                text: "Text".to_string(),
                font_size: 48.0,
                color_rgba: [255, 255, 255, 255],
                pos_x: 0.1,
                pos_y: 0.85,
            });
        }
        self.selected_clip_id = None;
        self.selected_text_clip_id = Some(clip_id);
    }
}
