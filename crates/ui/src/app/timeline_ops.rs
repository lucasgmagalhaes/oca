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

use avcore::timeline::{
    AudioRole, ClipInstance, ShapeClip, ShapeKind, TextClip, Timeline, Track, TrackKind,
};

use super::App;

impl App {
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
        self.push_undo_snapshot();
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
                position_keyframes: vec![],
                scale_keyframes: vec![],
                rotation_keyframes: vec![],
                opacity_keyframes: vec![],
                gain_keyframes: vec![],
                brightness_keyframes: vec![],
                contrast_keyframes: vec![],
                saturation_keyframes: vec![],
                crop_x_keyframes: vec![],
                crop_y_keyframes: vec![],
                crop_w_keyframes: vec![],
                crop_h_keyframes: vec![],
                deflicker_enabled: false,
                lut_path: String::new(),
                layer_scale_x: 1.0,
                layer_scale_y: 1.0,
                stabilization_intensity: 0.0,
                background_removal_enabled: false,
                background_removal_mask_path: String::new(),
            });
    }

    /// Inserts `asset_id` onto the timeline at `start_secs` — what dropping an asset dragged
    /// out of the media library onto the timeline strip does. Prefers `preferred_track_id` if
    /// it exists and matches the asset's kind (the track row the drop landed on); otherwise
    /// falls back to the same track-resolution as [`App::add_asset_to_timeline`] (first
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
        self.push_undo_snapshot();
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
                position_keyframes: vec![],
                scale_keyframes: vec![],
                rotation_keyframes: vec![],
                opacity_keyframes: vec![],
                gain_keyframes: vec![],
                brightness_keyframes: vec![],
                contrast_keyframes: vec![],
                saturation_keyframes: vec![],
                crop_x_keyframes: vec![],
                crop_y_keyframes: vec![],
                crop_w_keyframes: vec![],
                crop_h_keyframes: vec![],
                deflicker_enabled: false,
                lut_path: String::new(),
                layer_scale_x: 1.0,
                layer_scale_y: 1.0,
                stabilization_intensity: 0.0,
                background_removal_enabled: false,
                background_removal_mask_path: String::new(),
            });
    }

    /// Looks up `asset_id` in the active project's media library and returns its track kind
    /// and duration, or `None` if it isn't there.
    pub(super) fn asset_kind_and_duration(&self, asset_id: u64) -> Option<(TrackKind, f64)> {
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
        self.push_undo_snapshot();
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
        self.push_undo_snapshot();
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
    /// for every `set_selected_clip_*` setter. Pushes an undo snapshot first via
    /// [`App::push_undo_snapshot_for_drag`], so every effect-property setter (gain, crop,
    /// color adjustments, keyframes, ...) gets undo coverage for free without each of their
    /// ~20 call sites in `properties_panel.rs` needing its own drag-started check.
    pub(super) fn with_selected_clip_mut(
        &mut self,
        f: impl FnOnce(&mut avcore::timeline::ClipInstance),
    ) {
        let Some(clip_id) = self.selected_clip_id else {
            return;
        };
        self.push_undo_snapshot_for_drag();
        let balance = {
            let Some(clip) = self.active_project_mut().timeline_mut().clip_mut(clip_id) else {
                return;
            };
            f(clip);
            let effective_saturation =
                if clip.color_filter == avcore::timeline::ColorFilter::BlackAndWhite {
                    0.0
                } else {
                    clip.saturation
                };
            (clip.brightness, clip.contrast, effective_saturation)
        };
        // Cheap and harmless even for a setter that didn't touch color balance at all -- a
        // no-op push of the clip's own unchanged values. See App::push_live_balance_update's
        // doc comment for why this lives here rather than in each of the ~20 individual
        // set_selected_clip_* setters.
        self.push_live_balance_update(clip_id, balance.0, balance.1, balance.2);
    }

    /// Drags `clip_id`'s left edge to `new_start_secs` — what dragging the left handle on a
    /// timeline clip does. A no-op if the clip isn't found or the drag would violate
    /// [`avcore::timeline::ClipInstance::trim_start`]'s bounds (start/source-in going
    /// negative, or shrinking past [`App::MIN_TRIM_DURATION_SECS`]).
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

    /// [`App::move_clip`], but if `clip_id` is a composite block member every other clip
    /// sharing its `composite_id` moves by the same delta, on the same track — what dragging a
    /// composite block's body does, so the whole block reads as one clip per `request.md`'s
    /// Fase 3 "blocos compostos" spec. A no-op if `clip_id` isn't found; falls back to a plain
    /// [`App::move_clip`] if it isn't a composite member.
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
    pub fn move_clip_to_track(&mut self, clip_id: u64, target_track_id: u64, new_start_secs: f64) {
        self.active_project_mut().timeline_mut().move_clip_to_track(
            clip_id,
            target_track_id,
            new_start_secs,
        );
    }

    /// The source asset's own full duration for `clip_id`'s footage, if both the clip and its
    /// asset can be found — the upper bound every named-trim-mode method below passes as
    /// `max_source_out_secs` so an edit can't ask for footage past the actual source media's
    /// end. Mirrors [`App::trim_clip_end`]'s own inline lookup, factored out here since the
    /// roll/slide edits below need it for more than one clip at a time.
    fn clip_asset_max_source_out_secs(&self, clip_id: u64) -> Option<f64> {
        let asset_id = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .find(|c| c.id == clip_id)
            .map(|c| c.asset_id)?;
        self.active_project()
            .media_library
            .iter()
            .find(|a| a.id == asset_id)
            .map(|a| a.duration_secs)
    }

    /// Ripple-trims `clip_id`'s left edge to `new_start_secs` — Premiere/DaVinci/FCP's
    /// "Ripple" tool (`ROADMAP.md` P2 item 11, the [`EditorTool::Ripple`] toolbar mode): unlike
    /// [`App::trim_clip_start`], every later clip on the same track shifts by the same delta so
    /// no gap is left behind. A no-op if the clip isn't found or the edit would violate its own
    /// trim bounds — see [`avcore::timeline::Track::ripple_trim_start`].
    pub fn ripple_trim_clip_start(&mut self, clip_id: u64, new_start_secs: f64) {
        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if track.ripple_trim_start(
                clip_id,
                new_start_secs.max(0.0),
                Self::MIN_TRIM_DURATION_SECS,
            ) {
                return;
            }
        }
    }

    /// [`App::ripple_trim_clip_start`]'s mirror for the right edge.
    pub fn ripple_trim_clip_end(&mut self, clip_id: u64, new_end_secs: f64) {
        let max_source_out_secs = self.clip_asset_max_source_out_secs(clip_id);
        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if track.ripple_trim_end(
                clip_id,
                new_end_secs,
                Self::MIN_TRIM_DURATION_SECS,
                max_source_out_secs,
            ) {
                return;
            }
        }
    }

    /// Rolls the shared boundary between `clip_id` and its next neighbor to
    /// `new_boundary_secs` — Premiere/DaVinci/FCP's "Roll" tool (`ROADMAP.md` P2 item 11, the
    /// [`EditorTool::Roll`] toolbar mode). `clip_id` must be the *earlier* clip of the pair —
    /// [`App::roll_edit_from_start_edge`] resolves that when the timeline panel's drag grabbed
    /// the later clip's own start edge instead.
    pub fn roll_edit_clip(&mut self, clip_id: u64, new_boundary_secs: f64) {
        let max_source_out_secs = self.clip_asset_max_source_out_secs(clip_id);
        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if track.roll_edit(
                clip_id,
                new_boundary_secs,
                Self::MIN_TRIM_DURATION_SECS,
                max_source_out_secs,
            ) {
                return;
            }
        }
    }

    /// [`App::roll_edit_clip`], but for a drag that grabbed `edited_clip_id`'s *start* edge
    /// instead of its end — the same seam, rolled from the other clip's side. Resolves the
    /// earlier clip in the pair (`edited_clip_id`'s previous neighbor on its track) and
    /// delegates. A no-op if `edited_clip_id` has no previous neighbor.
    pub fn roll_edit_from_start_edge(&mut self, edited_clip_id: u64, new_boundary_secs: f64) {
        let previous_id = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .find_map(|t| t.previous_clip_id(edited_clip_id));
        if let Some(previous_id) = previous_id {
            self.roll_edit_clip(previous_id, new_boundary_secs);
        }
    }

    /// Slips `clip_id`'s source in/out points by `delta_secs`, without moving it on the
    /// timeline or changing its duration — Premiere/DaVinci/FCP's "Slip" tool (`ROADMAP.md` P2
    /// item 11, the [`EditorTool::Slip`] toolbar mode). A no-op if the clip isn't found or the
    /// edit would violate its own trim bounds — see [`avcore::timeline::ClipInstance::slip`].
    pub fn slip_clip(&mut self, clip_id: u64, delta_secs: f64) {
        let max_source_out_secs = self.clip_asset_max_source_out_secs(clip_id);
        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if let Some(clip) = track.clip_mut(clip_id) {
                clip.slip(delta_secs, max_source_out_secs);
                return;
            }
        }
    }

    /// Slides `clip_id` to `new_start_secs`, keeping its own duration/source range unchanged —
    /// Premiere/DaVinci/FCP's "Slide" tool (`ROADMAP.md` P2 item 11, the [`EditorTool::Slide`]
    /// toolbar mode): its immediate neighbors' in/out points adjust to absorb the move. A
    /// no-op if the clip isn't found or `new_start_secs` is negative.
    pub fn slide_clip(&mut self, clip_id: u64, new_start_secs: f64) {
        let previous_id = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .find_map(|t| t.previous_clip_id(clip_id));
        let prev_max_source_out_secs =
            previous_id.and_then(|id| self.clip_asset_max_source_out_secs(id));

        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if track.slide_clip(
                clip_id,
                new_start_secs.max(0.0),
                Self::MIN_TRIM_DURATION_SECS,
                prev_max_source_out_secs,
            ) {
                return;
            }
        }
    }

    /// Whether a clip is waiting in the clipboard for [`App::paste_clip_at_playhead`] — lets
    /// the timeline context menu grey out "Colar" instead of pasting nothing.
    pub fn has_clipboard_clip(&self) -> bool {
        self.clipboard_clip.is_some()
    }

    /// Copies `selected_clip_id` (and the track kind it's on) to [`App::clipboard_clip`] —
    /// what `Ctrl+C`/the timeline context menu's "Copiar" do. If the selected clip is a
    /// composite block member, every clip sharing its `composite_id` is captured too (not just
    /// the one clicked), so [`App::paste_clip_at_playhead`] can paste the whole block back as
    /// one unit — per `request.md`'s Fase 3 "reutilizado ... como se fosse um clipe só" spec.
    /// A no-op if nothing is selected.
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
                let selected = t.clips.iter().find(|c| c.id == clip_id)?;
                let group: Vec<_> = match selected.composite_id {
                    Some(composite_id) => t
                        .clips
                        .iter()
                        .filter(|c| c.composite_id == Some(composite_id))
                        .cloned()
                        .collect(),
                    None => vec![selected.clone()],
                };
                Some((group, t.kind))
            });
        if let Some(copied) = found {
            self.clipboard_clip = Some(copied);
        }
    }

    /// [`App::copy_selected_clip`] followed by [`App::delete_selected_clip`] — what
    /// `Ctrl+X`/the context menu's "Recortar" do.
    pub fn cut_selected_clip(&mut self) {
        self.copy_selected_clip();
        self.delete_selected_clip();
    }

    /// Pastes [`App::clipboard_clip`] as new, freshly-id'd clip(s) at the playhead's current
    /// position on the active sequence — what `Ctrl+V`/the context menu's "Colar" do. Lands on
    /// a matching-kind track the same way [`App::add_asset_to_timeline`] does (first existing
    /// track of that kind, auto-created if none exists); the earliest copied clip always lands
    /// exactly on the playhead, not wherever the context menu happened to be opened — a known
    /// simplification. A no-op if the clipboard is empty. Works across sequence tabs and even
    /// across projects, since `clipboard_clip` isn't scoped to either.
    ///
    /// A copied composite block (more than one clip in [`App::clipboard_clip`]) pastes back as
    /// one block: every other copied clip keeps its original offset relative to the earliest
    /// one, and all pasted clips share one fresh `composite_id` — a single copied clip (not a
    /// composite member) stays standalone, same as before.
    pub fn paste_clip_at_playhead(&mut self) {
        let Some((copied, kind)) = self.clipboard_clip.clone() else {
            return;
        };
        self.push_undo_snapshot();
        let playhead_secs = self.active_project().timeline().playhead_secs;
        let timeline = self.active_project_mut().timeline_mut();
        let track_index = resolve_or_create_track(timeline, kind, None);

        let group_min_start = copied
            .iter()
            .map(|c| c.start_secs)
            .fold(f64::INFINITY, f64::min);
        let new_composite_id = (copied.len() > 1).then(|| {
            timeline.tracks[track_index]
                .clips
                .iter()
                .filter_map(|c| c.composite_id)
                .max()
                .unwrap_or(0)
                + 1
        });

        for source in copied {
            let clip_id = next_clip_id(timeline);
            let start_secs = playhead_secs + (source.start_secs - group_min_start);
            timeline.tracks[track_index]
                .clips
                .push(avcore::timeline::ClipInstance {
                    id: clip_id,
                    asset_id: source.asset_id,
                    start_secs,
                    source_in_secs: source.source_in_secs,
                    source_out_secs: source.source_out_secs,
                    composite_id: new_composite_id,
                    gain_db: source.gain_db,
                    frozen: source.frozen,
                    speed_factor: source.speed_factor,
                    crop_x: source.crop_x,
                    crop_y: source.crop_y,
                    crop_w: source.crop_w,
                    crop_h: source.crop_h,
                    mask_shape: source.mask_shape,
                    mask_corner_radius: source.mask_corner_radius,
                    flipped_h: source.flipped_h,
                    color_filter: source.color_filter,
                    vignette_intensity: source.vignette_intensity,
                    brightness: source.brightness,
                    contrast: source.contrast,
                    saturation: source.saturation,
                    sharpen: source.sharpen,
                    chroma_key_enabled: source.chroma_key_enabled,
                    chroma_key_color: source.chroma_key_color,
                    chroma_key_tolerance: source.chroma_key_tolerance,
                    blur_intensity: source.blur_intensity,
                    shake_intensity: source.shake_intensity,
                    glitch_intensity: source.glitch_intensity,
                    pixelize_intensity: source.pixelize_intensity,
                    transition_in: source.transition_in,
                    transition_duration_secs: source.transition_duration_secs,
                    position_keyframes: source.position_keyframes,
                    scale_keyframes: source.scale_keyframes,
                    rotation_keyframes: source.rotation_keyframes,
                    opacity_keyframes: source.opacity_keyframes,
                    gain_keyframes: source.gain_keyframes,
                    brightness_keyframes: source.brightness_keyframes,
                    contrast_keyframes: source.contrast_keyframes,
                    saturation_keyframes: source.saturation_keyframes,
                    crop_x_keyframes: source.crop_x_keyframes,
                    crop_y_keyframes: source.crop_y_keyframes,
                    crop_w_keyframes: source.crop_w_keyframes,
                    crop_h_keyframes: source.crop_h_keyframes,
                    deflicker_enabled: source.deflicker_enabled,
                    lut_path: source.lut_path,
                    layer_scale_x: source.layer_scale_x,
                    layer_scale_y: source.layer_scale_y,
                    stabilization_intensity: source.stabilization_intensity,
                    // A pasted clip keeps the same source_in_secs/source_out_secs as the copied
                    // original, so a matte generated for that exact range (unlike a split's
                    // halves, whose ranges change) is still valid to carry over.
                    background_removal_enabled: source.background_removal_enabled,
                    background_removal_mask_path: source.background_removal_mask_path,
                });
        }
    }

    /// Adds/removes `clip_id` from [`App::multi_selected_clip_ids`] — what `Ctrl+click`ing
    /// a timeline clip does, building up a set of candidates for
    /// [`App::merge_into_composite`].
    pub fn toggle_multi_select(&mut self, clip_id: u64) {
        if !self.multi_selected_clip_ids.remove(&clip_id) {
            self.multi_selected_clip_ids.insert(clip_id);
        }
    }

    /// Merges every clip in [`App::multi_selected_clip_ids`] into one composite block — what
    /// the toolbar's "Mesclar em bloco composto" button and the timeline context menu's
    /// matching entry both do (per `request.md`'s Fase 3 "blocos compostos" spec, and its
    /// context-menu spec listing this among the actions it should offer too). Assigns them all
    /// a fresh `composite_id` and clears the
    /// multi-selection. A no-op, leaving the multi-selection untouched so the user can fix
    /// their pick, if fewer than two ids were selected or they aren't all on the same track —
    /// composite blocks don't span tracks yet.
    pub fn merge_into_composite(&mut self) {
        if self.multi_selected_clip_ids.len() < 2 {
            return;
        }
        self.push_undo_snapshot();
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
    /// [`App::paste_selected_clip_formatting`] — lets the timeline context menu grey out
    /// "Colar formatação" otherwise.
    pub fn has_formatting_clipboard(&self) -> bool {
        self.formatting_clipboard.is_some()
    }

    /// Copies `selected_clip_id`'s gain/freeze/speed/crop/mask/flip/color-filter/vignette/
    /// color-adjustment settings to [`App::formatting_clipboard`] — what `Ctrl+Shift+C`/the
    /// context menu's "Copiar formatação" do. A no-op if nothing is selected.
    pub fn copy_selected_clip_formatting(&mut self) {
        let Some(clip) = self.selected_clip() else {
            return;
        };
        self.formatting_clipboard = Some(clip.formatting());
    }

    /// Applies [`App::formatting_clipboard`] onto `selected_clip_id`, without touching any
    /// other field (position, trim, composite membership) — what `Ctrl+Shift+V`/the context
    /// menu's "Colar formatação" do. A no-op if nothing is selected or the clipboard is empty.
    pub fn paste_selected_clip_formatting(&mut self) {
        let Some(formatting) = self.formatting_clipboard.clone() else {
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
        self.push_undo_snapshot();
        let timeline = self.active_project_mut().timeline_mut();
        create_new_track(timeline, TrackKind::Video);
    }

    /// Toggles the `visible` flag on the track with `track_id` in the active sequence. A no-op
    /// if the track isn't found (shouldn't happen from the timeline UI, but safe regardless).
    pub fn toggle_track_visibility(&mut self, track_id: u64) {
        let timeline = self.active_project_mut().timeline_mut();
        if let Some(track) = timeline.tracks.iter_mut().find(|t| t.id == track_id) {
            track.visible = !track.visible;
        }
    }

    /// Sets the `AudioRole` on the track with `track_id` — what the timeline track header's
    /// role picker does (D2, `spec/architecture/differentiators.md`: highlight detection needs
    /// to know which track is the mic vs. game audio). Not undo-tracked, same as
    /// [`Self::toggle_track_visibility`] — metadata about a track, not an edit to its content.
    /// A no-op if the track isn't found.
    pub fn set_track_audio_role(&mut self, track_id: u64, role: avcore::AudioRole) {
        let timeline = self.active_project_mut().timeline_mut();
        if let Some(track) = timeline.tracks.iter_mut().find(|t| t.id == track_id) {
            track.audio_role = role;
        }
    }
}

/// Appends a brand-new, always-fresh empty track of `kind` to `timeline` — unlike
/// [`resolve_or_create_track`], this never reuses an existing track, since callers that need
/// this (layer templates applying several layers of the same kind, e.g. two video layers) need
/// each layer on its own track regardless of what's already there. Name is `V<n>`/`A<n>`/`T<n>`
/// where `n` is one past the count of existing tracks of that kind — so a timeline with one
/// video track "V1" gets a second track "V2". Returns the new track's index.
pub(super) fn create_new_track(
    timeline: &mut avcore::timeline::Timeline,
    kind: TrackKind,
) -> usize {
    let count = timeline.tracks.iter().filter(|t| t.kind == kind).count();
    let track_id = timeline.tracks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    let prefix = match kind {
        TrackKind::Video => "V",
        TrackKind::Audio => "A",
        TrackKind::Text => "T",
        TrackKind::Shape => "S",
    };
    timeline.tracks.push(avcore::timeline::Track {
        id: track_id,
        name: format!("{prefix}{}", count + 1),
        kind,
        clips: Vec::new(),
        text_clips: Vec::new(),
        shape_clips: Vec::new(),
        visible: true,
        audio_role: AudioRole::Unspecified,
    });
    timeline.tracks.len() - 1
}

/// Finds the track to place a new clip of `kind` on, for [`App::add_asset_to_timeline`] and
/// [`App::add_asset_to_timeline_at`]: `preferred_track_id` if it exists and matches `kind`,
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
        TrackKind::Shape => "S1",
    };
    timeline.tracks.push(avcore::timeline::Track {
        id: track_id,
        name: name.to_string(),
        kind,
        clips: Vec::new(),
        text_clips: Vec::new(),
        shape_clips: Vec::new(),
        visible: true,
        audio_role: AudioRole::Unspecified,
    });
    timeline.tracks.len() - 1
}

/// The next free clip id across every track in `timeline`, including text and shape clips — one
/// past the current max, `1` if the timeline has no clips yet. Covers [`ClipInstance`]s,
/// [`TextClip`]s, and [`ShapeClip`]s so their ids are globally unique within a timeline.
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
    let shape_max = timeline
        .tracks
        .iter()
        .flat_map(|t| &t.shape_clips)
        .map(|c| c.id)
        .max()
        .unwrap_or(0);
    video_audio_max.max(text_max).max(shape_max) + 1
}

impl App {
    /// Appends a new text track (`TrackKind::Text`) to the active sequence's timeline. The
    /// track is named using [`crate::i18n::Text::DefaultTextTrackName`]. A no-op if the
    /// project has no sequences.
    pub fn add_text_track(&mut self) {
        use crate::i18n::Text;
        self.push_undo_snapshot();
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
                shape_clips: Vec::new(),
                visible: true,
                audio_role: AudioRole::Unspecified,
            });
    }

    /// Appends a new [`TextClip`] to the first text track in the active sequence, starting at
    /// the current playhead position and lasting 3 seconds. Auto-creates the track when this is
    /// the first manually inserted text overlay and selects the new clip immediately.
    pub fn add_text_clip(&mut self) {
        use crate::i18n::Text;

        self.push_undo_snapshot();
        let playhead_secs = self.active_project().timeline().playhead_secs;
        let default_text = Text::DefaultTextContent.tr(self.locale).to_string();
        let timeline = self.active_project_mut().timeline_mut();
        let track_index = resolve_or_create_track(timeline, TrackKind::Text, None);
        let clip_id = next_clip_id(timeline);
        timeline.tracks[track_index].text_clips.push(TextClip {
            id: clip_id,
            start_secs: playhead_secs,
            duration_secs: 3.0,
            text: default_text,
            font_size: 48.0,
            font_family: Default::default(),
            font_style: Default::default(),
            color_rgba: [255, 255, 255, 255],
            background_rgba: [0, 0, 0, 0],
            background_padding: 8.0,
            background_corner_radius: 8.0,
            pos_x: 0.1,
            pos_y: 0.85,
            words: Vec::new(),
            highlight_enabled: false,
            highlight_color_rgba: [255, 220, 0, 255],
        });
        self.selected_clip_id = None;
        self.selected_shape_clip_id = None;
        self.selected_text_clip_id = Some(clip_id);
        self.invalidate_preview_rendering();
    }

    /// Appends a new shape track (`TrackKind::Shape`) to the active sequence's timeline. The
    /// track is named using [`crate::i18n::Text::DefaultShapeTrackName`]. Mirrors
    /// [`App::add_text_track`].
    pub fn add_shape_track(&mut self) {
        use crate::i18n::Text;
        self.push_undo_snapshot();
        let locale = self.locale;
        let track_id = {
            let timeline = self.active_project().timeline();
            timeline.tracks.iter().map(|t| t.id).max().unwrap_or(0) + 1
        };
        let name = Text::DefaultShapeTrackName.tr(locale).to_string();
        self.active_project_mut()
            .timeline_mut()
            .tracks
            .push(avcore::timeline::Track {
                id: track_id,
                name,
                kind: TrackKind::Shape,
                clips: Vec::new(),
                text_clips: Vec::new(),
                shape_clips: Vec::new(),
                visible: true,
                audio_role: AudioRole::Unspecified,
            });
    }

    /// Appends a new [`ShapeClip`] (a default rectangle, centered, half the canvas size) to the
    /// first shape track in the active sequence, starting at the current playhead position and
    /// lasting 3 seconds. Auto-creates a shape track if none exists yet, matching manual text
    /// insertion. Selects the new clip immediately so the properties panel shows its controls.
    pub fn add_shape_clip(&mut self) {
        self.push_undo_snapshot();
        let playhead_secs = self.active_project().timeline().playhead_secs;
        let timeline = self.active_project_mut().timeline_mut();
        let track_index = resolve_or_create_track(timeline, TrackKind::Shape, None);
        let clip_id = next_clip_id(timeline);
        timeline.tracks[track_index].shape_clips.push(ShapeClip {
            id: clip_id,
            start_secs: playhead_secs,
            duration_secs: 3.0,
            shape_kind: ShapeKind::rectangle(),
            center_x: 0.5,
            center_y: 0.5,
            width: 0.3,
            height: 0.3,
            rotation_deg: 0.0,
            color_rgba: [255, 255, 255, 255],
            stroke_thickness_px: 0.0,
        });
        self.selected_clip_id = None;
        self.selected_text_clip_id = None;
        self.selected_shape_clip_id = Some(clip_id);
    }

    /// Enters "draw a custom shape" mode — `request.md`'s Fase 4 "forma personalizada" ask,
    /// the one gap the fixed-preset shapes above don't cover. What the toolbar's "Desenhar
    /// forma" button does: clears any in-progress drawing and starts a fresh, empty point
    /// list. The preview panel (`screens::editor::layer_transform_preview`) reads
    /// [`App::drawing_shape_points`] each frame while it's `Some` and switches into a
    /// click-to-place-vertex mode instead of its usual layer drag/resize handling; see that
    /// function's doc comment for why a loaded preview frame is a precondition. Overwrites
    /// (does not append to) any drawing already in progress.
    pub fn start_drawing_custom_shape(&mut self) {
        self.drawing_shape_points = Some(Vec::new());
    }

    /// Discards the in-progress custom-shape drawing without creating a clip — what pressing
    /// Escape while drawing does.
    pub fn cancel_drawing_custom_shape(&mut self) {
        self.drawing_shape_points = None;
    }

    /// Appends one clicked point (canvas-fraction coordinates — same space as
    /// [`ShapeClip::center_x`]/`_y`, clamped to `0.0..=1.0`) to the in-progress custom shape.
    /// A no-op if [`App::start_drawing_custom_shape`] hasn't been called (or the drawing was
    /// already finished/cancelled) — lets the preview panel call this unconditionally on every
    /// click without checking the mode itself first.
    pub fn push_drawing_shape_point(&mut self, x: f32, y: f32) {
        if let Some(points) = self.drawing_shape_points.as_mut() {
            points.push((x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)));
        }
    }

    /// Finishes the in-progress custom-shape drawing — what pressing Enter with at least 3
    /// points placed does. A no-op (drawing mode stays active) if fewer than 3 points have
    /// been placed yet, since a polygon needs at least a triangle.
    ///
    /// The clicked points are absolute canvas-fraction coordinates; [`ShapeKind::Polygon`]
    /// stores vertices relative to the shape's own local unit square instead (see that
    /// variant's doc comment), so this derives a bounding box across all clicked points,
    /// centers/sizes the new [`ShapeClip`] on it, and re-expresses each point as an offset
    /// from that box's center divided by its width/height. `rotation_deg` starts at `0.0` (the
    /// shape is drawn axis-aligned to how it was clicked) — the properties panel's existing
    /// rotation control still applies afterward, same as any other shape.
    pub fn finish_drawing_custom_shape(&mut self) {
        let Some(points) = self.drawing_shape_points.as_ref() else {
            return;
        };
        if points.len() < 3 {
            return;
        }
        let points = self.drawing_shape_points.take().unwrap();
        self.push_undo_snapshot();

        let min_x = points.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
        let max_x = points.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
        let min_y = points.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        let max_y = points.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;
        // Floors the bounding box away from zero so near-collinear clicks (e.g. three points
        // almost in a vertical line) can't produce a divide-by-zero below.
        let width = (max_x - min_x).max(0.02);
        let height = (max_y - min_y).max(0.02);
        let local_vertices: Vec<(f32, f32)> = points
            .iter()
            .map(|&(x, y)| ((x - center_x) / width, (y - center_y) / height))
            .collect();

        let playhead_secs = self.active_project().timeline().playhead_secs;
        let timeline = self.active_project_mut().timeline_mut();
        let track_index = resolve_or_create_track(timeline, TrackKind::Shape, None);
        let clip_id = next_clip_id(timeline);
        timeline.tracks[track_index].shape_clips.push(ShapeClip {
            id: clip_id,
            start_secs: playhead_secs,
            duration_secs: 3.0,
            shape_kind: ShapeKind::Polygon(local_vertices),
            center_x,
            center_y,
            width,
            height,
            rotation_deg: 0.0,
            color_rgba: [255, 255, 255, 255],
            stroke_thickness_px: 0.0,
        });
        self.selected_clip_id = None;
        self.selected_text_clip_id = None;
        self.selected_shape_clip_id = Some(clip_id);
    }
}
