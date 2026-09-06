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

use eframe::egui::{self, RichText};

use crate::app::{
    App, BRIGHTNESS_RANGE, CONTRAST_RANGE, LAYER_SCALE_RANGE, SATURATION_RANGE, SHARPEN_RANGE,
    VIGNETTE_INTENSITY_RANGE,
};
use crate::components;
use crate::i18n::{Locale, Text};
use crate::theme;

use super::background_effects::{background_removal_properties, chroma_key_properties};
use super::chrome::effects_panel_browser;
use super::keyframe_editors::{color_filter_label, f32_keyframe_editor};
use super::privacy_blur::privacy_blur_properties;
use super::visual_effects::{
    other_effects_properties, stabilization_properties, transition_properties,
};

pub(super) fn effects_properties(app: &mut App, ui: &mut egui::Ui, clip_id: u64, locale: Locale) {
    if app.properties_tab != crate::app::PropertiesTab::Effects {
        return;
    }

    let Some(clip) = app.selected_clip() else {
        return;
    };
    let mut color_filter = clip.color_filter;
    let mut lut_path = clip.lut_path.clone();
    let (mut layer_scale_x, mut layer_scale_y) = (clip.layer_scale_x, clip.layer_scale_y);
    let mut vignette_intensity = clip.vignette_intensity;
    let (mut brightness, mut contrast, mut saturation) =
        (clip.brightness, clip.contrast, clip.saturation);
    let mut sharpen = clip.sharpen;
    let mut chroma_key_enabled = clip.chroma_key_enabled;
    let mut chroma_key_color = clip.chroma_key_color;
    let mut chroma_key_tolerance = clip.chroma_key_tolerance;
    let mut background_removal_enabled = clip.background_removal_enabled;
    let mut privacy_blur_enabled = clip.privacy_blur_enabled;
    let mut privacy_blur_sigma = clip.privacy_blur_sigma;
    let mut blur_intensity = clip.blur_intensity;
    let mut shake_intensity = clip.shake_intensity;
    let mut glitch_intensity = clip.glitch_intensity;
    let mut pixelize_intensity = clip.pixelize_intensity;
    let mut stabilization_intensity = clip.stabilization_intensity;
    let mut transition_in = clip.transition_in;
    let mut transition_duration_secs = clip.transition_duration_secs;
    let brightness_keyframes = clip.brightness_keyframes.clone();
    let contrast_keyframes = clip.contrast_keyframes.clone();
    let saturation_keyframes = clip.saturation_keyframes.clone();

    let effects_track_kind = app.selected_clip_track_kind();
    effects_panel_browser(app, ui, locale, true, effects_track_kind);
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

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
                    RichText::new(if name.is_empty() { "-" } else { &name })
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
                if !lut_path.is_empty() && ui.button(Text::ClearLut.tr(locale)).clicked() {
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
        (layer_scale_x - 1.0).abs() > 1e-4 || (layer_scale_y - 1.0).abs() > 1e-4,
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
        clip_id,
        Text::PropVignette.tr(locale),
        Text::VignetteExportNote.tr(locale),
        vignette_intensity > 0.0,
        |ui| {
            ui.add(
                egui::Slider::new(&mut vignette_intensity, VIGNETTE_INTENSITY_RANGE)
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
        brightness != 0.0 || (contrast - 1.0).abs() > 1e-4 || (saturation - 1.0).abs() > 1e-4,
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

    let mut new_brightness_keyframes = None;
    if components::property_section(
        ui,
        clip_id,
        Text::PropBrightnessKeyframes.tr(locale),
        Text::ColorKeyframesExportNote.tr(locale),
        !brightness_keyframes.is_empty(),
        |ui| {
            new_brightness_keyframes =
                f32_keyframe_editor(ui, &brightness_keyframes, BRIGHTNESS_RANGE, 0.0, locale);
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
            new_contrast_keyframes =
                f32_keyframe_editor(ui, &contrast_keyframes, CONTRAST_RANGE, 1.0, locale);
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
            new_saturation_keyframes =
                f32_keyframe_editor(ui, &saturation_keyframes, SATURATION_RANGE, 1.0, locale);
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
            ui.add(egui::Slider::new(&mut sharpen, SHARPEN_RANGE).fixed_decimals(2))
                .changed()
        },
    ) {
        app.set_selected_clip_sharpen(sharpen);
    }

    chroma_key_properties(
        app,
        ui,
        &mut chroma_key_enabled,
        &mut chroma_key_color,
        &mut chroma_key_tolerance,
        locale,
    );
    background_removal_properties(app, ui, &mut background_removal_enabled, locale);

    privacy_blur_properties(
        app,
        ui,
        &mut privacy_blur_enabled,
        &mut privacy_blur_sigma,
        locale,
    );

    other_effects_properties(
        app,
        ui,
        clip_id,
        &mut blur_intensity,
        &mut shake_intensity,
        &mut glitch_intensity,
        &mut pixelize_intensity,
        locale,
    );
    stabilization_properties(app, ui, clip_id, &mut stabilization_intensity, locale);
    transition_properties(
        app,
        ui,
        clip_id,
        &mut transition_in,
        &mut transition_duration_secs,
        locale,
    );
}
