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

//! P4 item 22, "Smart bins" (`spec/ROADMAP.md`): rule-based media-pool folders in the Editor's
//! Library panel. See [`avcore::SmartBin`] for the filter model itself — this module is just the
//! `App`-level create/edit/select/delete plumbing and [`App::show_smart_bin_modal`].

use avcore::SmartBin;
use eframe::egui;

use crate::components;
use crate::i18n::Text;

use super::{App, MediaLibraryFilter};

impl App {
    /// Shows the create/edit modal for `editing_smart_bin` (P4 item 22, "Smart bins") — name,
    /// kind filter (Any/Video/Audio), file-name-contains text, and a has-audio tri-state, plus
    /// Save/Cancel and, for an existing bin (`id != 0`), Delete. Committed via
    /// [`App::commit_smart_bin_draft`]/discarded via [`App::cancel_smart_bin_draft`] on
    /// Escape/Cancel, same modal-lifecycle shape every other draft-editing modal here uses.
    pub(super) fn show_smart_bin_modal(&mut self, ctx: &egui::Context) {
        if self.editing_smart_bin.is_none() {
            return;
        }
        let locale = self.locale;
        let is_existing = self.editing_smart_bin.as_ref().is_some_and(|b| b.id != 0);
        let modal = egui::Modal::new(egui::Id::new("smart_bin_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let mut deleted = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(360.0);
            components::modal_title(ui, Text::SmartBinEditTitle.tr(locale));
            ui.add_space(8.0);
            let draft = self.editing_smart_bin.as_mut().unwrap();

            ui.label(Text::SmartBinNameLabel.tr(locale));
            let name_edit =
                ui.add(egui::TextEdit::singleline(&mut draft.name).desired_width(f32::INFINITY));
            name_edit.request_focus();
            ui.add_space(8.0);

            ui.label(Text::SmartBinKindLabel.tr(locale));
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut draft.kind_filter,
                    None,
                    Text::SmartBinKindAny.tr(locale),
                );
                ui.selectable_value(
                    &mut draft.kind_filter,
                    Some(avcore::MediaKind::Video),
                    Text::SmartBinKindVideo.tr(locale),
                );
                ui.selectable_value(
                    &mut draft.kind_filter,
                    Some(avcore::MediaKind::Audio),
                    Text::SmartBinKindAudio.tr(locale),
                );
            });
            ui.add_space(8.0);

            ui.label(Text::SmartBinNameContainsLabel.tr(locale));
            ui.add(
                egui::TextEdit::singleline(&mut draft.name_contains).desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);

            ui.label(Text::SmartBinAudioLabel.tr(locale));
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut draft.requires_audio,
                    None,
                    Text::SmartBinAudioAny.tr(locale),
                );
                ui.selectable_value(
                    &mut draft.requires_audio,
                    Some(true),
                    Text::SmartBinAudioYes.tr(locale),
                );
                ui.selectable_value(
                    &mut draft.requires_audio,
                    Some(false),
                    Text::SmartBinAudioNo.tr(locale),
                );
            });
            ui.add_space(8.0);

            if name_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                confirmed = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.horizontal(|ui| {
                if components::primary_button(ui, Text::SmartBinSave.tr(locale)).clicked() {
                    confirmed = true;
                }
                if ui.button(Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
                if is_existing && ui.button(Text::SmartBinDelete.tr(locale)).clicked() {
                    deleted = true;
                }
            });
        });

        if deleted {
            if let Some(bin_id) = self.editing_smart_bin.as_ref().map(|b| b.id) {
                self.delete_smart_bin(bin_id);
            }
            self.editing_smart_bin = None;
            return;
        }
        if response.should_close() || cancelled {
            self.cancel_smart_bin_draft();
            return;
        }
        if confirmed {
            self.commit_smart_bin_draft();
        }
    }
    /// Opens the smart-bin modal with a fresh, unnamed draft (`id == 0`) — what the Library
    /// panel's "+ New Bin" button does.
    pub fn begin_new_smart_bin(&mut self) {
        self.editing_smart_bin = Some(SmartBin::new(0, String::new()));
    }

    /// Opens the smart-bin modal pre-filled with `bin_id`'s current rules, for in-place editing
    /// — a no-op if `bin_id` doesn't exist.
    pub fn begin_edit_smart_bin(&mut self, bin_id: u64) {
        let Some(bin) = self
            .active_project()
            .smart_bins
            .iter()
            .find(|b| b.id == bin_id)
        else {
            return;
        };
        self.editing_smart_bin = Some(bin.clone());
    }

    /// Commits `editing_smart_bin` (a no-op if the draft's name is blank, or if `None` — nothing
    /// to commit): creates a new bin for an `id == 0` draft and switches the Library panel's
    /// active filter to it, or overwrites the existing bin's rules in place otherwise. Either
    /// way, closes the modal.
    pub fn commit_smart_bin_draft(&mut self) {
        let Some(mut draft) = self.editing_smart_bin.take() else {
            return;
        };
        draft.name = draft.name.trim().to_string();
        if draft.name.is_empty() {
            return;
        }

        if draft.id == 0 {
            let project = self.active_project_mut();
            let id = project.add_smart_bin(draft.name);
            if let Some(bin) = project.smart_bin_mut(id) {
                bin.kind_filter = draft.kind_filter;
                bin.name_contains = draft.name_contains;
                bin.requires_audio = draft.requires_audio;
            }
            self.media_filter = MediaLibraryFilter::SmartBin(id);
        } else if let Some(bin) = self.active_project_mut().smart_bin_mut(draft.id) {
            *bin = draft;
        }
    }

    /// Discards `editing_smart_bin` without saving — Escape/Cancel on the modal.
    pub fn cancel_smart_bin_draft(&mut self) {
        self.editing_smart_bin = None;
    }

    /// Removes `bin_id` and clears the Library panel's active filter if it was the one removed
    /// (a deleted bin can't stay "selected").
    pub fn delete_smart_bin(&mut self, bin_id: u64) {
        self.active_project_mut().remove_smart_bin(bin_id);
        if self.media_filter == MediaLibraryFilter::SmartBin(bin_id) {
            self.media_filter = MediaLibraryFilter::All;
        }
    }
}
