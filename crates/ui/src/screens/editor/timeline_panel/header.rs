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

//! Timeline title, sequence tabs, and zoom controls.

use eframe::egui::{self, RichText};

use crate::app::App;
use crate::i18n::Text;
use crate::theme;

use super::{MAX_PX_PER_SEC, MIN_PX_PER_SEC};

pub(super) fn timeline_header(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(Text::Timeline.tr(app.locale))
                .size(11.0)
                .color(theme::TEXT_MUTED),
        );
        ui.add_space(theme::SPACE_MD);
        super::super::sequence_tab_bar(app, ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            const ZOOM_STEP_FACTOR: f32 = 1.25;
            if ui
                .button(RichText::new("+").size(13.0))
                .on_hover_text(Text::TimelineZoomIn.tr(app.locale))
                .clicked()
            {
                app.timeline_px_per_sec = (app.timeline_px_per_sec * ZOOM_STEP_FACTOR)
                    .clamp(MIN_PX_PER_SEC, MAX_PX_PER_SEC);
            }
            ui.add(
                egui::Slider::new(
                    &mut app.timeline_px_per_sec,
                    MIN_PX_PER_SEC..=MAX_PX_PER_SEC,
                )
                .show_value(false)
                .logarithmic(true),
            );
            if ui
                .button(RichText::new("-").size(13.0))
                .on_hover_text(Text::TimelineZoomOut.tr(app.locale))
                .clicked()
            {
                app.timeline_px_per_sec = (app.timeline_px_per_sec / ZOOM_STEP_FACTOR)
                    .clamp(MIN_PX_PER_SEC, MAX_PX_PER_SEC);
            }
            ui.label(
                RichText::new(Text::TimelineZoom.tr(app.locale))
                    .size(11.0)
                    .color(theme::TEXT_MUTED),
            );
        });
    });
    ui.separator();
}
