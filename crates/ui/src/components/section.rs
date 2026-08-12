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
    ui.add_space(6.0);
}
