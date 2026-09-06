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

pub(super) fn file_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.menu_button(Text::MenuFile.tr(locale), |ui| {
        if ui
            .button(Text::Save.tr(locale))
            .on_hover_text("Ctrl+S")
            .clicked()
        {
            super::super::save_active_project(app);
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
            super::super::export_srt_for_active_sequence(app);
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
        if ui.button(Text::ExportOtio.tr(locale)).clicked() {
            let sequence_name = app.active_project().active_sequence().name.clone();
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("OpenTimelineIO", &["otio"])
                .set_file_name(format!("{sequence_name}.otio"))
                .save_file()
            {
                app.export_otio_for_active_sequence(path);
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
        if ui.button(Text::ImportOtio.tr(locale)).clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("OpenTimelineIO", &["otio"])
                .pick_file()
            {
                app.import_otio_into_new_sequence(path, None);
            }
            ui.close();
        }
        if ui
            .button(Text::ImportOtioWithMediaRoot.tr(locale))
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("OpenTimelineIO", &["otio"])
                .pick_file()
            {
                let media_root = rfd::FileDialog::new().pick_folder();
                app.import_otio_into_new_sequence(path, media_root);
            }
            ui.close();
        }
    });
}

pub(super) fn sequence_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
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
        ui.separator();
        // The "drag an existing sequence tab onto another timeline" direction
        // `App::create_compound_clip_from_selected_clip`'s own doc comment flags as not shipped
        // -- every *other* sequence in this project, inserted as a compound clip onto the
        // active sequence's timeline (App::insert_sequence_as_compound_clip's own doc comment
        // has the full placement/cycle-safety contract).
        ui.menu_button(Text::MenuSequenceInsertAsCompoundClip.tr(locale), |ui| {
            let others: Vec<(u64, String)> = app
                .active_project()
                .sequences
                .iter()
                .filter(|s| s.id != app.active_project().active_sequence().id)
                .map(|s| (s.id, s.name.clone()))
                .collect();
            if others.is_empty() {
                ui.label(Text::MenuSequenceInsertAsCompoundClipEmpty.tr(locale));
            }
            for (sequence_id, name) in others {
                if ui.button(name).clicked() {
                    app.insert_sequence_as_compound_clip(sequence_id);
                    ui.close();
                }
            }
        });
    });
}
