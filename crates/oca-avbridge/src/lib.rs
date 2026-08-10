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
    fn oca_avbridge_remux_copy(in_path: *const c_char, out_path: *const c_char) -> c_int;
    fn oca_avbridge_encode_export(
        in_path: *const c_char,
        out_path: *const c_char,
        target_lufs: f32,
    ) -> c_int;
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

/// What [`remux_copy`] failed on.
#[derive(Debug)]
pub enum RemuxError {
    /// `in_path`/`out_path` contains a NUL byte and can't be handed to the C API.
    InvalidPath(NulError),
    /// Couldn't open `in_path`.
    OpenInput,
    /// Couldn't read `in_path`'s streams.
    StreamInfo,
    /// Couldn't guess an output format for `out_path` (unrecognized extension).
    AllocOutput,
    /// Couldn't create an output stream matching one of the input's streams.
    NewStream,
    /// Couldn't open `out_path` for writing.
    OpenOutput,
    /// Failed writing the output container's header.
    WriteHeader,
    /// Failed partway through writing packets — `out_path` may be a truncated/invalid file.
    WriteFrame,
    /// The C side returned a status code this crate doesn't know about.
    Unknown(c_int),
}

impl std::fmt::Display for RemuxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemuxError::InvalidPath(e) => write!(f, "path is not a valid C string: {e}"),
            RemuxError::OpenInput => write!(f, "failed to open input"),
            RemuxError::StreamInfo => write!(f, "failed to read stream info"),
            RemuxError::AllocOutput => write!(f, "failed to allocate output context"),
            RemuxError::NewStream => write!(f, "failed to create an output stream"),
            RemuxError::OpenOutput => write!(f, "failed to open output for writing"),
            RemuxError::WriteHeader => write!(f, "failed to write output header"),
            RemuxError::WriteFrame => write!(f, "failed to write a frame"),
            RemuxError::Unknown(code) => write!(f, "unknown remux status code: {code}"),
        }
    }
}

impl std::error::Error for RemuxError {}

/// Demuxes `in_path` and remuxes every video/audio stream to `out_path` unchanged — no decode,
/// no encode, no filtering. Equivalent to `ffmpeg -i in_path -c copy out_path`.
pub fn remux_copy(in_path: &Path, out_path: &Path) -> Result<(), RemuxError> {
    let c_in = CString::new(in_path.to_string_lossy().as_bytes()).map_err(RemuxError::InvalidPath)?;
    let c_out =
        CString::new(out_path.to_string_lossy().as_bytes()).map_err(RemuxError::InvalidPath)?;

    // SAFETY: c_in/c_out are valid NUL-terminated C strings for the duration of this call.
    // `bridge.c` closes the input context, output I/O, and frees the output context on every
    // exit path.
    let status = unsafe { oca_avbridge_remux_copy(c_in.as_ptr(), c_out.as_ptr()) };

    match status {
        0 => Ok(()),
        1 => Err(RemuxError::OpenInput),
        2 => Err(RemuxError::StreamInfo),
        3 => Err(RemuxError::AllocOutput),
        4 => Err(RemuxError::NewStream),
        5 => Err(RemuxError::OpenOutput),
        6 => Err(RemuxError::WriteHeader),
        7 => Err(RemuxError::WriteFrame),
        other => Err(RemuxError::Unknown(other)),
    }
}

/// What [`encode_export`] failed on.
#[derive(Debug)]
pub enum EncodeError {
    /// `in_path`/`out_path` contains a NUL byte and can't be handed to the C API.
    InvalidPath(NulError),
    OpenInput,
    StreamInfo,
    AllocOutput,
    NewStream,
    OpenOutput,
    WriteHeader,
    WriteFrame,
    /// `in_path` has no audio stream to normalize.
    NoAudioStream,
    /// Couldn't find/open the audio decoder.
    Decoder,
    /// Couldn't build the loudnorm/limiter filter graph.
    FilterGraph,
    /// Couldn't find/open the AAC encoder.
    Encoder,
    /// A decode/filter/encode call failed mid-stream (not at setup).
    Pipeline,
    /// The C side returned a status code this crate doesn't know about.
    Unknown(c_int),
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::InvalidPath(e) => write!(f, "path is not a valid C string: {e}"),
            EncodeError::OpenInput => write!(f, "failed to open input"),
            EncodeError::StreamInfo => write!(f, "failed to read stream info"),
            EncodeError::AllocOutput => write!(f, "failed to allocate output context"),
            EncodeError::NewStream => write!(f, "failed to create an output stream"),
            EncodeError::OpenOutput => write!(f, "failed to open output for writing"),
            EncodeError::WriteHeader => write!(f, "failed to write output header"),
            EncodeError::WriteFrame => write!(f, "failed to write a frame"),
            EncodeError::NoAudioStream => write!(f, "input has no audio stream"),
            EncodeError::Decoder => write!(f, "failed to open the audio decoder"),
            EncodeError::FilterGraph => write!(f, "failed to build the audio filter graph"),
            EncodeError::Encoder => write!(f, "failed to open the AAC encoder"),
            EncodeError::Pipeline => write!(f, "decode/filter/encode pipeline failed mid-stream"),
            EncodeError::Unknown(code) => write!(f, "unknown encode status code: {code}"),
        }
    }
}

impl std::error::Error for EncodeError {}

/// Renders `in_path` to `out_path`: video passthrough-copied, audio decoded, normalized
/// (`loudnorm` to `target_lufs` + a true-peak safety limiter) and re-encoded to AAC 192kbps.
/// Fails with [`EncodeError::NoAudioStream`] if `in_path` has no audio stream.
///
/// No progress reporting or cancellation yet — the whole file renders in this one blocking
/// call (see impl-004c in `features/fase1/task_breakdown.md`).
pub fn encode_export(in_path: &Path, out_path: &Path, target_lufs: f32) -> Result<(), EncodeError> {
    let c_in = CString::new(in_path.to_string_lossy().as_bytes()).map_err(EncodeError::InvalidPath)?;
    let c_out =
        CString::new(out_path.to_string_lossy().as_bytes()).map_err(EncodeError::InvalidPath)?;

    // SAFETY: c_in/c_out are valid NUL-terminated C strings for the duration of this call.
    // `bridge.c` frees the decoder/encoder/filter-graph contexts, output I/O, and output
    // context on every exit path.
    let status = unsafe { oca_avbridge_encode_export(c_in.as_ptr(), c_out.as_ptr(), target_lufs) };

    match status {
        0 => Ok(()),
        1 => Err(EncodeError::OpenInput),
        2 => Err(EncodeError::StreamInfo),
        3 => Err(EncodeError::AllocOutput),
        4 => Err(EncodeError::NewStream),
        5 => Err(EncodeError::OpenOutput),
        6 => Err(EncodeError::WriteHeader),
        7 => Err(EncodeError::WriteFrame),
        8 => Err(EncodeError::NoAudioStream),
        9 => Err(EncodeError::Decoder),
        10 => Err(EncodeError::FilterGraph),
        11 => Err(EncodeError::Encoder),
        12 => Err(EncodeError::Pipeline),
        other => Err(EncodeError::Unknown(other)),
    }
}
