// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use super::App;

impl App {
    /// Minimum clip duration a drag-trim is allowed to shrink a clip to — small enough to feel
    /// unrestrictive, large enough that a clip can't accidentally get dragged down to
    /// (near-)zero length.
    const MIN_TRIM_DURATION_SECS: f64 = 0.1;

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

    /// Repositions the text clip with `text_clip_id` to `new_start_secs` — what dragging a
    /// text-overlay block's body does on the timeline. Unlike [`App::move_clip`], a text clip
    /// has no source asset to bound it against, so this is a plain reposition with no drop-below-
    /// zero. A no-op if the clip isn't found.
    pub fn move_text_clip(&mut self, text_clip_id: u64, new_start_secs: f64) {
        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if let Some(tc) = track.text_clip_mut(text_clip_id) {
                tc.start_secs = new_start_secs.max(0.0);
                return;
            }
        }
    }

    /// Drags the text clip's left edge to `new_start_secs`, keeping its right edge fixed —
    /// [`App::trim_clip_start`]'s counterpart for [`avcore::timeline::TextClip`]. A no-op if the
    /// clip isn't found; bounded so the clip never shrinks below
    /// [`App::MIN_TRIM_DURATION_SECS`] or starts before zero.
    pub fn trim_text_clip_start(&mut self, text_clip_id: u64, new_start_secs: f64) {
        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if let Some(tc) = track.text_clip_mut(text_clip_id) {
                let end_secs = tc.start_secs + tc.duration_secs;
                let new_start_secs = new_start_secs
                    .max(0.0)
                    .min(end_secs - Self::MIN_TRIM_DURATION_SECS);
                tc.duration_secs = end_secs - new_start_secs;
                tc.start_secs = new_start_secs;
                return;
            }
        }
    }

    /// Drags the text clip's right edge to `new_end_secs`, keeping its start fixed —
    /// [`App::trim_clip_end`]'s counterpart for [`avcore::timeline::TextClip`]. A no-op if the
    /// clip isn't found; bounded so the clip never shrinks below
    /// [`App::MIN_TRIM_DURATION_SECS`].
    pub fn trim_text_clip_end(&mut self, text_clip_id: u64, new_end_secs: f64) {
        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if let Some(tc) = track.text_clip_mut(text_clip_id) {
                tc.duration_secs = (new_end_secs - tc.start_secs).max(Self::MIN_TRIM_DURATION_SECS);
                return;
            }
        }
    }

    /// [`App::move_text_clip`]'s counterpart for [`avcore::timeline::ShapeClip`].
    pub fn move_shape_clip(&mut self, shape_clip_id: u64, new_start_secs: f64) {
        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if let Some(sc) = track.shape_clip_mut(shape_clip_id) {
                sc.start_secs = new_start_secs.max(0.0);
                return;
            }
        }
    }

    /// [`App::trim_text_clip_start`]'s counterpart for [`avcore::timeline::ShapeClip`].
    pub fn trim_shape_clip_start(&mut self, shape_clip_id: u64, new_start_secs: f64) {
        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if let Some(sc) = track.shape_clip_mut(shape_clip_id) {
                let end_secs = sc.start_secs + sc.duration_secs;
                let new_start_secs = new_start_secs
                    .max(0.0)
                    .min(end_secs - Self::MIN_TRIM_DURATION_SECS);
                sc.duration_secs = end_secs - new_start_secs;
                sc.start_secs = new_start_secs;
                return;
            }
        }
    }

    /// [`App::trim_text_clip_end`]'s counterpart for [`avcore::timeline::ShapeClip`].
    pub fn trim_shape_clip_end(&mut self, shape_clip_id: u64, new_end_secs: f64) {
        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if let Some(sc) = track.shape_clip_mut(shape_clip_id) {
                sc.duration_secs = (new_end_secs - sc.start_secs).max(Self::MIN_TRIM_DURATION_SECS);
                return;
            }
        }
    }
}
