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

use eframe::egui::{self, Color32, RichText, Ui};

use crate::theme;

/// A pill-shaped chip in an arbitrary foreground/background color pair. Prefer
/// [`tag_accent`]/[`tag_outline`]/[`tag_error`] for the palette's standard combinations.
pub fn tag(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(theme::RADIUS_PILL)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(fg).strong());
        });
}

/// A tag in the teal accent color — used for positive/active states (e.g. "Renderizando").
pub fn tag_accent(ui: &mut Ui, text: &str) {
    tag(ui, text, theme::ACCENT, theme::ACCENT_TINT);
}

/// A tag in a neutral outline color — used for passive states (e.g. "Na fila").
pub fn tag_outline(ui: &mut Ui, text: &str) {
    tag(ui, text, theme::TEXT_SECONDARY, theme::SURFACE_2);
}

/// A tag in the error color — used for failure states (e.g. "Falhou").
pub fn tag_error(ui: &mut Ui, text: &str) {
    tag(ui, text, theme::ERROR, theme::ERROR_TINT);
}

/// A tag in the warning color — used for caution/non-terminal states (e.g. "Pausado").
pub fn tag_warning(ui: &mut Ui, text: &str) {
    tag(ui, text, theme::WARNING, theme::WARNING_TINT);
}
