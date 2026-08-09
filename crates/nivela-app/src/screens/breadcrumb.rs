use eframe::egui::{self, RichText};

use crate::app::{NivelaApp, Screen};
use crate::theme;

pub fn show(app: &mut NivelaApp, ui: &mut egui::Ui) {
    egui::Panel::top("breadcrumb")
        .exact_size(46.0)
        .frame(
            egui::Frame::new()
                .fill(theme::BG)
                .stroke(egui::Stroke::new(1.0, theme::BORDER))
                .inner_margin(egui::Margin::symmetric(18, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(RichText::new("NivelaEditor").size(13.0).color(theme::TEXT_MUTED));
                ui.label(RichText::new("›").color(theme::TEXT_MUTED));
                ui.label(RichText::new(app.screen.title()).size(13.0).strong());

                if app.screen == Screen::Editor {
                    ui.label(RichText::new("›").color(theme::TEXT_MUTED));
                    ui.label(
                        RichText::new(&app.active_project().name)
                            .size(13.0)
                            .color(theme::TEXT_SECONDARY),
                    );
                    ui.add_space(2.0);
                    egui::Frame::new()
                        .fill(theme::ACCENT)
                        .corner_radius(200)
                        .show(ui, |ui| {
                            ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                        })
                        .response
                        .on_hover_text("Alterações não salvas");
                }
            });
        });
}
