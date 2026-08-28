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

//! P4 item 22, "Smart bins" (`spec/ROADMAP.md`): rule-based media-pool folders in the Editor's
//! Library panel. See [`avcore::SmartBin`] for the filter model itself — this module is just the
//! `App`-level create/edit/select/delete plumbing and [`App::show_smart_bin_modal`].

use avcore::SmartBin;

use super::App;

impl App {
    /// Opens the smart-bin modal with a fresh, unnamed draft (`id == 0`) — what the Library
    /// panel's "+ New Bin" button does.
    pub fn begin_new_smart_bin(&mut self) {
        self.editing_smart_bin = Some(SmartBin::new(0, String::new()));
    }

    /// Opens the smart-bin modal pre-filled with `bin_id`'s current rules, for in-place editing
    /// — a no-op if `bin_id` doesn't exist.
    pub fn begin_edit_smart_bin(&mut self, bin_id: u64) {
        let Some(bin) = self
            .active_project()
            .smart_bins
            .iter()
            .find(|b| b.id == bin_id)
        else {
            return;
        };
        self.editing_smart_bin = Some(bin.clone());
    }

    /// Commits `editing_smart_bin` (a no-op if the draft's name is blank, or if `None` — nothing
    /// to commit): creates a new bin for an `id == 0` draft and switches the Library panel's
    /// active filter to it, or overwrites the existing bin's rules in place otherwise. Either
    /// way, closes the modal.
    pub fn commit_smart_bin_draft(&mut self) {
        let Some(mut draft) = self.editing_smart_bin.take() else {
            return;
        };
        draft.name = draft.name.trim().to_string();
        if draft.name.is_empty() {
            return;
        }

        if draft.id == 0 {
            let project = self.active_project_mut();
            let id = project.add_smart_bin(draft.name);
            if let Some(bin) = project.smart_bin_mut(id) {
                bin.kind_filter = draft.kind_filter;
                bin.name_contains = draft.name_contains;
                bin.requires_audio = draft.requires_audio;
            }
            self.active_smart_bin_id = Some(id);
        } else if let Some(bin) = self.active_project_mut().smart_bin_mut(draft.id) {
            *bin = draft;
        }
    }

    /// Discards `editing_smart_bin` without saving — Escape/Cancel on the modal.
    pub fn cancel_smart_bin_draft(&mut self) {
        self.editing_smart_bin = None;
    }

    /// Removes `bin_id` and clears the Library panel's active filter if it was the one removed
    /// (a deleted bin can't stay "selected").
    pub fn delete_smart_bin(&mut self, bin_id: u64) {
        self.active_project_mut().remove_smart_bin(bin_id);
        if self.active_smart_bin_id == Some(bin_id) {
            self.active_smart_bin_id = None;
        }
    }
}
