//! Generates lightweight "proxy" transcodes for editing/scrubbing — the same trick CapCut,
//! Premiere and Resolve use: decode a small, easy-to-seek proxy file while editing, then
//! conform back to the original full-resolution source only at export time. Keeps timeline
//! scrubbing and preview responsive on large 4K/60fps gameplay captures that would otherwise
//! be expensive to decode every frame.
//!
//! Like [`crate::probe`]/[`crate::render`], the transcode ([`ensure_proxy`]) goes through
//! `oca-avbridge`'s FFI — no subprocess — wrapped around a pure path/freshness check
//! ([`proxy_path_for`], [`is_up_to_date`]).

use std::fs;
use std::path::{Path, PathBuf};

use crate::project::Project;

/// Proxies are downscaled to this height (width follows the source's aspect ratio) — matches
/// the resolution most NLEs default proxies to: enough detail to judge framing and cuts,
/// cheap enough to decode in real time while scrubbing.
pub const PROXY_HEIGHT: u32 = 540;

#[derive(Debug)]
pub enum ProxyError {
    /// Couldn't create the proxy cache directory.
    Io(std::io::Error),
    /// `oca-avbridge` failed before or during the decode/scale/encode pipeline.
    Bridge(avbridge::ProxyError),
}

impl std::fmt::Display for ProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyError::Io(e) => write!(f, "failed to prepare the proxy cache directory: {e}"),
            ProxyError::Bridge(e) => write!(f, "failed to generate proxy: {e}"),
        }
    }
}

impl std::error::Error for ProxyError {}

/// Where a project's imported clips' editing proxies are cached: a hidden sibling folder next
/// to the project file (`myproject.json` -> `.myproject_proxies/`), or a temp folder for a
/// project that hasn't been saved yet (proxies there won't survive a reboot, but neither would
/// anything else about an unsaved project).
pub fn cache_dir_for_project(project: &Project) -> PathBuf {
    match &project.file_path {
        Some(path) => {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("project");
            path.with_file_name(format!(".{stem}_proxies"))
        }
        None => std::env::temp_dir().join("oca_unsaved_proxies"),
    }
}

/// The proxy file's path for `source` inside `proxy_dir`, without checking whether it exists
/// yet. Pure so it's usable both to look up an existing proxy and to test the naming scheme.
pub fn proxy_path_for(source: &Path, proxy_dir: &Path) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("clip");
    proxy_dir.join(format!("{stem}_proxy.mp4"))
}

/// Returns the proxy for `source`, generating it into `proxy_dir` via `oca-avbridge`'s FFI
/// first if it doesn't already exist or is older than the source (e.g. the source was
/// re-recorded or replaced). Skips the transcode entirely when the cached proxy is already
/// fresh.
pub fn ensure_proxy(source: &Path, proxy_dir: &Path) -> Result<PathBuf, ProxyError> {
    let proxy_path = proxy_path_for(source, proxy_dir);

    if is_up_to_date(source, &proxy_path) {
        return Ok(proxy_path);
    }

    fs::create_dir_all(proxy_dir).map_err(ProxyError::Io)?;

    avbridge::generate_proxy(source, &proxy_path, PROXY_HEIGHT).map_err(ProxyError::Bridge)?;

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
#[path = "proxy/proxy_test.rs"]
mod tests;
