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

//! Dialogs that mutate project sequences and timeline tracks.

use eframe::egui;

use crate::components;
use crate::i18n;
use crate::theme;

use super::App;

impl App {
    /// Shows the rename-project modal when `renaming_project` is `Some`. Commits the new name on
    /// Enter or the "Rename" button, discards on Escape or "Cancel". If the project has a saved
    /// file on disk the updated project is written back immediately so the name persists.
    pub(super) fn show_rename_project_modal(&mut self, ctx: &egui::Context) {
        let Some((idx, _, _)) = self.renaming_project.as_ref() else {
            return;
        };
        let idx = *idx;
        let locale = self.locale;
        let modal = egui::Modal::new(egui::Id::new("rename_project_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(380.0);
            components::modal_title(ui, i18n::Text::RenameProjectTitle.tr(locale));
            ui.add_space(8.0);
            let (_, name_buf, summary_buf) = self.renaming_project.as_mut().unwrap();
            ui.label(i18n::Text::ProjectNameLabel.tr(locale));
            let name_edit =
                ui.add(egui::TextEdit::singleline(name_buf).desired_width(f32::INFINITY));
            name_edit.request_focus();
            if name_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                confirmed = true;
            }
            ui.add_space(4.0);
            ui.label(i18n::Text::ProjectSummaryLabel.tr(locale));
            ui.add(
                egui::TextEdit::multiline(summary_buf)
                    .desired_width(f32::INFINITY)
                    .desired_rows(3),
            );
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if components::primary_button(ui, i18n::Text::RenameProjectConfirm.tr(locale))
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(i18n::Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.renaming_project = None;
            return;
        }
        if confirmed {
            if let Some((_, new_name, new_summary)) = self.renaming_project.take() {
                let name = new_name.trim().to_string();
                let save_result = if !name.is_empty() {
                    self.open_projects.get_mut(idx).and_then(|project| {
                        project.name = name;
                        project.summary = new_summary.trim().to_string();
                        project
                            .file_path
                            .clone()
                            .map(|path| avcore::save_project_to_file(project, &path))
                    })
                } else {
                    None
                };
                if let Some(Err(error)) = save_result {
                    self.push_toast(format!("Failed to save project settings: {error}"));
                }
            }
        }
    }

    /// Shows the rename-sequence modal when `renaming_sequence` is `Some`. Commits on Enter or
    /// the Rename button; discards on Escape or Cancel. The active project is marked dirty so
    /// autosave and manual save pick it up.
    pub(super) fn show_rename_sequence_modal(&mut self, ctx: &egui::Context) {
        let Some((seq_idx, _)) = self.renaming_sequence.as_ref() else {
            return;
        };
        let seq_idx = *seq_idx;
        let locale = self.locale;
        let modal = egui::Modal::new(egui::Id::new("rename_sequence_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(320.0);
            components::modal_title(ui, i18n::Text::RenameSequenceTitle.tr(locale));
            ui.add_space(8.0);
            let buf = &mut self.renaming_sequence.as_mut().unwrap().1;
            let text_edit = ui.add(egui::TextEdit::singleline(buf).desired_width(f32::INFINITY));
            text_edit.request_focus();
            if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                confirmed = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if components::primary_button(ui, i18n::Text::RenameProjectConfirm.tr(locale))
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(i18n::Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.renaming_sequence = None;
            return;
        }
        if confirmed {
            if let Some((_, new_name)) = self.renaming_sequence.take() {
                let name = new_name.trim().to_string();
                let proj = self.active_project_mut();
                if !name.is_empty() && seq_idx < proj.sequences.len() {
                    proj.sequences[seq_idx].name = name;
                }
            }
        }
    }

    /// Confirms deletion of one sequence tab. The staged target is a stable sequence id, so
    /// reordering can never make this modal delete a different tab. [`App::delete_sequence`]
    /// and the core model both enforce the one-sequence minimum.
    pub(super) fn show_delete_sequence_modal(&mut self, ctx: &egui::Context) {
        let Some((sequence_id, name)) = self.deleting_sequence.clone() else {
            return;
        };
        let locale = self.locale;
        // Recomputed fresh every time this modal is open rather than cached alongside
        // `deleting_sequence` — cheap (a linear scan over every other sequence's clips), and
        // avoids the modal showing a stale answer if the referencing sequence changed while it
        // was open (e.g. the compound clip itself got deleted from another tab in the meantime).
        let referencing_names: Vec<String> = self
            .active_project()
            .sequences_referencing_as_compound_clip(sequence_id)
            .iter()
            .map(|s| s.name.clone())
            .collect();
        let modal = egui::Modal::new(egui::Id::new("delete_sequence_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(360.0);
            components::modal_title(ui, i18n::Text::DeleteSequenceTitle.tr(locale));
            ui.add_space(8.0);
            ui.label(i18n::delete_sequence_prompt(locale, &name));
            if !referencing_names.is_empty() {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(
                        i18n::Text::DeleteSequenceCompoundClipWarning
                            .tr(locale)
                            .replace("{sequences}", &referencing_names.join(", ")),
                    )
                    .color(theme::WARNING),
                );
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .button(i18n::Text::DeleteSequenceConfirm.tr(locale))
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(i18n::Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.deleting_sequence = None;
            return;
        }
        if confirmed {
            self.deleting_sequence = None;
            self.delete_sequence(sequence_id);
        }
    }

    /// Shows the rename-track modal when `renaming_track` is `Some` — Section 49's "Rename
    /// Track". Mirrors `show_rename_sequence_modal` exactly.
    pub(super) fn show_rename_track_modal(&mut self, ctx: &egui::Context) {
        let Some((track_id, _)) = self.renaming_track.as_ref() else {
            return;
        };
        let track_id = *track_id;
        let locale = self.locale;
        let modal = egui::Modal::new(egui::Id::new("rename_track_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(320.0);
            components::modal_title(ui, i18n::Text::RenameTrackTitle.tr(locale));
            ui.add_space(8.0);
            let buf = &mut self.renaming_track.as_mut().unwrap().1;
            let text_edit = ui.add(egui::TextEdit::singleline(buf).desired_width(f32::INFINITY));
            text_edit.request_focus();
            if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                confirmed = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if components::primary_button(ui, i18n::Text::RenameProjectConfirm.tr(locale))
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(i18n::Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.renaming_track = None;
            return;
        }
        if confirmed {
            if let Some((_, new_name)) = self.renaming_track.take() {
                self.rename_track(track_id, new_name);
            }
        }
    }

    /// Confirms deletion of one timeline track — Section 49's "Delete Track", gated on the
    /// spec's own "Deleting a track containing clips requires confirmation" rule (the caller,
    /// the track header's context menu, only stages this when the track actually has content;
    /// an empty track deletes immediately with no modal). Mirrors `show_delete_sequence_modal`.
    pub(super) fn show_delete_track_modal(&mut self, ctx: &egui::Context) {
        let Some((track_id, name)) = self.deleting_track.clone() else {
            return;
        };
        let locale = self.locale;
        let modal = egui::Modal::new(egui::Id::new("delete_track_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(360.0);
            components::modal_title(ui, i18n::Text::DeleteTrackTitle.tr(locale));
            ui.add_space(8.0);
            ui.label(i18n::delete_track_prompt(locale, &name));
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .button(i18n::Text::DeleteTrackConfirm.tr(locale))
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(i18n::Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.deleting_track = None;
            return;
        }
        if confirmed {
            self.deleting_track = None;
            self.delete_track(track_id);
        }
    }
}
