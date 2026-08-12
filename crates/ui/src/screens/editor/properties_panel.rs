use eframe::egui::{self, RichText};

use crate::app::{
    OcaApp, BLUR_INTENSITY_RANGE, BRIGHTNESS_RANGE, CHROMA_KEY_TOLERANCE_RANGE, CONTRAST_RANGE,
    CROP_MIN_SIZE, GAIN_DB_RANGE, GLITCH_INTENSITY_RANGE, MASK_CORNER_RADIUS_RANGE,
    PIXELIZE_INTENSITY_RANGE, SATURATION_RANGE, SHAKE_INTENSITY_RANGE, SHARPEN_RANGE,
    SPEED_FACTOR_RANGE, VIGNETTE_INTENSITY_RANGE,
};
use crate::components;
use crate::i18n::Text;
use crate::theme;

pub(super) fn properties_panel(app: &mut OcaApp, ui: &mut egui::Ui, width: f32, height: f32) {
    let locale = app.locale;
    egui::Frame::new()
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(height);
            ui.vertical(|ui| {
                components::section_label(ui, Text::SelectedClip.tr(locale));
                let Some(asset) = app.selected_asset() else {
                    ui.label(
                        RichText::new(Text::NoClipSelected.tr(locale))
                            .color(theme::TEXT_MUTED),
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
                    let (_, target) = crate::app::LUFS_PROFILES[app.prefs.lufs_profile];
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
                    let mut speed_factor = clip.speed_factor;
                    let (mut crop_x, mut crop_y, mut crop_w, mut crop_h) =
                        (clip.crop_x, clip.crop_y, clip.crop_w, clip.crop_h);
                    let mut mask_shape = clip.mask_shape;
                    let mut mask_corner_radius = clip.mask_corner_radius;
                    let mut flipped_h = clip.flipped_h;
                    let mut color_filter = clip.color_filter;
                    let mut vignette_intensity = clip.vignette_intensity;
                    let (mut brightness, mut contrast, mut saturation) =
                        (clip.brightness, clip.contrast, clip.saturation);
                    let mut sharpen = clip.sharpen;
                    let mut chroma_key_enabled = clip.chroma_key_enabled;
                    let mut chroma_key_color = clip.chroma_key_color;
                    let mut chroma_key_tolerance = clip.chroma_key_tolerance;
                    let mut blur_intensity = clip.blur_intensity;
                    let mut shake_intensity = clip.shake_intensity;
                    let mut glitch_intensity = clip.glitch_intensity;
                    let mut pixelize_intensity = clip.pixelize_intensity;
                    let mut transition_in = clip.transition_in;
                    let mut transition_duration_secs = clip.transition_duration_secs;
                    let (mut zoom_start, mut zoom_end) = (clip.zoom_start, clip.zoom_end);
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
                    if app.selected_clip_track_kind()
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
                    if app.selected_clip_track_kind()
                        == Some(avcore::timeline::TrackKind::Video)
                    {
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
                                if ui.button(Text::CropReset.tr(locale)).clicked() {
                                    app.set_selected_clip_crop(0.0, 0.0, 1.0, 1.0);
                                }
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

                        if components::property_section(
                            ui,
                            Text::PropZoom.tr(locale),
                            Text::ZoomExportNote.tr(locale),
                            |ui| {
                                let mut changed = ui
                                    .add(
                                        egui::Slider::new(
                                            &mut zoom_start,
                                            crate::app::ZOOM_RANGE,
                                        )
                                        .text(Text::PropZoomStart.tr(locale)),
                                    )
                                    .changed();
                                changed |= ui
                                    .add(
                                        egui::Slider::new(
                                            &mut zoom_end,
                                            crate::app::ZOOM_RANGE,
                                        )
                                        .text(Text::PropZoomEnd.tr(locale)),
                                    )
                                    .changed();
                                changed
                            },
                        ) {
                            app.set_selected_clip_zoom(zoom_start, zoom_end);
                        }
                    }
                }
            });
        });
}

fn mask_shape_label(
    shape: avcore::timeline::MaskShape,
    locale: crate::i18n::Locale,
) -> String {
    match shape {
        avcore::timeline::MaskShape::None => Text::MaskNone.tr(locale).to_string(),
        avcore::timeline::MaskShape::Circle => Text::MaskCircle.tr(locale).to_string(),
        avcore::timeline::MaskShape::RoundedRect => {
            Text::MaskRoundedRect.tr(locale).to_string()
        }
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
        avcore::timeline::TransitionType::HardCut => {
            Text::TransitionHardCut.tr(locale).to_string()
        }
        avcore::timeline::TransitionType::Slide => Text::TransitionSlide.tr(locale).to_string(),
        avcore::timeline::TransitionType::Zoom => Text::TransitionZoom.tr(locale).to_string(),
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
