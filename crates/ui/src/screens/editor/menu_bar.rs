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

//! The Editor's top File/Edit/View/Sequence/Clip/Markers/Graphics/Help menu bar, per
//! `spec/architecture/editor-ui-visual-redesign.md`'s Top bar mapping — a second way to reach
//! actions the toolbar (`super::toolbar`) and per-clip/per-tab context menus already expose,
//! not new business logic of its own. Coexists with the toolbar (confirmed with the user before
//! building this — the doc left "does the toolbar still exist alongside it" as an open
//! decision); nothing is removed from it. `Window` still isn't built: no panel-layout-save/
//! restore UI exists behind it yet, and this file is wiring, not a place to invent one. `Help`
//! *is* built — it turned out the redesign doc's original "no About/docs dialog anywhere in the
//! app" finding was stale (Fase 8's auto-update work had already added one, reachable only from
//! Preferences until now) — see `help_menu` below.

use eframe::egui;

use crate::app::App;
use crate::i18n::Text;

pub(crate) fn menu_bar(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    egui::MenuBar::new().ui(ui, |ui| {
        file_menu(app, ui, locale);
        edit_menu(app, ui, locale);
        view_menu(app, ui, locale);
        sequence_menu(app, ui, locale);
        clip_menu(app, ui, locale);
        markers_menu(app, ui, locale);
        graphics_menu(app, ui, locale);
        analyze_menu(app, ui, locale);
        help_menu(app, ui, locale);
    });
}

fn file_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.menu_button(Text::MenuFile.tr(locale), |ui| {
        if ui
            .button(Text::Save.tr(locale))
            .on_hover_text("Ctrl+S")
            .clicked()
        {
            super::save_active_project(app);
            ui.close();
        }
        if ui.button(Text::Export.tr(locale)).clicked() {
            app.screen = crate::app::Screen::Queue;
            ui.close();
        }
        ui.separator();
        if ui
            .button(Text::ExportSrt.tr(locale))
            .on_hover_text(Text::ExportSrtHint.tr(locale))
            .clicked()
        {
            super::export_srt_for_active_sequence(app);
            ui.close();
        }
        if ui.button(Text::ExportCollabBundle.tr(locale)).clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("oca collaboration bundle", &["zip"])
                .set_file_name(format!("{}.zip", app.active_project().name))
                .save_file()
            {
                app.export_collab_bundle(path);
            }
            ui.close();
        }
        ui.separator();
        if ui.button(Text::ImportGameplayEvents.tr(locale)).clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("json", &["json"])
                .pick_file()
            {
                app.import_gameplay_events(path);
            }
            ui.close();
        }
    });
}

fn edit_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.menu_button(Text::MenuEdit.tr(locale), |ui| {
        if ui
            .add_enabled(
                app.can_undo(),
                egui::Button::new(Text::ShortcutUndo.tr(locale)),
            )
            .on_hover_text(app.prefs.key_bindings.undo.display())
            .clicked()
        {
            app.undo();
            ui.close();
        }
        if ui
            .add_enabled(
                app.can_redo(),
                egui::Button::new(Text::ShortcutRedo.tr(locale)),
            )
            .on_hover_text(app.prefs.key_bindings.redo.display())
            .clicked()
        {
            app.redo();
            ui.close();
        }
        ui.separator();
        if ui
            .button(Text::ContextMenuCopy.tr(locale))
            .on_hover_text("Ctrl+C")
            .clicked()
        {
            app.copy_selected_clip();
            ui.close();
        }
        if ui
            .button(Text::ContextMenuCut.tr(locale))
            .on_hover_text("Ctrl+X")
            .clicked()
        {
            app.cut_selected_clip();
            ui.close();
        }
        if ui
            .button(Text::ContextMenuPaste.tr(locale))
            .on_hover_text("Ctrl+V")
            .clicked()
        {
            app.paste_clip_at_playhead();
            ui.close();
        }
        ui.separator();
        if ui
            .button(Text::ContextMenuCopyFormatting.tr(locale))
            .on_hover_text(app.prefs.key_bindings.copy_formatting.display())
            .clicked()
        {
            app.copy_selected_clip_formatting();
            ui.close();
        }
        if ui
            .button(Text::ContextMenuPasteFormatting.tr(locale))
            .on_hover_text(app.prefs.key_bindings.paste_formatting.display())
            .clicked()
        {
            app.paste_selected_clip_formatting();
            ui.close();
        }
    });
}

fn view_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.menu_button(Text::MenuView.tr(locale), |ui| {
        if ui
            .checkbox(
                &mut app.timeline_index_open,
                Text::TimelineIndexToggle.tr(locale),
            )
            .clicked()
        {
            ui.close();
        }
        if ui
            .checkbox(
                &mut app.transcript_panel_open,
                Text::TranscriptPanelToggle.tr(locale),
            )
            .clicked()
        {
            ui.close();
        }
        if ui
            .checkbox(
                &mut app.preview_state.scopes_enabled,
                Text::PreviewScopesToggle.tr(locale),
            )
            .clicked()
        {
            ui.close();
        }
        ui.separator();
        if ui.button(Text::EnterFullscreenPreview.tr(locale)).clicked() {
            app.toggle_fullscreen_preview();
            ui.close();
        }
    });
}

fn sequence_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.menu_button(Text::MenuSequence.tr(locale), |ui| {
        let index = app.active_project().active_sequence;
        let count = app.active_project().sequences.len();
        let name = app.active_project().sequences[index].name.clone();

        if ui.button(Text::MenuSequenceAddTab.tr(locale)).clicked() {
            app.add_sequence();
            ui.close();
        }
        ui.separator();
        if ui.button(Text::MenuSequenceRename.tr(locale)).clicked() {
            app.renaming_sequence = Some((index, name.clone()));
            ui.close();
        }
        if ui.button(Text::MenuSequenceDuplicate.tr(locale)).clicked() {
            app.duplicate_sequence(index);
            ui.close();
        }
        if ui
            .add_enabled(
                index > 0,
                egui::Button::new(Text::MenuSequenceMoveLeft.tr(locale)),
            )
            .clicked()
        {
            app.move_sequence(index, index - 1);
            ui.close();
        }
        if ui
            .add_enabled(
                index + 1 < count,
                egui::Button::new(Text::MenuSequenceMoveRight.tr(locale)),
            )
            .clicked()
        {
            app.move_sequence(index, index + 1);
            ui.close();
        }
        ui.separator();
        if ui
            .add_enabled(
                count > 1,
                egui::Button::new(Text::MenuSequenceDelete.tr(locale)),
            )
            .clicked()
        {
            app.deleting_sequence = Some((app.active_project().sequences[index].id, name));
            ui.close();
        }
        ui.separator();
        if ui.button(Text::AddVideoTrack.tr(locale)).clicked() {
            app.add_video_track();
            ui.close();
        }
        if ui
            .button(Text::CreateMulticamGroup.tr(locale))
            .on_hover_text("1-9")
            .clicked()
        {
            app.create_multicam_group_from_video_tracks();
            ui.close();
        }
    });
}

fn clip_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
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
            for &[r, g, b] in super::timeline_panel::CLIP_COLOR_LABEL_PALETTE {
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

fn markers_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
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

fn graphics_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
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
fn analyze_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
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

/// "Help" — just the one real destination this app has: the About modal (version, update-check
/// status, install/restart flow — already fully built, `App::open_about`), previously only
/// reachable via Preferences. No separate "Documentation" entry: there's no hosted docs site for
/// this project to link to, and inventing one would be exactly the kind of unverified URL
/// CLAUDE.md's own rules say not to guess at.
fn help_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.menu_button(Text::MenuHelp.tr(locale), |ui| {
        if ui.button(Text::MenuHelpAbout.tr(locale)).clicked() {
            app.open_about();
            ui.close();
        }
    });
}
