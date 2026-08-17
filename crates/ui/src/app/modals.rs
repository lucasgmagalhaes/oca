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

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use tracing::{debug, error, info, warn};

use crate::i18n::{self, Text};
use crate::screens;
use crate::theme;

use super::export::next_available_path;
use super::App;

impl App {
    /// Queues a short-lived error message to be shown as a floating overlay at the bottom-right
    /// of the window. Replaces silent `eprintln!` calls for user-facing errors.
    pub fn push_toast(&mut self, message: String) {
        warn!(message = %message, "user-facing error toast");
        self.toasts.push((message, Instant::now()));
    }

    /// Renders any active toasts and evicts ones older than 4 seconds. Called from [`ui`] each
    /// frame; uses `egui::Area` in `Foreground` order so toasts float above all panels.
    pub(super) fn show_toasts(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        self.toasts
            .retain(|(_, born)| now.duration_since(*born) < Duration::from_secs(4));
        if self.toasts.is_empty() {
            return;
        }
        egui::Area::new(egui::Id::new("toasts"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    for (msg, born) in self.toasts.iter().rev().take(3) {
                        let age = now.duration_since(*born).as_secs_f32();
                        let alpha = if age > 3.0 { 1.0 - (age - 3.0) } else { 1.0 };
                        egui::Frame::new()
                            .fill(theme::ERROR.linear_multiply(alpha))
                            .corner_radius(6)
                            .inner_margin(egui::Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.set_max_width(360.0);
                                ui.label(
                                    egui::RichText::new(msg.as_str())
                                        .color(egui::Color32::WHITE.linear_multiply(alpha)),
                                );
                            });
                        ui.add_space(6.0);
                    }
                });
            });
    }

    /// Shows the preferences modal when `prefs_open` is set, overlaying whatever screen is
    /// currently active. Closing the modal (clicking outside or pressing Escape) clears the
    /// flag. Content is [`crate::screens::prefs::show`] — unchanged from when it was a full
    /// screen, just wrapped in an `egui::Modal` instead.
    pub(super) fn show_prefs_modal(&mut self, ctx: &egui::Context) {
        if !self.prefs_open {
            return;
        }
        let modal = egui::Modal::new(egui::Id::new("prefs_modal"));
        let response = modal.show(ctx, |ui| {
            ui.set_width(600.0);
            screens::prefs::show(self, ui);
        });
        if response.should_close() {
            self.prefs_open = false;
        }
    }

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
            ui.label(
                egui::RichText::new(i18n::Text::RenameProjectTitle.tr(locale))
                    .size(15.0)
                    .strong(),
            );
            ui.add_space(10.0);
            let (_, name_buf, summary_buf) = self.renaming_project.as_mut().unwrap();
            ui.label(i18n::Text::ProjectNameLabel.tr(locale));
            let name_edit =
                ui.add(egui::TextEdit::singleline(name_buf).desired_width(f32::INFINITY));
            name_edit.request_focus();
            if name_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                confirmed = true;
            }
            ui.add_space(6.0);
            ui.label(i18n::Text::ProjectSummaryLabel.tr(locale));
            ui.add(
                egui::TextEdit::multiline(summary_buf)
                    .desired_width(f32::INFINITY)
                    .desired_rows(3),
            );
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui
                    .button(i18n::Text::RenameProjectConfirm.tr(locale))
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
                if !name.is_empty() && idx < self.projects.len() {
                    self.projects[idx].name = name;
                    self.projects[idx].summary = new_summary.trim().to_string();
                    if let Some(path) = self.projects[idx].file_path.clone() {
                        if let Err(e) = avcore::save_project_to_file(&self.projects[idx], &path) {
                            self.push_toast(format!("Failed to save project settings: {e}"));
                        }
                    }
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
            ui.label(
                egui::RichText::new(i18n::Text::RenameSequenceTitle.tr(locale))
                    .size(15.0)
                    .strong(),
            );
            ui.add_space(10.0);
            let buf = &mut self.renaming_sequence.as_mut().unwrap().1;
            let text_edit = ui.add(egui::TextEdit::singleline(buf).desired_width(f32::INFINITY));
            text_edit.request_focus();
            if text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                confirmed = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui
                    .button(i18n::Text::RenameProjectConfirm.tr(locale))
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

    /// Writes the active project to a `<name>.autosave.ocproj` recovery file next to the
    /// project's own save file, subject to a 2-second idle debounce and a 30-second forced-save
    /// ceiling. Skips silently if the project has never been saved (no `file_path` yet) or
    /// hasn't changed. Serializes on the calling thread (fast, in-memory) then writes on a
    /// background thread so the UI never blocks on file I/O.
    pub(super) fn pump_autosave(&mut self) {
        if !self.project_dirty || self.projects.is_empty() {
            return;
        }
        let Some(file_path) = self.active_project().file_path.clone() else {
            return;
        };
        let now = Instant::now();
        let debounce_done = self
            .last_edit_instant
            .map(|t| now.duration_since(t) >= Duration::from_secs(2))
            .unwrap_or(false);
        let ceiling_hit = self
            .last_autosave_instant
            .map(|t| now.duration_since(t) >= Duration::from_secs(30))
            .unwrap_or(false);
        if !debounce_done && !ceiling_hit {
            return;
        }
        self.sync_panel_layout_into_active_project();
        let bytes = match avcore::persistence::to_ocproj_bytes(self.active_project()) {
            Ok(b) => b,
            Err(_) => return,
        };
        let autosave_path = file_path.with_extension("autosave.ocproj");
        debug!(path = %autosave_path.display(), "writing autosave");
        self.project_dirty = false;
        self.last_autosave_instant = Some(now);
        std::thread::spawn(move || {
            if let Err(e) = std::fs::write(&autosave_path, &bytes) {
                error!(path = %autosave_path.display(), error = %e, "autosave write failed");
            }
        });
    }

    /// If a newer autosave was found when the active project was opened ([`autosave_restore_pending`]
    /// is set), shows a modal offering to restore or discard it. Restore replaces the active
    /// project's data in place (keeping its `file_path` and id). Discard deletes the autosave file.
    pub(super) fn pump_autosave_restore(&mut self, ctx: &egui::Context) {
        let Some(autosave_path) = self.autosave_restore_pending.clone() else {
            return;
        };
        let locale = self.locale;
        let modal = egui::Modal::new(egui::Id::new("autosave_restore"));
        let response = modal.show(ctx, |ui| {
            ui.set_width(360.0);
            ui.label(Text::AutosaveFound.tr(locale));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button(Text::AutosaveRestore.tr(locale)).clicked() {
                    if let Ok(mut restored) = avcore::load_project_from_file(&autosave_path) {
                        let file_path = self.active_project().file_path.clone();
                        let id = self.active_project().id;
                        restored.file_path = file_path;
                        restored.id = id;
                        info!(path = %autosave_path.display(), "autosave restored");
                        *self.active_project_mut() = restored;
                        self.project_dirty = false;
                        self.load_panel_layout_for_active_project();
                    }
                    self.autosave_restore_pending = None;
                }
                if ui.button(Text::AutosaveDiscard.tr(locale)).clicked() {
                    info!(path = %autosave_path.display(), "autosave discarded");
                    let _ = std::fs::remove_file(&autosave_path);
                    self.autosave_restore_pending = None;
                }
            });
        });
        if response.should_close() {
            self.autosave_restore_pending = None;
        }
    }

    /// Shows the Overwrite/Rename/Cancel modal when [`App::pending_export_conflict`] is
    /// `Some` — the output path a queued export was about to use already exists on disk.
    /// Overwrite queues it as-is; Rename picks the first free `name (2).mp4`-style sibling via
    /// [`next_available_path`] and queues that instead; Cancel (or Escape) drops the job.
    pub(super) fn show_export_conflict_modal(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending_export_conflict.as_ref() else {
            return;
        };
        let locale = self.locale;
        let filename = pending
            .output_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| pending.output_path.display().to_string());
        let renamed_preview = next_available_path(&pending.output_path);
        let renamed_filename = renamed_preview
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| renamed_preview.display().to_string());

        let modal = egui::Modal::new(egui::Id::new("export_conflict_modal"));
        let mut choice: Option<bool> = None; // Some(true) = overwrite, Some(false) = rename
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(380.0);
            ui.label(
                egui::RichText::new(Text::ExportFileExistsTitle.tr(locale))
                    .size(15.0)
                    .strong(),
            );
            ui.add_space(8.0);
            ui.label(
                Text::ExportFileExistsBody
                    .tr(locale)
                    .replace("{name}", &filename),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    Text::ExportFileExistsRenamedTo
                        .tr(locale)
                        .replace("{name}", &renamed_filename),
                )
                .size(11.0)
                .color(theme::TEXT_MUTED),
            );
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui
                    .button(Text::ExportFileExistsOverwrite.tr(locale))
                    .clicked()
                {
                    choice = Some(true);
                }
                if ui.button(Text::ExportFileExistsRename.tr(locale)).clicked() {
                    choice = Some(false);
                }
                if ui.button(Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.pending_export_conflict = None;
            return;
        }
        if let Some(overwrite) = choice {
            let pending = self.pending_export_conflict.take().unwrap();
            let output_path = if overwrite {
                pending.output_path
            } else {
                next_available_path(&pending.output_path)
            };
            self.queue_export(
                pending.title,
                pending.track_segments,
                pending.audio_segments,
                pending.text_segments,
                pending.shape_segments,
                pending.canvas,
                pending.target_lufs,
                output_path.display().to_string(),
            );
        }
    }

    /// Flags that a project with `file_path` should offer autosave restoration on open, if
    /// `<file_path>.autosave.ocproj` exists and is newer than the project file itself.
    pub fn check_autosave_on_open(&mut self, file_path: &Path) {
        let autosave_path = file_path.with_extension("autosave.ocproj");
        if autosave_is_newer(&autosave_path, file_path) {
            self.autosave_restore_pending = Some(autosave_path);
        }
    }

    /// Shows the "save as template" naming modal when [`App::saving_layer_template`] is
    /// `Some`. Commits via [`App::commit_save_layer_template`] on Enter or the Save button
    /// (a no-op, leaving the modal open, while the name is blank); discards on Escape/Cancel.
    pub(super) fn show_save_layer_template_modal(&mut self, ctx: &egui::Context) {
        if self.saving_layer_template.is_none() {
            return;
        }
        let locale = self.locale;
        let modal = egui::Modal::new(egui::Id::new("save_layer_template_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(320.0);
            ui.label(
                egui::RichText::new(Text::SaveTemplateTitle.tr(locale))
                    .size(15.0)
                    .strong(),
            );
            ui.add_space(10.0);
            ui.label(Text::TemplateNameLabel.tr(locale));
            let buf = &mut self.saving_layer_template.as_mut().unwrap().1;
            let name_edit = ui.add(egui::TextEdit::singleline(buf).desired_width(f32::INFINITY));
            name_edit.request_focus();
            if name_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                confirmed = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let name_blank = self
                    .saving_layer_template
                    .as_ref()
                    .is_some_and(|(_, n)| n.trim().is_empty());
                if ui
                    .add_enabled(
                        !name_blank,
                        egui::Button::new(Text::SaveTemplateConfirm.tr(locale)),
                    )
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.saving_layer_template = None;
            return;
        }
        if confirmed {
            self.commit_save_layer_template();
        }
    }

    /// Shows the "Texto-pra-fala" modal when [`App::tts_modal_text`] is `Some`. "Gerar" starts
    /// the background synthesis ([`App::spawn_generate_tts`]) and closes the modal immediately
    /// (a no-op, leaving the modal open, while the buffer is blank or no voice model is
    /// configured); Escape/Cancel discards without generating anything.
    pub(super) fn show_tts_modal(&mut self, ctx: &egui::Context) {
        if self.tts_modal_text.is_none() {
            return;
        }
        let locale = self.locale;
        let model_configured = !self.prefs.tts_model_path.is_empty();
        let modal = egui::Modal::new(egui::Id::new("tts_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(420.0);
            ui.label(
                egui::RichText::new(Text::TtsModalTitle.tr(locale))
                    .size(15.0)
                    .strong(),
            );
            ui.add_space(10.0);
            if !model_configured {
                ui.label(
                    egui::RichText::new(Text::TtsNoModelConfigured.tr(locale))
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(10.0);
            }
            let buf = self.tts_modal_text.as_mut().unwrap();
            ui.add(
                egui::TextEdit::multiline(buf)
                    .desired_width(f32::INFINITY)
                    .desired_rows(4),
            );
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let text_blank = self
                    .tts_modal_text
                    .as_ref()
                    .is_some_and(|t| t.trim().is_empty());
                if ui
                    .add_enabled(
                        model_configured && !text_blank,
                        egui::Button::new(Text::TtsGenerate.tr(locale)),
                    )
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.tts_modal_text = None;
            return;
        }
        if confirmed {
            self.spawn_generate_tts();
        }
    }

    /// Shows the "Baixar do YouTube" modal when [`App::youtube_modal_url`] is `Some`. Unlike
    /// [`Self::show_tts_modal`], stays open across "Baixar" (submit) — it shows a progress bar
    /// while [`App::youtube_downloading`] is `true` and any [`App::youtube_download_error`]
    /// inline, closing only once a download actually completes
    /// ([`App::pump_youtube_download`]) or the user cancels/closes it explicitly.
    pub(super) fn show_youtube_download_modal(&mut self, ctx: &egui::Context) {
        if self.youtube_modal_url.is_none() {
            return;
        }
        let locale = self.locale;
        let downloading = self.youtube_downloading;
        let modal = egui::Modal::new(egui::Id::new("youtube_download_modal"));
        let mut start = false;
        let mut cancel_download = false;
        let mut close = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(420.0);
            ui.label(
                egui::RichText::new(Text::YoutubeDownloadModalTitle.tr(locale))
                    .size(15.0)
                    .strong(),
            );
            ui.add_space(10.0);
            ui.add_enabled_ui(!downloading, |ui| {
                let buf = self.youtube_modal_url.as_mut().unwrap();
                ui.add(
                    egui::TextEdit::singleline(buf)
                        .hint_text(Text::YoutubeDownloadUrlHint.tr(locale))
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.youtube_modal_format,
                        super::YoutubeFormatChoice::Mp4,
                        Text::YoutubeDownloadFormatMp4.tr(locale),
                    );
                    ui.selectable_value(
                        &mut self.youtube_modal_format,
                        super::YoutubeFormatChoice::Mp3,
                        Text::YoutubeDownloadFormatMp3.tr(locale),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label(Text::YoutubeDownloadQuality.tr(locale));
                    match self.youtube_modal_format {
                        super::YoutubeFormatChoice::Mp4 => {
                            egui::ComboBox::new("youtube_mp4_quality", "")
                                .selected_text(self.youtube_modal_mp4_quality.label())
                                .show_ui(ui, |ui| {
                                    for q in avcore::Mp4Quality::ALL {
                                        ui.selectable_value(
                                            &mut self.youtube_modal_mp4_quality,
                                            q,
                                            q.label(),
                                        );
                                    }
                                });
                        }
                        super::YoutubeFormatChoice::Mp3 => {
                            egui::ComboBox::new("youtube_mp3_bitrate", "")
                                .selected_text(self.youtube_modal_mp3_bitrate.label())
                                .show_ui(ui, |ui| {
                                    for b in avcore::Mp3Bitrate::ALL {
                                        ui.selectable_value(
                                            &mut self.youtube_modal_mp3_bitrate,
                                            b,
                                            b.label(),
                                        );
                                    }
                                });
                        }
                    }
                });
            });
            ui.add_space(10.0);
            if downloading {
                ui.add(
                    egui::ProgressBar::new(self.youtube_download_progress)
                        .text(Text::YoutubeDownloadInProgress.tr(locale)),
                );
                ui.add_space(10.0);
            }
            if let Some(err) = &self.youtube_download_error {
                ui.label(egui::RichText::new(err.as_str()).color(theme::ERROR));
                ui.add_space(10.0);
            }
            if !downloading && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
            ui.horizontal(|ui| {
                let url_blank = self
                    .youtube_modal_url
                    .as_ref()
                    .is_some_and(|u| u.trim().is_empty());
                if downloading {
                    if ui.button(Text::YoutubeDownloadCancel.tr(locale)).clicked() {
                        cancel_download = true;
                    }
                } else {
                    if ui
                        .add_enabled(
                            !url_blank,
                            egui::Button::new(Text::YoutubeDownloadStart.tr(locale)),
                        )
                        .clicked()
                    {
                        start = true;
                    }
                    if ui.button(Text::CancelJob.tr(locale)).clicked() {
                        close = true;
                    }
                }
            });
        });
        if (response.should_close() || close) && !downloading {
            self.close_youtube_modal();
            return;
        }
        if start {
            self.spawn_youtube_download();
        }
        if cancel_download {
            self.request_cancel_youtube_download();
        }
    }

    /// Shows the saved-templates list popup when [`App::layer_templates_menu_open`] is set —
    /// picking "Aplicar" on a row opens the apply-template modal ([`App::begin_apply_layer_template`])
    /// and closes this one; "🗑" deletes that entry immediately.
    pub(super) fn show_layer_templates_menu(&mut self, ctx: &egui::Context) {
        if !self.layer_templates_menu_open {
            return;
        }
        let locale = self.locale;
        let modal = egui::Modal::new(egui::Id::new("layer_templates_menu"));
        let mut apply_index = None;
        let mut delete_index = None;
        let response = modal.show(ctx, |ui| {
            ui.set_width(320.0);
            ui.label(
                egui::RichText::new(Text::Templates.tr(locale))
                    .size(15.0)
                    .strong(),
            );
            ui.add_space(10.0);
            if self.prefs.saved_layer_templates.is_empty() {
                ui.label(
                    egui::RichText::new(Text::NoSavedTemplates.tr(locale)).color(theme::TEXT_MUTED),
                );
            }
            for (index, template) in self.prefs.saved_layer_templates.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(&template.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(Text::DeleteTemplate.tr(locale)).clicked() {
                            delete_index = Some(index);
                        }
                        if ui.button(Text::ApplyTemplate.tr(locale)).clicked() {
                            apply_index = Some(index);
                        }
                    });
                });
            }
            ui.add_space(10.0);
            if ui.button(Text::CancelJob.tr(locale)).clicked() {
                self.layer_templates_menu_open = false;
            }
        });
        if response.should_close() {
            self.layer_templates_menu_open = false;
        }
        if let Some(index) = delete_index {
            self.delete_layer_template(index);
        }
        if let Some(index) = apply_index {
            self.begin_apply_layer_template(index);
        }
    }

    /// Shows the apply-template modal when [`App::applying_layer_template`] is `Some` — one
    /// asset dropdown per saved layer, filtered to media-library assets of that layer's own
    /// [`avcore::timeline::TrackKind`]. "Criar camadas" is only enabled once every slot has a
    /// pick; confirming calls [`App::confirm_apply_layer_template`].
    pub(super) fn show_apply_layer_template_modal(&mut self, ctx: &egui::Context) {
        let Some((template_index, _)) = self.applying_layer_template.as_ref() else {
            return;
        };
        let Some(template) = self
            .prefs
            .saved_layer_templates
            .get(*template_index)
            .cloned()
        else {
            self.applying_layer_template = None;
            return;
        };
        let locale = self.locale;
        let media_library = self.active_project().media_library.clone();
        let modal = egui::Modal::new(egui::Id::new("apply_layer_template_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(360.0);
            ui.label(
                egui::RichText::new(format!(
                    "{}: {}",
                    Text::ApplyTemplateTitle.tr(locale),
                    template.name
                ))
                .size(15.0)
                .strong(),
            );
            ui.add_space(10.0);
            let layer_asset_ids = &mut self.applying_layer_template.as_mut().unwrap().1;
            for (index, (kind, _formatting)) in template.layers.iter().enumerate() {
                let kind_label = match kind {
                    avcore::timeline::TrackKind::Video => Text::TrackKindVideo.tr(locale),
                    avcore::timeline::TrackKind::Audio => Text::TrackKindAudio.tr(locale),
                    avcore::timeline::TrackKind::Text => Text::TrackKindText.tr(locale),
                    avcore::timeline::TrackKind::Shape => Text::TrackKindShape.tr(locale),
                };
                ui.label(format!(
                    "{} {} ({kind_label})",
                    Text::ApplyTemplateLayerLabel.tr(locale),
                    index + 1
                ));
                let selected_name = layer_asset_ids[index]
                    .and_then(|id| media_library.iter().find(|a| a.id == id))
                    .map(|a| a.file_name.clone())
                    .unwrap_or_else(|| Text::ApplyTemplatePickAsset.tr(locale).to_string());
                egui::ComboBox::from_id_salt(("apply_template_layer_asset", index))
                    .selected_text(selected_name)
                    .show_ui(ui, |ui| {
                        for asset in media_library
                            .iter()
                            .filter(|a| media_kind_matches(a.kind, *kind))
                        {
                            ui.selectable_value(
                                &mut layer_asset_ids[index],
                                Some(asset.id),
                                &asset.file_name,
                            );
                        }
                    });
                ui.add_space(6.0);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let all_picked = layer_asset_ids.iter().all(|a| a.is_some());
                if ui
                    .add_enabled(
                        all_picked,
                        egui::Button::new(Text::ApplyTemplateConfirm.tr(locale)),
                    )
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.applying_layer_template = None;
            return;
        }
        if confirmed {
            self.confirm_apply_layer_template();
        }
    }
}

/// `true` if a media asset of `kind` belongs on a track of `track_kind` — video assets on video
/// tracks, audio on audio (text tracks never take a media asset directly, so always `false`
/// there).
fn media_kind_matches(kind: avcore::MediaKind, track_kind: avcore::timeline::TrackKind) -> bool {
    matches!(
        (kind, track_kind),
        (avcore::MediaKind::Video, avcore::timeline::TrackKind::Video)
            | (avcore::MediaKind::Audio, avcore::timeline::TrackKind::Audio)
    )
}

/// Returns `true` if `autosave_path` exists and has a modification time strictly newer than
/// `project_path`. Returns `false` if either file's metadata can't be read or the timestamps
/// can't be compared.
fn autosave_is_newer(autosave_path: &Path, project_path: &Path) -> bool {
    let Ok(as_meta) = std::fs::metadata(autosave_path) else {
        return false;
    };
    let Ok(proj_meta) = std::fs::metadata(project_path) else {
        return true;
    };
    let Ok(as_time) = as_meta.modified() else {
        return false;
    };
    let Ok(proj_time) = proj_meta.modified() else {
        return false;
    };
    as_time > proj_time
}
