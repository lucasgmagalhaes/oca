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

//! CF-05 (`spec/architecture/competitive-feature-plan.md`): a thin `App` wrapper around
//! `avcore::interchange`/`avcore::interchange::otio_json` so the Editor menu bar's "Export
//! OpenTimelineIO (.otio)..." action has something to call once its own `rfd::FileDialog` picks
//! a path — same shape [`crate::app::collab_bundle`] already established for the collaboration
//! bundle export button (build the real payload in `core`, this module only turns the result
//! into a toast or a written file).

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
}
