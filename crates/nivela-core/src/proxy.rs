//! Generates lightweight "proxy" transcodes for editing/scrubbing — the same trick CapCut,
//! Premiere and Resolve use: decode a small, easy-to-seek proxy file while editing, then
//! conform back to the original full-resolution source only at export time. Keeps timeline
//! scrubbing and preview responsive on large 4K/60fps gameplay captures that would otherwise
//! be expensive to decode every frame.
//!
//! Like [`crate::probe`]/[`crate::loudness`], the `ffmpeg` invocation ([`ensure_proxy`]) is a
//! thin wrapper — here around a pure path/freshness check ([`proxy_path_for`],
//! [`is_up_to_date`]) rather than a text parser, since there's no output to parse.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Proxies are downscaled to this height (width follows the source's aspect ratio) — matches
/// the resolution most NLEs default proxies to: enough detail to judge framing and cuts,
/// cheap enough to decode in real time while scrubbing.
pub const PROXY_HEIGHT: u32 = 540;

#[derive(Debug)]
pub enum ProxyError {
    /// Couldn't create the proxy cache directory.
    Io(std::io::Error),
    /// Couldn't spawn the `ffmpeg` process.
    Spawn(std::io::Error),
    /// `ffmpeg` ran but exited with a non-zero status.
    ExitStatus { code: Option<i32>, stderr: String },
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyError::Io(e) => write!(f, "failed to prepare the proxy cache directory: {e}"),
            ProxyError::Spawn(e) => write!(f, "failed to run ffmpeg: {e}"),
            ProxyError::ExitStatus { code, stderr } => {
                write!(f, "ffmpeg exited with code {code:?}: {stderr}")
            }
        }
    }
}

impl std::error::Error for ProxyError {}

/// The proxy file's path for `source` inside `proxy_dir`, without checking whether it exists
/// yet. Pure so it's usable both to look up an existing proxy and to test the naming scheme.
pub fn proxy_path_for(source: &Path, proxy_dir: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("clip");
    proxy_dir.join(format!("{stem}_proxy.mp4"))
}

/// Returns the proxy for `source`, generating it into `proxy_dir` with `ffmpeg` first if it
/// doesn't already exist or is older than the source (e.g. the source was re-recorded or
/// replaced). Skips the transcode — and the `ffmpeg` dependency entirely — when the cached
/// proxy is already fresh.
pub fn ensure_proxy(source: &Path, proxy_dir: &Path) -> Result<PathBuf, ProxyError> {
    let proxy_path = proxy_path_for(source, proxy_dir);

    if is_up_to_date(source, &proxy_path) {
        return Ok(proxy_path);
    }

    fs::create_dir_all(proxy_dir).map_err(ProxyError::Io)?;

    let output = Command::new("ffmpeg")
        .args(["-y", "-nostdin", "-hide_banner", "-loglevel", "error"])
        .arg("-i")
        .arg(source)
        .args(["-vf", &format!("scale=-2:{PROXY_HEIGHT}")])
        .args(["-c:v", "libx264", "-preset", "ultrafast", "-crf", "28"])
        .args(["-c:a", "aac", "-b:a", "128k"])
        .arg(&proxy_path)
        .output()
        .map_err(ProxyError::Spawn)?;

    if !output.status.success() {
        return Err(ProxyError::ExitStatus {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    Ok(proxy_path)
}

/// Whether `proxy_path` exists and was last modified at or after `source` — i.e. whether it's
/// safe to reuse instead of re-transcoding.
fn is_up_to_date(source: &Path, proxy_path: &Path) -> bool {
    let (Ok(source_meta), Ok(proxy_meta)) = (fs::metadata(source), fs::metadata(proxy_path)) else {
        return false;
    };
    let (Ok(source_modified), Ok(proxy_modified)) = (source_meta.modified(), proxy_meta.modified())
    else {
        return false;
    };
    proxy_modified >= source_modified
}

#[cfg(test)]
mod tests;
