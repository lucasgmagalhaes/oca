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

use eframe::egui::{self, Color32, RichText};

use crate::app::{App, Screen};
use crate::i18n;
use crate::theme;

const ITEMS: [(Screen, &str); 6] = [
    (Screen::Home, "⌂"),
    (Screen::Editor, "✂"),
    (Screen::Library, "▤"),
    (Screen::SoundLibrary, "♫"),
    (Screen::Queue, "≡"),
    (Screen::WatchFolder, "🧹"),
];

/// Width of the compact icon-only rail — each button is a single centered glyph, tooltip-only
/// for its accessible name (matching the icon-rail convention from the editor workspace mock,
/// `oca-editor-mock.html`'s `.nav-rail`) rather than the wider icon+stacked-label rail this used
/// to be.
const RAIL_WIDTH: f32 = 52.0;
/// One rail button is square-ish (36x36 in the mock); rectangular here only so the accent
/// active-indicator bar along the left edge has somewhere unclipped to draw.
const BUTTON_SIZE: egui::Vec2 = egui::vec2(36.0, 36.0);

/// Renders the left icon rail and handles screen-switching clicks.
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    egui::Panel::left("nav_rail")
        .resizable(false)
        .exact_size(RAIL_WIDTH)
        .frame(
            egui::Frame::new()
                .fill(theme::SURFACE)
                .stroke(egui::Stroke::new(1.0, theme::BORDER))
                .inner_margin(egui::Margin::symmetric(0, 12)),
        )
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                egui::Frame::new()
                    .fill(theme::ACCENT.gamma_multiply(0.4))
                    .corner_radius(theme::RADIUS_MD)
                    .inner_margin(egui::Margin::same(4))
                    .show(ui, |ui| {
                        ui.label(RichText::new("P").strong().color(theme::TEXT_PRIMARY));
                    });
                ui.add_space(12.0);

                for (screen, icon) in ITEMS {
                    let label = i18n::nav_label(app.locale, screen);
                    rail_button(
                        ui,
                        app.screen == screen,
                        icon,
                        egui::FontFamily::Proportional,
                        label,
                    )
                    .clicked()
                    .then(|| {
                        app.screen = screen;
                        app.prefs_open = false;
                    });
                }

                ui.add_space(4.0);
                let prefs_label = i18n::Text::NavPrefs.tr(app.locale);
                if rail_button(
                    ui,
                    app.prefs_open,
                    crate::icons::SETTINGS_STR,
                    crate::icons::family(),
                    prefs_label,
                )
                .clicked()
                {
                    app.prefs_open = !app.prefs_open;
                }
            });
        });
}

/// One icon-only rail button: a centered glyph, an accent-tinted rounded background plus a
/// left-edge accent bar when active, and a plain hover tint otherwise. Tooltip carries the
/// accessible name (`widget_info` below) since there's no visible label glyph.
fn rail_button(
    ui: &mut egui::Ui,
    active: bool,
    icon: &str,
    icon_family: egui::FontFamily,
    label: &str,
) -> egui::Response {
    let color = if active {
        theme::ACCENT
    } else {
        theme::TEXT_MUTED
    };

    let (rect, response) = ui.allocate_exact_size(BUTTON_SIZE, egui::Sense::click());
    // Hand-painted below rather than a real `ui.button()`, so without this it carries no
    // accessible name — screen readers and UI-Automation-driven e2e tests alike would only
    // ever see an unlabeled clickable rect at this position.
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    if ui.is_rect_visible(rect) {
        let bg: Option<Color32> = if active {
            Some(theme::ACCENT_TINT)
        } else if response.hovered() {
            Some(theme::SURFACE_2)
        } else {
            None
        };
        if let Some(bg) = bg {
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::same(theme::RADIUS_MD), bg);
        }
        if active {
            let bar_rect = egui::Rect::from_min_size(
                rect.left_center() - egui::vec2(4.0, 9.0),
                egui::vec2(3.0, 18.0),
            );
            ui.painter()
                .rect_filled(bar_rect, egui::CornerRadius::same(2), theme::ACCENT);
        }
        ui.painter_at(rect).text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::new(16.0, icon_family),
            color,
        );
    }
    ui.add_space(4.0);
    response.on_hover_text(label)
}
