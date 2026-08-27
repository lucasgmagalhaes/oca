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

use avcore::MarkerKind;

use super::App;

impl App {
    /// Opens/closes the Timeline Index panel — what the toolbar's "🏷" button does.
    pub fn toggle_timeline_index(&mut self) {
        self.timeline_index_open = !self.timeline_index_open;
    }

    /// Adds a new marker of `kind` at the active sequence's current playhead position — what
    /// the Timeline Index panel's "Add" buttons do. Returns the new marker's id.
    pub fn add_marker_at_playhead(&mut self, kind: MarkerKind) -> u64 {
        self.push_undo_snapshot();
        let playhead = self.active_project().timeline().playhead_secs;
        self.active_project_mut()
            .timeline_mut()
            .add_marker(playhead, kind)
    }

    /// Removes `marker_id`, if it exists — what the Timeline Index panel's trash-can button
    /// does.
    pub fn remove_marker(&mut self, marker_id: u64) {
        self.push_undo_snapshot();
        self.active_project_mut()
            .timeline_mut()
            .remove_marker(marker_id);
    }

    /// Sets `marker_id`'s label, if it exists — what typing into the Timeline Index panel's
    /// inline text field does. Uses [`App::push_undo_snapshot_for_drag`]'s coalescing, same as
    /// every other text-field write-back in this codebase, so each keystroke doesn't become its
    /// own undo step.
    pub fn set_marker_label(&mut self, marker_id: u64, label: String) {
        self.push_undo_snapshot_for_drag();
        if let Some(marker) = self
            .active_project_mut()
            .timeline_mut()
            .marker_mut(marker_id)
        {
            marker.label = label;
        }
    }

    /// Sets `marker_id`'s [`MarkerKind`], if it exists.
    pub fn set_marker_kind(&mut self, marker_id: u64, kind: MarkerKind) {
        self.push_undo_snapshot();
        if let Some(marker) = self
            .active_project_mut()
            .timeline_mut()
            .marker_mut(marker_id)
        {
            marker.kind = kind;
        }
    }

    /// Flips `marker_id`'s `completed` flag, if it exists — only meaningful for a
    /// [`MarkerKind::ToDo`] marker, but harmless (if pointless) to call on any other kind.
    pub fn toggle_marker_completed(&mut self, marker_id: u64) {
        self.push_undo_snapshot();
        if let Some(marker) = self
            .active_project_mut()
            .timeline_mut()
            .marker_mut(marker_id)
        {
            marker.completed = !marker.completed;
        }
    }
}
