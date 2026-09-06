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

use crate::app::{App, CROP_MIN_SIZE};
use crate::components;
use crate::i18n::{Locale, Text};

use super::keyframe_editors::f32_keyframe_editor;

pub(super) fn crop_properties(app: &mut App, ui: &mut egui::Ui, clip_id: u64, locale: Locale) {
    if app.selected_clip_track_kind() != Some(avcore::timeline::TrackKind::Video)
        || app.properties_tab != crate::app::PropertiesTab::Inspector
    {
        return;
    }

    let Some(clip) = app.selected_clip() else {
        return;
    };
    let (mut crop_x, mut crop_y, mut crop_w, mut crop_h) =
        (clip.crop_x, clip.crop_y, clip.crop_w, clip.crop_h);
    let mut reframe_seed_point = clip.reframe_seed_point;
    let crop_x_keyframes = clip.crop_x_keyframes.clone();
    let crop_y_keyframes = clip.crop_y_keyframes.clone();
    let crop_w_keyframes = clip.crop_w_keyframes.clone();
    let crop_h_keyframes = clip.crop_h_keyframes.clone();

    components::property_section(
        ui,
        clip_id,
        Text::PropCrop.tr(locale),
        Text::CropExportNote.tr(locale),
        crop_x != 0.0 || crop_y != 0.0 || crop_w != 1.0 || crop_h != 1.0,
        |ui| {
            let mut crop_changed = false;
            ui.horizontal(|ui| {
                crop_changed |= ui
                    .add(
                        egui::DragValue::new(&mut crop_x)
                            .speed(0.01)
                            .range(0.0..=1.0)
                            .prefix("x "),
                    )
                    .changed();
                crop_changed |= ui
                    .add(
                        egui::DragValue::new(&mut crop_y)
                            .speed(0.01)
                            .range(0.0..=1.0)
                            .prefix("y "),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                crop_changed |= ui
                    .add(
                        egui::DragValue::new(&mut crop_w)
                            .speed(0.01)
                            .range(CROP_MIN_SIZE..=1.0)
                            .prefix("w "),
                    )
                    .changed();
                crop_changed |= ui
                    .add(
                        egui::DragValue::new(&mut crop_h)
                            .speed(0.01)
                            .range(CROP_MIN_SIZE..=1.0)
                            .prefix("h "),
                    )
                    .changed();
            });
            if crop_changed {
                app.set_selected_clip_crop(crop_x, crop_y, crop_w, crop_h);
            }
            ui.horizontal(|ui| {
                if ui.button(Text::CropReset.tr(locale)).clicked() {
                    app.set_selected_clip_crop(0.0, 0.0, 1.0, 1.0);
                }
                let reframing = app.auto_reframe_state.auto_reframing_clip_id.is_some();
                let label = if reframing {
                    Text::AutoReframeInProgress.tr(locale)
                } else {
                    Text::AutoReframeAction.tr(locale)
                };
                if ui
                    .add_enabled(!reframing, egui::Button::new(label))
                    .clicked()
                {
                    app.spawn_auto_reframe_selected_clip();
                }
                let dynamic_reframing = app
                    .dynamic_reframe_state
                    .dynamic_reframing_clip_id
                    .is_some();
                let dynamic_label = if dynamic_reframing {
                    Text::DynamicReframeInProgress.tr(locale)
                } else {
                    Text::DynamicReframeAction.tr(locale)
                };
                if ui
                    .add_enabled(!dynamic_reframing, egui::Button::new(dynamic_label))
                    .on_hover_text(Text::DynamicReframeHint.tr(locale))
                    .clicked()
                {
                    app.spawn_dynamic_reframe_selected_clip();
                }
            });
            let mut seed_enabled = reframe_seed_point.is_some();
            let (mut seed_x, mut seed_y) = reframe_seed_point.unwrap_or((0.5, 0.5));
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut seed_enabled, Text::ReframeSeedPointToggle.tr(locale))
                    .on_hover_text(Text::ReframeSeedPointHint.tr(locale))
                    .changed()
                {
                    reframe_seed_point = seed_enabled.then_some((seed_x, seed_y));
                    app.set_selected_clip_reframe_seed_point(reframe_seed_point);
                }
            });
            if seed_enabled {
                ui.horizontal(|ui| {
                    let mut seed_changed = false;
                    seed_changed |= ui
                        .add(
                            egui::DragValue::new(&mut seed_x)
                                .speed(0.01)
                                .range(0.0..=1.0)
                                .prefix("x "),
                        )
                        .changed();
                    seed_changed |= ui
                        .add(
                            egui::DragValue::new(&mut seed_y)
                                .speed(0.01)
                                .range(0.0..=1.0)
                                .prefix("y "),
                        )
                        .changed();
                    if seed_changed {
                        reframe_seed_point = Some((seed_x, seed_y));
                        app.set_selected_clip_reframe_seed_point(reframe_seed_point);
                    }
                });
            }
            crop_changed
        },
    );

    let mut new_crop_x_keyframes = None;
    if components::property_section(
        ui,
        clip_id,
        Text::PropCropXKeyframes.tr(locale),
        Text::CropKeyframesExportNote.tr(locale),
        !crop_x_keyframes.is_empty(),
        |ui| {
            new_crop_x_keyframes =
                f32_keyframe_editor(ui, &crop_x_keyframes, 0.0..=1.0, 0.0, locale);
            new_crop_x_keyframes.is_some()
        },
    ) {
        if let Some(kfs) = new_crop_x_keyframes {
            app.set_selected_clip_crop_x_keyframes(kfs);
        }
    }

    let mut new_crop_y_keyframes = None;
    if components::property_section(
        ui,
        clip_id,
        Text::PropCropYKeyframes.tr(locale),
        Text::CropKeyframesExportNote.tr(locale),
        !crop_y_keyframes.is_empty(),
        |ui| {
            new_crop_y_keyframes =
                f32_keyframe_editor(ui, &crop_y_keyframes, 0.0..=1.0, 0.0, locale);
            new_crop_y_keyframes.is_some()
        },
    ) {
        if let Some(kfs) = new_crop_y_keyframes {
            app.set_selected_clip_crop_y_keyframes(kfs);
        }
    }

    let mut new_crop_w_keyframes = None;
    if components::property_section(
        ui,
        clip_id,
        Text::PropCropWKeyframes.tr(locale),
        Text::CropKeyframesExportNote.tr(locale),
        !crop_w_keyframes.is_empty(),
        |ui| {
            new_crop_w_keyframes =
                f32_keyframe_editor(ui, &crop_w_keyframes, CROP_MIN_SIZE..=1.0, 1.0, locale);
            new_crop_w_keyframes.is_some()
        },
    ) {
        if let Some(kfs) = new_crop_w_keyframes {
            app.set_selected_clip_crop_w_keyframes(kfs);
        }
    }

    let mut new_crop_h_keyframes = None;
    if components::property_section(
        ui,
        clip_id,
        Text::PropCropHKeyframes.tr(locale),
        Text::CropKeyframesExportNote.tr(locale),
        !crop_h_keyframes.is_empty(),
        |ui| {
            new_crop_h_keyframes =
                f32_keyframe_editor(ui, &crop_h_keyframes, CROP_MIN_SIZE..=1.0, 1.0, locale);
            new_crop_h_keyframes.is_some()
        },
    ) {
        if let Some(kfs) = new_crop_h_keyframes {
            app.set_selected_clip_crop_h_keyframes(kfs);
        }
    }
}
