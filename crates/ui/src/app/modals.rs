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
