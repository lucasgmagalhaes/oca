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

/// Deferred commands emitted by track headers and their context menus.
pub(super) struct TrackCommands {
    pub(super) toggle_visibility: Vec<u64>,
    pub(super) toggle_lock: Vec<u64>,
    pub(super) toggle_collapsed: Vec<u64>,
    pub(super) audio_roles: Vec<(u64, avcore::AudioRole)>,
    pub(super) color_labels: Vec<(u64, Option<[u8; 3]>)>,
    pub(super) renames: Vec<(u64, String)>,
    pub(super) duplicates: Vec<u64>,
    pub(super) move_up: Vec<u64>,
    pub(super) move_down: Vec<u64>,
    pub(super) razor_splits: Vec<(u64, f64)>,
    pub(super) deletes: Vec<(u64, String)>,
}

pub(super) fn apply_track_commands(app: &mut App, commands: TrackCommands) {
    for track_id in commands.toggle_visibility {
        app.toggle_track_visibility(track_id);
    }
    for track_id in commands.toggle_lock {
        app.toggle_track_locked(track_id);
    }
    for track_id in commands.toggle_collapsed {
        if !app.collapsed_track_ids.remove(&track_id) {
            app.collapsed_track_ids.insert(track_id);
        }
    }
    for (track_id, role) in commands.audio_roles {
        app.set_track_audio_role(track_id, role);
    }
    for (track_id, color_label) in commands.color_labels {
        app.set_track_color_label(track_id, color_label);
    }
    for (track_id, name) in commands.renames {
        app.renaming_track = Some((track_id, name));
    }
    for track_id in commands.duplicates {
        app.duplicate_track(track_id);
    }
    for track_id in commands.move_up {
        app.move_track_up(track_id);
    }
    for track_id in commands.move_down {
        app.move_track_down(track_id);
    }
    for (track_id, at_secs) in commands.razor_splits {
        app.split_track_clip_at(track_id, at_secs);
    }
    for (track_id, name) in commands.deletes {
        let has_content = app.active_project().timeline().tracks.iter().any(|track| {
            track.id == track_id
                && (!track.clips.is_empty()
                    || !track.text_clips.is_empty()
                    || !track.shape_clips.is_empty())
        });
        if has_content {
            app.deleting_track = Some((track_id, name));
        } else {
            app.delete_track(track_id);
        }
    }
}
