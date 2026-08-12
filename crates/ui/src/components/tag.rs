use eframe::egui::{self, Color32, RichText, Ui};

use crate::theme;

/// A pill-shaped chip in an arbitrary foreground/background color pair. Prefer
/// [`tag_accent`]/[`tag_outline`]/[`tag_error`] for the palette's standard combinations.
pub fn tag(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(200)
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
