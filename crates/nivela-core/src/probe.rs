//! Reads codec, resolution, fps, bitrate and duration from a media file via `oca-avbridge`'s
//! FFI probe — no subprocess, no stdout parsing.

use std::path::{Path, PathBuf};

use crate::media::{MediaAsset, MediaKind};

#[derive(Debug)]
pub enum ProbeError {
    /// Couldn't open the file — bad path, unreadable, or an unrecognized container.
    Open,
    /// The container opened but its streams couldn't be read.
    StreamInfo,
    /// The file has neither a video nor an audio stream.
    NoMediaStream,
    /// Anything else the bridge reported (invalid path encoding, unknown status code).
    Bridge(String),
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeError::Open => write!(f, "failed to open input"),
            ProbeError::StreamInfo => write!(f, "failed to read stream info"),
            ProbeError::NoMediaStream => write!(f, "no video or audio stream found"),
            ProbeError::Bridge(msg) => write!(f, "probe failed: {msg}"),
        }
    }
}

impl std::error::Error for ProbeError {}

impl From<oca_avbridge::ProbeError> for ProbeError {
    fn from(e: oca_avbridge::ProbeError) -> Self {
        match e {
            oca_avbridge::ProbeError::Open => ProbeError::Open,
            oca_avbridge::ProbeError::StreamInfo => ProbeError::StreamInfo,
            oca_avbridge::ProbeError::NoMediaStream => ProbeError::NoMediaStream,
            other => ProbeError::Bridge(other.to_string()),
        }
    }
}

/// The fields of a probed file relevant to the editor, already shaped like what
/// [`MediaAsset`] needs — everything else the bridge reports is discarded.
#[derive(Debug, Clone, PartialEq)]
pub struct ProbedMedia {
    pub kind: MediaKind,
    pub duration_secs: f64,
    pub codec: String,
    /// Overall container bitrate in Mbps (falls back to the primary stream's bitrate if the
    /// container doesn't report one, e.g. some raw audio formats).
    pub bitrate_mbps: f32,
    pub resolution: Option<(u32, u32)>,
    pub fps: Option<f32>,
    pub sample_rate_khz: Option<f32>,
}

impl ProbedMedia {
    /// Turns this probe result into a full [`MediaAsset`] ready for a project's media
    /// library. `loudness` starts `None` — call [`crate::measure_loudness`] separately and
    /// set it once that pass completes.
    pub fn into_media_asset(self, id: u64, file_name: String, source_path: PathBuf) -> MediaAsset {
        MediaAsset {
            id,
            file_name,
            source_path,
            kind: self.kind,
            duration_secs: self.duration_secs,
            codec: self.codec,
            source_bitrate_mbps: self.bitrate_mbps,
            resolution: self.resolution,
            fps: self.fps,
            sample_rate_khz: self.sample_rate_khz,
            // LUFS/true-peak/LRA come from a separate loudnorm pass — see `crate::loudness`.
            loudness: None,
            // Generated on demand — see `crate::proxy::ensure_proxy`.
            proxy_path: None,
        }
    }
}

/// Probes `path` via `oca_avbridge::probe` and returns its parsed metadata. Picks the first
/// video stream if there is one, otherwise the first audio stream.
pub fn probe_media(path: &Path) -> Result<ProbedMedia, ProbeError> {
    let info = oca_avbridge::probe(path)?;

    Ok(ProbedMedia {
        kind: match info.kind {
            oca_avbridge::StreamKind::Video => MediaKind::Video,
            oca_avbridge::StreamKind::Audio => MediaKind::Audio,
        },
        duration_secs: info.duration_secs,
        codec: info.codec_name,
        bitrate_mbps: info
            .bit_rate
            .map(|bps| (bps as f64 / 1_000_000.0) as f32)
            .unwrap_or(0.0),
        resolution: info.resolution,
        fps: info.fps,
        sample_rate_khz: info.sample_rate_hz.map(|hz| hz as f32 / 1000.0),
    })
}
