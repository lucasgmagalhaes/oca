//! `oca-avbridge` — thin C bridge over libavformat/libavcodec/libavutil (FFmpeg), called via
//! FFI. Replaces spawning `ffprobe`/`ffmpeg` as external processes: `nivela-core` links this
//! crate directly, so probing and encoding never shell out or parse subprocess stdout.
//!
//! The C surface (`csrc/bridge.c`) is written for this project — not a full auto-generated
//! FFmpeg binding — so it stays small and auditable. See [`build.rs`](../build.rs) for how the
//! FFmpeg dev libs (`FFMPEG_DIR`) are located and linked.

use std::ffi::{CStr, CString, NulError};
use std::os::raw::{c_char, c_int, c_longlong};
use std::path::Path;

#[repr(C)]
struct RawProbeInfo {
    has_video: c_int,
    duration_secs: f64,
    codec_name: [c_char; 32],
    bit_rate: c_longlong,
    width: c_int,
    height: c_int,
    fps_num: c_int,
    fps_den: c_int,
    sample_rate_hz: c_int,
}

unsafe extern "C" {
    fn oca_avbridge_version() -> u32;
    fn oca_avbridge_probe(path: *const c_char, out: *mut RawProbeInfo) -> c_int;
}

/// libavformat's packed version number (same encoding as `LIBAVFORMAT_VERSION_INT` /
/// `avformat_version()`): `(major << 16) | (minor << 8) | micro`.
pub fn avbridge_version() -> u32 {
    // SAFETY: oca_avbridge_version() takes no arguments, returns a plain u32, and has no
    // documented failure mode in libavformat — it's a pure accessor.
    unsafe { oca_avbridge_version() }
}

/// What [`probe`] failed on.
#[derive(Debug)]
pub enum ProbeError {
    /// `path` contains a NUL byte and can't be handed to the C API.
    InvalidPath(NulError),
    /// `avformat_open_input` failed — bad path, unreadable file, or an unrecognized container.
    Open,
    /// `avformat_find_stream_info` failed — the container opened but its streams couldn't be
    /// read.
    StreamInfo,
    /// The file has neither a video nor an audio stream.
    NoMediaStream,
    /// The C side returned a status code this crate doesn't know about (version skew between
    /// `bridge.h` and this file).
    Unknown(c_int),
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeError::InvalidPath(e) => write!(f, "path is not a valid C string: {e}"),
            ProbeError::Open => write!(f, "failed to open input"),
            ProbeError::StreamInfo => write!(f, "failed to read stream info"),
            ProbeError::NoMediaStream => write!(f, "no video or audio stream found"),
            ProbeError::Unknown(code) => write!(f, "unknown probe status code: {code}"),
        }
    }
}

impl std::error::Error for ProbeError {}

/// The picked stream's kind — mirrors [`ProbeInfo::has_video`] but as an enum for callers that
/// want to match on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamKind {
    Video,
    Audio,
}

/// Metadata read from a media file's container/stream headers — no decoding.
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeInfo {
    pub kind: StreamKind,
    pub duration_secs: f64,
    pub codec_name: String,
    /// Bits per second; `None` if neither the container nor the stream reports one.
    pub bit_rate: Option<u64>,
    /// `None` when [`Self::kind`] is [`StreamKind::Audio`].
    pub resolution: Option<(u32, u32)>,
    /// `None` when [`Self::kind`] is [`StreamKind::Audio`] or the rate is unreported.
    pub fps: Option<f32>,
    /// `None` when [`Self::kind`] is [`StreamKind::Video`] or the rate is unreported.
    pub sample_rate_hz: Option<u32>,
}

/// Probes the media file at `path` for codec, resolution, fps, bitrate and duration.
///
/// Picks the first video stream if there is one, otherwise the first audio stream — matching
/// the ffprobe-based version this replaces.
pub fn probe(path: &Path) -> Result<ProbeInfo, ProbeError> {
    let c_path = CString::new(path.to_string_lossy().as_bytes()).map_err(ProbeError::InvalidPath)?;
    let mut raw = RawProbeInfo {
        has_video: 0,
        duration_secs: 0.0,
        codec_name: [0; 32],
        bit_rate: 0,
        width: 0,
        height: 0,
        fps_num: 0,
        fps_den: 0,
        sample_rate_hz: 0,
    };

    // SAFETY: c_path is a valid NUL-terminated C string for the duration of this call, and
    // `raw` is a valid, fully-initialized `RawProbeInfo` the C side writes into on success (and
    // leaves untouched on error — checked below before reading any field except the status
    // code). `bridge.c` closes the `AVFormatContext` on every exit path.
    let status = unsafe { oca_avbridge_probe(c_path.as_ptr(), &mut raw) };

    match status {
        0 => {}
        1 => return Err(ProbeError::Open),
        2 => return Err(ProbeError::StreamInfo),
        3 => return Err(ProbeError::NoMediaStream),
        other => return Err(ProbeError::Unknown(other)),
    }

    // SAFETY: on success the C side NUL-terminates codec_name within the 32-byte buffer.
    let codec_name = unsafe { CStr::from_ptr(raw.codec_name.as_ptr()) }
        .to_string_lossy()
        .into_owned();

    let kind = if raw.has_video != 0 {
        StreamKind::Video
    } else {
        StreamKind::Audio
    };

    Ok(ProbeInfo {
        kind,
        duration_secs: raw.duration_secs,
        codec_name,
        bit_rate: (raw.bit_rate > 0).then_some(raw.bit_rate as u64),
        resolution: (kind == StreamKind::Video).then_some((raw.width as u32, raw.height as u32)),
        fps: (kind == StreamKind::Video && raw.fps_den != 0)
            .then_some(raw.fps_num as f32 / raw.fps_den as f32),
        sample_rate_hz: (kind == StreamKind::Audio && raw.sample_rate_hz > 0)
            .then_some(raw.sample_rate_hz as u32),
    })
}
