use eframe::egui::{self, Color32, RichText, Ui};

use crate::theme;

pub fn section_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .size(11.0)
            .color(theme::TEXT_MUTED)
            .strong(),
    );
    ui.add_space(6.0);
}

pub fn tag(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(200)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.0).color(fg).strong());
        });
}

pub fn tag_accent(ui: &mut Ui, text: &str) {
    tag(ui, text, theme::ACCENT, theme::ACCENT_TINT);
}

pub fn tag_outline(ui: &mut Ui, text: &str) {
    tag(ui, text, theme::TEXT_SECONDARY, theme::SURFACE_2);
}

pub fn tag_error(ui: &mut Ui, text: &str) {
    tag(ui, text, theme::ERROR, theme::ERROR_TINT);
}

pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::SURFACE)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(10)
        .inner_margin(egui::Margin::same(14))
}
