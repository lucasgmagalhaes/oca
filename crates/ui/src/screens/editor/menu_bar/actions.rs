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
use crate::i18n::Text;

pub(super) fn clip_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.menu_button(Text::MenuClip.tr(locale), |ui| {
        if ui.button(Text::ContextMenuSplit.tr(locale)).clicked() {
            app.split_at_playhead();
            ui.close();
        }
        ui.menu_button(Text::MenuClipMode.tr(locale), |ui| {
            for (tool, text) in [
                (crate::app::EditorTool::Select, Text::ToolSelect),
                (crate::app::EditorTool::Trim, Text::ToolTrim),
                (crate::app::EditorTool::Ripple, Text::ToolRipple),
                (crate::app::EditorTool::Roll, Text::ToolRoll),
                (crate::app::EditorTool::Slip, Text::ToolSlip),
                (crate::app::EditorTool::Slide, Text::ToolSlide),
            ] {
                if ui
                    .selectable_label(app.tool == tool, text.tr(locale))
                    .clicked()
                {
                    app.tool = tool;
                    ui.close();
                }
            }
        });
        ui.separator();
        let has_selection = app.selected_clip_id.is_some();
        if ui
            .add_enabled(
                app.multi_selected_clip_ids.len() >= 2,
                egui::Button::new(Text::MergeIntoComposite.tr(locale)),
            )
            .on_hover_text(Text::MergeIntoCompositeHint.tr(locale))
            .clicked()
        {
            app.merge_into_composite();
            ui.close();
        }
        if ui
            .add_enabled(
                !app.multi_selected_clip_ids.is_empty(),
                egui::Button::new(Text::SaveAsTemplate.tr(locale)),
            )
            .on_hover_text(Text::SaveAsTemplateHint.tr(locale))
            .clicked()
        {
            app.begin_save_layer_template();
            ui.close();
        }
        if ui.button(Text::Templates.tr(locale)).clicked() {
            app.layer_templates_menu_open = true;
            ui.close();
        }
        ui.separator();
        if ui
            .add_enabled(
                has_selection,
                egui::Button::new(Text::ContextMenuDetachAudio.tr(locale)),
            )
            .clicked()
        {
            app.detach_audio_from_selected_clip();
            ui.close();
        }
        ui.menu_button(Text::ContextMenuSpeedRamp.tr(locale), |ui| {
            if ui
                .add_enabled(
                    has_selection,
                    egui::Button::new(Text::SpeedRampSlowToFast.tr(locale)),
                )
                .clicked()
            {
                app.apply_speed_ramp_to_selected_clip(0.5, 2.0, 4);
                ui.close();
            }
            if ui
                .add_enabled(
                    has_selection,
                    egui::Button::new(Text::SpeedRampFastToSlow.tr(locale)),
                )
                .clicked()
            {
                app.apply_speed_ramp_to_selected_clip(2.0, 0.5, 4);
                ui.close();
            }
        });
        ui.separator();
        ui.menu_button(Text::ContextMenuColorLabel.tr(locale), |ui| {
            for &[r, g, b] in super::super::timeline_panel::CLIP_COLOR_LABEL_PALETTE {
                let swatch = egui::Color32::from_rgb(r, g, b);
                if ui
                    .add_enabled(has_selection, egui::Button::new("  ").fill(swatch))
                    .clicked()
                {
                    if let Some(id) = app.selected_clip_id {
                        app.set_clip_color_label(id, Some([r, g, b]));
                    }
                    ui.close();
                }
            }
            ui.separator();
            if ui
                .add_enabled(
                    has_selection,
                    egui::Button::new(Text::ContextMenuColorLabelClear.tr(locale)),
                )
                .clicked()
            {
                if let Some(id) = app.selected_clip_id {
                    app.set_clip_color_label(id, None);
                }
                ui.close();
            }
        });
    });
}

pub(super) fn markers_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.menu_button(Text::MenuMarkers.tr(locale), |ui| {
        if ui
            .button(Text::TimelineIndexAddStandard.tr(locale))
            .clicked()
        {
            app.add_marker_at_playhead(avcore::MarkerKind::Standard);
            ui.close();
        }
        if ui.button(Text::TimelineIndexAddToDo.tr(locale)).clicked() {
            app.add_marker_at_playhead(avcore::MarkerKind::ToDo);
            ui.close();
        }
        if ui
            .button(Text::TimelineIndexAddChapter.tr(locale))
            .clicked()
        {
            app.add_marker_at_playhead(avcore::MarkerKind::Chapter);
            ui.close();
        }
        ui.separator();
        if ui
            .checkbox(
                &mut app.timeline_index_open,
                Text::TimelineIndexToggle.tr(locale),
            )
            .clicked()
        {
            ui.close();
        }
    });
}

pub(super) fn graphics_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.menu_button(Text::MenuGraphics.tr(locale), |ui| {
        if ui.button(Text::AddTextTrack.tr(locale)).clicked() {
            app.add_text_track();
            ui.close();
        }
        if ui.button(Text::AddTextClip.tr(locale)).clicked() {
            app.add_text_clip();
            ui.close();
        }
        ui.separator();
        if ui.button(Text::AddShapeTrack.tr(locale)).clicked() {
            app.add_shape_track();
            ui.close();
        }
        if ui.button(Text::AddShapeClip.tr(locale)).clicked() {
            app.add_shape_clip();
            ui.close();
        }
        if ui.button(Text::DrawCustomShape.tr(locale)).clicked() {
            if app.preview_state.preview_texture.is_some() {
                app.start_drawing_custom_shape();
            } else {
                app.push_toast(Text::ShapeDrawNeedsPreview.tr(locale).to_string());
            }
            ui.close();
        }
    });
}

/// Sequence-wide detection/analysis passes (silence, speech edits, chapters, highlights,
/// gameplay events, shorts pack) — moved here from the Editor toolbar (`super::toolbar`) to fix
/// a real overflow bug (the toolbar cut off past the visible window width at 1920px once every
/// tool/track/analyze/export action was crammed into one row) and to match the OCA mockup's
/// minimal tool rail, per `spec/architecture/editor-ui-visual-redesign.md`. Reach-for-
/// occasionally passes over the whole sequence, not moment-to-moment editing tools, so a menu
/// fits better than a permanent toolbar button anyway (progressive disclosure).
pub(super) fn analyze_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.menu_button(Text::AnalyzeMenu.tr(locale), |ui| {
        if ui.button(Text::DetectSilence.tr(locale)).clicked() {
            app.begin_silence_review();
            ui.close();
        }
        if ui.button(Text::DetectSpeechEdits.tr(locale)).clicked() {
            app.begin_transcript_proposals();
            ui.close();
        }
        if ui.button(Text::DetectChapters.tr(locale)).clicked() {
            app.spawn_detect_scene_cuts_for_selected_clip();
            ui.close();
        }
        if ui.button(Text::ExportChapters.tr(locale)).clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("text", &["txt"])
                .set_file_name("chapters.txt")
                .save_file()
            {
                app.export_chapters_txt(path);
            }
            ui.close();
        }
        if ui.button(Text::DetectHighlights.tr(locale)).clicked() {
            app.detect_highlights();
            ui.close();
        }
        if ui.button(Text::ImportGameplayEvents.tr(locale)).clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("json", &["json"])
                .pick_file()
            {
                app.import_gameplay_events(path);
            }
            ui.close();
        }
        if ui.button(Text::LoadGraphicTemplate.tr(locale)).clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("json", &["json"])
                .pick_file()
            {
                app.load_graphic_template_from_file(path);
            }
            ui.close();
        }
        ui.separator();
        if ui.button(Text::ShortsPack.tr(locale)).clicked() {
            let mut dialog = rfd::FileDialog::new();
            if !app.prefs.output_folder.is_empty() {
                dialog = dialog.set_directory(&app.prefs.output_folder);
            }
            if let Some(output_dir) = dialog.pick_folder() {
                app.spawn_shorts_pack(output_dir);
            }
            ui.close();
        }
    });
}
