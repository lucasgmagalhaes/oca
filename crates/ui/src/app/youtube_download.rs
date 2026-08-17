use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use avcore::YoutubeDownloadTarget;

use super::{youtube_downloads_dir, App, YoutubeDownloadEvent, YoutubeFormatChoice};

impl App {
    /// Opens the "Baixar do YouTube" modal with an empty URL field — what the Mídia screen's
    /// button does. A no-op if the modal is already open or a download is already running.
    pub fn open_youtube_modal(&mut self) {
        if self.youtube_modal_url.is_some() || self.youtube_downloading {
            return;
        }
        self.youtube_modal_url = Some(String::new());
        self.youtube_download_error = None;
    }

    /// Closes the modal — what its Cancel/close button does. A no-op while a download is
    /// actually running (use [`App::request_cancel_youtube_download`] first); otherwise the
    /// modal would close while a background thread keeps writing progress nobody's watching.
    pub fn close_youtube_modal(&mut self) {
        if self.youtube_downloading {
            return;
        }
        self.youtube_modal_url = None;
        self.youtube_download_error = None;
    }

    /// Starts downloading [`App::youtube_modal_url`] on a background thread, at
    /// [`App::youtube_modal_format`]'s quality/bitrate — what the modal's "Baixar" button does.
    /// A no-op if a download is already running, the URL is blank, or `yt-dlp` isn't installed
    /// (surfaces [`App::youtube_download_error`] in that last case instead of ever spawning a
    /// thread that would just fail immediately). On success, the downloaded file is imported
    /// into the active project's media library the same way any other file drop is
    /// ([`App::spawn_import`], via [`App::pump_youtube_download`]).
    pub fn spawn_youtube_download(&mut self) {
        if self.youtube_downloading {
            return;
        }
        let Some(url) = self.youtube_modal_url.clone() else {
            return;
        };
        let url = url.trim().to_string();
        if url.is_empty() {
            return;
        }
        if !avcore::is_yt_dlp_available() {
            self.youtube_download_error = Some(
                crate::i18n::Text::YoutubeDownloadToolMissing
                    .tr(self.locale)
                    .to_string(),
            );
            return;
        }

        self.youtube_downloading = true;
        self.youtube_download_progress = 0.0;
        self.youtube_download_error = None;
        let cancel = Arc::new(AtomicBool::new(false));
        self.youtube_download_cancel = Some(Arc::clone(&cancel));

        let target = match self.youtube_modal_format {
            YoutubeFormatChoice::Mp4 => YoutubeDownloadTarget::Mp4(self.youtube_modal_mp4_quality),
            YoutubeFormatChoice::Mp3 => YoutubeDownloadTarget::Mp3(self.youtube_modal_mp3_bitrate),
        };
        let out_dir = youtube_downloads_dir();
        let tx = self.youtube_download_tx.clone();
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
        if let Some(cancel) = &self.youtube_download_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Applies YouTube-download progress/completion to app state. Called once per frame from
    /// [`eframe::App::ui`], same as [`App::pump_text_to_speech`].
    pub(super) fn pump_youtube_download(&mut self) {
        while let Ok(event) = self.youtube_download_rx.try_recv() {
            match event {
                YoutubeDownloadEvent::Progress(pct) => {
                    self.youtube_download_progress = pct;
                }
                YoutubeDownloadEvent::Done { path } => {
                    self.youtube_downloading = false;
                    self.youtube_download_cancel = None;
                    self.youtube_modal_url = None;
                    self.spawn_import(vec![path]);
                }
                YoutubeDownloadEvent::Failed { message } => {
                    self.youtube_downloading = false;
                    self.youtube_download_cancel = None;
                    tracing::error!(error = %message, "youtube download failed");
                    self.youtube_download_error = Some(message);
                }
            }
        }
    }
}
