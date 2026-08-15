use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use avcore::model_download::{
    download_background_removal_model, download_reframe_model, download_whisper_model,
    DownloadOutcome, WhisperModelSize,
};

use super::{models_dir, App, ModelDownloadEvent, ModelKind};

impl App {
    /// Downloads `size`'s GGML model on a background thread — what Preferences' model-size
    /// buttons do. A no-op if a download is already running. On success, `prefs.whisper_model_path`
    /// is set to the downloaded file's path automatically (see [`App::pump_model_download`]) —
    /// no separate "now go pick the file" step.
    pub fn spawn_download_whisper_model(&mut self, size: WhisperModelSize) {
        if self.cancel_model_download.is_some() {
            return;
        }
        self.model_download_progress = Some((0, 0));
        self.model_download_kind = Some(ModelKind::Whisper);
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel_model_download = Some(Arc::clone(&cancel));

        let dest_dir = models_dir();
        let tx = self.model_download_tx.clone();
        std::thread::spawn(move || {
            let result = download_whisper_model(size, &dest_dir, &cancel, |downloaded, total| {
                let _ = tx.send(ModelDownloadEvent::Progress { downloaded, total });
            });
            let event = match result {
                Ok(DownloadOutcome::Completed(path)) => ModelDownloadEvent::Done {
                    kind: ModelKind::Whisper,
                    path,
                },
                Ok(DownloadOutcome::Cancelled) => ModelDownloadEvent::Cancelled,
                Err(e) => ModelDownloadEvent::Failed {
                    message: e.to_string(),
                },
            };
            let _ = tx.send(event);
        });
    }

    /// Downloads the UltraFace auto-reframe model on a background thread — what Preferences'
    /// "Baixar modelo" button (auto-reframe section) does. Same shape as
    /// [`App::spawn_download_whisper_model`], just a single fixed model rather than a size
    /// picker. A no-op if a download is already running.
    pub fn spawn_download_reframe_model(&mut self) {
        if self.cancel_model_download.is_some() {
            return;
        }
        self.model_download_progress = Some((0, 0));
        self.model_download_kind = Some(ModelKind::Reframe);
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel_model_download = Some(Arc::clone(&cancel));

        let dest_dir = models_dir();
        let tx = self.model_download_tx.clone();
        std::thread::spawn(move || {
            let result = download_reframe_model(&dest_dir, &cancel, |downloaded, total| {
                let _ = tx.send(ModelDownloadEvent::Progress { downloaded, total });
            });
            let event = match result {
                Ok(DownloadOutcome::Completed(path)) => ModelDownloadEvent::Done {
                    kind: ModelKind::Reframe,
                    path,
                },
                Ok(DownloadOutcome::Cancelled) => ModelDownloadEvent::Cancelled,
                Err(e) => ModelDownloadEvent::Failed {
                    message: e.to_string(),
                },
            };
            let _ = tx.send(event);
        });
    }

    /// Downloads the MODNet background-removal model on a background thread — what
    /// Preferences' "Baixar modelo" button (background-removal section) does. Same shape as
    /// [`App::spawn_download_reframe_model`]. A no-op if a download is already running.
    pub fn spawn_download_background_removal_model(&mut self) {
        if self.cancel_model_download.is_some() {
            return;
        }
        self.model_download_progress = Some((0, 0));
        self.model_download_kind = Some(ModelKind::BackgroundRemoval);
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel_model_download = Some(Arc::clone(&cancel));

        let dest_dir = models_dir();
        let tx = self.model_download_tx.clone();
        std::thread::spawn(move || {
            let result =
                download_background_removal_model(&dest_dir, &cancel, |downloaded, total| {
                    let _ = tx.send(ModelDownloadEvent::Progress { downloaded, total });
                });
            let event = match result {
                Ok(DownloadOutcome::Completed(path)) => ModelDownloadEvent::Done {
                    kind: ModelKind::BackgroundRemoval,
                    path,
                },
                Ok(DownloadOutcome::Cancelled) => ModelDownloadEvent::Cancelled,
                Err(e) => ModelDownloadEvent::Failed {
                    message: e.to_string(),
                },
            };
            let _ = tx.send(event);
        });
    }

    /// Cancels the in-flight model download, if any — what Preferences' Cancel button (shown
    /// only while a download is running) does. A no-op if nothing is downloading.
    pub fn request_cancel_model_download(&mut self) {
        if let Some(cancel) = &self.cancel_model_download {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Applies model-download progress/completion to app state. Called once per frame from
    /// [`eframe::App::ui`], same as [`App::pump_transcribe`].
    pub(super) fn pump_model_download(&mut self) {
        while let Ok(event) = self.model_download_rx.try_recv() {
            match event {
                ModelDownloadEvent::Progress { downloaded, total } => {
                    self.model_download_progress = Some((downloaded, total));
                }
                ModelDownloadEvent::Done { kind, path } => {
                    self.model_download_progress = None;
                    self.model_download_kind = None;
                    self.cancel_model_download = None;
                    tracing::info!(path = %path.display(), kind = ?kind, "model download complete");
                    match kind {
                        ModelKind::Whisper => {
                            self.prefs.whisper_model_path = path.display().to_string()
                        }
                        ModelKind::Reframe => {
                            self.prefs.reframe_model_path = path.display().to_string()
                        }
                        ModelKind::BackgroundRemoval => {
                            self.prefs.background_removal_model_path = path.display().to_string()
                        }
                    }
                    self.save_prefs();
                }
                ModelDownloadEvent::Cancelled => {
                    self.model_download_progress = None;
                    self.model_download_kind = None;
                    self.cancel_model_download = None;
                }
                ModelDownloadEvent::Failed { message } => {
                    self.model_download_progress = None;
                    self.model_download_kind = None;
                    self.cancel_model_download = None;
                    tracing::error!(error = %message, "model download failed");
                    self.push_toast(format!("Model download failed: {message}"));
                }
            }
        }
    }
}
