use eframe::egui::{self, RichText};

use crate::app::{OcaApp, Screen};
use crate::i18n::{self, Text};
use crate::theme;

/// Renders the top breadcrumb bar: app name, current screen title, and (in the Editor) the
/// active project's name with an "unsaved changes" dot.
pub fn show(app: &mut OcaApp, ui: &mut egui::Ui) {
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
                ui.label(
                    RichText::new(Text::AppName.tr(app.locale))
                        .size(13.0)
                        .color(theme::TEXT_MUTED),
                );
                ui.label(RichText::new("›").color(theme::TEXT_MUTED));
                ui.label(
                    RichText::new(i18n::screen_title(app.locale, app.screen))
                        .size(13.0)
                        .strong(),
                );

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
                        .on_hover_text(Text::UnsavedChanges.tr(app.locale));
                }
            });
        });
}
