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

use eframe::egui::{self, Ui};

/// A dropdown over a fixed, known set of `Copy + PartialEq` enum values (mask shape, color
/// filter, transition type, ...), each rendered via `label`. Returns whether the selection
/// changed.
pub fn enum_combo<T: Copy + PartialEq>(
    ui: &mut Ui,
    id_salt: &str,
    options: &[T],
    selected: &mut T,
    label: impl Fn(T) -> String,
) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(label(*selected))
        .show_ui(ui, |ui| {
            for &option in options {
                changed |= ui
                    .selectable_value(selected, option, label(option))
                    .changed();
            }
        });
    changed
}
