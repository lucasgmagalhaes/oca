//! Wraps `ffprobe` to read codec, resolution, fps, bitrate and duration from a media file.
//!
//! The binary invocation ([`probe_media`]) is a thin shell around [`parse_probe_json`], which
//! does the actual work and takes a plain string — that split is what makes the parsing logic
//! unit-testable without an `ffprobe` binary on the machine running the tests (see the fixtures
//! in the `tests` module below).

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::media::{MediaAsset, MediaKind};

#[derive(Debug)]
pub enum ProbeError {
    /// Couldn't spawn the `ffprobe` process (e.g. it isn't installed / not on `PATH`).
    Spawn(std::io::Error),
    /// `ffprobe` ran but exited with a non-zero status.
    ExitStatus { code: Option<i32>, stderr: String },
    /// stdout wasn't valid ffprobe JSON.
    Json(serde_json::Error),
    /// The file has neither a video nor an audio stream ffprobe could describe.
    NoMediaStream,
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeError::Spawn(e) => write!(f, "failed to run ffprobe: {e}"),
            ProbeError::ExitStatus { code, stderr } => {
                write!(f, "ffprobe exited with code {code:?}: {stderr}")
            }
            ProbeError::Json(e) => write!(f, "failed to parse ffprobe output: {e}"),
            ProbeError::NoMediaStream => write!(f, "no video or audio stream found"),
        }
    }
}

impl std::error::Error for ProbeError {}

/// The fields of a probed file relevant to the editor, already shaped like what
/// [`MediaAsset`] needs — everything else ffprobe reports is discarded.
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
    pub fn into_media_asset(self, id: u64, file_name: String) -> MediaAsset {
        MediaAsset {
            id,
            file_name,
            kind: self.kind,
            duration_secs: self.duration_secs,
            codec: self.codec,
            source_bitrate_mbps: self.bitrate_mbps,
            resolution: self.resolution,
            fps: self.fps,
            sample_rate_khz: self.sample_rate_khz,
            // LUFS/true-peak/LRA come from a separate loudnorm pass — see `crate::loudness`.
            loudness: None,
        }
    }
}

/// Runs `ffprobe` against `path` and returns its parsed metadata.
pub fn probe_media(path: &Path) -> Result<ProbedMedia, ProbeError> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .map_err(ProbeError::Spawn)?;

    if !output.status.success() {
        return Err(ProbeError::ExitStatus {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_probe_json(&stdout)
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    streams: Vec<FfprobeStream>,
    format: FfprobeFormat,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
    bit_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: String,
    codec_name: String,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    sample_rate: Option<String>,
    bit_rate: Option<String>,
    duration: Option<String>,
}

/// Parses raw `ffprobe -show_format -show_streams -print_format json` stdout.
///
/// Picks the first `video` stream if there is one (making this a video asset), otherwise
/// falls back to the first `audio` stream. Any other stream types (subtitles, data, attached
/// pictures) are ignored.
pub fn parse_probe_json(json: &str) -> Result<ProbedMedia, ProbeError> {
    let parsed: FfprobeOutput = serde_json::from_str(json).map_err(ProbeError::Json)?;

    let video = parsed.streams.iter().find(|s| s.codec_type == "video");
    let audio = parsed.streams.iter().find(|s| s.codec_type == "audio");
    let stream = video.or(audio).ok_or(ProbeError::NoMediaStream)?;

    let duration_secs = parsed
        .format
        .duration
        .as_deref()
        .or(stream.duration.as_deref())
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(0.0);

    let bitrate_mbps = parsed
        .format
        .bit_rate
        .as_deref()
        .or(stream.bit_rate.as_deref())
        .and_then(|b| b.parse::<f64>().ok())
        .map(|bps| (bps / 1_000_000.0) as f32)
        .unwrap_or(0.0);

    Ok(ProbedMedia {
        kind: if video.is_some() {
            MediaKind::Video
        } else {
            MediaKind::Audio
        },
        duration_secs,
        codec: stream.codec_name.clone(),
        bitrate_mbps,
        resolution: match (stream.width, stream.height) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        },
        fps: stream.r_frame_rate.as_deref().and_then(parse_frame_rate),
        sample_rate_khz: if video.is_some() {
            None
        } else {
            stream
                .sample_rate
                .as_deref()
                .and_then(|s| s.parse::<f32>().ok())
                .map(|hz| hz / 1000.0)
        },
    })
}

/// ffprobe reports frame rate as a `"num/den"` fraction (e.g. `"60000/1001"` for 59.94 fps).
fn parse_frame_rate(raw: &str) -> Option<f32> {
    let (num, den) = raw.split_once('/')?;
    let num: f32 = num.parse().ok()?;
    let den: f32 = den.parse().ok()?;
    if den == 0.0 {
        None
    } else {
        Some(num / den)
    }
}
