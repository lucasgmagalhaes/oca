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

//! The dark/teal color palette (matching `ui.html` / the design comp) and the egui `Visuals`
//! it's applied through. Screens reference these constants directly rather than going
//! through egui's default palette, so the whole app reads as one consistent theme.

use egui::{Color32, CornerRadius, Stroke, Visuals};

pub const BG: Color32 = Color32::from_rgb(0x12, 0x16, 0x1c);
pub const SURFACE: Color32 = Color32::from_rgb(0x1a, 0x20, 0x29);
pub const SURFACE_2: Color32 = Color32::from_rgb(0x21, 0x28, 0x36);
pub const BORDER: Color32 = Color32::from_rgb(0x2b, 0x33, 0x40);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xee, 0xf1, 0xf4);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xa9, 0xb2, 0xbd);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x6c, 0x76, 0x83);
pub const ACCENT: Color32 = Color32::from_rgb(0x2e, 0xa3, 0x9e);
pub const ACCENT_TINT: Color32 = Color32::from_rgba_premultiplied(0x0a, 0x28, 0x27, 0x80);
pub const ACCENT_2: Color32 = Color32::from_rgb(0x4f, 0x7c, 0xe0);
pub const ERROR: Color32 = Color32::from_rgb(0xe0, 0x57, 0x4f);
pub const ERROR_TINT: Color32 = Color32::from_rgba_premultiplied(0x2a, 0x11, 0x10, 0x80);

/// Applies the dark/teal palette used throughout the HTML mockups to the egui context.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);

    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE;
    visuals.faint_bg_color = SURFACE_2;
    visuals.extreme_bg_color = BG;
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT;

    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.weak_bg_fill = SURFACE_2;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);

    visuals.widgets.inactive.bg_fill = SURFACE_2;
    visuals.widgets.inactive.weak_bg_fill = SURFACE_2;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(8);

    visuals.widgets.hovered.bg_fill = SURFACE_2;
    visuals.widgets.hovered.weak_bg_fill = SURFACE_2;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(8);

    visuals.widgets.active.bg_fill = ACCENT.gamma_multiply(0.3);
    visuals.widgets.active.weak_bg_fill = ACCENT.gamma_multiply(0.3);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.active.corner_radius = CornerRadius::same(8);

    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.window_stroke = Stroke::new(1.0, BORDER);

    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
    });
}
