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
use tracing::warn;

use crate::components;
use crate::i18n::{self, Text};
use crate::screens;
use crate::theme;

use super::color::{parse_color_value, COLOR_PRESETS};
use super::App;

const RELEASES_PAGE_URL: &str = "https://github.com/lucasgmagalhaes/oca/releases";

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
                            .corner_radius(theme::RADIUS_MD)
                            .inner_margin(egui::Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.set_max_width(360.0);
                                ui.label(
                                    egui::RichText::new(msg.as_str())
                                        .color(egui::Color32::WHITE.linear_multiply(alpha)),
                                );
                            });
                        ui.add_space(4.0);
                    }
                });
            });
    }

    /// Shows the reusable text-color editor. Changes stay in modal-local state until Apply,
    /// while the manual field accepts the same HEX/RGB(A) formats documented in its hint.
    pub(super) fn show_text_color_modal(&mut self, ctx: &egui::Context) {
        let Some(mut edit) = self.text_color_edit.take() else {
            return;
        };
        let locale = self.locale;
        let mut confirmed = false;
        let mut cancelled = false;
        let modal = egui::Modal::new(egui::Id::new("text_color_modal"));
        let response = modal.show(ctx, |ui| {
            ui.set_width(340.0);
            components::modal_title(ui, Text::TextColorPickerTitle.tr(locale));
            let target_label = match edit.target {
                super::TextColorTarget::Foreground => Text::PropTextColor.tr(locale),
                super::TextColorTarget::Background => Text::PropTextBackgroundColor.tr(locale),
                super::TextColorTarget::Highlight => Text::PropTextHighlightColor.tr(locale),
            };
            ui.label(egui::RichText::new(target_label).color(theme::TEXT_MUTED));
            ui.add_space(8.0);

            let mut color = egui::Color32::from_rgba_unmultiplied(
                edit.rgba[0],
                edit.rgba[1],
                edit.rgba[2],
                edit.rgba[3],
            );
            if egui::color_picker::color_picker_color32(
                ui,
                &mut color,
                egui::color_picker::Alpha::OnlyBlend,
            ) {
                edit.rgba = color.to_srgba_unmultiplied();
                edit.manual_input = super::format_color_hex(edit.rgba);
                edit.manual_invalid = false;
            }

            ui.add_space(8.0);
            ui.label(Text::TextColorPickerPresets.tr(locale));
            for row in COLOR_PRESETS.chunks(8) {
                ui.horizontal(|ui| {
                    for &rgba in row {
                        let preset = egui::Color32::from_rgba_unmultiplied(
                            rgba[0], rgba[1], rgba[2], rgba[3],
                        );
                        let selected = rgba == edit.rgba;
                        let response = ui
                            .add(
                                egui::Button::new("")
                                    .fill(preset)
                                    .stroke(egui::Stroke::new(
                                        if selected { 2.0 } else { 1.0 },
                                        if selected {
                                            theme::ACCENT
                                        } else {
                                            theme::BORDER
                                        },
                                    ))
                                    .min_size(egui::vec2(30.0, 24.0)),
                            )
                            .on_hover_text(super::format_color_hex(rgba));
                        if response.clicked() {
                            edit.rgba = rgba;
                            edit.manual_input = super::format_color_hex(rgba);
                            edit.manual_invalid = false;
                        }
                    }
                });
            }

            ui.add_space(8.0);
            ui.label(Text::TextColorPickerManual.tr(locale));
            let manual_response = ui.add(
                egui::TextEdit::singleline(&mut edit.manual_input)
                    .desired_width(f32::INFINITY)
                    .hint_text(Text::TextColorPickerManualHint.tr(locale)),
            );
            let use_manual = ui
                .button(Text::TextColorPickerUseValue.tr(locale))
                .clicked()
                || (manual_response.lost_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter)));
            if use_manual {
                match parse_color_value(&edit.manual_input) {
                    Ok(rgba) => {
                        edit.rgba = rgba;
                        edit.manual_input = super::format_color_hex(rgba);
                        edit.manual_invalid = false;
                    }
                    Err(()) => edit.manual_invalid = true,
                }
            }
            if edit.manual_invalid {
                ui.label(
                    egui::RichText::new(Text::TextColorPickerInvalid.tr(locale))
                        .color(theme::ERROR),
                );
            }

            if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if components::primary_button(ui, Text::TextColorPickerApply.tr(locale)).clicked() {
                    match parse_color_value(&edit.manual_input) {
                        Ok(rgba) => {
                            edit.rgba = rgba;
                            edit.manual_input = super::format_color_hex(rgba);
                            edit.manual_invalid = false;
                            confirmed = true;
                        }
                        Err(()) => edit.manual_invalid = true,
                    }
                }
                if ui.button(Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });

        if response.should_close() || cancelled {
            return;
        }
        self.text_color_edit = Some(edit);
        if confirmed {
            self.confirm_text_color_edit();
        }
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

    /// Opens the About modal and closes Preferences so the two modal layers never stack.
    pub fn open_about(&mut self) {
        self.prefs_open = false;
        self.about_open = true;
    }

    /// Closes the About modal without changing update-check state.
    pub fn close_about(&mut self) {
        self.about_open = false;
    }

    /// Shows the installed version and drives the explicit download/install/restart flow. The
    /// releases link stays useful throughout, including on unsupported platforms and failures.
    pub(super) fn show_about_modal(&mut self, ctx: &egui::Context) {
        if !self.about_open {
            return;
        }

        let locale = self.locale;
        let status = self.update_check_status.clone();
        let mut close = false;
        let mut install = false;
        let mut restart = false;
        let response = egui::Modal::new(egui::Id::new("about_modal")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(Text::AppName.tr(locale))
                        .size(28.0)
                        .strong()
                        .color(theme::ACCENT),
                );
                ui.label(
                    egui::RichText::new(Text::AboutDescription.tr(locale))
                        .color(theme::TEXT_SECONDARY),
                );
            });
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(Text::AboutInstalledVersion.tr(locale));
                ui.label(egui::RichText::new(env!("CARGO_PKG_VERSION")).strong());
            });
            ui.add_space(8.0);

            let release_url = match &status {
                super::UpdateCheckStatus::Checking => {
                    ui.label(Text::AboutCheckingUpdates.tr(locale));
                    RELEASES_PAGE_URL
                }
                super::UpdateCheckStatus::UpToDate => {
                    ui.label(Text::AboutUpToDate.tr(locale));
                    RELEASES_PAGE_URL
                }
                super::UpdateCheckStatus::Failed => {
                    ui.label(Text::AboutCheckFailed.tr(locale));
                    RELEASES_PAGE_URL
                }
                super::UpdateCheckStatus::Available(update) => {
                    ui.label(format!(
                        "{} {}",
                        Text::UpdateAvailable.tr(locale),
                        update.version
                    ));
                    if !avcore::auto_update_supported() || !update.auto_update_available {
                        ui.add_space(4.0);
                        ui.label(Text::AboutManualInstallOnly.tr(locale));
                    }
                    &update.html_url
                }
                super::UpdateCheckStatus::Installing(update) => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(Text::AboutInstalling.tr(locale));
                    });
                    &update.html_url
                }
                super::UpdateCheckStatus::RestartRequired(update) => {
                    ui.label(format!(
                        "{} {}.",
                        Text::AboutRestartRequired.tr(locale),
                        update.version
                    ));
                    &update.html_url
                }
                super::UpdateCheckStatus::InstallFailed(update) => {
                    ui.label(
                        egui::RichText::new(Text::AboutInstallFailed.tr(locale))
                            .color(theme::ERROR),
                    );
                    &update.html_url
                }
            };
            ui.add_space(8.0);
            ui.hyperlink_to(Text::AboutViewReleases.tr(locale), release_url);
            ui.add_space(16.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(Text::WindowClose.tr(locale)).clicked() {
                    close = true;
                }
                match &status {
                    super::UpdateCheckStatus::Available(update)
                        if avcore::auto_update_supported() && update.auto_update_available =>
                    {
                        if ui.button(Text::AboutDownloadInstall.tr(locale)).clicked() {
                            install = true;
                        }
                    }
                    super::UpdateCheckStatus::InstallFailed(update)
                        if avcore::auto_update_supported() && update.auto_update_available =>
                    {
                        if ui.button(Text::AboutRetryInstall.tr(locale)).clicked() {
                            install = true;
                        }
                    }
                    super::UpdateCheckStatus::RestartRequired(_)
                        if ui.button(Text::AboutRestartNow.tr(locale)).clicked() =>
                    {
                        restart = true;
                    }
                    _ => {}
                }
            });
        });

        if install {
            self.install_available_update();
        }
        if restart {
            self.restart_after_update(ctx);
        }
        if response.should_close() || close {
            self.close_about();
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
            components::modal_title(ui, Text::SaveTemplateTitle.tr(locale));
            ui.add_space(8.0);
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
            ui.add_space(8.0);
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
        if self.tts_state.tts_modal_text.is_none() {
            return;
        }
        let locale = self.locale;
        let model_configured = !self.prefs.tts_model_path.is_empty();
        let modal = egui::Modal::new(egui::Id::new("tts_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(420.0);
            components::modal_title(ui, Text::TtsModalTitle.tr(locale));
            ui.add_space(8.0);
            if !model_configured {
                ui.label(
                    egui::RichText::new(Text::TtsNoModelConfigured.tr(locale))
                        .color(theme::TEXT_DISABLED),
                );
                ui.add_space(8.0);
            }
            let buf = self.tts_state.tts_modal_text.as_mut().unwrap();
            ui.add(
                egui::TextEdit::multiline(buf)
                    .desired_width(f32::INFINITY)
                    .desired_rows(4),
            );
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let text_blank = self
                    .tts_state
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
            self.tts_state.tts_modal_text = None;
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
        if self.youtube_download_state.youtube_modal_url.is_none() {
            return;
        }
        let locale = self.locale;
        let downloading = self.youtube_download_state.youtube_downloading;
        let modal = egui::Modal::new(egui::Id::new("youtube_download_modal"));
        let mut start = false;
        let mut cancel_download = false;
        let mut close = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(420.0);
            components::modal_title(ui, Text::YoutubeDownloadModalTitle.tr(locale));
            ui.add_space(8.0);
            ui.add_enabled_ui(!downloading, |ui| {
                let buf = self
                    .youtube_download_state
                    .youtube_modal_url
                    .as_mut()
                    .unwrap();
                ui.add(
                    egui::TextEdit::singleline(buf)
                        .hint_text(Text::YoutubeDownloadUrlHint.tr(locale))
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.youtube_download_state.youtube_modal_format,
                        super::YoutubeFormatChoice::Mp4,
                        Text::YoutubeDownloadFormatMp4.tr(locale),
                    );
                    ui.selectable_value(
                        &mut self.youtube_download_state.youtube_modal_format,
                        super::YoutubeFormatChoice::Mp3,
                        Text::YoutubeDownloadFormatMp3.tr(locale),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label(Text::YoutubeDownloadQuality.tr(locale));
                    match self.youtube_download_state.youtube_modal_format {
                        super::YoutubeFormatChoice::Mp4 => {
                            egui::ComboBox::new("youtube_mp4_quality", "")
                                .selected_text(
                                    self.youtube_download_state
                                        .youtube_modal_mp4_quality
                                        .label(),
                                )
                                .show_ui(ui, |ui| {
                                    for q in avcore::Mp4Quality::ALL {
                                        ui.selectable_value(
                                            &mut self
                                                .youtube_download_state
                                                .youtube_modal_mp4_quality,
                                            q,
                                            q.label(),
                                        );
                                    }
                                });
                        }
                        super::YoutubeFormatChoice::Mp3 => {
                            egui::ComboBox::new("youtube_mp3_bitrate", "")
                                .selected_text(
                                    self.youtube_download_state
                                        .youtube_modal_mp3_bitrate
                                        .label(),
                                )
                                .show_ui(ui, |ui| {
                                    for b in avcore::Mp3Bitrate::ALL {
                                        ui.selectable_value(
                                            &mut self
                                                .youtube_download_state
                                                .youtube_modal_mp3_bitrate,
                                            b,
                                            b.label(),
                                        );
                                    }
                                });
                        }
                    }
                });
            });
            ui.add_space(8.0);
            if downloading {
                ui.add(
                    egui::ProgressBar::new(self.youtube_download_state.youtube_download_progress)
                        .text(Text::YoutubeDownloadInProgress.tr(locale)),
                );
                ui.add_space(8.0);
            }
            if let Some(err) = &self.youtube_download_state.youtube_download_error {
                ui.label(egui::RichText::new(err.as_str()).color(theme::ERROR));
                ui.add_space(8.0);
            }
            if !downloading && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
            ui.horizontal(|ui| {
                let url_blank = self
                    .youtube_download_state
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
            components::modal_title(ui, Text::Templates.tr(locale));
            ui.add_space(8.0);
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
                        if components::primary_button(ui, Text::ApplyTemplate.tr(locale)).clicked()
                        {
                            apply_index = Some(index);
                        }
                    });
                });
            }
            ui.add_space(8.0);
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
            components::modal_title(
                ui,
                &format!("{}: {}", Text::ApplyTemplateTitle.tr(locale), template.name),
            );
            ui.add_space(8.0);
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
                ui.add_space(4.0);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(4.0);
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

    /// Shows the parameter fill-in modal when [`App::pending_graphic_template_apply`] is `Some`
    /// (CF-07 slice 3) — one row per declared [`avcore::motion_template::TemplateParameter`], a
    /// plain text field for `Text` and a color picker for `Color`. Confirming calls
    /// [`App::confirm_apply_graphic_template`]; cancelling or Escape calls [`App::
    /// cancel_apply_graphic_template`].
    pub(super) fn show_apply_graphic_template_modal(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending_graphic_template_apply.as_ref() else {
            return;
        };
        let locale = self.locale;
        let template_name = pending.template.name.clone();
        let parameters = pending.template.parameters.clone();
        let modal = egui::Modal::new(egui::Id::new("apply_graphic_template_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(340.0);
            components::modal_title(
                ui,
                &format!(
                    "{}: {}",
                    Text::GraphicTemplateApplyTitle.tr(locale),
                    template_name
                ),
            );
            ui.add_space(8.0);
            let pending = self.pending_graphic_template_apply.as_mut().unwrap();
            for parameter in &parameters {
                ui.label(&parameter.label);
                match parameter.kind {
                    avcore::motion_template::TemplateParameterKind::Text => {
                        let value = pending.text_values.entry(parameter.id.clone()).or_default();
                        ui.text_edit_singleline(value);
                    }
                    avcore::motion_template::TemplateParameterKind::Color => {
                        let value = pending
                            .color_values
                            .entry(parameter.id.clone())
                            .or_insert([255, 255, 255, 255]);
                        let mut color = egui::Color32::from_rgba_premultiplied(
                            value[0], value[1], value[2], value[3],
                        );
                        if ui.color_edit_button_srgba(&mut color).changed() {
                            *value = [color.r(), color.g(), color.b(), color.a()];
                        }
                    }
                }
                ui.add_space(4.0);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if components::primary_button(ui, Text::GraphicTemplateApplyConfirm.tr(locale))
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
            self.cancel_apply_graphic_template();
            return;
        }
        if confirmed {
            self.confirm_apply_graphic_template();
        }
    }

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
