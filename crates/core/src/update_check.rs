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

//! Checks GitHub Releases for a newer published version than the running build —
//! `request.md`'s Fase 8 "Versão e auto-update" ask, scoped down to the check-and-notify half.
//! Downloading and replacing the running binary ("baixar e aplicar a atualização") isn't
//! implemented here; this only tells the user a newer version exists and links to the release
//! page so they can grab it themselves. No telemetry, no background polling loop — one request,
//! called once per app launch (see `ui`'s `App::spawn_update_check`).

use serde::Deserialize;

/// GitHub's "latest release" endpoint for this project — 404s until the repository has at
/// least one published (non-draft, non-prerelease) release.
const RELEASES_API_URL: &str = "https://api.github.com/repos/lucasgmagalhaes/oca/releases/latest";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatestRelease {
    /// Normalized version string with any leading `v` stripped (GitHub tags are conventionally
    /// `v1.2.3`; `Cargo.toml`'s own `CARGO_PKG_VERSION` has none), for [`is_newer`] to compare
    /// against `CARGO_PKG_VERSION` on equal footing.
    pub version: String,
    /// The release's own GitHub page — where a "download" link points, since this module
    /// doesn't fetch or apply the update itself.
    pub html_url: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
}

#[derive(Debug)]
pub enum UpdateCheckError {
    /// The HTTP request itself failed (DNS, TLS, connection, non-2xx status — including a
    /// plain 404 for "no releases published yet").
    Request(String),
    /// The response didn't parse as the JSON shape GitHub's release API is documented to
    /// return.
    Parse(String),
}

impl std::fmt::Display for UpdateCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateCheckError::Request(e) => write!(f, "update check request failed: {e}"),
            UpdateCheckError::Parse(e) => write!(f, "update check response failed to parse: {e}"),
        }
    }
}

impl std::error::Error for UpdateCheckError {}

/// Fetches the latest published GitHub release for this project. A plain `ureq::get` + manual
/// `serde_json::from_str` rather than `ureq`'s own `into_json` — that method is gated behind a
/// `json` Cargo feature this crate doesn't enable, and every other dependency already used here
/// (`serde`/`serde_json`) covers the same ground without adding one.
pub fn fetch_latest_release() -> Result<LatestRelease, UpdateCheckError> {
    // GitHub's API rejects requests with no User-Agent header.
    let mut response = ureq::get(RELEASES_API_URL)
        .header("User-Agent", "oca-update-check")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| UpdateCheckError::Request(e.to_string()))?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|e| UpdateCheckError::Request(e.to_string()))?;
    let parsed: GithubRelease =
        serde_json::from_str(&body).map_err(|e| UpdateCheckError::Parse(e.to_string()))?;
    Ok(LatestRelease {
        version: parsed.tag_name.trim_start_matches('v').to_string(),
        html_url: parsed.html_url,
    })
}

/// Whether `latest` is a newer version than `current` — both dotted `major.minor.patch`-style
/// strings. Compares each dot-separated segment's leading digits numerically, left to right;
/// a segment with no leading digits (e.g. a `-beta` suffix glued onto the last number instead
/// of separated by its own dot) reads as `0` rather than erroring, so this never panics on
/// unexpected input — worst case it just under- or over-estimates equality on a malformed
/// version, never crashes the update check.
pub fn is_newer(current: &str, latest: &str) -> bool {
    fn segment_value(segment: &str) -> u64 {
        segment
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(0)
    }
    fn segments(v: &str) -> Vec<u64> {
        v.split('.').map(segment_value).collect()
    }

    let current_segments = segments(current);
    let latest_segments = segments(latest);
    let len = current_segments.len().max(latest_segments.len());
    for i in 0..len {
        let c = current_segments.get(i).copied().unwrap_or(0);
        let l = latest_segments.get(i).copied().unwrap_or(0);
        if l != c {
            return l > c;
        }
    }
    false
}
