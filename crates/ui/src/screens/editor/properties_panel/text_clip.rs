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

use crate::app::App;
use crate::components;
use crate::i18n::Text;
use crate::theme;

/// Renders the properties panel content for a selected text overlay clip. Shows controls for
/// text content, font size, RGBA color, X/Y position, start time, and duration. Applies
/// changes immediately by mutating the clip through the active project's timeline.
pub(super) fn text_clip_properties(
    app: &mut App,
    ui: &mut egui::Ui,
    tc_id: u64,
    locale: crate::i18n::Locale,
) {
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

    // Position keyframes -- animate pos_x/pos_y over this clip's own on-timeline duration (see
    // TextClip::pos_x_keyframes' doc comment).
    let mut new_pos_x_keyframes = None;
    if components::property_section(
        ui,
        Text::PropTextPosXKeyframes.tr(locale),
        Text::TextPositionKeyframesExportNote.tr(locale),
        |ui| {
            new_pos_x_keyframes = super::keyframe_editors::f32_keyframe_editor(
                ui,
                &tc.pos_x_keyframes,
                0.0..=1.0,
                tc.pos_x,
                locale,
            );
            new_pos_x_keyframes.is_some()
        },
    ) {
        if let Some(kfs) = new_pos_x_keyframes {
            tc.pos_x_keyframes = kfs;
            changed = true;
        }
    }
    let mut new_pos_y_keyframes = None;
    if components::property_section(
        ui,
        Text::PropTextPosYKeyframes.tr(locale),
        Text::TextPositionKeyframesExportNote.tr(locale),
        |ui| {
            new_pos_y_keyframes = super::keyframe_editors::f32_keyframe_editor(
                ui,
                &tc.pos_y_keyframes,
                0.0..=1.0,
                tc.pos_y,
                locale,
            );
            new_pos_y_keyframes.is_some()
        },
    ) {
        if let Some(kfs) = new_pos_y_keyframes {
            tc.pos_y_keyframes = kfs;
            changed = true;
        }
    }

    // Scale keyframes -- animate the text's overall size around its own baked position anchor
    // over this clip's own on-timeline duration (see TextClip::scale_keyframes' doc comment).
    let mut new_scale_keyframes = None;
    if components::property_section(
        ui,
        Text::PropTextScaleKeyframes.tr(locale),
        Text::TextScaleKeyframesExportNote.tr(locale),
        |ui| {
            new_scale_keyframes = super::keyframe_editors::f32_keyframe_editor(
                ui,
                &tc.scale_keyframes,
                crate::app::SCALE_RANGE,
                1.0,
                locale,
            );
            new_scale_keyframes.is_some()
        },
    ) {
        if let Some(kfs) = new_scale_keyframes {
            tc.scale_keyframes = kfs;
            changed = true;
        }
    }

    // Rotation keyframes -- animate the text's own clockwise rotation around its baked position
    // anchor over this clip's own on-timeline duration (see TextClip::rotation_keyframes' doc
    // comment).
    let mut new_rotation_keyframes = None;
    if components::property_section(
        ui,
        Text::PropTextRotationKeyframes.tr(locale),
        Text::TextRotationKeyframesExportNote.tr(locale),
        |ui| {
            new_rotation_keyframes = super::keyframe_editors::f32_keyframe_editor(
                ui,
                &tc.rotation_keyframes,
                -180.0..=180.0,
                0.0,
                locale,
            );
            new_rotation_keyframes.is_some()
        },
    ) {
        if let Some(kfs) = new_rotation_keyframes {
            tc.rotation_keyframes = kfs;
            changed = true;
        }
    }

    // Opacity keyframes -- fade the text in/out over its own on-timeline duration (see
    // TextClip::opacity_keyframes' doc comment).
    let mut new_opacity_keyframes = None;
    if components::property_section(
        ui,
        Text::PropTextOpacityKeyframes.tr(locale),
        Text::TextOpacityKeyframesExportNote.tr(locale),
        |ui| {
            new_opacity_keyframes = super::keyframe_editors::f32_keyframe_editor(
                ui,
                &tc.opacity_keyframes,
                0.0..=1.0,
                1.0,
                locale,
            );
            new_opacity_keyframes.is_some()
        },
    ) {
        if let Some(kfs) = new_opacity_keyframes {
            tc.opacity_keyframes = kfs;
            changed = true;
        }
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
