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

use eframe::egui::{self, Color32, RichText};

use crate::app::{App, Screen};
use crate::i18n;
use crate::theme;

const ITEMS: [(Screen, &str); 5] = [
    (Screen::Home, "⌂"),
    (Screen::Editor, "✂"),
    (Screen::Library, "▤"),
    (Screen::SoundLibrary, "♫"),
    (Screen::Queue, "≡"),
];

/// Renders the left icon rail and handles screen-switching clicks.
pub fn show(app: &mut App, ui: &mut egui::Ui) {
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

                let prefs_label = i18n::Text::NavPrefs.tr(app.locale);
                prefs_button(ui, app, "⚙", prefs_label);
            });
        });
}

fn rail_button(ui: &mut egui::Ui, app: &mut App, screen: Screen, icon: &str, label: &str) {
    let active = app.screen == screen;
    let color = if active {
        theme::ACCENT
    } else {
        theme::TEXT_MUTED
    };

    let (rect, response) = ui.allocate_exact_size(egui::vec2(52.0, 46.0), egui::Sense::click());
    // Hand-painted below rather than a real `ui.button()`, so without this it carries no
    // accessible name — screen readers and UI-Automation-driven e2e tests alike would only
    // ever see an unlabeled clickable rect at this position.
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
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
        app.prefs_open = false;
    }
    ui.add_space(2.0);
}

fn prefs_button(ui: &mut egui::Ui, app: &mut App, icon: &str, label: &str) {
    let active = app.prefs_open;
    let color = if active {
        theme::ACCENT
    } else {
        theme::TEXT_MUTED
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(52.0, 46.0), egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
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
        app.prefs_open = !app.prefs_open;
    }
    ui.add_space(2.0);
}
