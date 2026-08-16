//! Downloads a YouTube video as MP3 (audio only) or MP4 (video), at a chosen quality, by
//! embedding a Python interpreter (via `pyo3`) and calling yt-dlp's own Python library API
//! (`yt_dlp.YoutubeDL`) directly — no subprocess, no parsing human-readable progress text off
//! stdout. This crate doesn't implement its own YouTube extractor — the site's playback
//! machinery is a moving target yt-dlp already tracks and updates for. Not part of
//! `request.md`'s phased plan — an ad hoc addition, same footing as the custom title bar.
//!
//! **This is a materially different, heavier dependency than anything else in this crate.**
//! Every other native tool oca depends on (FFmpeg, GStreamer, Whisper, Piper, ONNX Runtime) is
//! a C/C++ library linked or an executable spawned — this is a whole embedded CPython
//! interpreter, dynamically linked against `libpython`/`python3*.dll` at **build time**, and
//! needing that same (or ABI-compatible) Python runtime present at **launch time** — not just
//! when this feature is used. If the target machine has no matching Python installed,
//! `ui.exe` itself fails to start (a missing shared-library load failure), not just this one
//! feature — unlike, say, a missing `yt-dlp` binary under the previous subprocess-based
//! design, which only broke this one feature gracefully. `yt_dlp` (the Python package itself,
//! `pip install yt-dlp`) additionally needs to be importable in that interpreter. Fase 8's
//! "motores embutidos no instalador" plan does not yet account for bundling a Python runtime
//! at all — this is a real, currently-unresolved packaging gap this feature introduces, beyond
//! every other "not bundled yet, downloaded/expected on demand" gap already documented
//! elsewhere in this codebase.
//!
//! **Verification caveat:** this module's `pyo3`/`yt_dlp` API usage was checked against
//! `pyo3` 0.29.2's real published docs (`Python::attach`, `PyCFunction::new_closure`,
//! `PyDictMethods::set_item`, `PyAnyMethods::call`/`call_method`, all confirmed by their exact
//! signatures rather than guessed) and yt-dlp's own `YoutubeDL.py` source (`progress_hooks`'
//! dict shape, `postprocessor_hooks`, `add_progress_hook`, `DownloadCancelled`) — but could not
//! be exercised against a real download on any machine during development (no Python
//! environment with `yt_dlp` installed and matching this crate's linked Python ABI available
//! in this sandbox). Treat as unverified beyond static/API-signature-level review until run
//! for real once, same posture this file's neighbors (`background_removal.rs`,
//! `motion_tracking.rs`, `update_check.rs`) already carry for their own unexercised paths.
//! One specific assumption in particular hasn't been confirmed against real yt-dlp behavior:
//! that raising `yt_dlp.utils.DownloadCancelled` from inside a `progress_hooks` callback
//! (rather than from `match_filter`, its one *documented* use in the source reviewed) actually
//! propagates out of `extract_info()` as an abort rather than being swallowed or mishandled.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use pyo3::types::{
    PyAnyMethods, PyCFunction, PyDict, PyDictMethods, PyTuple, PyTupleMethods, PyTypeMethods,
};
use pyo3::{Bound, PyAny, PyErr, PyResult, Python};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mp4Quality {
    P360,
    P480,
    P720,
    P1080,
    Best,
}

impl Mp4Quality {
    pub const ALL: [Mp4Quality; 5] = [
        Mp4Quality::P360,
        Mp4Quality::P480,
        Mp4Quality::P720,
        Mp4Quality::P1080,
        Mp4Quality::Best,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Mp4Quality::P360 => "360p",
            Mp4Quality::P480 => "480p",
            Mp4Quality::P720 => "720p",
            Mp4Quality::P1080 => "1080p",
            Mp4Quality::Best => "Best",
        }
    }

    /// yt-dlp's `format` selector string — prefers a separate video+audio pair capped at this
    /// height, falling back to a single pre-muxed stream at the same cap if the site has one.
    fn format_selector(self) -> String {
        match self {
            Mp4Quality::Best => "bestvideo+bestaudio/best".to_string(),
            _ => {
                let h = match self {
                    Mp4Quality::P360 => 360,
                    Mp4Quality::P480 => 480,
                    Mp4Quality::P720 => 720,
                    Mp4Quality::P1080 => 1080,
                    Mp4Quality::Best => unreachable!(),
                };
                format!("bestvideo[height<={h}]+bestaudio/best[height<={h}]")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mp3Bitrate {
    K128,
    K192,
    K320,
}

impl Mp3Bitrate {
    pub const ALL: [Mp3Bitrate; 3] = [Mp3Bitrate::K128, Mp3Bitrate::K192, Mp3Bitrate::K320];

    pub fn label(self) -> &'static str {
        match self {
            Mp3Bitrate::K128 => "128 kbps",
            Mp3Bitrate::K192 => "192 kbps",
            Mp3Bitrate::K320 => "320 kbps",
        }
    }

    fn kbps(self) -> u32 {
        match self {
            Mp3Bitrate::K128 => 128,
            Mp3Bitrate::K192 => 192,
            Mp3Bitrate::K320 => 320,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YoutubeDownloadTarget {
    Mp4(Mp4Quality),
    Mp3(Mp3Bitrate),
}

#[derive(Debug)]
pub enum YoutubeDownloadError {
    /// The Python interpreter came up but `import yt_dlp` failed — the package isn't
    /// installed in it.
    ToolNotFound,
    /// Failed to create the destination directory.
    Io(std::io::Error),
    /// `yt_dlp.YoutubeDL(...).extract_info(...)` raised — the message is the Python
    /// exception's string representation. Also used for a cancelled download (message
    /// `"cancelled"`), same convention `avcore::youtube_download`'s previous subprocess-based
    /// implementation used, since there's no dedicated `Cancelled` outcome variant here either.
    Failed(String),
    /// `extract_info` returned successfully but its info dict had no
    /// `requested_downloads[0].filepath` — shouldn't happen in practice, but the caller needs a
    /// real path, not a guess at one.
    NoOutputFile,
}

impl std::fmt::Display for YoutubeDownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YoutubeDownloadError::ToolNotFound => {
                write!(
                    f,
                    "yt_dlp Python package not importable — pip install yt-dlp"
                )
            }
            YoutubeDownloadError::Io(e) => write!(f, "failed to prepare download directory: {e}"),
            YoutubeDownloadError::Failed(msg) => write!(f, "yt-dlp failed: {msg}"),
            YoutubeDownloadError::NoOutputFile => {
                write!(f, "yt-dlp finished but reported no output file")
            }
        }
    }
}

impl std::error::Error for YoutubeDownloadError {}

/// Whether `yt_dlp` is importable in the embedded interpreter at all — checked once up front
/// so the UI can show a clear "install yt-dlp" message instead of a generic failure once a
/// download is already underway.
pub fn is_yt_dlp_available() -> bool {
    Python::attach(|py| py.import("yt_dlp").is_ok())
}

/// Downloads `url` into `dest_dir` (created if it doesn't exist) as `target`'s format/quality,
/// reporting fractional progress (`0.0..=1.0`) as yt-dlp's `progress_hooks` report it.
/// `cancel` is an `Arc` (not a plain reference) because the progress-hook closure handed to
/// yt-dlp must be `'static` (`pyo3::types::PyCFunction::new_closure`'s bound) — it outlives
/// this function call from Python's perspective, held by the `YoutubeDL` instance for the
/// whole `extract_info` call.
pub fn download_youtube(
    url: &str,
    target: YoutubeDownloadTarget,
    dest_dir: &Path,
    cancel: Arc<AtomicBool>,
    on_progress: impl FnMut(f32) + Send + 'static,
) -> Result<PathBuf, YoutubeDownloadError> {
    std::fs::create_dir_all(dest_dir).map_err(YoutubeDownloadError::Io)?;

    let outtmpl = dest_dir
        .join("%(title).200B [%(id)s].%(ext)s")
        .display()
        .to_string();
    let on_progress = Mutex::new(on_progress);

    Python::attach(|py| {
        let yt_dlp = py
            .import("yt_dlp")
            .map_err(|_| YoutubeDownloadError::ToolNotFound)?;
        let utils = py
            .import("yt_dlp.utils")
            .map_err(|_| YoutubeDownloadError::ToolNotFound)?;
        let cancelled_cls = utils
            .getattr("DownloadCancelled")
            .map_err(|e| YoutubeDownloadError::Failed(e.to_string()))?;

        let opts = PyDict::new(py);
        opts.set_item("outtmpl", &outtmpl)
            .and_then(|_| opts.set_item("noplaylist", true))
            .and_then(|_| opts.set_item("quiet", true))
            .and_then(|_| opts.set_item("no_warnings", true))
            .map_err(|e| YoutubeDownloadError::Failed(e.to_string()))?;
        match target {
            YoutubeDownloadTarget::Mp4(quality) => {
                opts.set_item("format", quality.format_selector())
                    .and_then(|_| opts.set_item("merge_output_format", "mp4"))
                    .map_err(|e| YoutubeDownloadError::Failed(e.to_string()))?;
            }
            YoutubeDownloadTarget::Mp3(bitrate) => {
                let pp = PyDict::new(py);
                pp.set_item("key", "FFmpegExtractAudio")
                    .and_then(|_| pp.set_item("preferredcodec", "mp3"))
                    .and_then(|_| pp.set_item("preferredquality", bitrate.kbps().to_string()))
                    .map_err(|e| YoutubeDownloadError::Failed(e.to_string()))?;
                let pps = PyTuple::new(py, [pp])
                    .map_err(|e| YoutubeDownloadError::Failed(e.to_string()))?;
                opts.set_item("postprocessors", pps)
                    .map_err(|e| YoutubeDownloadError::Failed(e.to_string()))?;
            }
        }

        let cancelled_cls_owned = cancelled_cls.unbind();
        let hook = PyCFunction::new_closure(
            py,
            None,
            None,
            move |args: &Bound<'_, PyTuple>, _kwargs: Option<&Bound<'_, PyDict>>| -> PyResult<()> {
                // The GIL is already held on this thread — Python is the one calling us —
                // so the token comes from the already-`Bound` argument rather than a fresh
                // `Python::attach` (which would be a same-thread-reentrant acquisition; `.py()`
                // avoids relying on that being sound).
                let py = args.py();
                if cancel.load(Ordering::Relaxed) {
                    let cls = cancelled_cls_owned.bind(py);
                    let exc = cls.call1(("cancelled",))?;
                    return Err(PyErr::from_value(exc));
                }
                let Ok(d) = args.get_item(0) else {
                    return Ok(());
                };
                let status: String = d
                    .get_item("status")
                    .and_then(|v| v.extract())
                    .unwrap_or_default();
                let fraction = if status == "finished" {
                    Some(1.0)
                } else if status == "downloading" {
                    let downloaded: f64 = d
                        .get_item("downloaded_bytes")
                        .and_then(|v| v.extract())
                        .unwrap_or(0.0);
                    let total: f64 = d
                        .get_item("total_bytes")
                        .and_then(|v| v.extract())
                        .or_else(|_| d.get_item("total_bytes_estimate").and_then(|v| v.extract()))
                        .unwrap_or(0.0);
                    if total > 0.0 {
                        Some((downloaded / total).clamp(0.0, 1.0) as f32)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(fraction) = fraction {
                    if let Ok(mut cb) = on_progress.lock() {
                        cb(fraction);
                    }
                }
                Ok(())
            },
        )
        .map_err(|e| YoutubeDownloadError::Failed(e.to_string()))?;
        let hooks =
            PyTuple::new(py, [hook]).map_err(|e| YoutubeDownloadError::Failed(e.to_string()))?;
        opts.set_item("progress_hooks", hooks)
            .map_err(|e| YoutubeDownloadError::Failed(e.to_string()))?;

        let ydl_cls = yt_dlp
            .getattr("YoutubeDL")
            .map_err(|e| YoutubeDownloadError::Failed(e.to_string()))?;
        let ydl = ydl_cls
            .call1((opts,))
            .map_err(|e| YoutubeDownloadError::Failed(e.to_string()))?;

        let kwargs = PyDict::new(py);
        kwargs
            .set_item("download", true)
            .map_err(|e| YoutubeDownloadError::Failed(e.to_string()))?;
        let info = ydl
            .call_method("extract_info", (url,), Some(&kwargs))
            .map_err(|e| classify_extract_error(py, &e))?;

        extract_final_path(&info).ok_or(YoutubeDownloadError::NoOutputFile)
    })
}

/// Turns a Python exception from `extract_info` into a [`YoutubeDownloadError`] — `Failed`
/// with the `"cancelled"` message specifically when the exception is (or wraps) a
/// `DownloadCancelled`, so [`crate::youtube_download`]'s caller can tell a deliberate
/// cancellation apart from a real failure the same way it always could under the previous
/// subprocess-based implementation.
fn classify_extract_error(py: Python<'_>, err: &PyErr) -> YoutubeDownloadError {
    let is_cancelled = err
        .value(py)
        .get_type()
        .name()
        .map(|n| n.to_string() == "DownloadCancelled")
        .unwrap_or(false);
    if is_cancelled {
        YoutubeDownloadError::Failed("cancelled".to_string())
    } else {
        YoutubeDownloadError::Failed(err.to_string())
    }
}

/// Reads `info['requested_downloads'][0]['filepath']` — the same field yt-dlp's own CLI
/// `--print after_move:filepath` template resolves to, set only after any post-processing
/// (merge, audio extraction) has produced the truly final file.
fn extract_final_path(info: &Bound<'_, PyAny>) -> Option<PathBuf> {
    let downloads = info.get_item("requested_downloads").ok()?;
    let first = downloads.get_item(0).ok()?;
    let filepath: String = first.get_item("filepath").ok()?.extract().ok()?;
    Some(PathBuf::from(filepath))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mp4_quality_format_selectors_cap_by_height_except_best() {
        assert!(Mp4Quality::P720.format_selector().contains("height<=720"));
        assert_eq!(
            Mp4Quality::Best.format_selector(),
            "bestvideo+bestaudio/best"
        );
    }

    #[test]
    fn mp3_bitrate_kbps_matches_label() {
        assert_eq!(Mp3Bitrate::K192.kbps(), 192);
    }
}
