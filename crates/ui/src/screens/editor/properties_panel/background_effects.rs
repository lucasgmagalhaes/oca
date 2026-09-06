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

use crate::app::{App, CHROMA_KEY_TOLERANCE_RANGE};
use crate::components;
use crate::i18n::{Locale, Text};

/// Renders the chroma-key controls and writes the changed clip settings.
pub(super) fn chroma_key_properties(
    app: &mut App,
    ui: &mut egui::Ui,
    enabled: &mut bool,
    color: &mut [u8; 4],
    tolerance: &mut f32,
    locale: Locale,
) {
    if components::property_block(ui, Text::ChromaKeyExportNote.tr(locale), |ui| {
        let mut changed = ui
            .checkbox(enabled, Text::PropChromaKey.tr(locale))
            .changed();
        if *enabled {
            ui.horizontal(|ui| {
                ui.label(Text::ChromaKeyColor.tr(locale));
                changed |= ui.color_edit_button_srgb(color).changed();
            });
            changed |= ui
                .add(
                    egui::Slider::new(tolerance, CHROMA_KEY_TOLERANCE_RANGE)
                        .text(Text::ChromaKeyTolerance.tr(locale)),
                )
                .changed();
        }
        changed
    }) {
        app.set_selected_clip_chroma_key(*enabled, *color, *tolerance);
    }
}

/// Renders background-removal controls and disables matte generation while it is in flight.
pub(super) fn background_removal_properties(
    app: &mut App,
    ui: &mut egui::Ui,
    enabled: &mut bool,
    locale: Locale,
) {
    if components::property_block(ui, Text::BackgroundRemovalExportNote.tr(locale), |ui| {
        let changed = ui
            .checkbox(enabled, Text::PropBackgroundRemoval.tr(locale))
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
    }) {
        app.set_selected_clip_background_removal(*enabled);
    }
}
