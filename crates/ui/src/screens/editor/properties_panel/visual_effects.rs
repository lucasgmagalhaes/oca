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

use crate::app::{
    App, BLUR_INTENSITY_RANGE, GLITCH_INTENSITY_RANGE, PIXELIZE_INTENSITY_RANGE,
    SHAKE_INTENSITY_RANGE, STABILIZATION_INTENSITY_RANGE, TRANSITION_DURATION_RANGE,
};
use crate::components;
use crate::i18n::{Locale, Text};

use super::keyframe_editors::transition_type_label;

pub(super) fn other_effects_properties(
    app: &mut App,
    ui: &mut egui::Ui,
    clip_id: u64,
    blur: &mut f32,
    shake: &mut f32,
    glitch: &mut f32,
    pixelize: &mut f32,
    locale: Locale,
) {
    if components::property_section(
        ui,
        clip_id,
        Text::PropOtherEffects.tr(locale),
        Text::OtherEffectsExportNote.tr(locale),
        *blur > 0.0 || *shake > 0.0 || *glitch > 0.0 || *pixelize > 0.0,
        |ui| {
            let mut changed = ui
                .add(egui::Slider::new(blur, BLUR_INTENSITY_RANGE).text(Text::PropBlur.tr(locale)))
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(shake, SHAKE_INTENSITY_RANGE)
                        .text(Text::PropShake.tr(locale)),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(glitch, GLITCH_INTENSITY_RANGE)
                        .text(Text::PropGlitch.tr(locale)),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(pixelize, PIXELIZE_INTENSITY_RANGE)
                        .text(Text::PropPixelize.tr(locale)),
                )
                .changed();
            changed
        },
    ) {
        app.set_selected_clip_blur(*blur);
        app.set_selected_clip_shake(*shake);
        app.set_selected_clip_glitch(*glitch);
        app.set_selected_clip_pixelize(*pixelize);
    }
}

pub(super) fn stabilization_properties(
    app: &mut App,
    ui: &mut egui::Ui,
    clip_id: u64,
    intensity: &mut f32,
    locale: Locale,
) {
    if components::property_section(
        ui,
        clip_id,
        Text::PropStabilization.tr(locale),
        Text::StabilizationExportNote.tr(locale),
        *intensity > 0.0,
        |ui| {
            ui.add(egui::Slider::new(intensity, STABILIZATION_INTENSITY_RANGE).fixed_decimals(2))
                .changed()
        },
    ) {
        app.set_selected_clip_stabilization(*intensity);
    }
}

pub(super) fn transition_properties(
    app: &mut App,
    ui: &mut egui::Ui,
    clip_id: u64,
    transition: &mut avcore::timeline::TransitionType,
    duration_secs: &mut f32,
    locale: Locale,
) {
    if components::property_section(
        ui,
        clip_id,
        Text::PropTransition.tr(locale),
        Text::TransitionExportNote.tr(locale),
        *transition != avcore::timeline::TransitionType::None,
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
                transition,
                |value| transition_type_label(value, locale),
            );
            changed |= ui
                .add(
                    egui::Slider::new(duration_secs, TRANSITION_DURATION_RANGE)
                        .suffix(" s")
                        .fixed_decimals(2)
                        .text(Text::PropTransitionDuration.tr(locale)),
                )
                .changed();
            changed
        },
    ) {
        app.set_selected_clip_transition(*transition, *duration_secs);
    }
}
