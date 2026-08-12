use eframe::egui;

use crate::theme;

/// The standard bordered/rounded card background used for project cards, media cards, and
/// export queue rows.
pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::SURFACE)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(10)
        .inner_margin(egui::Margin::same(14))
}
