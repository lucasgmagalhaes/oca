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

use eframe::egui::{RichText, Ui};

use crate::components::section::section_label;
use crate::theme;

/// The separator/spacing/note shell every clip-properties block shares: a leading `separator()`
/// and spacing, the caller's `content`, then a trailing muted export-status note. Returns
/// whatever `content` reports (typically "did this block's value change").
pub fn property_block(ui: &mut Ui, note: &str, content: impl FnOnce(&mut Ui) -> bool) -> bool {
    ui.add_space(theme::SPACE_SM);
    ui.separator();
    ui.add_space(theme::SPACE_SM);
    let changed = content(ui);
    ui.add_space(theme::SPACE_XS);
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

/// A muted field label with no separator/note ceremony — the lighter sibling of
/// [`property_section`] for a dense list of simple fields (shape/text clip properties) where a
/// separator+export-note per field would be excessive. Caller draws its own widget immediately
/// after calling this, matching the shape every existing hand-rolled label+widget pair already
/// used, just named instead of duplicated inline at each call site.
pub fn property_row(ui: &mut Ui, label: &str) {
    ui.label(RichText::new(label).size(12.0).color(theme::TEXT_MUTED));
}
