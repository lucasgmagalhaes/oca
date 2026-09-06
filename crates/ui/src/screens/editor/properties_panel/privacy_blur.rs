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

/// Renders and applies the selected clip's privacy-blur controls.
pub(super) fn privacy_blur_properties(
    app: &mut App,
    ui: &mut egui::Ui,
    enabled: &mut bool,
    sigma: &mut f32,
    locale: Locale,
) {
    let mut sigma_changed = false;
    if components::property_block(ui, Text::PrivacyBlurExportNote.tr(locale), |ui| {
        let changed = ui
            .checkbox(enabled, Text::PropPrivacyBlur.tr(locale))
            .changed();
        sigma_changed = ui
            .add(
                egui::Slider::new(sigma, crate::app::PRIVACY_BLUR_SIGMA_RANGE)
                    .text(Text::PrivacyBlurSigma.tr(locale)),
            )
            .changed();
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut app.privacy_blur_region.privacy_blur_center_x)
                    .speed(0.01)
                    .range(0.0..=1.0)
                    .prefix("x "),
            );
            ui.add(
                egui::DragValue::new(&mut app.privacy_blur_region.privacy_blur_center_y)
                    .speed(0.01)
                    .range(0.0..=1.0)
                    .prefix("y "),
            );
            if ui.button(Text::PrivacyBlurRegionReset.tr(locale)).clicked() {
                app.privacy_blur_region.privacy_blur_center_x = 0.5;
                app.privacy_blur_region.privacy_blur_center_y = 0.5;
            }
        });
        ui.add(
            egui::Slider::new(
                &mut app.privacy_blur_region.privacy_blur_width,
                crate::app::MOTION_TRACK_SIZE_RANGE,
            )
            .text(Text::PrivacyBlurRegionWidth.tr(locale)),
        );
        ui.add(
            egui::Slider::new(
                &mut app.privacy_blur_region.privacy_blur_height,
                crate::app::MOTION_TRACK_SIZE_RANGE,
            )
            .text(Text::PrivacyBlurRegionHeight.tr(locale)),
        );
        let generating = app
            .privacy_blur_generation_state
            .privacy_blur_generating_clip_id
            .is_some();
        let label = if generating {
            Text::PrivacyBlurGenerating.tr(locale)
        } else {
            Text::PrivacyBlurApply.tr(locale)
        };
        if ui
            .add_enabled(!generating, egui::Button::new(label))
            .clicked()
        {
            app.spawn_apply_privacy_blur_for_selected_clip();
        }
        changed
    }) {
        app.set_selected_clip_privacy_blur_enabled(*enabled);
    }
    if sigma_changed {
        app.set_selected_clip_privacy_blur_sigma(*sigma);
    }
}
