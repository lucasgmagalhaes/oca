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

use eframe::egui;

use crate::app::App;
use crate::theme;

pub(super) fn audio_meter_column(app: &App, ui: &mut egui::Ui, height: f32) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(4, 12))
        .show(ui, |ui| {
            ui.set_height(height);
            ui.vertical_centered(|ui| {
                super::properties_panel::stereo_db_meter(
                    ui,
                    app.current_audio_level(),
                    (height - 24.0).max(40.0),
                );
            });
        });
}

/// Hit-testable width of a [`resizable_divider`]/[`resizable_divider_horizontal`] handle — wider
/// than the 1px line it draws, since a bare 1px strip is unreliable to grab with a mouse.
const DIVIDER_HIT_WIDTH: f32 = 6.0;

/// A draggable divider between two side-by-side panels (per `request.md`'s Fase 3 "painéis de
/// UI redimensionáveis" spec). Dragging it left/right adjusts `*width` by the pointer's
/// horizontal movement — `sign` is `1.0` when `*width` belongs to the panel on the divider's
/// left (dragging right grows it) or `-1.0` when it belongs to the panel on the right (dragging
/// right shrinks it) — clamped to `[min_width, max_width]`.
pub(super) fn resizable_divider(
    ui: &mut egui::Ui,
    height: f32,
    width: &mut f32,
    min_width: f32,
    max_width: f32,
    sign: f32,
) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(DIVIDER_HIT_WIDTH, height), egui::Sense::drag());
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    if response.dragged() {
        *width = (*width + sign * response.drag_delta().x).clamp(min_width, max_width);
    }
    let line_x = rect.center().x;
    ui.painter().line_segment(
        [
            egui::pos2(line_x, rect.top()),
            egui::pos2(line_x, rect.bottom()),
        ],
        egui::Stroke::new(1.0, theme::BORDER),
    );
}

/// Same idea as [`resizable_divider`] but for the horizontal boundary above the timeline strip:
/// dragging it up grows `*height` (the timeline), dragging it down shrinks it, clamped to
/// `[min_height, max_height]`.
pub(super) fn resizable_divider_horizontal(
    ui: &mut egui::Ui,
    width: f32,
    height: &mut f32,
    min_height: f32,
    max_height: f32,
) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, DIVIDER_HIT_WIDTH), egui::Sense::drag());
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    if response.dragged() {
        *height = (*height - response.drag_delta().y).clamp(min_height, max_height);
    }
    let line_y = rect.center().y;
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), line_y),
            egui::pos2(rect.right(), line_y),
        ],
        egui::Stroke::new(1.0, theme::BORDER),
    );
}
