//! `oca-avbridge` — thin C bridge over libavformat/libavcodec/libavutil (FFmpeg), called via
//! FFI. Replaces spawning `ffprobe`/`ffmpeg` as external processes: `core` links this
//! crate directly, so probing and encoding never shell out or parse subprocess stdout.
//!
//! The C surface (`csrc/bridge.c`) is written for this project — not a full auto-generated
//! FFmpeg binding — so it stays small and auditable. See [`build.rs`](../build.rs) for how the
//! FFmpeg dev libs (`FFMPEG_DIR`) are located and linked.

use std::ffi::{c_void, CStr, CString, NulError};
use std::os::raw::{c_char, c_int, c_longlong};
use std::path::Path;
use std::sync::atomic::AtomicBool;

#[repr(C)]
struct RawClipSegment {
    source_path: *const c_char,
    source_in_secs: f64,
    source_out_secs: f64,
    gain_db: f32,
    video_filter: *const c_char,
    frozen: c_int,
    speed_factor: f32,
    zoom_start: f32,
    zoom_end: f32,
    transition_in: c_int,
    transition_duration_secs: f32,
    /// Start position of this clip on the shared timeline in seconds. Used by
    /// `avbridge_encode_timeline_export_multi` to determine which overlay tracks are active at
    /// any given decoded-frame time. Ignored by `avbridge_encode_timeline_export`.
    timeline_start_secs: f64,
}

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
    fn avbridge_version() -> u32;
    fn avbridge_probe(path: *const c_char, out: *mut RawProbeInfo) -> c_int;
    fn avbridge_remux_copy(in_path: *const c_char, out_path: *const c_char) -> c_int;
    fn avbridge_encode_export(
        in_path: *const c_char,
        out_path: *const c_char,
        target_lufs: f32,
        progress_cb: Option<unsafe extern "C" fn(user_data: *mut c_void, seconds: f64)>,
        progress_user_data: *mut c_void,
        cancel: *const u8,
    ) -> c_int;
    fn avbridge_encode_timeline_export(
        segments: *const RawClipSegment,
        segment_count: c_int,
        canvas_width: c_int,
        canvas_height: c_int,
        canvas_fps_num: c_int,
        canvas_fps_den: c_int,
        canvas_bit_rate_bps: c_longlong,
        out_path: *const c_char,
        target_lufs: f32,
        progress_cb: Option<unsafe extern "C" fn(user_data: *mut c_void, seconds: f64)>,
        progress_user_data: *mut c_void,
        cancel: *const u8,
    ) -> c_int;
    fn avbridge_encode_timeline_export_multi(
        track_segs: *const *const RawClipSegment,
        track_n_segs: *const c_int,
        n_tracks: c_int,
        canvas_width: c_int,
        canvas_height: c_int,
        canvas_fps_num: c_int,
        canvas_fps_den: c_int,
        canvas_bit_rate_bps: c_longlong,
        out_path: *const c_char,
        target_lufs: f32,
        progress_cb: Option<unsafe extern "C" fn(user_data: *mut c_void, seconds: f64)>,
        progress_user_data: *mut c_void,
        cancel: *const u8,
    ) -> c_int;
    fn avbridge_measure_loudness(
        in_path: *const c_char,
        out_json: *mut c_char,
        out_json_len: usize,
    ) -> c_int;
    fn avbridge_generate_proxy(
        in_path: *const c_char,
        out_path: *const c_char,
        target_height: c_int,
    ) -> c_int;
    fn avbridge_generate_waveform(
        in_path: *const c_char,
        bucket_count: c_int,
        out_min: *mut f32,
        out_max: *mut f32,
    ) -> c_int;
}

/// Trampoline handed to the C side as `progress_cb`; `user_data` is a `*mut F` for whatever
/// closure [`encode_export`] was called with.
unsafe extern "C" fn progress_trampoline<F: FnMut(f64)>(user_data: *mut c_void, seconds: f64) {
    // SAFETY: `user_data` was set to `&mut on_progress` for the duration of the single
    // `avbridge_encode_export` call this trampoline is only ever invoked from, and the
    // pointee outlives that call (it's a stack local in `encode_export`, held for the call).
    let closure = unsafe { &mut *(user_data as *mut F) };
    closure(seconds);
}

/// libavformat's packed version number (same encoding as `LIBAVFORMAT_VERSION_INT` /
/// `avformat_version()`): `(major << 16) | (minor << 8) | micro`.
pub fn version() -> u32 {
    // SAFETY: avbridge_version() takes no arguments, returns a plain u32, and has no
    // documented failure mode in libavformat — it's a pure accessor.
    unsafe { avbridge_version() }
}

/// What [`probe`] failed on.
#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    /// `path` contains a NUL byte and can't be handed to the C API.
    #[error("path is not a valid C string: {0}")]
    InvalidPath(NulError),
    /// `avformat_open_input` failed — bad path, unreadable file, or an unrecognized container.
    #[error("failed to open input")]
    Open,
    /// `avformat_find_stream_info` failed — the container opened but its streams couldn't be
    /// read.
    #[error("failed to read stream info")]
    StreamInfo,
    /// The file has neither a video nor an audio stream.
    #[error("no video or audio stream found")]
    NoMediaStream,
    /// The C side returned a status code this crate doesn't know about (version skew between
    /// `bridge.h` and this file).
    #[error("unknown probe status code: {0}")]
    Unknown(c_int),
}

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
    let c_path =
        CString::new(path.to_string_lossy().as_bytes()).map_err(ProbeError::InvalidPath)?;
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
    let status = unsafe { avbridge_probe(c_path.as_ptr(), &mut raw) };

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
#[derive(Debug, thiserror::Error)]
pub enum RemuxError {
    /// `in_path`/`out_path` contains a NUL byte and can't be handed to the C API.
    #[error("path is not a valid C string: {0}")]
    InvalidPath(NulError),
    /// Couldn't open `in_path`.
    #[error("failed to open input")]
    OpenInput,
    /// Couldn't read `in_path`'s streams.
    #[error("failed to read stream info")]
    StreamInfo,
    /// Couldn't guess an output format for `out_path` (unrecognized extension).
    #[error("failed to allocate output context")]
    AllocOutput,
    /// Couldn't create an output stream matching one of the input's streams.
    #[error("failed to create an output stream")]
    NewStream,
    /// Couldn't open `out_path` for writing.
    #[error("failed to open output for writing")]
    OpenOutput,
    /// Failed writing the output container's header.
    #[error("failed to write output header")]
    WriteHeader,
    /// Failed partway through writing packets — `out_path` may be a truncated/invalid file.
    #[error("failed to write a frame")]
    WriteFrame,
    /// The C side returned a status code this crate doesn't know about.
    #[error("unknown remux status code: {0}")]
    Unknown(c_int),
}

/// Demuxes `in_path` and remuxes every video/audio stream to `out_path` unchanged — no decode,
/// no encode, no filtering. Equivalent to `ffmpeg -i in_path -c copy out_path`.
pub fn remux_copy(in_path: &Path, out_path: &Path) -> Result<(), RemuxError> {
    let c_in =
        CString::new(in_path.to_string_lossy().as_bytes()).map_err(RemuxError::InvalidPath)?;
    let c_out =
        CString::new(out_path.to_string_lossy().as_bytes()).map_err(RemuxError::InvalidPath)?;

    // SAFETY: c_in/c_out are valid NUL-terminated C strings for the duration of this call.
    // `bridge.c` closes the input context, output I/O, and frees the output context on every
    // exit path.
    let status = unsafe { avbridge_remux_copy(c_in.as_ptr(), c_out.as_ptr()) };

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
#[derive(Debug, thiserror::Error)]
pub enum EncodeError {
    /// `in_path`/`out_path` contains a NUL byte and can't be handed to the C API.
    #[error("path is not a valid C string: {0}")]
    InvalidPath(NulError),
    #[error("failed to open input")]
    OpenInput,
    #[error("failed to read stream info")]
    StreamInfo,
    #[error("failed to allocate output context")]
    AllocOutput,
    #[error("failed to create an output stream")]
    NewStream,
    #[error("failed to open output for writing")]
    OpenOutput,
    #[error("failed to write output header")]
    WriteHeader,
    #[error("failed to write a frame")]
    WriteFrame,
    /// `in_path` has no audio stream to normalize.
    #[error("input has no audio stream")]
    NoAudioStream,
    /// Couldn't find/open the audio decoder.
    #[error("failed to open the audio decoder")]
    Decoder,
    /// Couldn't build the loudnorm/limiter filter graph.
    #[error("failed to build the audio filter graph")]
    FilterGraph,
    /// Couldn't find/open the AAC encoder.
    #[error("failed to open the AAC encoder")]
    Encoder,
    /// A decode/filter/encode call failed mid-stream (not at setup).
    #[error("decode/filter/encode pipeline failed mid-stream")]
    Pipeline,
    /// (`encode_timeline_export` only) a segment has no video stream.
    #[error("a segment has no video stream")]
    NoVideoStream,
    /// (`encode_timeline_export` only) a later segment's audio format (sample rate/format/
    /// channel layout) doesn't match the first segment's.
    #[error("a segment's audio format doesn't match the first segment's")]
    AudioFormatMismatch,
    /// (`encode_timeline_export` only) `segments` was empty.
    #[error("no segments to render")]
    EmptyTimeline,
    /// The C side returned a status code this crate doesn't know about.
    #[error("unknown encode status code: {0}")]
    Unknown(c_int),
}

/// How [`encode_export`] ended: all the way through, or stopped early because `cancel` was
/// set. Mirrors `avcore::render::RenderOutcome`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeOutcome {
    Completed,
    Cancelled,
}

/// Renders `in_path` to `out_path`: video passthrough-copied, audio decoded, normalized
/// (`loudnorm` to `target_lufs` + a true-peak safety limiter) and re-encoded to AAC 192kbps.
/// Fails with [`EncodeError::NoAudioStream`] if `in_path` has no audio stream.
///
/// Calls `on_progress(seconds_processed)` as packets are read, and checks `cancel` between
/// them — if another thread sets it, encoding stops (without writing a trailer, so `out_path`
/// is left truncated/invalid) and this returns `Ok(EncodeOutcome::Cancelled)` rather than
/// treating the cancellation as a failure.
pub fn encode_export<F: FnMut(f64)>(
    in_path: &Path,
    out_path: &Path,
    target_lufs: f32,
    cancel: &AtomicBool,
    mut on_progress: F,
) -> Result<EncodeOutcome, EncodeError> {
    let c_in =
        CString::new(in_path.to_string_lossy().as_bytes()).map_err(EncodeError::InvalidPath)?;
    let c_out =
        CString::new(out_path.to_string_lossy().as_bytes()).map_err(EncodeError::InvalidPath)?;

    // SAFETY: c_in/c_out are valid NUL-terminated C strings for the duration of this call.
    // `bridge.c` frees the decoder/encoder/filter-graph contexts, output I/O, and output
    // context on every exit path. `progress_trampoline::<F>` matches `on_progress`'s captured
    // type F, and `&mut on_progress` outlives the call (it's this function's own stack frame).
    // `cancel.as_ptr()` is documented to have the same size/alignment/bit-validity as `bool`
    // (0/1), which the C side reads as `uint8_t`.
    let status = unsafe {
        avbridge_encode_export(
            c_in.as_ptr(),
            c_out.as_ptr(),
            target_lufs,
            Some(progress_trampoline::<F>),
            &mut on_progress as *mut _ as *mut c_void,
            cancel.as_ptr() as *const u8,
        )
    };

    match status {
        0 => Ok(EncodeOutcome::Completed),
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
        13 => Ok(EncodeOutcome::Cancelled),
        other => Err(EncodeError::Unknown(other)),
    }
}

/// One trimmed clip in an [`encode_timeline_export`] timeline: a source file, the range of it
/// to use (`source_in_secs..source_out_secs`), a linear gain in dB, and a pre-built avfilter
/// chain description for this clip's own video effects (empty string = none — the segment
/// still gets canvas-conformed and re-encoded).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClipSegment {
    pub source_path: std::path::PathBuf,
    pub source_in_secs: f64,
    pub source_out_secs: f64,
    pub gain_db: f32,
    pub video_filter: String,
    /// `true` holds the first decoded video frame at/after `source_in_secs` for this segment's
    /// whole trimmed duration instead of playing through the range — a freeze frame.
    /// `video_filter` still applies to every held frame. Audio is unaffected either way — still
    /// decoded/played across the full `source_in_secs..source_out_secs` range.
    pub frozen: bool,
    /// Playback speed multiplier — `1.0` is normal speed, `2.0` is double speed,
    /// `0.5` is half speed. Drives a `setpts=PTS/speed` filter before the fps
    /// conform stage (video) and an `atempo=speed` command in the shared audio
    /// filter graph. Audio is clamped to `[0.5, 100.0]` (atempo's range).
    pub speed_factor: f32,
    /// Ken-burns zoom: linearly interpolated from `zoom_start` to `zoom_end` over the clip's
    /// output duration. `1.0` is no zoom. Handled in `bridge.c` via an animated crop+scale
    /// filter after the canvas fps conform step.
    pub zoom_start: f32,
    /// See [`ClipSegment::zoom_start`].
    pub zoom_end: f32,
    /// Transition effect at this clip's entry. `0` = None/HardCut (no effect), `1` = Fade
    /// (fade in from black), `2` = Slide (reveal from left), `3` = Zoom (scale from 50% to
    /// 100%). Handled in `bridge.c` by appending an animated avfilter expression after
    /// `video_filter` and before the final `format=yuv420p` conform step.
    pub transition_in: u8,
    /// Duration of [`ClipSegment::transition_in`] in seconds. Ignored when `transition_in` is
    /// `0`. Converted to a frame count in `bridge.c` using the canvas fps.
    pub transition_duration_secs: f32,
    /// Start position of this clip on the shared timeline in seconds.  Used by
    /// [`encode_timeline_export_multi`] to determine which overlay tracks are active at any
    /// given decoded-frame time.  Ignored by [`encode_timeline_export`] (single-track function
    /// concatenates in order with no gaps).  Defaults to `0.0` for backwards-compatible
    /// deserialization of jobs saved before this field existed.
    #[serde(default)]
    pub timeline_start_secs: f64,
}

/// The fixed output frame size/rate every segment in an [`encode_timeline_export`] call is
/// scaled/padded/frame-rate-conformed onto.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub fps_num: u32,
    pub fps_den: u32,
    /// Target video bitrate in bits per second for the whole timeline's single encoder —
    /// unlike [`encode_export`]'s exact source-bitrate passthrough, this is a target the
    /// libopenh264 encoder aims for (there's no way to guarantee an exact output bitrate once
    /// video is re-encoded rather than stream-copied).
    pub bit_rate_bps: i64,
}

/// Renders `segments` as one continuous export onto a `canvas_width`x`canvas_height` canvas
/// at `canvas_fps_num`/`canvas_fps_den`: video is decoded, each segment's own filter chain
/// applied (prefixed with a canvas-conform scale/pad/fps stage), and re-encoded via
/// libopenh264 — unlike [`encode_export`], video is never stream-copied, since each clip may
/// need a different filter chain. Audio across all segments runs through ONE continuous
/// loudnorm+limiter graph (so normalization sees the whole timeline) before being re-encoded
/// to AAC — every segment's audio must therefore share the same sample rate/format/channel
/// layout ([`EncodeError::AudioFormatMismatch`] otherwise), and every segment must have both a
/// video and an audio stream ([`EncodeError::NoVideoStream`] / [`EncodeError::NoAudioStream`]).
///
/// Calls `on_progress(seconds_processed)` with the cumulative timeline position, and checks
/// `cancel` between packets — same cancellation contract as [`encode_export`].
pub fn encode_timeline_export<F: FnMut(f64)>(
    segments: &[ClipSegment],
    canvas: Canvas,
    out_path: &Path,
    target_lufs: f32,
    cancel: &AtomicBool,
    mut on_progress: F,
) -> Result<EncodeOutcome, EncodeError> {
    if segments.is_empty() {
        return Err(EncodeError::EmptyTimeline);
    }

    let c_out =
        CString::new(out_path.to_string_lossy().as_bytes()).map_err(EncodeError::InvalidPath)?;

    let mut c_paths = Vec::with_capacity(segments.len());
    let mut c_filters = Vec::with_capacity(segments.len());
    for seg in segments {
        c_paths.push(
            CString::new(seg.source_path.to_string_lossy().as_bytes())
                .map_err(EncodeError::InvalidPath)?,
        );
        c_filters
            .push(CString::new(seg.video_filter.as_bytes()).map_err(EncodeError::InvalidPath)?);
    }
    let raw_segments: Vec<RawClipSegment> = segments
        .iter()
        .zip(c_paths.iter())
        .zip(c_filters.iter())
        .map(|((seg, path), filt)| RawClipSegment {
            source_path: path.as_ptr(),
            source_in_secs: seg.source_in_secs,
            source_out_secs: seg.source_out_secs,
            gain_db: seg.gain_db,
            video_filter: filt.as_ptr(),
            frozen: seg.frozen as c_int,
            speed_factor: seg.speed_factor,
            zoom_start: seg.zoom_start,
            zoom_end: seg.zoom_end,
            transition_in: seg.transition_in as c_int,
            transition_duration_secs: seg.transition_duration_secs,
            timeline_start_secs: seg.timeline_start_secs,
        })
        .collect();

    // SAFETY: raw_segments' pointers stay valid for the call — c_paths/c_filters (which they
    // point into) and raw_segments itself are all stack locals held alive until this function
    // returns, none of them mutated during the call. c_out, progress_trampoline::<F>, and
    // cancel.as_ptr() carry the same safety argument as encode_export's identical call.
    let status = unsafe {
        avbridge_encode_timeline_export(
            raw_segments.as_ptr(),
            raw_segments.len() as c_int,
            canvas.width as c_int,
            canvas.height as c_int,
            canvas.fps_num as c_int,
            canvas.fps_den as c_int,
            canvas.bit_rate_bps as c_longlong,
            c_out.as_ptr(),
            target_lufs,
            Some(progress_trampoline::<F>),
            &mut on_progress as *mut _ as *mut c_void,
            cancel.as_ptr() as *const u8,
        )
    };

    match status {
        0 => Ok(EncodeOutcome::Completed),
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
        13 => Ok(EncodeOutcome::Cancelled),
        14 => Err(EncodeError::NoVideoStream),
        15 => Err(EncodeError::AudioFormatMismatch),
        16 => Err(EncodeError::EmptyTimeline),
        other => Err(EncodeError::Unknown(other)),
    }
}

/// Multi-track variant of [`encode_timeline_export`]: composites N tracks of segments via an
/// avfilter `overlay` chain. Track 0 drives the output (its segments play in order, audio comes
/// from track 0 only). Tracks 1..n_tracks-1 are overlaid on top wherever their
/// `timeline_start_secs`-based windows overlap with the current track-0 frame's timeline
/// position.
///
/// When `n_tracks == 1`, delegates to [`encode_timeline_export`] unchanged — this is the
/// guaranteed backward-compatible path that exercises no new C code. Additional tracks beyond
/// index 1 are silently ignored in the current implementation (only two-input overlay is built).
///
/// Same cancellation and progress contract as [`encode_timeline_export`].
pub fn encode_timeline_export_multi<F: FnMut(f64)>(
    tracks: &[Vec<ClipSegment>],
    canvas: Canvas,
    out_path: &Path,
    target_lufs: f32,
    cancel: &AtomicBool,
    mut on_progress: F,
) -> Result<EncodeOutcome, EncodeError> {
    if tracks.is_empty() || tracks[0].is_empty() {
        return Err(EncodeError::EmptyTimeline);
    }
    if tracks.len() == 1 {
        return encode_timeline_export(&tracks[0], canvas, out_path, target_lufs, cancel, on_progress);
    }

    let c_out =
        CString::new(out_path.to_string_lossy().as_bytes()).map_err(EncodeError::InvalidPath)?;

    // Build CString storage and RawClipSegment vecs for each track.
    let mut per_track_paths: Vec<Vec<CString>>  = Vec::with_capacity(tracks.len());
    let mut per_track_filts: Vec<Vec<CString>>  = Vec::with_capacity(tracks.len());
    let mut per_track_raw:   Vec<Vec<RawClipSegment>> = Vec::with_capacity(tracks.len());
    for segs in tracks {
        let mut paths = Vec::with_capacity(segs.len());
        let mut filts = Vec::with_capacity(segs.len());
        for seg in segs {
            paths.push(
                CString::new(seg.source_path.to_string_lossy().as_bytes())
                    .map_err(EncodeError::InvalidPath)?,
            );
            filts.push(
                CString::new(seg.video_filter.as_bytes())
                    .map_err(EncodeError::InvalidPath)?,
            );
        }
        let raw: Vec<RawClipSegment> = segs
            .iter()
            .zip(paths.iter())
            .zip(filts.iter())
            .map(|((seg, path), filt)| RawClipSegment {
                source_path: path.as_ptr(),
                source_in_secs: seg.source_in_secs,
                source_out_secs: seg.source_out_secs,
                gain_db: seg.gain_db,
                video_filter: filt.as_ptr(),
                frozen: seg.frozen as c_int,
                speed_factor: seg.speed_factor,
                zoom_start: seg.zoom_start,
                zoom_end: seg.zoom_end,
                transition_in: seg.transition_in as c_int,
                transition_duration_secs: seg.transition_duration_secs,
                timeline_start_secs: seg.timeline_start_secs,
            })
            .collect();
        per_track_paths.push(paths);
        per_track_filts.push(filts);
        per_track_raw.push(raw);
    }

    // Build pointer + count arrays for the C call.
    let track_ptrs: Vec<*const RawClipSegment> = per_track_raw.iter().map(|v| v.as_ptr()).collect();
    let track_counts: Vec<c_int> = per_track_raw.iter().map(|v| v.len() as c_int).collect();

    // SAFETY: all pointer-backing storage (per_track_paths, per_track_filts, per_track_raw,
    // track_ptrs, track_counts, c_out) is held alive until after avbridge_encode_timeline_export_multi
    // returns. The function does not retain any pointer after returning. cancel is an AtomicBool
    // aligned to at least 1 byte; its value is read atomically by the C side between frames.
    let status = unsafe {
        avbridge_encode_timeline_export_multi(
            track_ptrs.as_ptr(),
            track_counts.as_ptr(),
            tracks.len() as c_int,
            canvas.width as c_int,
            canvas.height as c_int,
            canvas.fps_num as c_int,
            canvas.fps_den as c_int,
            canvas.bit_rate_bps as c_longlong,
            c_out.as_ptr(),
            target_lufs,
            Some(progress_trampoline::<F>),
            &mut on_progress as *mut _ as *mut c_void,
            cancel.as_ptr() as *const u8,
        )
    };

    match status {
        0 => Ok(EncodeOutcome::Completed),
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
        13 => Ok(EncodeOutcome::Cancelled),
        14 => Err(EncodeError::NoVideoStream),
        15 => Err(EncodeError::AudioFormatMismatch),
        16 => Err(EncodeError::EmptyTimeline),
        other => Err(EncodeError::Unknown(other)),
    }
}

/// What [`measure_loudness_json`] failed on.
#[derive(Debug, thiserror::Error)]
pub enum LoudnessError {
    /// `path` contains a NUL byte and can't be handed to the C API.
    #[error("path is not a valid C string: {0}")]
    InvalidPath(NulError),
    #[error("failed to open input")]
    OpenInput,
    #[error("failed to read stream info")]
    StreamInfo,
    /// `path` has no audio stream to measure.
    #[error("input has no audio stream")]
    NoAudioStream,
    /// Couldn't find/open the audio decoder.
    #[error("failed to open the audio decoder")]
    Decoder,
    /// Couldn't build the loudnorm filter graph.
    #[error("failed to build the loudnorm filter graph")]
    FilterGraph,
    /// A decode/filter call failed mid-stream (not at setup).
    #[error("decode/filter pipeline failed mid-stream")]
    Pipeline,
    /// The pipeline ran to completion but no loudnorm JSON report was captured.
    #[error("no loudnorm report was captured")]
    NoReport,
    /// The captured report wasn't valid UTF-8 (shouldn't happen — loudnorm's JSON output is
    /// ASCII — but the buffer is truncated if it doesn't fit, which could in principle split
    /// a multi-byte sequence).
    #[error("loudnorm report wasn't valid UTF-8: {0}")]
    InvalidUtf8(std::str::Utf8Error),
    /// The C side returned a status code this crate doesn't know about.
    #[error("unknown loudness status code: {0}")]
    Unknown(c_int),
}

/// Measures integrated loudness / true peak / loudness range via a single-pass `loudnorm`
/// analysis (`I=-16:TP=-1.5:LRA=11`). Returns the raw JSON report text — parsing it is the
/// caller's job (e.g. `avcore::loudness::parse_loudnorm_stderr`, which works on this
/// text unchanged since it never cared whether the text came from a subprocess or here).
///
/// Not thread-safe — see the `// SAFETY:` note below and the doc comment on the C side.
/// Callers must serialize calls to this function (across the whole process, not just per
/// `Path`) with each other and with nothing else that installs an `av_log` callback.
pub fn measure_loudness_json(path: &Path) -> Result<String, LoudnessError> {
    let c_path =
        CString::new(path.to_string_lossy().as_bytes()).map_err(LoudnessError::InvalidPath)?;

    let mut buf = vec![0u8; 4096];

    // SAFETY: c_path is a valid NUL-terminated C string for the duration of this call. `buf`
    // is a valid, writable buffer of `buf.len()` bytes; the C side never writes past that
    // length and always NUL-terminates within it on success. `bridge.c` closes the decoder
    // and filter graph contexts on every exit path.
    let status = unsafe {
        avbridge_measure_loudness(c_path.as_ptr(), buf.as_mut_ptr() as *mut c_char, buf.len())
    };

    match status {
        0 => {}
        1 => return Err(LoudnessError::OpenInput),
        2 => return Err(LoudnessError::StreamInfo),
        3 => return Err(LoudnessError::NoAudioStream),
        4 => return Err(LoudnessError::Decoder),
        5 => return Err(LoudnessError::FilterGraph),
        6 => return Err(LoudnessError::Pipeline),
        7 => return Err(LoudnessError::NoReport),
        other => return Err(LoudnessError::Unknown(other)),
    }

    // SAFETY: on success the C side NUL-terminates the report within `buf`.
    let json = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
    json.to_str()
        .map(str::to_owned)
        .map_err(LoudnessError::InvalidUtf8)
}

/// What [`generate_proxy`] failed on.
#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    /// `in_path`/`out_path` contains a NUL byte and can't be handed to the C API.
    #[error("path is not a valid C string: {0}")]
    InvalidPath(NulError),
    #[error("failed to open input")]
    OpenInput,
    #[error("failed to read stream info")]
    StreamInfo,
    /// `in_path` has no video stream to make a proxy of.
    #[error("input has no video stream")]
    NoVideoStream,
    #[error("failed to allocate output context")]
    AllocOutput,
    #[error("failed to create an output stream")]
    NewStream,
    #[error("failed to open output for writing")]
    OpenOutput,
    #[error("failed to write output header")]
    WriteHeader,
    #[error("failed to write a frame")]
    WriteFrame,
    /// Couldn't find/open the video or audio decoder.
    #[error("failed to open a decoder")]
    Decoder,
    /// Couldn't find/open the `libopenh264` video encoder or the AAC audio encoder.
    #[error("failed to open an encoder")]
    Encoder,
    /// Couldn't build the video scaler.
    #[error("failed to build the video scaler")]
    Scaler,
    /// Couldn't build the (filterless, format-conversion-only) audio graph.
    #[error("failed to build the audio format-match graph")]
    FilterGraph,
    /// A decode/scale/encode call failed mid-stream (not at setup).
    #[error("decode/scale/encode pipeline failed mid-stream")]
    Pipeline,
    /// The C side returned a status code this crate doesn't know about.
    #[error("unknown proxy status code: {0}")]
    Unknown(c_int),
}

/// Generates a downscaled editing proxy of `in_path`'s video (height = `target_height`, width
/// computed to preserve the source's aspect ratio) via `libopenh264` (BSD-licensed — this
/// LGPL FFmpeg build has no `libx264`/GPL). Any audio stream is re-encoded to AAC 128kbps
/// unchanged otherwise. Fails with [`ProxyError::NoVideoStream`] if `in_path` has no video
/// stream; a video-only source produces a video-only proxy, no error.
pub fn generate_proxy(
    in_path: &Path,
    out_path: &Path,
    target_height: u32,
) -> Result<(), ProxyError> {
    let c_in =
        CString::new(in_path.to_string_lossy().as_bytes()).map_err(ProxyError::InvalidPath)?;
    let c_out =
        CString::new(out_path.to_string_lossy().as_bytes()).map_err(ProxyError::InvalidPath)?;

    // SAFETY: c_in/c_out are valid NUL-terminated C strings for the duration of this call.
    // `bridge.c` frees the decoder/encoder/scaler/filter-graph contexts, output I/O, and
    // output context on every exit path.
    let status =
        unsafe { avbridge_generate_proxy(c_in.as_ptr(), c_out.as_ptr(), target_height as c_int) };

    match status {
        0 => Ok(()),
        1 => Err(ProxyError::OpenInput),
        2 => Err(ProxyError::StreamInfo),
        3 => Err(ProxyError::NoVideoStream),
        4 => Err(ProxyError::AllocOutput),
        5 => Err(ProxyError::NewStream),
        6 => Err(ProxyError::OpenOutput),
        7 => Err(ProxyError::WriteHeader),
        8 => Err(ProxyError::WriteFrame),
        9 => Err(ProxyError::Decoder),
        10 => Err(ProxyError::Encoder),
        11 => Err(ProxyError::Scaler),
        12 => Err(ProxyError::FilterGraph),
        13 => Err(ProxyError::Pipeline),
        other => Err(ProxyError::Unknown(other)),
    }
}

/// What [`generate_waveform`] failed on.
#[derive(Debug, thiserror::Error)]
pub enum WaveformError {
    /// `path` contains a NUL byte and can't be handed to the C API.
    #[error("path is not a valid C string: {0}")]
    InvalidPath(NulError),
    #[error("failed to open input")]
    OpenInput,
    #[error("failed to read stream info")]
    StreamInfo,
    /// `path` has no audio stream to compute a waveform from.
    #[error("input has no audio stream")]
    NoAudioStream,
    /// Couldn't find/open the audio decoder.
    #[error("failed to open the audio decoder")]
    Decoder,
    /// Couldn't build the mono-downmix filter graph.
    #[error("failed to build the mono-downmix filter graph")]
    FilterGraph,
    /// A decode/filter call failed mid-stream (not at setup).
    #[error("decode/filter pipeline failed mid-stream")]
    Pipeline,
    /// The C side returned a status code this crate doesn't know about.
    #[error("unknown waveform status code: {0}")]
    Unknown(c_int),
}

/// Computes `bucket_count` per-bucket (min, max) amplitude peaks — each in `[-1.0, 1.0]` — of
/// `path`'s audio stream downmixed to mono, for waveform rendering. The full duration is
/// divided into equal-length buckets by sample index, so a bucket with no samples in it (e.g.
/// the duration estimate undershot) comes back as `(0.0, 0.0)` rather than left uninitialized.
pub fn generate_waveform(
    path: &Path,
    bucket_count: usize,
) -> Result<Vec<(f32, f32)>, WaveformError> {
    let c_path =
        CString::new(path.to_string_lossy().as_bytes()).map_err(WaveformError::InvalidPath)?;

    let mut mins = vec![0.0f32; bucket_count];
    let mut maxs = vec![0.0f32; bucket_count];

    // SAFETY: c_path is a valid NUL-terminated C string for the duration of this call.
    // `mins`/`maxs` are valid, writable buffers of `bucket_count` f32s each — the C side never
    // writes past that length. `bridge.c` closes the decoder and filter graph contexts on
    // every exit path.
    let status = unsafe {
        avbridge_generate_waveform(
            c_path.as_ptr(),
            bucket_count as c_int,
            mins.as_mut_ptr(),
            maxs.as_mut_ptr(),
        )
    };

    match status {
        0 => {}
        1 => return Err(WaveformError::OpenInput),
        2 => return Err(WaveformError::StreamInfo),
        3 => return Err(WaveformError::NoAudioStream),
        4 => return Err(WaveformError::Decoder),
        5 => return Err(WaveformError::FilterGraph),
        6 => return Err(WaveformError::Pipeline),
        other => return Err(WaveformError::Unknown(other)),
    }

    Ok(mins.into_iter().zip(maxs).collect())
}
