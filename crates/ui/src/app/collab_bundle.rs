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

//! D7 (`spec/architecture/differentiators.md`): thin `App` wrappers around
//! `avcore::collab_bundle` so the Editor toolbar's "Export Collaboration Bundle..." and the
//! Início screen's "Import Collaboration Bundle..." actions have something to call once their
//! own `rfd::FileDialog` picks a path — the actual archive I/O lives entirely in `core`, this
//! module only owns turning its `Result` into a toast (or, for import, a newly opened project).

use std::path::PathBuf;

use crate::i18n::Text;

use super::App;

impl App {
    /// Packages the active project's `.ocproj` snapshot plus its already-generated editing
    /// proxies into a portable handoff bundle at `output_zip_path` — what the Editor toolbar's
    /// "Export Collaboration Bundle..." button does once its save-file dialog picks a
    /// destination.
    pub fn export_collab_bundle(&mut self, output_zip_path: PathBuf) {
        match avcore::export_collab_bundle(self.active_project(), &output_zip_path) {
            Ok(()) => {
                self.push_toast(Text::CollabBundleExported.tr(self.locale).to_string());
            }
            Err(e) => self.push_toast(format!("Failed to export collaboration bundle: {e}")),
        }
    }

    /// Unpacks a bundle written by [`App::export_collab_bundle`] into `dest_project_path` and
    /// opens the resulting project — what the Início screen's "Import Collaboration Bundle..."
    /// does once its two file dialogs (pick the `.zip`, then pick a destination folder) resolve.
    pub fn import_collab_bundle(&mut self, zip_path: PathBuf, dest_project_path: PathBuf) {
        match avcore::import_collab_bundle(&zip_path, &dest_project_path) {
            Ok(project) => {
                self.add_and_open_project(project);
                self.check_autosave_on_open(&dest_project_path);
            }
            Err(e) => self.push_toast(format!("Failed to import collaboration bundle: {e}")),
        }
    }
}
