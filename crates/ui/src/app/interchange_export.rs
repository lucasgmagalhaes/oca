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

//! CF-05 (`spec/architecture/competitive-feature-plan.md`): thin `App` wrappers around
//! `avcore::interchange`/`avcore::interchange::otio_json` so the Editor menu bar's "Export
//! OpenTimelineIO (.otio)..." and "Import OpenTimelineIO (.otio)..." actions have something to
//! call once their own `rfd::FileDialog` picks a path — same shape [`crate::app::collab_bundle`]
//! already established for the collaboration bundle export/import buttons (build the real
//! payload in `core`, this module only turns the result into a toast, a written file, or a new
//! sequence tab).

use std::path::PathBuf;

use crate::i18n::Text;

use super::App;

impl App {
    /// Writes the active sequence's editorial subset (track order, clip source ranges, gaps,
    /// markers, transition kind, speed) as a real OpenTimelineIO `.otio` JSON document at
    /// `output_path` — what the Editor menu bar's "Export OpenTimelineIO (.otio)..." action does
    /// once its save-file dialog picks a destination. See `avcore::interchange::otio_json`'s own
    /// doc comment for exactly which schema versions are written and why; effects/color grading/
    /// masks/keyframe animation don't round-trip through this format at all (a fact this app
    /// tells the user via `avcore::interchange_compatibility_report` before export in a later
    /// pass — this method itself doesn't gate on it, matching `export_collab_bundle`'s own "no
    /// export precondition beyond a save path" shape).
    pub fn export_otio_for_active_sequence(&mut self, output_path: PathBuf) {
        let project = self.active_project().clone();
        let sequence = project.active_sequence().clone();
        let interchange = avcore::interchange::sequence_to_interchange(&sequence, &project);
        let json =
            avcore::interchange::otio_json::serialize_interchange_timeline_pretty(&interchange);
        match std::fs::write(&output_path, json) {
            Ok(()) => {
                self.push_toast(Text::OtioExported.tr(self.locale).to_string());
            }
            Err(e) => self.push_toast(format!("Failed to export OpenTimelineIO file: {e}")),
        }
    }

    /// Reads a real `.otio` file at `input_path` and imports its supported editorial subset
    /// (track order, clip source ranges, gaps, markers, transition kind, speed — see
    /// `avcore::interchange::otio_json`'s own doc comment for exactly what's recognized versus
    /// approximated-with-a-warning) into a brand-new sequence tab in the active project, the
    /// exact shape `App::add_sequence`'s own "append and switch to it" already established —
    /// never overwrites an existing sequence. Every clip's media reference is resolved against
    /// the *active project's* own media library
    /// (`avcore::interchange::interchange_to_timeline`'s own contract); a clip whose source
    /// media isn't already imported into this project is skipped and counted as a warning
    /// rather than linked to the wrong asset. What the Editor menu bar's "Import OpenTimelineIO
    /// (.otio)..." action does once its open-file dialog picks a source file.
    pub fn import_otio_into_new_sequence(&mut self, input_path: std::path::PathBuf) {
        let bytes = match std::fs::read_to_string(&input_path) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.push_toast(format!("Failed to read {}: {e}", input_path.display()));
                return;
            }
        };
        let json: serde_json::Value = match serde_json::from_str(&bytes) {
            Ok(json) => json,
            Err(e) => {
                self.push_toast(format!("Not a valid JSON file: {e}"));
                return;
            }
        };
        let import = match avcore::interchange::otio_json::parse_otio_json(&json) {
            Ok(import) => import,
            Err(e) => {
                self.push_toast(format!("Not a recognizable OpenTimelineIO document: {e}"));
                return;
            }
        };

        let sequence_name = if import.timeline.name.trim().is_empty() {
            input_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Imported".to_string())
        } else {
            import.timeline.name.clone()
        };

        let project = self.active_project_mut();
        project.new_sequence(sequence_name);
        // Fresh sequence, fresh timeline -- ids are scoped to their owning sequence (see
        // `Project::duplicate_sequence`'s own doc comment), so starting over from 1 here can
        // never collide with any other sequence's own track/clip/marker ids.
        let mut next_id = 1u64;
        let placement =
            avcore::interchange::interchange_to_timeline(&import.timeline, project, &mut next_id);
        project.active_sequence_mut().timeline = placement.timeline;
        self.reset_sequence_context();

        let warning_count = import.warnings.len() + placement.warnings.len();
        if warning_count == 0 {
            self.push_toast(Text::OtioImported.tr(self.locale).to_string());
        } else {
            self.push_toast(
                Text::OtioImportedWithWarnings
                    .tr(self.locale)
                    .replace("{n}", &warning_count.to_string()),
            );
        }
    }
}
