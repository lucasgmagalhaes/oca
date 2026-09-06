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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use avcore::YoutubeDownloadTarget;
use eframe::egui;

use crate::components;
use crate::i18n::Text;
use crate::theme;

use super::{youtube_downloads_dir, App, YoutubeDownloadEvent, YoutubeFormatChoice};

impl App {
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
    /// Opens the "Baixar do YouTube" modal with an empty URL field — what the Mídia screen's
    /// button does. A no-op if the modal is already open or a download is already running.
    pub fn open_youtube_modal(&mut self) {
        if self.youtube_download_state.youtube_modal_url.is_some()
            || self.youtube_download_state.youtube_downloading
        {
            return;
        }
        self.youtube_download_state.youtube_modal_url = Some(String::new());
        self.youtube_download_state.youtube_download_error = None;
    }

    /// Closes the modal — what its Cancel/close button does. A no-op while a download is
    /// actually running (use [`App::request_cancel_youtube_download`] first); otherwise the
    /// modal would close while a background thread keeps writing progress nobody's watching.
    pub fn close_youtube_modal(&mut self) {
        if self.youtube_download_state.youtube_downloading {
            return;
        }
        self.youtube_download_state.youtube_modal_url = None;
        self.youtube_download_state.youtube_download_error = None;
    }

    /// Starts downloading [`App::youtube_modal_url`] on a background thread, at
    /// [`App::youtube_modal_format`]'s quality/bitrate — what the modal's "Baixar" button does.
    /// A no-op if a download is already running, the URL is blank, or `yt-dlp` isn't installed
    /// (surfaces [`App::youtube_download_error`] in that last case instead of ever spawning a
    /// thread that would just fail immediately). On success, the downloaded file is imported
    /// into the active project's media library the same way any other file drop is
    /// ([`App::spawn_import`], via [`App::pump_youtube_download`]).
    pub fn spawn_youtube_download(&mut self) {
        if self.youtube_download_state.youtube_downloading {
            return;
        }
        let Some(url) = self.youtube_download_state.youtube_modal_url.clone() else {
            return;
        };
        let url = url.trim().to_string();
        if url.is_empty() {
            return;
        }
        if !avcore::is_yt_dlp_available() {
            self.youtube_download_state.youtube_download_error = Some(
                crate::i18n::Text::YoutubeDownloadToolMissing
                    .tr(self.locale)
                    .to_string(),
            );
            return;
        }

        self.youtube_download_state.youtube_downloading = true;
        self.youtube_download_state.youtube_download_progress = 0.0;
        self.youtube_download_state.youtube_download_error = None;
        let cancel = Arc::new(AtomicBool::new(false));
        self.youtube_download_state.youtube_download_cancel = Some(Arc::clone(&cancel));

        let target = match self.youtube_download_state.youtube_modal_format {
            YoutubeFormatChoice::Mp4 => {
                YoutubeDownloadTarget::Mp4(self.youtube_download_state.youtube_modal_mp4_quality)
            }
            YoutubeFormatChoice::Mp3 => {
                YoutubeDownloadTarget::Mp3(self.youtube_download_state.youtube_modal_mp3_bitrate)
            }
        };
        let out_dir = youtube_downloads_dir();
        let tx = self.youtube_download_state.youtube_download_tx.clone();
        std::thread::spawn(move || {
            let progress_tx = tx.clone();
            let result = avcore::download_youtube(&url, target, &out_dir, &cancel, move |pct| {
                let _ = progress_tx.send(YoutubeDownloadEvent::Progress(pct));
            });
            let event = match result {
                Ok(path) => YoutubeDownloadEvent::Done { path },
                Err(e) => YoutubeDownloadEvent::Failed {
                    message: e.to_string(),
                },
            };
            let _ = tx.send(event);
        });
    }

    /// Cancels the in-flight YouTube download, if any — what the modal's Cancel button (shown
    /// only while downloading) does. A no-op if nothing is downloading.
    pub fn request_cancel_youtube_download(&mut self) {
        if let Some(cancel) = &self.youtube_download_state.youtube_download_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Applies YouTube-download progress/completion to app state. Called once per frame from
    /// [`eframe::App::ui`], same as [`App::pump_text_to_speech`].
    pub(super) fn pump_youtube_download(&mut self) {
        while let Ok(event) = self.youtube_download_state.youtube_download_rx.try_recv() {
            match event {
                YoutubeDownloadEvent::Progress(pct) => {
                    self.youtube_download_state.youtube_download_progress = pct;
                }
                YoutubeDownloadEvent::Done { path } => {
                    self.youtube_download_state.youtube_downloading = false;
                    self.youtube_download_state.youtube_download_cancel = None;
                    self.youtube_download_state.youtube_modal_url = None;
                    self.spawn_import(vec![path]);
                }
                YoutubeDownloadEvent::Failed { message } => {
                    self.youtube_download_state.youtube_downloading = false;
                    self.youtube_download_state.youtube_download_cancel = None;
                    tracing::error!(error = %message, "youtube download failed");
                    self.youtube_download_state.youtube_download_error = Some(message);
                }
            }
        }
    }
}
