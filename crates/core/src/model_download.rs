//! Downloads a local GGML Whisper model on demand — closes the gap where `transcribe()`
//! (see [`crate::transcribe`]) needs a model file the app never had any way to actually obtain.
//! Fase 8's packaging plan calls for bundling the model inside the installer eventually
//! ("Modelo do Whisper... incluídos no instalador"); until that's built, this is how a fresh
//! install gets one — a one-time download the user (or `ui`'s "Transcrever" flow) triggers,
//! not a build-time or install-time step.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// A downloadable Whisper model size, from <https://huggingface.co/ggerganov/whisper.cpp>.
/// `approx_size_mb` is for display only (a progress bar with no total yet, or a picker showing
/// roughly what a choice costs to download) — the real size comes from the download response's
/// `Content-Length`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhisperModelSize {
    Tiny,
    Base,
    Small,
}

impl WhisperModelSize {
    pub const ALL: [WhisperModelSize; 3] = [
        WhisperModelSize::Tiny,
        WhisperModelSize::Base,
        WhisperModelSize::Small,
    ];

    pub fn filename(self) -> &'static str {
        match self {
            WhisperModelSize::Tiny => "ggml-tiny.bin",
            WhisperModelSize::Base => "ggml-base.bin",
            WhisperModelSize::Small => "ggml-small.bin",
        }
    }

    pub fn approx_size_mb(self) -> u32 {
        match self {
            WhisperModelSize::Tiny => 75,
            WhisperModelSize::Base => 142,
            WhisperModelSize::Small => 466,
        }
    }

    fn url(self) -> String {
        format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            self.filename()
        )
    }
}

/// How a download ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadOutcome {
    /// The model file's final path (`dest_dir/<size's filename>`).
    Completed(PathBuf),
    Cancelled,
}

#[derive(Debug)]
pub enum DownloadError {
    /// The HTTP request itself failed (DNS, TLS, connection, non-2xx status, ...).
    Request(String),
    /// Failed to create `dest_dir`, open/write the temp file, or rename it into place.
    Io(std::io::Error),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::Request(e) => write!(f, "download request failed: {e}"),
            DownloadError::Io(e) => write!(f, "download I/O failed: {e}"),
        }
    }
}

impl std::error::Error for DownloadError {}

/// Downloads `size`'s GGML model file into `dest_dir` (created if it doesn't exist), streaming
/// straight to `<dest_dir>/<filename>.part` and renaming to the final `<dest_dir>/<filename>`
/// only once the whole body has arrived — a reader that opens the final path mid-download (or
/// after a cancelled/failed one) never sees a truncated file.
///
/// Calls `on_progress(bytes_downloaded, total_bytes)` as data arrives; `total_bytes` is `0` if
/// the server didn't send a `Content-Length` (caller should treat that as "unknown", not "done").
/// Checks `cancel` between chunks — if set, stops, deletes the partial `.part` file, and returns
/// `Ok(DownloadOutcome::Cancelled)` rather than treating the cancellation as a failure (same
/// convention as [`crate::render::render_export`]/[`crate::transcribe::transcribe`]).
pub fn download_whisper_model(
    size: WhisperModelSize,
    dest_dir: &Path,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<DownloadOutcome, DownloadError> {
    std::fs::create_dir_all(dest_dir).map_err(DownloadError::Io)?;
    let final_path = dest_dir.join(size.filename());
    let tmp_path = dest_dir.join(format!("{}.part", size.filename()));

    let response = ureq::get(&size.url())
        .call()
        .map_err(|e| DownloadError::Request(e.to_string()))?;
    let total: u64 = response
        .header("Content-Length")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let mut reader = response.into_reader();
    let mut file = std::fs::File::create(&tmp_path).map_err(DownloadError::Io)?;
    let mut buf = [0u8; 64 * 1024];
    let mut downloaded: u64 = 0;

    loop {
        if cancel.load(Ordering::Relaxed) {
            drop(file);
            let _ = std::fs::remove_file(&tmp_path);
            return Ok(DownloadOutcome::Cancelled);
        }
        let n = reader.read(&mut buf).map_err(DownloadError::Io)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(DownloadError::Io)?;
        downloaded += n as u64;
        on_progress(downloaded, total);
    }
    drop(file);

    std::fs::rename(&tmp_path, &final_path).map_err(DownloadError::Io)?;
    Ok(DownloadOutcome::Completed(final_path))
}
