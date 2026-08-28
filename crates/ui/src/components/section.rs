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

use crate::theme;

/// An uppercase, muted section header (e.g. "BIBLIOTECA DE MÍDIA").
pub fn section_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .size(11.0)
            .color(theme::TEXT_MUTED)
            .strong(),
    );
    ui.add_space(theme::SPACE_SM);
}

/// A top-level screen title (Library/Sound Library/Prefs/Queue's "Mídia"/"Sons"/... header) —
/// the de facto standard this codebase already converged on independently at every one of those
/// call sites before this helper existed; naming it stops a fifth screen from picking its own
/// literal.
pub fn page_title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(20.0).strong());
}

/// A modal dialog's title — the de facto standard across every `egui::Modal` in `app/modals.rs`
/// and `app/transcript_panel.rs` before this helper existed.
pub fn modal_title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(15.0).strong());
}
