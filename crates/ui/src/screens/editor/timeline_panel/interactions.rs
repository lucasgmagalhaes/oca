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

use eframe::egui;

use crate::app::{App, EditorTool};

use super::snap::ClipDrag;
use super::TrimEdge;

/// Applies the deferred trim and drag commands collected while rendering timeline rows.
pub(super) fn apply_clip_interactions(
    app: &mut App,
    trim_requests: Vec<(u64, TrimEdge)>,
    text_trim_requests: Vec<(u64, TrimEdge)>,
    shape_trim_requests: Vec<(u64, TrimEdge)>,
    text_clip_drags: Vec<(u64, f64)>,
    shape_clip_drags: Vec<(u64, f64)>,
    clip_drags: Vec<ClipDrag>,
    track_rows: &[(u64, avcore::timeline::TrackKind, egui::Rect)],
) {
    for (clip_id, edge) in trim_requests {
        match (app.tool, edge) {
            (EditorTool::Ripple, TrimEdge::Start(secs)) => {
                app.ripple_trim_clip_start(clip_id, secs)
            }
            (EditorTool::Ripple, TrimEdge::End(secs)) => app.ripple_trim_clip_end(clip_id, secs),
            (EditorTool::Roll, TrimEdge::Start(secs)) => {
                app.roll_edit_from_start_edge(clip_id, secs)
            }
            (EditorTool::Roll, TrimEdge::End(secs)) => app.roll_edit_clip(clip_id, secs),
            (_, TrimEdge::Start(secs)) => app.trim_clip_start(clip_id, secs),
            (_, TrimEdge::End(secs)) => app.trim_clip_end(clip_id, secs),
        }
    }
    for (clip_id, edge) in text_trim_requests {
        match edge {
            TrimEdge::Start(secs) => app.trim_text_clip_start(clip_id, secs),
            TrimEdge::End(secs) => app.trim_text_clip_end(clip_id, secs),
        }
    }
    for (clip_id, edge) in shape_trim_requests {
        match edge {
            TrimEdge::Start(secs) => app.trim_shape_clip_start(clip_id, secs),
            TrimEdge::End(secs) => app.trim_shape_clip_end(clip_id, secs),
        }
    }
    for (clip_id, new_start_secs) in text_clip_drags {
        app.move_text_clip(clip_id, new_start_secs);
    }
    for (clip_id, new_start_secs) in shape_clip_drags {
        app.move_shape_clip(clip_id, new_start_secs);
    }
    for drag in clip_drags {
        if app.tool == EditorTool::Slip {
            let old_start_secs = app
                .active_project()
                .timeline()
                .tracks
                .iter()
                .flat_map(|track| &track.clips)
                .find(|clip| clip.id == drag.clip_id)
                .map(|clip| clip.start_secs);
            if let Some(old_start_secs) = old_start_secs {
                app.slip_clip(drag.clip_id, drag.new_start_secs - old_start_secs);
            }
            continue;
        }
        if app.tool == EditorTool::Slide {
            app.slide_clip(drag.clip_id, drag.new_start_secs);
            continue;
        }
        let target_track_id = track_rows
            .iter()
            .find(|(_, kind, rect)| *kind == drag.kind && rect.y_range().contains(drag.pointer_y))
            .map(|(id, _, _)| *id);
        let is_composite = app
            .active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|track| &track.clips)
            .find(|clip| clip.id == drag.clip_id)
            .is_some_and(|clip| clip.composite_id.is_some());
        match target_track_id {
            Some(track_id) if track_id != drag.source_track_id && !is_composite => {
                app.move_clip_to_track(drag.clip_id, track_id, drag.new_start_secs);
            }
            _ => app.move_clip_with_group(drag.clip_id, drag.new_start_secs),
        }
    }
}
