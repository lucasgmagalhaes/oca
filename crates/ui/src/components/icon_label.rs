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

use eframe::egui::{text::LayoutJob, Color32, FontFamily, FontId, TextFormat};

/// Builds a two-section [`LayoutJob`] — an icon-font glyph followed by a space and a plain-text
/// label — for a button that needs both in a single widget (e.g. the editor toolbar's
/// "✂ Cortar" style tool buttons). [`egui::RichText`]/[`icon_button`](super::icon_button) can
/// only carry one font for their whole string, so mixing the icon font
/// (`crate::icons::family()`) with the app's default proportional font for the label needs a
/// `LayoutJob`'s per-section [`TextFormat`] instead.
///
/// `icon`/`label` take separate colors since a toolbar tool button's active/inactive state is
/// usually expressed as one shared color for both (pass the same value twice), but a caller
/// that wants the icon and label to diverge (e.g. an accent icon on muted label text) can.
pub fn icon_label_job(
    icon: &str,
    icon_family: FontFamily,
    icon_size: f32,
    icon_color: Color32,
    label: &str,
    label_size: f32,
    label_color: Color32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        icon,
        0.0,
        TextFormat {
            font_id: FontId::new(icon_size, icon_family),
            color: icon_color,
            ..Default::default()
        },
    );
    job.append(
        &format!(" {label}"),
        0.0,
        TextFormat {
            font_id: FontId::new(label_size, FontFamily::Proportional),
            color: label_color,
            ..Default::default()
        },
    );
    job
}
