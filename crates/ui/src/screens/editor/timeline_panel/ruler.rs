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

use super::draw::{draw_marker_ticks, draw_playhead, draw_ruler_ticks};
use super::snap::{snap_to_nearest, SnapTargets};

/// Immutable inputs shared by all ruler interactions in one timeline frame.
pub(super) struct RulerLayout<'a> {
    pub(super) track_label_width: f32,
    pub(super) ruler_height: f32,
    pub(super) canvas_content_width: f32,
    pub(super) px_per_sec: f32,
    pub(super) hscroll_source: egui::containers::scroll_area::ScrollSource,
    pub(super) hand_active: bool,
    pub(super) snap_enabled: bool,
    pub(super) snap_targets: &'a SnapTargets,
}

/// Renders the timeline ruler and returns its top edge plus any horizontal pan change.
pub(super) fn timeline_ruler(
    app: &mut App,
    ui: &mut egui::Ui,
    layout: RulerLayout<'_>,
) -> (f32, Option<f32>) {
    let mut ruler_top = 0.0_f32;
    let mut new_pan_px = None;
    ui.horizontal(|ui| {
        ui.add_space(layout.track_label_width);
        let ruler_scroll = egui::ScrollArea::horizontal()
            .id_salt("timeline_ruler_hscroll")
            .scroll_source(layout.hscroll_source)
            .scroll_bar_visibility(egui::containers::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .horizontal_scroll_offset(app.timeline_pan_px)
            .show(ui, |ui| {
                let sense = if layout.hand_active {
                    egui::Sense::hover()
                } else {
                    egui::Sense::click_and_drag()
                };
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(layout.canvas_content_width, layout.ruler_height),
                    sense,
                );
                ruler_top = rect.top();
                ui.painter().rect_filled(rect, 0, theme::SURFACE_2);
                draw_ruler_ticks(ui.painter(), rect, layout.px_per_sec);
                if let Some(pos) = response.interact_pointer_pos() {
                    let secs = ((pos.x - rect.left()) / layout.px_per_sec).max(0.0) as f64;
                    let secs = if layout.snap_enabled {
                        snap_to_nearest(secs, &layout.snap_targets.all(), layout.px_per_sec)
                    } else {
                        secs
                    };
                    app.active_project_mut().timeline_mut().playhead_secs = secs;
                }
                let markers = app.active_project().timeline().markers.clone();
                if let Some(seek_secs) = draw_marker_ticks(ui, rect, &markers, layout.px_per_sec) {
                    app.active_project_mut().timeline_mut().playhead_secs = seek_secs;
                }
                draw_playhead(
                    ui,
                    rect,
                    app.active_project().timeline().playhead_secs,
                    layout.px_per_sec,
                    2.0,
                );
            });
        if (ruler_scroll.state.offset.x - app.timeline_pan_px).abs() > 0.01 {
            new_pan_px = Some(ruler_scroll.state.offset.x);
        }
    });
    (ruler_top, new_pan_px)
}
