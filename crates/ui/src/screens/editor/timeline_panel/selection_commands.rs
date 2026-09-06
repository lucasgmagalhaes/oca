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

use crate::app::App;

/// Commands that change the active timeline selection or operate on selected clips.
pub(super) struct SelectionCommands {
    pub(super) clicked_clip_id: Option<u64>,
    pub(super) clicked_text_clip_id: Option<u64>,
    pub(super) clicked_shape_clip_id: Option<u64>,
    pub(super) delete_text_clip_requests: Vec<u64>,
    pub(super) delete_shape_clip_requests: Vec<u64>,
    pub(super) multi_select_requests: Vec<u64>,
    pub(super) copy_requests: Vec<u64>,
    pub(super) cut_requests: Vec<u64>,
    pub(super) copy_formatting_requests: Vec<u64>,
    pub(super) paste_formatting_requests: Vec<u64>,
    pub(super) paste_requested: bool,
    pub(super) merge_into_composite_requested: bool,
    pub(super) create_compound_clip_requested: bool,
    pub(super) open_nested_sequence_request: Option<u64>,
}

pub(super) fn apply_selection_commands(app: &mut App, commands: SelectionCommands) {
    if let Some(id) = commands.clicked_clip_id {
        app.selected_text_clip_id = None;
        app.selected_shape_clip_id = None;
        app.select_timeline_clip(id);
    }
    if let Some(id) = commands.clicked_text_clip_id {
        app.selected_clip_id = None;
        app.selected_shape_clip_id = None;
        app.selected_text_clip_id = Some(id);
    }
    if let Some(id) = commands.clicked_shape_clip_id {
        app.selected_clip_id = None;
        app.selected_text_clip_id = None;
        app.selected_shape_clip_id = Some(id);
    }
    for text_clip_id in commands.delete_text_clip_requests {
        let timeline = app.active_project_mut().timeline_mut();
        for track in &mut timeline.tracks {
            if track.kind == avcore::timeline::TrackKind::Text {
                track.text_clips.retain(|clip| clip.id != text_clip_id);
            }
        }
        if app.selected_text_clip_id == Some(text_clip_id) {
            app.selected_text_clip_id = None;
        }
    }
    for shape_clip_id in commands.delete_shape_clip_requests {
        let timeline = app.active_project_mut().timeline_mut();
        for track in &mut timeline.tracks {
            if track.kind == avcore::timeline::TrackKind::Shape {
                track.shape_clips.retain(|clip| clip.id != shape_clip_id);
            }
        }
        if app.selected_shape_clip_id == Some(shape_clip_id) {
            app.selected_shape_clip_id = None;
        }
    }
    for clip_id in commands.multi_select_requests {
        app.toggle_multi_select(clip_id);
    }
    for clip_id in commands.copy_requests {
        app.selected_clip_id = Some(clip_id);
        app.copy_selected_clip();
    }
    for clip_id in commands.cut_requests {
        app.selected_clip_id = Some(clip_id);
        app.cut_selected_clip();
    }
    for clip_id in commands.copy_formatting_requests {
        app.selected_clip_id = Some(clip_id);
        app.copy_selected_clip_formatting();
    }
    for clip_id in commands.paste_formatting_requests {
        app.selected_clip_id = Some(clip_id);
        app.paste_selected_clip_formatting();
    }
    if commands.paste_requested {
        app.paste_clip_at_playhead();
    }
    if commands.merge_into_composite_requested {
        app.merge_into_composite();
    }
    if commands.create_compound_clip_requested {
        app.create_compound_clip_from_selected_clip();
    }
    if let Some(nested_id) = commands.open_nested_sequence_request {
        app.open_nested_sequence(nested_id);
    }
}
