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
use crate::components;
use crate::i18n::{Locale, Text};

/// Renders the motion-tracking region controls and dispatches the selected clip's tracking job.
/// Keeping these controls together makes their shared disabled state explicit while a job runs.
pub(super) fn motion_tracking_properties(
    app: &mut App,
    ui: &mut egui::Ui,
    clip_id: u64,
    locale: Locale,
) {
    let tracking = app.motion_tracking_state.motion_tracking_clip_id.is_some();
    components::property_section(
        ui,
        clip_id,
        Text::PropMotionTrackRegion.tr(locale),
        Text::PropMotionTrackRegionHint.tr(locale),
        false,
        |ui| {
            ui.add_enabled_ui(!tracking, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut app.motion_track_region.motion_track_center_x)
                            .speed(0.01)
                            .range(0.0..=1.0)
                            .prefix("x "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut app.motion_track_region.motion_track_center_y)
                            .speed(0.01)
                            .range(0.0..=1.0)
                            .prefix("y "),
                    );
                    if ui.button(Text::MotionTrackRegionReset.tr(locale)).clicked() {
                        app.motion_track_region.motion_track_center_x = 0.5;
                        app.motion_track_region.motion_track_center_y = 0.5;
                    }
                });
                let pick_label = if app.motion_track_region.picking_motion_track_region {
                    Text::MotionTrackRegionPickActive.tr(locale)
                } else {
                    Text::MotionTrackRegionPick.tr(locale)
                };
                if ui.button(pick_label).clicked() {
                    if app.motion_track_region.picking_motion_track_region {
                        app.stop_picking_motion_track_region();
                    } else if app.preview_state.preview_texture.is_some() {
                        app.start_picking_motion_track_region();
                    } else {
                        app.push_toast(
                            Text::MotionTrackRegionPickNeedsPreview
                                .tr(locale)
                                .to_string(),
                        );
                    }
                }
                ui.add(
                    egui::Slider::new(
                        &mut app.motion_track_region.motion_track_width,
                        crate::app::MOTION_TRACK_SIZE_RANGE,
                    )
                    .text(Text::PropMotionTrackWidth.tr(locale)),
                );
                ui.add(
                    egui::Slider::new(
                        &mut app.motion_track_region.motion_track_height,
                        crate::app::MOTION_TRACK_SIZE_RANGE,
                    )
                    .text(Text::PropMotionTrackHeight.tr(locale)),
                );
                ui.add(
                    egui::Slider::new(
                        &mut app.motion_track_region.motion_track_search_radius,
                        crate::app::MOTION_TRACK_SEARCH_RADIUS_RANGE,
                    )
                    .text(Text::PropMotionTrackSearchRadius.tr(locale)),
                );
            });
            false
        },
    );
    let label = if tracking {
        Text::MotionTrackInProgress.tr(locale)
    } else {
        Text::MotionTrackAction.tr(locale)
    };
    if ui
        .add_enabled(!tracking, egui::Button::new(label))
        .clicked()
    {
        app.spawn_motion_track_selected_clip();
    }
}
