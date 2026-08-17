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

use eframe::egui::{RichText, Ui};

use crate::components::section::section_label;
use crate::theme;

/// The separator/spacing/note shell every clip-properties block shares: a leading `separator()`
/// and spacing, the caller's `content`, then a trailing muted export-status note. Returns
/// whatever `content` reports (typically "did this block's value change").
pub fn property_block(ui: &mut Ui, note: &str, content: impl FnOnce(&mut Ui) -> bool) -> bool {
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(6.0);
    let changed = content(ui);
    ui.add_space(4.0);
    ui.label(RichText::new(note).size(10.5).color(theme::TEXT_MUTED));
    changed
}

/// A [`property_block`] headed by a [`section_label`] (e.g. "GANHO", a slider, then the export
/// note) — the shape most clip-properties controls (sliders, combos) share.
pub fn property_section(
    ui: &mut Ui,
    label: &str,
    note: &str,
    content: impl FnOnce(&mut Ui) -> bool,
) -> bool {
    property_block(ui, note, |ui| {
        section_label(ui, label);
        content(ui)
    })
}

/// A [`property_block`] whose own header is a checkbox (e.g. "Congelar frame") rather than a
/// separate [`section_label`].
pub fn property_toggle(ui: &mut Ui, label: &str, note: &str, checked: &mut bool) -> bool {
    property_block(ui, note, |ui| ui.checkbox(checked, label).changed())
}
