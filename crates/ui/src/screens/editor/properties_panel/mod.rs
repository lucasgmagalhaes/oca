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

mod keyframe_editors;
mod shape_clip;
mod text_clip;

use eframe::egui::{self, RichText};

use crate::app::{
    App, BLUR_INTENSITY_RANGE, BRIGHTNESS_RANGE, CHROMA_KEY_TOLERANCE_RANGE, CONTRAST_RANGE,
    CROP_MIN_SIZE, GAIN_DB_RANGE, GLITCH_INTENSITY_RANGE, LAYER_SCALE_RANGE,
    MASK_CORNER_RADIUS_RANGE, PIXELIZE_INTENSITY_RANGE, SATURATION_RANGE, SHAKE_INTENSITY_RANGE,
    SHARPEN_RANGE, SPEED_FACTOR_RANGE, STABILIZATION_INTENSITY_RANGE, VIGNETTE_INTENSITY_RANGE,
    VOICE_CLEANUP_CEILING_RANGE, VOICE_CLEANUP_COMPRESSOR_RATIO_RANGE,
    VOICE_CLEANUP_COMPRESSOR_THRESHOLD_RANGE, VOICE_CLEANUP_NOISE_FLOOR_RANGE,
};
use crate::components;
use crate::i18n::Text;
use crate::theme;
use keyframe_editors::{
    blend_mode_label, color_filter_label, f32_keyframe_editor, mask_shape_label,
    position_keyframe_editor, transition_type_label,
};
use shape_clip::shape_clip_properties;
use text_clip::text_clip_properties;

pub(super) fn properties_panel(app: &mut App, ui: &mut egui::Ui, width: f32, height: f32) {
    let locale = app.locale;
    components::panel_frame().show(ui, |ui| {
        ui.set_width(width);
        ui.set_height(height);
        // Real clip selections carry a long, non-collapsible stack of property sections
        // (Transform/Crop/Composite/Speed/Effects/Audio, each with its own keyframe editor)
        // that easily exceeds a typical body_height — without a ScrollArea, this content
        // isn't clipped, it just paints straight past the panel's own allocated rect and
        // visually overlaps whatever sits below it in the Editor's vertical layout (the
        // timeline strip), which read as "the timeline disappeared" until traced back here.
        //
        // The ScrollArea is nested inside a plain `ui.vertical` rather than replacing it
        // directly (matching `media_library_panel`'s own working ScrollArea nesting one
        // level in) — putting `ScrollArea::show` directly as the first thing inside the
        // Frame's `set_width`/`set_height`'d ui measurably corrupted the content's
        // horizontal position (confirmed via a real-egui-rect debug log: labels rendered
        // squeezed into the panel's rightmost ~30px instead of starting at its left margin)
        // rather than merely its width, for reasons not fully root-caused — the vertical
        // wrapper is the fix a real precedent in this codebase already demonstrates works.
        ui.vertical(|ui| {
            egui::ScrollArea::vertical()
                .id_salt("properties_panel_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
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

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);
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
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(Text::ExportAutoNote.tr(locale))
                            .size(10.5)
                            .color(theme::TEXT_MUTED),
                    );

                    if let Some(clip) = app.selected_clip() {
                        let clip_id = clip.id;
                        let mut gain_db = clip.gain_db;
                        let mut frozen = clip.frozen;
                        let mut deflicker_enabled = clip.deflicker_enabled;
                        let mut speed_factor = clip.speed_factor;
                        let (mut crop_x, mut crop_y, mut crop_w, mut crop_h) =
                            (clip.crop_x, clip.crop_y, clip.crop_w, clip.crop_h);
                        let mut mask_shape = clip.mask_shape;
                        let mut mask_corner_radius = clip.mask_corner_radius;
                        let mut flipped_h = clip.flipped_h;
                        let mut blend_mode = clip.blend_mode;
                        let mut color_filter = clip.color_filter;
                        let mut lut_path = clip.lut_path.clone();
                        let (mut layer_scale_x, mut layer_scale_y) =
                            (clip.layer_scale_x, clip.layer_scale_y);
                        let (mut anchor_x, mut anchor_y) = (clip.anchor_x, clip.anchor_y);
                        let mut vignette_intensity = clip.vignette_intensity;
                        let (mut brightness, mut contrast, mut saturation) =
                            (clip.brightness, clip.contrast, clip.saturation);
                        let mut sharpen = clip.sharpen;
                        let mut chroma_key_enabled = clip.chroma_key_enabled;
                        let mut chroma_key_color = clip.chroma_key_color;
                        let mut chroma_key_tolerance = clip.chroma_key_tolerance;
                        let mut background_removal_enabled = clip.background_removal_enabled;
                        let mut voice_cleanup_enabled = clip.voice_cleanup_enabled;
                        let mut voice_cleanup_noise_floor_db = clip.voice_cleanup_noise_floor_db;
                        let mut voice_cleanup_compressor_threshold_db =
                            clip.voice_cleanup_compressor_threshold_db;
                        let mut voice_cleanup_compressor_ratio =
                            clip.voice_cleanup_compressor_ratio;
                        let mut voice_cleanup_ceiling_linear = clip.voice_cleanup_ceiling_linear;
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
                        let gain_keyframes = clip.gain_keyframes.clone();
                        let brightness_keyframes = clip.brightness_keyframes.clone();
                        let contrast_keyframes = clip.contrast_keyframes.clone();
                        let saturation_keyframes = clip.saturation_keyframes.clone();
                        let crop_x_keyframes = clip.crop_x_keyframes.clone();
                        let crop_y_keyframes = clip.crop_y_keyframes.clone();
                        let crop_w_keyframes = clip.crop_w_keyframes.clone();
                        let crop_h_keyframes = clip.crop_h_keyframes.clone();

                        properties_tab_bar(app, ui, locale);
                        let tab = app.properties_tab;

                        if tab == crate::app::PropertiesTab::Audio {
                            if components::property_section(
                                ui,
                                clip_id,
                                Text::PropGain.tr(locale),
                                Text::GainExportNote.tr(locale),
                                gain_db != 0.0,
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

                            let mut new_gain_keyframes = None;
                            if components::property_section(
                                ui,
                                clip_id,
                                Text::PropGainKeyframes.tr(locale),
                                Text::GainKeyframesExportNote.tr(locale),
                                !gain_keyframes.is_empty(),
                                |ui| {
                                    new_gain_keyframes = f32_keyframe_editor(
                                        ui,
                                        &gain_keyframes,
                                        GAIN_DB_RANGE,
                                        0.0,
                                        locale,
                                    );
                                    new_gain_keyframes.is_some()
                                },
                            ) {
                                if let Some(kfs) = new_gain_keyframes {
                                    app.set_selected_clip_gain_keyframes(kfs);
                                }
                            }
                        }

                        // "Congelar" only makes sense for a video block — audio clips have no
                        // frame to hold.
                        if tab == crate::app::PropertiesTab::Effects
                            && app.selected_clip_track_kind()
                                == Some(avcore::timeline::TrackKind::Video)
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
                        if tab == crate::app::PropertiesTab::Effects
                            && app.selected_clip_track_kind()
                                == Some(avcore::timeline::TrackKind::Video)
                            && components::property_toggle(
                                ui,
                                Text::PropDeflicker.tr(locale),
                                Text::DeflickerExportNote.tr(locale),
                                &mut deflicker_enabled,
                            )
                        {
                            app.set_selected_clip_deflicker(deflicker_enabled);
                        }

                        if tab == crate::app::PropertiesTab::Inspector
                            && components::property_section(
                                ui,
                                clip_id,
                                Text::PropSpeed.tr(locale),
                                Text::SpeedExportNote.tr(locale),
                                (speed_factor - 1.0).abs() > 1e-4,
                                |ui| {
                                    ui.add(
                                        egui::Slider::new(&mut speed_factor, SPEED_FACTOR_RANGE)
                                            .suffix("x")
                                            .fixed_decimals(2),
                                    )
                                    .changed()
                                },
                            )
                        {
                            app.set_selected_clip_speed(speed_factor);
                        }

                        // CF-03 voice cleanup (noise reduction + compressor + limiter, the proven
                        // Watch-Gameplay.ps1 chain) only makes sense on an audio block — no meaning
                        // for a video block's own embedded audio, same "clip-level toggle regardless
                        // of AudioRole" scope this field's own doc comment on ClipInstance describes.
                        let is_audio_clip = app.selected_clip_track_kind()
                            == Some(avcore::timeline::TrackKind::Audio);
                        if tab == crate::app::PropertiesTab::Audio
                            && is_audio_clip
                            && !voice_cleanup_enabled
                            && app.selected_clip_track_audio_role() == Some(avcore::AudioRole::Mic)
                        {
                            // A clip that predates its track's Mic role (or was moved there after
                            // creation) never gets `App::add_asset_to_timeline`'s "Mic by default"
                            // treatment — this is the non-blocking nudge for that case, never an
                            // auto-toggle, since the field stays a plain always-overridable per-clip
                            // setting either way.
                            ui.label(
                                RichText::new(Text::VoiceCleanupMicRoleSuggestion.tr(locale))
                                    .size(10.5)
                                    .color(theme::WARNING),
                            );
                        }
                        if tab == crate::app::PropertiesTab::Audio
                            && is_audio_clip
                            && components::property_block(
                                ui,
                                Text::VoiceCleanupExportNote.tr(locale),
                                |ui| {
                                    let mut changed = ui
                                        .checkbox(
                                            &mut voice_cleanup_enabled,
                                            Text::PropVoiceCleanup.tr(locale),
                                        )
                                        .changed();
                                    if voice_cleanup_enabled {
                                        changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut voice_cleanup_noise_floor_db,
                                                    VOICE_CLEANUP_NOISE_FLOOR_RANGE,
                                                )
                                                .suffix(" dB")
                                                .text(Text::VoiceCleanupNoiseFloor.tr(locale)),
                                            )
                                            .changed();
                                        changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut voice_cleanup_compressor_threshold_db,
                                                    VOICE_CLEANUP_COMPRESSOR_THRESHOLD_RANGE,
                                                )
                                                .suffix(" dB")
                                                .text(
                                                    Text::VoiceCleanupCompressorThreshold
                                                        .tr(locale),
                                                ),
                                            )
                                            .changed();
                                        changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut voice_cleanup_compressor_ratio,
                                                    VOICE_CLEANUP_COMPRESSOR_RATIO_RANGE,
                                                )
                                                .suffix(":1")
                                                .text(Text::VoiceCleanupCompressorRatio.tr(locale)),
                                            )
                                            .changed();
                                        changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut voice_cleanup_ceiling_linear,
                                                    VOICE_CLEANUP_CEILING_RANGE,
                                                )
                                                .fixed_decimals(2)
                                                .text(Text::VoiceCleanupCeiling.tr(locale)),
                                            )
                                            .changed();
                                    }
                                    changed
                                },
                            )
                        {
                            app.set_selected_clip_voice_cleanup(
                                voice_cleanup_enabled,
                                voice_cleanup_noise_floor_db,
                                voice_cleanup_compressor_threshold_db,
                                voice_cleanup_compressor_ratio,
                                voice_cleanup_ceiling_linear,
                            );
                        }
                        // Crop reframes the video frame itself — no meaning for an audio block.
                        if app.selected_clip_track_kind()
                            == Some(avcore::timeline::TrackKind::Video)
                        {
                            if tab == crate::app::PropertiesTab::Inspector {
                                components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropCrop.tr(locale),
                                    Text::CropExportNote.tr(locale),
                                    crop_x != 0.0
                                        || crop_y != 0.0
                                        || crop_w != 1.0
                                        || crop_h != 1.0,
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
                                            app.set_selected_clip_crop(
                                                crop_x, crop_y, crop_w, crop_h,
                                            );
                                        }
                                        ui.horizontal(|ui| {
                                            if ui.button(Text::CropReset.tr(locale)).clicked() {
                                                app.set_selected_clip_crop(0.0, 0.0, 1.0, 1.0);
                                            }
                                            let reframing = app
                                                .auto_reframe_state
                                                .auto_reframing_clip_id
                                                .is_some();
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
                                                .add_enabled(
                                                    !dynamic_reframing,
                                                    egui::Button::new(dynamic_label),
                                                )
                                                .on_hover_text(Text::DynamicReframeHint.tr(locale))
                                                .clicked()
                                            {
                                                app.spawn_dynamic_reframe_selected_clip();
                                            }
                                        });
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
                                        new_crop_x_keyframes = f32_keyframe_editor(
                                            ui,
                                            &crop_x_keyframes,
                                            0.0..=1.0,
                                            0.0,
                                            locale,
                                        );
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
                                        new_crop_y_keyframes = f32_keyframe_editor(
                                            ui,
                                            &crop_y_keyframes,
                                            0.0..=1.0,
                                            0.0,
                                            locale,
                                        );
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
                                        new_crop_w_keyframes = f32_keyframe_editor(
                                            ui,
                                            &crop_w_keyframes,
                                            CROP_MIN_SIZE..=1.0,
                                            1.0,
                                            locale,
                                        );
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
                                        new_crop_h_keyframes = f32_keyframe_editor(
                                            ui,
                                            &crop_h_keyframes,
                                            CROP_MIN_SIZE..=1.0,
                                            1.0,
                                            locale,
                                        );
                                        new_crop_h_keyframes.is_some()
                                    },
                                ) {
                                    if let Some(kfs) = new_crop_h_keyframes {
                                        app.set_selected_clip_crop_h_keyframes(kfs);
                                    }
                                }

                                let mask_changed = components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropMask.tr(locale),
                                    Text::MaskExportNote.tr(locale),
                                    mask_shape != avcore::timeline::MaskShape::None,
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

                                // Composite section (mockup: "COMPOSITE" groups Blend Mode with
                                // Opacity — opacity_keyframes lives further down, alongside the
                                // other keyframe editors, per this panel's existing grouping).
                                let blend_mode_changed = components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropBlendMode.tr(locale),
                                    Text::BlendModeExportNote.tr(locale),
                                    blend_mode != avcore::timeline::BlendMode::Normal,
                                    |ui| {
                                        components::enum_combo(
                                            ui,
                                            "blend_mode",
                                            &avcore::timeline::BlendMode::ALL,
                                            &mut blend_mode,
                                            |mode| blend_mode_label(mode).to_string(),
                                        )
                                    },
                                );
                                if blend_mode_changed {
                                    app.set_selected_clip_blend_mode(blend_mode);
                                }
                            }

                            if tab == crate::app::PropertiesTab::Effects {
                                let color_filter_changed = components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropColorFilter.tr(locale),
                                    Text::ColorFilterExportNote.tr(locale),
                                    color_filter != avcore::timeline::ColorFilter::None,
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
                                    clip_id,
                                    Text::PropLut.tr(locale),
                                    Text::LutExportNote.tr(locale),
                                    !lut_path.is_empty(),
                                    |ui| {
                                        let mut changed = false;
                                        ui.horizontal(|ui| {
                                            let name = std::path::Path::new(&lut_path)
                                                .file_name()
                                                .map(|n| n.to_string_lossy().to_string())
                                                .unwrap_or_default();
                                            ui.label(
                                                RichText::new(if name.is_empty() {
                                                    "—"
                                                } else {
                                                    &name
                                                })
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
                                    clip_id,
                                    Text::PropLayerSize.tr(locale),
                                    Text::LayerSizeExportNote.tr(locale),
                                    (layer_scale_x - 1.0).abs() > 1e-4
                                        || (layer_scale_y - 1.0).abs() > 1e-4,
                                    |ui| {
                                        let mut changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut layer_scale_x,
                                                    LAYER_SCALE_RANGE,
                                                )
                                                .text(Text::PropLayerWidth.tr(locale)),
                                            )
                                            .changed();
                                        changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut layer_scale_y,
                                                    LAYER_SCALE_RANGE,
                                                )
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
                                    clip_id,
                                    Text::PropVignette.tr(locale),
                                    Text::VignetteExportNote.tr(locale),
                                    vignette_intensity > 0.0,
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
                                    clip_id,
                                    Text::PropColorAdjust.tr(locale),
                                    Text::ColorAdjustExportNote.tr(locale),
                                    brightness != 0.0
                                        || (contrast - 1.0).abs() > 1e-4
                                        || (saturation - 1.0).abs() > 1e-4,
                                    |ui| {
                                        let mut changed = ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut brightness,
                                                    BRIGHTNESS_RANGE,
                                                )
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
                                                egui::Slider::new(
                                                    &mut saturation,
                                                    SATURATION_RANGE,
                                                )
                                                .text(Text::PropSaturation.tr(locale)),
                                            )
                                            .changed();
                                        changed
                                    },
                                ) {
                                    app.set_selected_clip_color_adjust(
                                        brightness, contrast, saturation,
                                    );
                                }

                                let mut new_brightness_keyframes = None;
                                if components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropBrightnessKeyframes.tr(locale),
                                    Text::ColorKeyframesExportNote.tr(locale),
                                    !brightness_keyframes.is_empty(),
                                    |ui| {
                                        new_brightness_keyframes = f32_keyframe_editor(
                                            ui,
                                            &brightness_keyframes,
                                            BRIGHTNESS_RANGE,
                                            0.0,
                                            locale,
                                        );
                                        new_brightness_keyframes.is_some()
                                    },
                                ) {
                                    if let Some(kfs) = new_brightness_keyframes {
                                        app.set_selected_clip_brightness_keyframes(kfs);
                                    }
                                }

                                let mut new_contrast_keyframes = None;
                                if components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropContrastKeyframes.tr(locale),
                                    Text::ColorKeyframesExportNote.tr(locale),
                                    !contrast_keyframes.is_empty(),
                                    |ui| {
                                        new_contrast_keyframes = f32_keyframe_editor(
                                            ui,
                                            &contrast_keyframes,
                                            CONTRAST_RANGE,
                                            1.0,
                                            locale,
                                        );
                                        new_contrast_keyframes.is_some()
                                    },
                                ) {
                                    if let Some(kfs) = new_contrast_keyframes {
                                        app.set_selected_clip_contrast_keyframes(kfs);
                                    }
                                }

                                let mut new_saturation_keyframes = None;
                                if components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropSaturationKeyframes.tr(locale),
                                    Text::ColorKeyframesExportNote.tr(locale),
                                    !saturation_keyframes.is_empty(),
                                    |ui| {
                                        new_saturation_keyframes = f32_keyframe_editor(
                                            ui,
                                            &saturation_keyframes,
                                            SATURATION_RANGE,
                                            1.0,
                                            locale,
                                        );
                                        new_saturation_keyframes.is_some()
                                    },
                                ) {
                                    if let Some(kfs) = new_saturation_keyframes {
                                        app.set_selected_clip_saturation_keyframes(kfs);
                                    }
                                }

                                if components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropSharpen.tr(locale),
                                    Text::SharpenExportNote.tr(locale),
                                    sharpen > 0.0,
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
                                        let generating = app
                                            .matte_generation_state
                                            .matte_generating_clip_id
                                            .is_some();
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
                                    app.set_selected_clip_background_removal(
                                        background_removal_enabled,
                                    );
                                }

                                if components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropOtherEffects.tr(locale),
                                    Text::OtherEffectsExportNote.tr(locale),
                                    blur_intensity > 0.0
                                        || shake_intensity > 0.0
                                        || glitch_intensity > 0.0
                                        || pixelize_intensity > 0.0,
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
                                    clip_id,
                                    Text::PropStabilization.tr(locale),
                                    Text::StabilizationExportNote.tr(locale),
                                    stabilization_intensity > 0.0,
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
                                    clip_id,
                                    Text::PropTransition.tr(locale),
                                    Text::TransitionExportNote.tr(locale),
                                    transition_in != avcore::timeline::TransitionType::None,
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
                            }

                            if tab == crate::app::PropertiesTab::Inspector {
                                let mut new_position_keyframes = None;
                                if components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropPositionKeyframes.tr(locale),
                                    Text::PositionExportNote.tr(locale),
                                    !position_keyframes.is_empty(),
                                    |ui| {
                                        new_position_keyframes = position_keyframe_editor(
                                            ui,
                                            &position_keyframes,
                                            locale,
                                        );
                                        new_position_keyframes.is_some()
                                    },
                                ) {
                                    if let Some(kfs) = new_position_keyframes {
                                        app.set_selected_clip_position_keyframes(kfs);
                                    }
                                }
                                {
                                    let tracking =
                                        app.motion_tracking_state.motion_tracking_clip_id.is_some();
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
                                                        egui::DragValue::new(
                                                            &mut app
                                                                .motion_track_region
                                                                .motion_track_center_x,
                                                        )
                                                        .speed(0.01)
                                                        .range(0.0..=1.0)
                                                        .prefix("x "),
                                                    );
                                                    ui.add(
                                                        egui::DragValue::new(
                                                            &mut app
                                                                .motion_track_region
                                                                .motion_track_center_y,
                                                        )
                                                        .speed(0.01)
                                                        .range(0.0..=1.0)
                                                        .prefix("y "),
                                                    );
                                                    if ui
                                                        .button(
                                                            Text::MotionTrackRegionReset.tr(locale),
                                                        )
                                                        .clicked()
                                                    {
                                                        app.motion_track_region
                                                            .motion_track_center_x = 0.5;
                                                        app.motion_track_region
                                                            .motion_track_center_y = 0.5;
                                                    }
                                                });
                                                let pick_label = if app
                                                    .motion_track_region
                                                    .picking_motion_track_region
                                                {
                                                    Text::MotionTrackRegionPickActive.tr(locale)
                                                } else {
                                                    Text::MotionTrackRegionPick.tr(locale)
                                                };
                                                if ui.button(pick_label).clicked() {
                                                    if app
                                                        .motion_track_region
                                                        .picking_motion_track_region
                                                    {
                                                        app.stop_picking_motion_track_region();
                                                    } else if app
                                                        .preview_state
                                                        .preview_texture
                                                        .is_some()
                                                    {
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
                                                        &mut app
                                                            .motion_track_region
                                                            .motion_track_width,
                                                        crate::app::MOTION_TRACK_SIZE_RANGE,
                                                    )
                                                    .text(Text::PropMotionTrackWidth.tr(locale)),
                                                );
                                                ui.add(
                                                    egui::Slider::new(
                                                        &mut app
                                                            .motion_track_region
                                                            .motion_track_height,
                                                        crate::app::MOTION_TRACK_SIZE_RANGE,
                                                    )
                                                    .text(Text::PropMotionTrackHeight.tr(locale)),
                                                );
                                                ui.add(
                                                egui::Slider::new(
                                                    &mut app
                                                        .motion_track_region
                                                        .motion_track_search_radius,
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
                                    clip_id,
                                    Text::PropScaleKeyframes.tr(locale),
                                    Text::ScaleExportNote.tr(locale),
                                    !scale_keyframes.is_empty(),
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
                                    clip_id,
                                    Text::PropRotationKeyframes.tr(locale),
                                    Text::RotationExportNote.tr(locale),
                                    !rotation_keyframes.is_empty(),
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

                                let anchor_changed = components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropAnchor.tr(locale),
                                    Text::AnchorExportNote.tr(locale),
                                    (anchor_x - 0.5).abs() > 1e-4 || (anchor_y - 0.5).abs() > 1e-4,
                                    |ui| {
                                        let mut changed = false;
                                        changed |= ui
                                            .add(
                                                egui::Slider::new(&mut anchor_x, 0.0..=1.0)
                                                    .text(Text::KeyframeX.tr(locale)),
                                            )
                                            .changed();
                                        changed |= ui
                                            .add(
                                                egui::Slider::new(&mut anchor_y, 0.0..=1.0)
                                                    .text(Text::KeyframeY.tr(locale)),
                                            )
                                            .changed();
                                        changed
                                    },
                                );
                                if anchor_changed {
                                    app.set_selected_clip_anchor(anchor_x, anchor_y);
                                }

                                let mut new_opacity_keyframes = None;
                                if components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropOpacityKeyframes.tr(locale),
                                    Text::OpacityExportNote.tr(locale),
                                    !opacity_keyframes.is_empty(),
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
                    }
                });
        });
    });
}

/// Inspector/Effects/Audio tab strip (`editor-ui-visual-redesign.md`'s Inspector mapping) —
/// a pure regrouping of the property sections already drawn below, matching
/// [`tool_button`](super::tool_button)'s active/inactive chip styling so the panel's chrome
/// stays consistent with the toolbar's own tab-like controls.
fn properties_tab_bar(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    use crate::app::PropertiesTab;
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for (tab, label) in [
            (
                PropertiesTab::Inspector,
                Text::PropertiesTabInspector.tr(locale),
            ),
            (
                PropertiesTab::Effects,
                Text::PropertiesTabEffects.tr(locale),
            ),
            (PropertiesTab::Audio, Text::PropertiesTabAudio.tr(locale)),
        ] {
            let active = app.properties_tab == tab;
            let text = RichText::new(label).color(if active {
                theme::ACCENT
            } else {
                theme::TEXT_SECONDARY
            });
            let button = egui::Button::new(text)
                .fill(if active {
                    theme::ACCENT_TINT
                } else {
                    egui::Color32::TRANSPARENT
                })
                .stroke(egui::Stroke::new(
                    1.0,
                    if active {
                        theme::ACCENT
                    } else {
                        egui::Color32::TRANSPARENT
                    },
                ));
            if ui.add(button).clicked() {
                app.properties_tab = tab;
            }
        }
    });
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);
}

pub(super) fn prop_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.0).color(theme::TEXT_MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(12.0));
        });
    });
}

/// The OCA mockup's "vertical stereo (L/R) audio meter with dB ticks"
/// (`spec/architecture/editor-ui-visual-redesign.md`'s Inspector section) — a real per-channel
/// meter now that `avcore::preview`'s buffer probe reports one (`AudioLevel::peak_l`/`rms_l`/
/// `peak_r`/`rms_r`), not the same mono value drawn twice into two bars the doc explicitly
/// called out as the thing not to do. `DB_TICKS`/`DB_FLOOR` set a fixed -60..0 dB display range,
/// matching a small meter widget's scope rather than a full calibrated broadcast meter.
/// Renders at `meter_height` tall — the caller decides how much vertical space it gets. Moved
/// out of the Audio tab's own content (where it used to be tab-gated, `stereo_db_meter(ui,
/// app.current_audio_level())` inside `if tab == PropertiesTab::Audio`) into its own persistent
/// column, per `spec/architecture/editor-ui-visual-redesign.md`'s Inspector section: the OCA
/// mockup shows this meter as a fixed vertical strip along the whole editor body's right edge,
/// visible regardless of which Inspector/Effects/Audio tab is active or whether a clip is even
/// selected — not nested inside one tab's content. See `screens::editor::audio_meter_column`.
pub(super) fn stereo_db_meter(ui: &mut egui::Ui, level: avcore::AudioLevel, meter_height: f32) {
    const BAR_WIDTH: f32 = 16.0;
    const BAR_GAP: f32 = 4.0;
    const LABEL_WIDTH: f32 = 26.0;
    const DB_FLOOR: f32 = -60.0;
    const DB_TICKS: [f32; 5] = [0.0, -6.0, -12.0, -24.0, -48.0];

    fn amplitude_to_unit(amplitude: f32) -> f32 {
        if amplitude <= 0.0 {
            return 0.0;
        }
        let db = 20.0 * amplitude.log10();
        ((db - DB_FLOOR) / -DB_FLOOR).clamp(0.0, 1.0)
    }

    let (rect, _response) = ui.allocate_exact_size(
        egui::vec2(LABEL_WIDTH + BAR_WIDTH * 2.0 + BAR_GAP, meter_height),
        egui::Sense::hover(),
    );
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let bars_left = rect.left() + LABEL_WIDTH;
    let l_rect = egui::Rect::from_min_size(
        egui::pos2(bars_left, rect.top()),
        egui::vec2(BAR_WIDTH, meter_height),
    );
    let r_rect = egui::Rect::from_min_size(
        egui::pos2(bars_left + BAR_WIDTH + BAR_GAP, rect.top()),
        egui::vec2(BAR_WIDTH, meter_height),
    );

    for db in DB_TICKS {
        let unit = ((db - DB_FLOOR) / -DB_FLOOR).clamp(0.0, 1.0);
        let y = rect.bottom() - unit * meter_height;
        painter.text(
            egui::pos2(rect.left(), y),
            egui::Align2::LEFT_CENTER,
            format!("{db:.0}"),
            egui::FontId::monospace(8.5),
            theme::TEXT_MUTED,
        );
        painter.hline(
            l_rect.left()..=r_rect.right(),
            y,
            egui::Stroke::new(1.0, theme::BORDER.gamma_multiply(0.6)),
        );
    }

    for (bar_rect, peak, rms) in [
        (l_rect, level.peak_l, level.rms_l),
        (r_rect, level.peak_r, level.rms_r),
    ] {
        painter.rect_filled(bar_rect, 2.0, theme::SURFACE_2);
        let rms_unit = amplitude_to_unit(rms);
        if rms_unit > 0.0 {
            let filled_h = rms_unit * bar_rect.height();
            let filled_rect = egui::Rect::from_min_size(
                egui::pos2(bar_rect.left(), bar_rect.bottom() - filled_h),
                egui::vec2(bar_rect.width(), filled_h),
            );
            // Standard green/yellow/red audio-meter convention — was a flat `theme::ACCENT`
            // regardless of level (confirmed via a real screenshot: a thick, unvarying purple
            // bar, not a meter). `ACCENT`/Petroleum Blue means interaction, not a passive level
            // readout, per the design doc's own "don't flood the interface with the accent"
            // rule anyway.
            let fill_color = if rms_unit > 0.9 {
                theme::ERROR
            } else if rms_unit > 0.7 {
                theme::WARNING
            } else {
                theme::SUCCESS
            };
            painter.rect_filled(filled_rect, 2.0, fill_color);
        }
        let peak_unit = amplitude_to_unit(peak);
        if peak_unit > 0.0 {
            let peak_y = bar_rect.bottom() - peak_unit * bar_rect.height();
            let color = if peak > 0.98 {
                theme::ERROR
            } else {
                theme::ACCENT_2
            };
            painter.hline(bar_rect.x_range(), peak_y, egui::Stroke::new(2.0, color));
        }
    }
}
