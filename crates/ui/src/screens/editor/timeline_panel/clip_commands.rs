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

pub(super) struct ClipCommands {
    pub(super) deletes: Vec<u64>,
    pub(super) color_labels: Vec<(u64, Option<[u8; 3]>)>,
    pub(super) detach_audio: Vec<u64>,
    pub(super) speed_ramps: Vec<(u64, f32, f32)>,
    pub(super) custom_speed_ramp: Option<u64>,
    pub(super) split_at_playhead: bool,
}

pub(super) fn apply_clip_commands(app: &mut App, commands: ClipCommands) {
    for clip_id in commands.deletes {
        app.selected_clip_id = Some(clip_id);
        app.delete_selected_clip();
    }
    for (clip_id, color_label) in commands.color_labels {
        app.set_clip_color_label(clip_id, color_label);
    }
    for clip_id in commands.detach_audio {
        app.selected_clip_id = Some(clip_id);
        app.detach_audio_from_selected_clip();
    }
    for (clip_id, start_speed, end_speed) in commands.speed_ramps {
        app.selected_clip_id = Some(clip_id);
        app.apply_speed_ramp_to_selected_clip(start_speed, end_speed, 4);
    }
    if let Some(clip_id) = commands.custom_speed_ramp {
        app.speed_ramp_dialog = Some((clip_id, 0.5, 2.0, "4".to_string(), false));
    }
    if commands.split_at_playhead {
        app.split_at_playhead();
    }
}
