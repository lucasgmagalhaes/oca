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

use eframe::egui::{self, RichText};

use crate::app::{
    App, BLUR_INTENSITY_RANGE, BRIGHTNESS_RANGE, CHROMA_KEY_TOLERANCE_RANGE, CONTRAST_RANGE,
    CROP_MIN_SIZE, GAIN_DB_RANGE, GLITCH_INTENSITY_RANGE, LAYER_SCALE_RANGE,
    MASK_CORNER_RADIUS_RANGE, PIXELIZE_INTENSITY_RANGE, SATURATION_RANGE, SHAKE_INTENSITY_RANGE,
    SHARPEN_RANGE, SPEED_FACTOR_RANGE, STABILIZATION_INTENSITY_RANGE, VIGNETTE_INTENSITY_RANGE,
};
use crate::components;
use crate::i18n::Text;
use crate::theme;

pub(super) fn properties_panel(app: &mut App, ui: &mut egui::Ui, width: f32, height: f32) {
    let locale = app.locale;
    egui::Frame::new()
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(height);
            ui.vertical(|ui| {
                // Text/shape clip properties take priority when one is selected — only one of
                // `selected_clip_id`/`selected_text_clip_id`/`selected_shape_clip_id` is ever
                // `Some` at a time (see `App`'s doc comments on those fields).
                if let Some(tc_id) = app.selected_text_clip_id {
                    text_clip_properties(app, ui, tc_id, locale);
                    return;
                }
                if let Some(sc_id) = app.selected_shape_clip_id {
                    shape_clip_properties(app, ui, sc_id, locale);
                    return;
                }

                components::section_label(ui, Text::SelectedClip.tr(locale));
                let Some(asset) = app.selected_asset() else {
                    ui.label(
                        RichText::new(Text::NoClipSelected.tr(locale)).color(theme::TEXT_MUTED),
                    );
                    return;
                };
                ui.label(RichText::new(&asset.file_name).size(13.0));
                ui.add_space(8.0);

                prop_row(ui, Text::PropCodec.tr(locale), &asset.codec);
                prop_row(
                    ui,
                    Text::PropSourceBitrate.tr(locale),
                    &format!("{:.0} Mbps", asset.source_bitrate_mbps),
                );
                if let Some((w, h)) = asset.resolution {
                    prop_row(ui, Text::PropResolution.tr(locale), &format!("{w}×{h}"));
                }
                if let Some(fps) = asset.fps {
                    prop_row(ui, Text::PropFps.tr(locale), &format!("{fps:.0}"));
                }
                if let Some(l) = &asset.loudness {
                    prop_row(
                        ui,
                        Text::PropLoudness.tr(locale),
                        &format!("{:.1} LUFS", l.integrated_lufs),
                    );
                }
                prop_row(
                    ui,
                    Text::PropProxy.tr(locale),
                    if asset.proxy_path.is_some() {
                        Text::ProxyPresent.tr(locale)
                    } else {
                        Text::ProxyAbsent.tr(locale)
                    },
                );

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);
                components::section_label(ui, Text::OnExport.tr(locale));
                ui.horizontal_wrapped(|ui| {
                    let target = app.active_sequence_export_settings().target_lufs;
                    components::tag_accent(
                        ui,
                        &format!("{} → {target:.0} LUFS", Text::NormalizeTo.tr(locale)),
                    );
                    components::tag_outline(
                        ui,
                        &format!(
                            "{} ({:.0} Mbps)",
                            Text::BitrateFromSource.tr(locale),
                            asset.source_bitrate_mbps
                        ),
                    );
                });
                ui.add_space(6.0);
                ui.label(
                    RichText::new(Text::ExportAutoNote.tr(locale))
                        .size(10.5)
                        .color(theme::TEXT_MUTED),
                );

                if let Some(clip) = app.selected_clip() {
                    let mut gain_db = clip.gain_db;
                    let mut frozen = clip.frozen;
                    let mut deflicker_enabled = clip.deflicker_enabled;
                    let mut speed_factor = clip.speed_factor;
                    let (mut crop_x, mut crop_y, mut crop_w, mut crop_h) =
                        (clip.crop_x, clip.crop_y, clip.crop_w, clip.crop_h);
                    let mut mask_shape = clip.mask_shape;
                    let mut mask_corner_radius = clip.mask_corner_radius;
                    let mut flipped_h = clip.flipped_h;
                    let mut color_filter = clip.color_filter;
                    let mut lut_path = clip.lut_path.clone();
                    let (mut layer_scale_x, mut layer_scale_y) =
                        (clip.layer_scale_x, clip.layer_scale_y);
                    let mut vignette_intensity = clip.vignette_intensity;
                    let (mut brightness, mut contrast, mut saturation) =
                        (clip.brightness, clip.contrast, clip.saturation);
                    let mut sharpen = clip.sharpen;
                    let mut chroma_key_enabled = clip.chroma_key_enabled;
                    let mut chroma_key_color = clip.chroma_key_color;
                    let mut chroma_key_tolerance = clip.chroma_key_tolerance;
                    let mut background_removal_enabled = clip.background_removal_enabled;
                    let mut blur_intensity = clip.blur_intensity;
                    let mut shake_intensity = clip.shake_intensity;
                    let mut glitch_intensity = clip.glitch_intensity;
                    let mut pixelize_intensity = clip.pixelize_intensity;
                    let mut stabilization_intensity = clip.stabilization_intensity;
                    let mut transition_in = clip.transition_in;
                    let mut transition_duration_secs = clip.transition_duration_secs;
                    // Cloned out up front (like every other field above) rather than read from
                    // `clip` later, so this immutable borrow of `app` doesn't need to stay alive
                    // across the `app.set_selected_clip_*` mutable calls further down.
                    let position_keyframes = clip.position_keyframes.clone();
                    let scale_keyframes = clip.scale_keyframes.clone();
                    let rotation_keyframes = clip.rotation_keyframes.clone();
                    let opacity_keyframes = clip.opacity_keyframes.clone();
                    if components::property_section(
                        ui,
                        Text::PropGain.tr(locale),
                        Text::GainExportNote.tr(locale),
                        |ui| {
                            ui.add(
                                egui::Slider::new(&mut gain_db, GAIN_DB_RANGE)
                                    .suffix(" dB")
                                    .fixed_decimals(1),
                            )
                            .changed()
                        },
                    ) {
                        app.set_selected_clip_gain(gain_db);
                    }

                    // "Congelar" only makes sense for a video block — audio clips have no
                    // frame to hold.
                    if app.selected_clip_track_kind() == Some(avcore::timeline::TrackKind::Video)
                        && components::property_toggle(
                            ui,
                            Text::PropFreeze.tr(locale),
                            Text::FreezeExportNote.tr(locale),
                            &mut frozen,
                        )
                    {
                        app.set_selected_clip_frozen(frozen);
                    }

                    // Deflicker only makes sense for a video block, same reasoning as "Congelar".
                    if app.selected_clip_track_kind() == Some(avcore::timeline::TrackKind::Video)
                        && components::property_toggle(
                            ui,
                            Text::PropDeflicker.tr(locale),
                            Text::DeflickerExportNote.tr(locale),
                            &mut deflicker_enabled,
                        )
                    {
                        app.set_selected_clip_deflicker(deflicker_enabled);
                    }

                    if components::property_section(
                        ui,
                        Text::PropSpeed.tr(locale),
                        Text::SpeedExportNote.tr(locale),
                        |ui| {
                            ui.add(
                                egui::Slider::new(&mut speed_factor, SPEED_FACTOR_RANGE)
                                    .suffix("x")
                                    .fixed_decimals(2),
                            )
                            .changed()
                        },
                    ) {
                        app.set_selected_clip_speed(speed_factor);
                    }

                    // Crop reframes the video frame itself — no meaning for an audio block.
                    if app.selected_clip_track_kind() == Some(avcore::timeline::TrackKind::Video) {
                        components::property_section(
                            ui,
                            Text::PropCrop.tr(locale),
                            Text::CropExportNote.tr(locale),
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
                                    let reframing = app.auto_reframing_clip_id.is_some();
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
                                });
                                crop_changed
                            },
                        );

                        let mask_changed = components::property_section(
                            ui,
                            Text::PropMask.tr(locale),
                            Text::MaskExportNote.tr(locale),
                            |ui| {
                                let mut mask_changed = components::enum_combo(
                                    ui,
                                    "mask_shape",
                                    &[
                                        avcore::timeline::MaskShape::None,
                                        avcore::timeline::MaskShape::Circle,
                                        avcore::timeline::MaskShape::RoundedRect,
                                    ],
                                    &mut mask_shape,
                                    |shape| mask_shape_label(shape, locale),
                                );
                                if mask_shape == avcore::timeline::MaskShape::RoundedRect {
                                    mask_changed |= ui
                                        .add(
                                            egui::Slider::new(
                                                &mut mask_corner_radius,
                                                MASK_CORNER_RADIUS_RANGE,
                                            )
                                            .text(Text::MaskCornerRadius.tr(locale)),
                                        )
                                        .changed();
                                }
                                mask_changed
                            },
                        );
                        if mask_changed {
                            app.set_selected_clip_mask(mask_shape, mask_corner_radius);
                        }

                        if components::property_toggle(
                            ui,
                            Text::PropFlip.tr(locale),
                            Text::FlipExportNote.tr(locale),
                            &mut flipped_h,
                        ) {
                            app.set_selected_clip_flip_h(flipped_h);
                        }

                        let color_filter_changed = components::property_section(
                            ui,
                            Text::PropColorFilter.tr(locale),
                            Text::ColorFilterExportNote.tr(locale),
                            |ui| {
                                components::enum_combo(
                                    ui,
                                    "color_filter",
                                    &[
                                        avcore::timeline::ColorFilter::None,
                                        avcore::timeline::ColorFilter::BlackAndWhite,
                                        avcore::timeline::ColorFilter::Sepia,
                                    ],
                                    &mut color_filter,
                                    |filter| color_filter_label(filter, locale),
                                )
                            },
                        );
                        if color_filter_changed {
                            app.set_selected_clip_color_filter(color_filter);
                        }

                        let lut_changed = components::property_section(
                            ui,
                            Text::PropLut.tr(locale),
                            Text::LutExportNote.tr(locale),
                            |ui| {
                                let mut changed = false;
                                ui.horizontal(|ui| {
                                    let name = std::path::Path::new(&lut_path)
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                    ui.label(
                                        RichText::new(if name.is_empty() { "—" } else { &name })
                                            .size(11.0)
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                    if ui.button(Text::Browse.tr(locale)).clicked() {
                                        if let Some(file) = rfd::FileDialog::new()
                                            .add_filter("3D LUT", &["cube"])
                                            .pick_file()
                                        {
                                            lut_path = file.display().to_string();
                                            changed = true;
                                        }
                                    }
                                    if !lut_path.is_empty()
                                        && ui.button(Text::ClearLut.tr(locale)).clicked()
                                    {
                                        lut_path.clear();
                                        changed = true;
                                    }
                                });
                                changed
                            },
                        );
                        if lut_changed {
                            app.set_selected_clip_lut(lut_path);
                        }

                        let layer_size_changed = components::property_section(
                            ui,
                            Text::PropLayerSize.tr(locale),
                            Text::LayerSizeExportNote.tr(locale),
                            |ui| {
                                let mut changed = ui
                                    .add(
                                        egui::Slider::new(&mut layer_scale_x, LAYER_SCALE_RANGE)
                                            .text(Text::PropLayerWidth.tr(locale)),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut layer_scale_y, LAYER_SCALE_RANGE)
                                            .text(Text::PropLayerHeight.tr(locale)),
                                    )
                                    .changed();
                                changed
                            },
                        );
                        if layer_size_changed {
                            app.set_selected_clip_layer_scale(layer_scale_x, layer_scale_y);
                        }

                        if components::property_section(
                            ui,
                            Text::PropVignette.tr(locale),
                            Text::VignetteExportNote.tr(locale),
                            |ui| {
                                ui.add(
                                    egui::Slider::new(
                                        &mut vignette_intensity,
                                        VIGNETTE_INTENSITY_RANGE,
                                    )
                                    .fixed_decimals(2),
                                )
                                .changed()
                            },
                        ) {
                            app.set_selected_clip_vignette(vignette_intensity);
                        }

                        if components::property_section(
                            ui,
                            Text::PropColorAdjust.tr(locale),
                            Text::ColorAdjustExportNote.tr(locale),
                            |ui| {
                                let mut changed = ui
                                    .add(
                                        egui::Slider::new(&mut brightness, BRIGHTNESS_RANGE)
                                            .text(Text::PropBrightness.tr(locale)),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut contrast, CONTRAST_RANGE)
                                            .text(Text::PropContrast.tr(locale)),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(&mut saturation, SATURATION_RANGE)
                                            .text(Text::PropSaturation.tr(locale)),
                                    )
                                    .changed();
                                changed
                            },
                        ) {
                            app.set_selected_clip_color_adjust(brightness, contrast, saturation);
                        }

                        if components::property_section(
                            ui,
                            Text::PropSharpen.tr(locale),
                            Text::SharpenExportNote.tr(locale),
                            |ui| {
                                ui.add(
                                    egui::Slider::new(&mut sharpen, SHARPEN_RANGE)
                                        .fixed_decimals(2),
                                )
                                .changed()
                            },
                        ) {
                            app.set_selected_clip_sharpen(sharpen);
                        }

                        if components::property_block(
                            ui,
                            Text::ChromaKeyExportNote.tr(locale),
                            |ui| {
                                let mut changed = ui
                                    .checkbox(
                                        &mut chroma_key_enabled,
                                        Text::PropChromaKey.tr(locale),
                                    )
                                    .changed();
                                if chroma_key_enabled {
                                    ui.horizontal(|ui| {
                                        ui.label(Text::ChromaKeyColor.tr(locale));
                                        changed |= ui
                                            .color_edit_button_srgb(&mut chroma_key_color)
                                            .changed();
                                    });
                                    changed |= ui
                                        .add(
                                            egui::Slider::new(
                                                &mut chroma_key_tolerance,
                                                CHROMA_KEY_TOLERANCE_RANGE,
                                            )
                                            .text(Text::ChromaKeyTolerance.tr(locale)),
                                        )
                                        .changed();
                                }
                                changed
                            },
                        ) {
                            app.set_selected_clip_chroma_key(
                                chroma_key_enabled,
                                chroma_key_color,
                                chroma_key_tolerance,
                            );
                        }

                        if components::property_block(
                            ui,
                            Text::BackgroundRemovalExportNote.tr(locale),
                            |ui| {
                                let changed = ui
                                    .checkbox(
                                        &mut background_removal_enabled,
                                        Text::PropBackgroundRemoval.tr(locale),
                                    )
                                    .changed();
                                let generating = app.matte_generating_clip_id.is_some();
                                let label = if generating {
                                    Text::BackgroundRemovalGenerating.tr(locale)
                                } else {
                                    Text::BackgroundRemovalGenerateMatte.tr(locale)
                                };
                                if ui
                                    .add_enabled(!generating, egui::Button::new(label))
                                    .clicked()
                                {
                                    app.spawn_generate_matte_for_selected_clip();
                                }
                                changed
                            },
                        ) {
                            app.set_selected_clip_background_removal(background_removal_enabled);
                        }

                        if components::property_section(
                            ui,
                            Text::PropOtherEffects.tr(locale),
                            Text::OtherEffectsExportNote.tr(locale),
                            |ui| {
                                let mut changed = ui
                                    .add(
                                        egui::Slider::new(
                                            &mut blur_intensity,
                                            BLUR_INTENSITY_RANGE,
                                        )
                                        .text(Text::PropBlur.tr(locale)),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(
                                            &mut shake_intensity,
                                            SHAKE_INTENSITY_RANGE,
                                        )
                                        .text(Text::PropShake.tr(locale)),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(
                                            &mut glitch_intensity,
                                            GLITCH_INTENSITY_RANGE,
                                        )
                                        .text(Text::PropGlitch.tr(locale)),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(
                                            &mut pixelize_intensity,
                                            PIXELIZE_INTENSITY_RANGE,
                                        )
                                        .text(Text::PropPixelize.tr(locale)),
                                    )
                                    .changed();
                                changed
                            },
                        ) {
                            app.set_selected_clip_blur(blur_intensity);
                            app.set_selected_clip_shake(shake_intensity);
                            app.set_selected_clip_glitch(glitch_intensity);
                            app.set_selected_clip_pixelize(pixelize_intensity);
                        }

                        let stabilization_changed = components::property_section(
                            ui,
                            Text::PropStabilization.tr(locale),
                            Text::StabilizationExportNote.tr(locale),
                            |ui| {
                                ui.add(
                                    egui::Slider::new(
                                        &mut stabilization_intensity,
                                        STABILIZATION_INTENSITY_RANGE,
                                    )
                                    .fixed_decimals(2),
                                )
                                .changed()
                            },
                        );
                        if stabilization_changed {
                            app.set_selected_clip_stabilization(stabilization_intensity);
                        }

                        let transition_changed = components::property_section(
                            ui,
                            Text::PropTransition.tr(locale),
                            Text::TransitionExportNote.tr(locale),
                            |ui| {
                                let mut changed = components::enum_combo(
                                    ui,
                                    "transition_in",
                                    &[
                                        avcore::timeline::TransitionType::None,
                                        avcore::timeline::TransitionType::Fade,
                                        avcore::timeline::TransitionType::HardCut,
                                        avcore::timeline::TransitionType::Slide,
                                        avcore::timeline::TransitionType::Zoom,
                                    ],
                                    &mut transition_in,
                                    |transition| transition_type_label(transition, locale),
                                );
                                changed |= ui
                                    .add(
                                        egui::Slider::new(
                                            &mut transition_duration_secs,
                                            crate::app::TRANSITION_DURATION_RANGE,
                                        )
                                        .suffix(" s")
                                        .fixed_decimals(2)
                                        .text(Text::PropTransitionDuration.tr(locale)),
                                    )
                                    .changed();
                                changed
                            },
                        );
                        if transition_changed {
                            app.set_selected_clip_transition(
                                transition_in,
                                transition_duration_secs,
                            );
                        }

                        let mut new_position_keyframes = None;
                        if components::property_section(
                            ui,
                            Text::PropPositionKeyframes.tr(locale),
                            Text::PositionExportNote.tr(locale),
                            |ui| {
                                new_position_keyframes =
                                    position_keyframe_editor(ui, &position_keyframes, locale);
                                new_position_keyframes.is_some()
                            },
                        ) {
                            if let Some(kfs) = new_position_keyframes {
                                app.set_selected_clip_position_keyframes(kfs);
                            }
                        }
                        {
                            let tracking = app.motion_tracking_clip_id.is_some();
                            components::property_section(
                                ui,
                                Text::PropMotionTrackRegion.tr(locale),
                                Text::PropMotionTrackRegionHint.tr(locale),
                                |ui| {
                                    ui.add_enabled_ui(!tracking, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut app.motion_track_center_x,
                                                )
                                                .speed(0.01)
                                                .range(0.0..=1.0)
                                                .prefix("x "),
                                            );
                                            ui.add(
                                                egui::DragValue::new(
                                                    &mut app.motion_track_center_y,
                                                )
                                                .speed(0.01)
                                                .range(0.0..=1.0)
                                                .prefix("y "),
                                            );
                                            if ui
                                                .button(Text::MotionTrackRegionReset.tr(locale))
                                                .clicked()
                                            {
                                                app.motion_track_center_x = 0.5;
                                                app.motion_track_center_y = 0.5;
                                            }
                                        });
                                        let pick_label = if app.picking_motion_track_region {
                                            Text::MotionTrackRegionPickActive.tr(locale)
                                        } else {
                                            Text::MotionTrackRegionPick.tr(locale)
                                        };
                                        if ui.button(pick_label).clicked() {
                                            if app.picking_motion_track_region {
                                                app.stop_picking_motion_track_region();
                                            } else if app.preview_texture.is_some() {
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
                                                &mut app.motion_track_width,
                                                crate::app::MOTION_TRACK_SIZE_RANGE,
                                            )
                                            .text(Text::PropMotionTrackWidth.tr(locale)),
                                        );
                                        ui.add(
                                            egui::Slider::new(
                                                &mut app.motion_track_height,
                                                crate::app::MOTION_TRACK_SIZE_RANGE,
                                            )
                                            .text(Text::PropMotionTrackHeight.tr(locale)),
                                        );
                                        ui.add(
                                            egui::Slider::new(
                                                &mut app.motion_track_search_radius,
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

                        let mut new_scale_keyframes = None;
                        if components::property_section(
                            ui,
                            Text::PropScaleKeyframes.tr(locale),
                            Text::ScaleExportNote.tr(locale),
                            |ui| {
                                new_scale_keyframes = f32_keyframe_editor(
                                    ui,
                                    &scale_keyframes,
                                    crate::app::SCALE_RANGE,
                                    1.0,
                                    locale,
                                );
                                new_scale_keyframes.is_some()
                            },
                        ) {
                            if let Some(kfs) = new_scale_keyframes {
                                app.set_selected_clip_scale_keyframes(kfs);
                            }
                        }

                        let mut new_rotation_keyframes = None;
                        if components::property_section(
                            ui,
                            Text::PropRotationKeyframes.tr(locale),
                            Text::RotationExportNote.tr(locale),
                            |ui| {
                                new_rotation_keyframes = f32_keyframe_editor(
                                    ui,
                                    &rotation_keyframes,
                                    -180.0..=180.0,
                                    0.0,
                                    locale,
                                );
                                new_rotation_keyframes.is_some()
                            },
                        ) {
                            if let Some(kfs) = new_rotation_keyframes {
                                app.set_selected_clip_rotation_keyframes(kfs);
                            }
                        }

                        let mut new_opacity_keyframes = None;
                        if components::property_section(
                            ui,
                            Text::PropOpacityKeyframes.tr(locale),
                            Text::OpacityExportNote.tr(locale),
                            |ui| {
                                new_opacity_keyframes = f32_keyframe_editor(
                                    ui,
                                    &opacity_keyframes,
                                    0.0..=1.0,
                                    1.0,
                                    locale,
                                );
                                new_opacity_keyframes.is_some()
                            },
                        ) {
                            if let Some(kfs) = new_opacity_keyframes {
                                app.set_selected_clip_opacity_keyframes(kfs);
                            }
                        }
                    }
                }
            });
        });
}

/// Renders an editable list of `(time_fraction, value)` keyframe rows plus an "add at 1.0"
/// button and a per-row delete button — the shared UI shape for scale/rotation/opacity
/// keyframes (position needs its own two-value-per-row variant, see
/// [`position_keyframe_editor`]). Returns `Some(new_list)` if the user added, removed, or
/// edited a row this frame, `None` otherwise — the caller only calls the corresponding
/// `app.set_selected_clip_*_keyframes` setter when this is `Some`, matching every other
/// property section's "only write back on change" convention.
fn f32_keyframe_editor(
    ui: &mut egui::Ui,
    keyframes: &[avcore::Keyframe<f32>],
    value_range: std::ops::RangeInclusive<f32>,
    default_value: f32,
    locale: crate::i18n::Locale,
) -> Option<Vec<avcore::Keyframe<f32>>> {
    let mut list = keyframes.to_vec();
    let mut changed = false;
    let mut remove_index = None;
    for (i, kf) in list.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui
                .add(
                    egui::Slider::new(&mut kf.time_fraction, 0.0..=1.0)
                        .text(Text::KeyframeTime.tr(locale)),
                )
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut kf.value, value_range.clone()))
                .changed();
            if ui.small_button("🗑").clicked() {
                remove_index = Some(i);
            }
        });
    }
    if let Some(i) = remove_index {
        list.remove(i);
        changed = true;
    }
    if ui.button(Text::AddKeyframe.tr(locale)).clicked() {
        list.push(avcore::Keyframe {
            time_fraction: 1.0,
            value: default_value,
        });
        changed = true;
    }
    if changed {
        Some(list)
    } else {
        None
    }
}

/// Position-keyframe counterpart of [`f32_keyframe_editor`] — each row edits `time_fraction`
/// plus both `x`/`y` components of the same [`avcore::Position`].
fn position_keyframe_editor(
    ui: &mut egui::Ui,
    keyframes: &[avcore::Keyframe<avcore::Position>],
    locale: crate::i18n::Locale,
) -> Option<Vec<avcore::Keyframe<avcore::Position>>> {
    let mut list = keyframes.to_vec();
    let mut changed = false;
    let mut remove_index = None;
    for (i, kf) in list.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui
                .add(
                    egui::Slider::new(&mut kf.time_fraction, 0.0..=1.0)
                        .text(Text::KeyframeTime.tr(locale)),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut kf.value.x, -1.0..=1.0).text(Text::KeyframeX.tr(locale)),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut kf.value.y, -1.0..=1.0).text(Text::KeyframeY.tr(locale)),
                )
                .changed();
            if ui.small_button("🗑").clicked() {
                remove_index = Some(i);
            }
        });
    }
    if let Some(i) = remove_index {
        list.remove(i);
        changed = true;
    }
    if ui.button(Text::AddKeyframe.tr(locale)).clicked() {
        list.push(avcore::Keyframe {
            time_fraction: 1.0,
            value: avcore::Position { x: 0.0, y: 0.0 },
        });
        changed = true;
    }
    if changed {
        Some(list)
    } else {
        None
    }
}

/// Vertex-list counterpart of [`f32_keyframe_editor`]/[`position_keyframe_editor`] — each row
/// edits one `(x, y)` vertex of a [`avcore::timeline::ShapeKind::Polygon`], in the shape's own
/// local unit square (see that variant's doc comment). Refuses to remove the third-to-last
/// vertex (a polygon needs at least 3 to stay a real shape) rather than letting the list
/// collapse to something degenerate.
fn polygon_vertex_editor(
    ui: &mut egui::Ui,
    vertices: &[(f32, f32)],
    locale: crate::i18n::Locale,
) -> Option<Vec<(f32, f32)>> {
    let mut list = vertices.to_vec();
    let count = list.len();
    let mut changed = false;
    let mut remove_index = None;
    for (i, v) in list.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui
                .add(
                    egui::DragValue::new(&mut v.0)
                        .speed(0.01)
                        .range(-2.0..=2.0)
                        .prefix("x "),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut v.1)
                        .speed(0.01)
                        .range(-2.0..=2.0)
                        .prefix("y "),
                )
                .changed();
            if count > 3 && ui.small_button("🗑").clicked() {
                remove_index = Some(i);
            }
        });
    }
    if let Some(i) = remove_index {
        list.remove(i);
        changed = true;
    }
    if ui.button(Text::ShapeAddVertex.tr(locale)).clicked() {
        // Offset from the last vertex rather than stacking exactly on top of it, so the new
        // point is easy to spot and drag into place.
        let (last_x, last_y) = list.last().copied().unwrap_or((0.0, 0.0));
        list.push((last_x + 0.1, last_y + 0.1));
        changed = true;
    }
    if changed {
        Some(list)
    } else {
        None
    }
}

fn mask_shape_label(shape: avcore::timeline::MaskShape, locale: crate::i18n::Locale) -> String {
    match shape {
        avcore::timeline::MaskShape::None => Text::MaskNone.tr(locale).to_string(),
        avcore::timeline::MaskShape::Circle => Text::MaskCircle.tr(locale).to_string(),
        avcore::timeline::MaskShape::RoundedRect => Text::MaskRoundedRect.tr(locale).to_string(),
    }
}

fn color_filter_label(
    filter: avcore::timeline::ColorFilter,
    locale: crate::i18n::Locale,
) -> String {
    match filter {
        avcore::timeline::ColorFilter::None => Text::ColorFilterNone.tr(locale).to_string(),
        avcore::timeline::ColorFilter::BlackAndWhite => {
            Text::ColorFilterBlackAndWhite.tr(locale).to_string()
        }
        avcore::timeline::ColorFilter::Sepia => Text::ColorFilterSepia.tr(locale).to_string(),
    }
}

fn transition_type_label(
    transition: avcore::timeline::TransitionType,
    locale: crate::i18n::Locale,
) -> String {
    match transition {
        avcore::timeline::TransitionType::None => Text::TransitionNone.tr(locale).to_string(),
        avcore::timeline::TransitionType::Fade => Text::TransitionFade.tr(locale).to_string(),
        avcore::timeline::TransitionType::HardCut => Text::TransitionHardCut.tr(locale).to_string(),
        avcore::timeline::TransitionType::Slide => Text::TransitionSlide.tr(locale).to_string(),
        avcore::timeline::TransitionType::Zoom => Text::TransitionZoom.tr(locale).to_string(),
    }
}

/// Renders the properties panel content for a selected text overlay clip. Shows controls for
/// text content, font size, RGBA color, X/Y position, start time, and duration. Applies
/// changes immediately by mutating the clip through the active project's timeline.
fn text_clip_properties(app: &mut App, ui: &mut egui::Ui, tc_id: u64, locale: crate::i18n::Locale) {
    components::section_label(ui, Text::SelectedTextClip.tr(locale));

    // Gather a copy of the current clip values to populate controls without holding a borrow.
    let current = app
        .active_project()
        .timeline()
        .tracks
        .iter()
        .filter(|t| t.kind == avcore::timeline::TrackKind::Text)
        .flat_map(|t| &t.text_clips)
        .find(|tc| tc.id == tc_id)
        .cloned();

    let Some(mut tc) = current else {
        ui.label(RichText::new(Text::NoTextClipSelected.tr(locale)).color(theme::TEXT_MUTED));
        return;
    };

    let mut changed = false;
    // Set alongside `changed` only for edits that can change which clip(s) cover the
    // playhead (start/duration) — see the full-vs-cheap-refresh choice at the end of this
    // function.
    let mut structural_changed = false;

    // Text content
    ui.label(
        RichText::new(Text::PropTextContent.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    let text_resp = ui.add(
        egui::TextEdit::singleline(&mut tc.text)
            .desired_width(f32::INFINITY)
            .hint_text(Text::TextContentHint.tr(locale)),
    );
    if text_resp.changed() {
        changed = true;
    }
    ui.add_space(4.0);

    // Bundled family/style metadata is cheap; the font file itself is parsed lazily by core
    // only when preview/export actually rasterizes this clip.
    ui.label(
        RichText::new(Text::PropTextFontFamily.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    let previous_family = tc.font_family;
    egui::ComboBox::from_id_salt(("text_font_family", tc_id))
        .selected_text(text_font_family_label(tc.font_family, locale))
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for family in avcore::TextFontFamily::ALL {
                ui.selectable_value(
                    &mut tc.font_family,
                    family,
                    text_font_family_label(family, locale),
                );
            }
        });
    if tc.font_family != previous_family {
        if !tc.font_family.supports_bold() {
            tc.font_style = avcore::TextFontStyle::Regular;
        }
        changed = true;
    }

    ui.label(
        RichText::new(Text::PropTextFontStyle.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    let previous_style = tc.font_style;
    ui.add_enabled_ui(tc.font_family.supports_bold(), |ui| {
        egui::ComboBox::from_id_salt(("text_font_style", tc_id))
            .selected_text(text_font_style_label(tc.font_style, locale))
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut tc.font_style,
                    avcore::TextFontStyle::Regular,
                    Text::TextFontRegular.tr(locale),
                );
                ui.selectable_value(
                    &mut tc.font_style,
                    avcore::TextFontStyle::Bold,
                    Text::TextFontBold.tr(locale),
                );
            });
    });
    if tc.font_style != previous_style {
        changed = true;
    }

    // Font size
    ui.label(
        RichText::new(Text::PropTextFontSize.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    if ui
        .add(egui::Slider::new(&mut tc.font_size, 10.0..=120.0).suffix(" pt"))
        .changed()
    {
        changed = true;
    }

    // Color modal: wheel, presets, and manual HEX/RGB input are shared by all text colors.
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(Text::PropTextColor.tr(locale))
                .size(12.0)
                .color(theme::TEXT_MUTED),
        );
        if text_color_button(ui, tc.color_rgba).clicked() {
            app.begin_text_color_edit(
                tc_id,
                crate::app::TextColorTarget::Foreground,
                tc.color_rgba,
            );
        }
    });

    let mut background_enabled = tc.background_rgba[3] > 0;
    if ui
        .checkbox(
            &mut background_enabled,
            Text::PropTextBackgroundEnabled.tr(locale),
        )
        .changed()
    {
        tc.background_rgba[3] = if background_enabled { 192 } else { 0 };
        changed = true;
    }
    ui.add_enabled_ui(background_enabled, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(Text::PropTextBackgroundColor.tr(locale))
                    .size(12.0)
                    .color(theme::TEXT_MUTED),
            );
            if text_color_button(ui, tc.background_rgba).clicked() {
                app.begin_text_color_edit(
                    tc_id,
                    crate::app::TextColorTarget::Background,
                    tc.background_rgba,
                );
            }
        });
        ui.label(
            RichText::new(Text::PropTextBackgroundPadding.tr(locale))
                .size(12.0)
                .color(theme::TEXT_MUTED),
        );
        if ui
            .add(egui::Slider::new(&mut tc.background_padding, 0.0..=64.0).suffix(" px"))
            .changed()
        {
            changed = true;
        }
        ui.label(
            RichText::new(Text::PropTextBackgroundRadius.tr(locale))
                .size(12.0)
                .color(theme::TEXT_MUTED),
        );
        if ui
            .add(egui::Slider::new(&mut tc.background_corner_radius, 0.0..=64.0).suffix(" px"))
            .changed()
        {
            changed = true;
        }
    });

    // Word-highlight (only meaningful for a clip generated from transcription, which is the
    // only path that populates `words` — a manually-typed block has no per-word timing to
    // highlight against).
    if !tc.words.is_empty() {
        ui.horizontal(|ui| {
            if ui
                .checkbox(
                    &mut tc.highlight_enabled,
                    Text::PropTextHighlightEnabled.tr(locale),
                )
                .changed()
            {
                changed = true;
            }
            if text_color_button(ui, tc.highlight_color_rgba).clicked() {
                app.begin_text_color_edit(
                    tc_id,
                    crate::app::TextColorTarget::Highlight,
                    tc.highlight_color_rgba,
                );
            }
        });
    }

    // Position
    ui.label(
        RichText::new(Text::PropTextPosX.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    if ui
        .add(
            egui::Slider::new(&mut tc.pos_x, 0.0..=1.0)
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
        )
        .changed()
    {
        changed = true;
    }
    ui.label(
        RichText::new(Text::PropTextPosY.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    if ui
        .add(
            egui::Slider::new(&mut tc.pos_y, 0.0..=1.0)
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
        )
        .changed()
    {
        changed = true;
    }

    // Start and duration
    ui.label(
        RichText::new(Text::PropTextStart.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    if ui
        .add(
            egui::DragValue::new(&mut tc.start_secs)
                .range(0.0..=f64::MAX)
                .speed(0.1)
                .suffix(" s"),
        )
        .changed()
    {
        changed = true;
        structural_changed = true;
    }
    ui.label(
        RichText::new(Text::PropTextDuration.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    if ui
        .add(
            egui::DragValue::new(&mut tc.duration_secs)
                .range(0.1..=f64::MAX)
                .speed(0.1)
                .suffix(" s"),
        )
        .changed()
    {
        changed = true;
        structural_changed = true;
    }

    ui.add_space(6.0);
    ui.label(
        RichText::new(Text::TextExportNote.tr(locale))
            .size(10.5)
            .color(theme::TEXT_MUTED),
    );

    // Apply changes back to the clip in the active project. `start_secs`/`duration_secs`
    // (structural_changed) can change which clips cover the playhead, so those still force a
    // full pipeline reopen via `ensure_preview_loaded`'s normal id-diffing path. Every other
    // field here (text, font, color, background, position, highlight) only changes this
    // clip's own rasterized look — if it's already part of the currently open composited
    // preview, `refresh_preview_text_content` pushes a fresh buffer into its existing
    // `appsrc` branch instead of tearing down and rebuilding the whole GStreamer pipeline
    // (background decoder, compositor, every other branch) on every dragged slider frame.
    if changed {
        app.push_undo_snapshot_for_drag();
        let timeline = app.active_project_mut().timeline_mut();
        for track in &mut timeline.tracks {
            if track.kind == avcore::timeline::TrackKind::Text {
                if let Some(existing) = track.text_clips.iter_mut().find(|c| c.id == tc_id) {
                    *existing = tc;
                    break;
                }
            }
        }
        if structural_changed {
            app.invalidate_preview_rendering();
        } else {
            app.refresh_preview_text_content(tc_id);
        }
    }
}

fn text_color_button(ui: &mut egui::Ui, rgba: [u8; 4]) -> egui::Response {
    let color = egui::Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3]);
    let luminance =
        0.299 * f32::from(rgba[0]) + 0.587 * f32::from(rgba[1]) + 0.114 * f32::from(rgba[2]);
    let label_color = if rgba[3] >= 128 && luminance > 160.0 {
        egui::Color32::BLACK
    } else {
        egui::Color32::WHITE
    };
    ui.add(
        egui::Button::new(
            egui::RichText::new(crate::app::format_color_hex(rgba)).color(label_color),
        )
        .fill(color)
        .min_size(egui::vec2(112.0, 22.0)),
    )
}

fn text_font_family_label(
    family: avcore::TextFontFamily,
    locale: crate::i18n::Locale,
) -> &'static str {
    match family {
        avcore::TextFontFamily::Lato => Text::TextFontLato.tr(locale),
        avcore::TextFontFamily::BebasNeue => Text::TextFontBebasNeue.tr(locale),
        avcore::TextFontFamily::PlayfairDisplay => Text::TextFontPlayfairDisplay.tr(locale),
        avcore::TextFontFamily::PatrickHand => Text::TextFontPatrickHand.tr(locale),
        avcore::TextFontFamily::AnonymousPro => Text::TextFontAnonymousPro.tr(locale),
        avcore::TextFontFamily::ArchivoBlack => Text::TextFontArchivoBlack.tr(locale),
    }
}

fn text_font_style_label(
    style: avcore::TextFontStyle,
    locale: crate::i18n::Locale,
) -> &'static str {
    match style {
        avcore::TextFontStyle::Regular => Text::TextFontRegular.tr(locale),
        avcore::TextFontStyle::Bold => Text::TextFontBold.tr(locale),
    }
}

/// A `Copy`-friendly stand-in for [`avcore::timeline::ShapeKind`] for use with
/// [`components::enum_combo`] (whose `T: Copy` bound `ShapeKind` itself can't satisfy — its
/// `Polygon` variant holds a `Vec`). Maps to/from the fixed presets via `ShapeKind::rectangle()`
/// etc. `Custom` stands for a `Polygon` that doesn't match any preset's exact vertex list; there
/// being no vertex-editing UI yet (see `ShapeKind::Polygon`'s doc comment), it's only ever shown
/// as the current value, never offered as something to pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapePreset {
    Ellipse,
    Rectangle,
    Triangle,
    Trapezoid,
    Arrow,
    Custom,
}

fn shape_kind_to_preset(kind: &avcore::timeline::ShapeKind) -> ShapePreset {
    use avcore::timeline::ShapeKind;
    if *kind == ShapeKind::Ellipse {
        ShapePreset::Ellipse
    } else if *kind == ShapeKind::rectangle() {
        ShapePreset::Rectangle
    } else if *kind == ShapeKind::triangle() {
        ShapePreset::Triangle
    } else if *kind == ShapeKind::trapezoid() {
        ShapePreset::Trapezoid
    } else if *kind == ShapeKind::arrow() {
        ShapePreset::Arrow
    } else {
        ShapePreset::Custom
    }
}

fn shape_preset_label(preset: ShapePreset, locale: crate::i18n::Locale) -> String {
    match preset {
        ShapePreset::Ellipse => Text::ShapePresetEllipse.tr(locale).to_string(),
        ShapePreset::Rectangle => Text::ShapePresetRectangle.tr(locale).to_string(),
        ShapePreset::Triangle => Text::ShapePresetTriangle.tr(locale).to_string(),
        ShapePreset::Trapezoid => Text::ShapePresetTrapezoid.tr(locale).to_string(),
        ShapePreset::Arrow => Text::ShapePresetArrow.tr(locale).to_string(),
        ShapePreset::Custom => Text::ShapePresetCustom.tr(locale).to_string(),
    }
}

/// Renders the properties panel content for a selected shape overlay clip. Shows controls for
/// the shape preset, RGBA color, center X/Y, width/height, rotation, outline thickness, start
/// time, and duration. Applies changes immediately by mutating the clip through the active
/// project's timeline — mirrors [`text_clip_properties`]'s clone-mutate-writeback shape.
fn shape_clip_properties(
    app: &mut App,
    ui: &mut egui::Ui,
    sc_id: u64,
    locale: crate::i18n::Locale,
) {
    components::section_label(ui, Text::SelectedShapeClip.tr(locale));

    let current = app
        .active_project()
        .timeline()
        .tracks
        .iter()
        .filter(|t| t.kind == avcore::timeline::TrackKind::Shape)
        .flat_map(|t| &t.shape_clips)
        .find(|sc| sc.id == sc_id)
        .cloned();

    let Some(mut sc) = current else {
        ui.label(RichText::new(Text::NoShapeClipSelected.tr(locale)).color(theme::TEXT_MUTED));
        return;
    };

    let mut changed = false;

    // Shape preset
    ui.label(
        RichText::new(Text::PropShapeKind.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    let mut preset = shape_kind_to_preset(&sc.shape_kind);
    let mut preset_options = vec![
        ShapePreset::Ellipse,
        ShapePreset::Rectangle,
        ShapePreset::Triangle,
        ShapePreset::Trapezoid,
        ShapePreset::Arrow,
    ];
    if preset == ShapePreset::Custom {
        preset_options.push(ShapePreset::Custom);
    }
    if components::enum_combo(ui, "shape_kind", &preset_options, &mut preset, |p| {
        shape_preset_label(p, locale)
    }) {
        sc.shape_kind = match preset {
            ShapePreset::Ellipse => avcore::timeline::ShapeKind::Ellipse,
            ShapePreset::Rectangle => avcore::timeline::ShapeKind::rectangle(),
            ShapePreset::Triangle => avcore::timeline::ShapeKind::triangle(),
            ShapePreset::Trapezoid => avcore::timeline::ShapeKind::trapezoid(),
            ShapePreset::Arrow => avcore::timeline::ShapeKind::arrow(),
            // Not reachable — Custom is only ever in `preset_options` when it was already the
            // current value, so selecting it again isn't a change `enum_combo` reports.
            ShapePreset::Custom => sc.shape_kind.clone(),
        };
        changed = true;
    }
    ui.add_space(4.0);

    // Vertex editor — every preset except Ellipse is already a Polygon under the hood (see
    // ShapeKind::rectangle() etc.), so this lets the user hand-edit any of them into a custom
    // shape, not just a dedicated "Custom" starting point.
    let polygon_vertices = match &sc.shape_kind {
        avcore::timeline::ShapeKind::Polygon(vertices) => Some(vertices.clone()),
        avcore::timeline::ShapeKind::Ellipse => None,
    };
    if let Some(vertices) = polygon_vertices {
        let mut new_vertices = None;
        if components::property_section(
            ui,
            Text::PropShapeVertices.tr(locale),
            Text::ShapeVerticesHint.tr(locale),
            |ui| {
                new_vertices = polygon_vertex_editor(ui, &vertices, locale);
                new_vertices.is_some()
            },
        ) {
            if let Some(v) = new_vertices {
                sc.shape_kind = avcore::timeline::ShapeKind::Polygon(v);
                changed = true;
            }
        }
    }

    // Color picker (RGBA — alpha controlled via the color picker's alpha channel).
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(Text::PropShapeColor.tr(locale))
                .size(12.0)
                .color(theme::TEXT_MUTED),
        );
        let mut color = egui::Color32::from_rgba_premultiplied(
            sc.color_rgba[0],
            sc.color_rgba[1],
            sc.color_rgba[2],
            sc.color_rgba[3],
        );
        if ui.color_edit_button_srgba(&mut color).changed() {
            sc.color_rgba = [color.r(), color.g(), color.b(), color.a()];
            changed = true;
        }
    });

    // Center position
    ui.label(
        RichText::new(Text::PropShapePosX.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    if ui
        .add(
            egui::Slider::new(&mut sc.center_x, 0.0..=1.0)
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
        )
        .changed()
    {
        changed = true;
    }
    ui.label(
        RichText::new(Text::PropShapePosY.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    if ui
        .add(
            egui::Slider::new(&mut sc.center_y, 0.0..=1.0)
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
        )
        .changed()
    {
        changed = true;
    }

    // Size
    ui.label(
        RichText::new(Text::PropShapeWidth.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    if ui
        .add(
            egui::Slider::new(&mut sc.width, 0.01..=1.0)
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
        )
        .changed()
    {
        changed = true;
    }
    ui.label(
        RichText::new(Text::PropShapeHeight.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    if ui
        .add(
            egui::Slider::new(&mut sc.height, 0.01..=1.0)
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
        )
        .changed()
    {
        changed = true;
    }

    // Rotation
    ui.label(
        RichText::new(Text::PropShapeRotation.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    if ui
        .add(egui::Slider::new(&mut sc.rotation_deg, 0.0..=360.0).suffix("°"))
        .changed()
    {
        changed = true;
    }

    // Outline thickness
    ui.label(
        RichText::new(Text::PropShapeStroke.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    if ui
        .add(
            egui::DragValue::new(&mut sc.stroke_thickness_px)
                .range(0.0..=f32::MAX)
                .speed(0.5)
                .suffix(" px"),
        )
        .changed()
    {
        changed = true;
    }
    ui.label(
        RichText::new(Text::PropShapeStrokeHint.tr(locale))
            .size(10.0)
            .color(theme::TEXT_MUTED),
    );

    // Start and duration
    ui.label(
        RichText::new(Text::PropShapeStart.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    if ui
        .add(
            egui::DragValue::new(&mut sc.start_secs)
                .range(0.0..=f64::MAX)
                .speed(0.1)
                .suffix(" s"),
        )
        .changed()
    {
        changed = true;
    }
    ui.label(
        RichText::new(Text::PropShapeDuration.tr(locale))
            .size(12.0)
            .color(theme::TEXT_MUTED),
    );
    if ui
        .add(
            egui::DragValue::new(&mut sc.duration_secs)
                .range(0.1..=f64::MAX)
                .speed(0.1)
                .suffix(" s"),
        )
        .changed()
    {
        changed = true;
    }

    ui.add_space(6.0);
    ui.label(
        RichText::new(Text::ShapeExportNote.tr(locale))
            .size(10.5)
            .color(theme::TEXT_MUTED),
    );

    // Apply changes back to the clip in the active project.
    if changed {
        app.push_undo_snapshot_for_drag();
        let timeline = app.active_project_mut().timeline_mut();
        for track in &mut timeline.tracks {
            if track.kind == avcore::timeline::TrackKind::Shape {
                if let Some(existing) = track.shape_clips.iter_mut().find(|c| c.id == sc_id) {
                    *existing = sc;
                    break;
                }
            }
        }
    }
}

pub(super) fn prop_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.0).color(theme::TEXT_MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(12.0));
        });
    });
}
