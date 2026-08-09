use eframe::egui::{self, Color32, RichText};

use crate::app::{NivelaApp, Screen};
use crate::i18n;
use crate::theme;

const ITEMS: [(Screen, &str); 5] = [
    (Screen::Home, "⌂"),
    (Screen::Editor, "✂"),
    (Screen::Library, "▤"),
    (Screen::Queue, "≡"),
    (Screen::Prefs, "⚙"),
];

/// Renders the left icon rail and handles screen-switching clicks.
pub fn show(app: &mut NivelaApp, ui: &mut egui::Ui) {
    egui::Panel::left("nav_rail")
        .resizable(false)
        .exact_size(60.0)
        .frame(
            egui::Frame::new()
                .fill(theme::SURFACE)
                .stroke(egui::Stroke::new(1.0, theme::BORDER))
                .inner_margin(egui::Margin::symmetric(0, 14)),
        )
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                egui::Frame::new()
                    .fill(theme::ACCENT.gamma_multiply(0.4))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::same(6))
                    .show(ui, |ui| {
                        ui.label(RichText::new("P").strong().color(theme::TEXT_PRIMARY));
                    });
                ui.add_space(14.0);

                for (screen, icon) in ITEMS {
                    let label = i18n::nav_label(app.locale, screen);
                    rail_button(ui, app, screen, icon, label);
                }
            });
        });
}

fn rail_button(ui: &mut egui::Ui, app: &mut NivelaApp, screen: Screen, icon: &str, label: &str) {
    let active = app.screen == screen;
    let color = if active { theme::ACCENT } else { theme::TEXT_MUTED };

    let (rect, response) = ui.allocate_exact_size(egui::vec2(52.0, 46.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let bg: Option<Color32> = if active {
            Some(theme::ACCENT.gamma_multiply(0.18))
        } else if response.hovered() {
            Some(theme::SURFACE_2)
        } else {
            None
        };
        if let Some(bg) = bg {
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::same(8), bg);
        }
        let painter = ui.painter_at(rect);
        painter.text(
            rect.center_top() + egui::vec2(0.0, 14.0),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(15.0),
            color,
        );
        painter.text(
            rect.center_bottom() - egui::vec2(0.0, 8.0),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(9.0),
            color,
        );
    }
    if response.clicked() {
        app.screen = screen;
    }
    ui.add_space(2.0);
}
