//! Renders a normalized export from a source file — Fase 4's render step. Applies loudness
//! normalization (`loudnorm` + a true-peak safety limiter, per Fase 2) via `oca-avbridge`'s
//! encode FFI while copying video untouched, which is what makes the export's bitrate equal
//! the source's bitrate exactly, one of this editor's two headline differentiators.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use avbridge::Canvas;

use crate::media::MediaAsset;
use crate::project::Sequence;
use crate::timeline::TrackKind;

#[derive(Debug)]
pub enum RenderError {
    OpenInput,
    StreamInfo,
    AllocOutput,
    NewStream,
    OpenOutput,
    WriteHeader,
    WriteFrame,
    /// `source` has no audio stream to normalize.
    NoAudioStream,
    Decoder,
    FilterGraph,
    Encoder,
    /// A decode/filter/encode call failed mid-render (not at setup).
    Pipeline,
    /// The FFI bridge returned a status this crate doesn't know about.
    Unknown(i32),
    /// (`render_timeline_export` only) a segment has no video stream.
    NoVideoStream,
    /// (`render_timeline_export` only) a later clip's audio format doesn't match the first
    /// clip's — see `avbridge::EncodeError::AudioFormatMismatch`.
    AudioFormatMismatch,
    /// (`render_timeline_export` only) the sequence's video track is missing or empty.
    EmptyTimeline,
    /// (`render_timeline_export` only) a clip's `asset_id` isn't in the project's media
    /// library.
    MissingAsset,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::OpenInput => write!(f, "failed to open input"),
            RenderError::StreamInfo => write!(f, "failed to read stream info"),
            RenderError::AllocOutput => write!(f, "failed to allocate output context"),
            RenderError::NewStream => write!(f, "failed to create an output stream"),
            RenderError::OpenOutput => write!(f, "failed to open output for writing"),
            RenderError::WriteHeader => write!(f, "failed to write output header"),
            RenderError::WriteFrame => write!(f, "failed to write a frame"),
            RenderError::NoAudioStream => write!(f, "source has no audio stream"),
            RenderError::Decoder => write!(f, "failed to open the audio decoder"),
            RenderError::FilterGraph => write!(f, "failed to build the audio filter graph"),
            RenderError::Encoder => write!(f, "failed to open the AAC encoder"),
            RenderError::Pipeline => write!(f, "decode/filter/encode pipeline failed mid-render"),
            RenderError::Unknown(code) => write!(f, "unknown render status code: {code}"),
            RenderError::NoVideoStream => write!(f, "a clip's source has no video stream"),
            RenderError::AudioFormatMismatch => {
                write!(
                    f,
                    "a clip's audio format doesn't match the sequence's first clip"
                )
            }
            RenderError::EmptyTimeline => write!(f, "sequence has no video clips to export"),
            RenderError::MissingAsset => write!(f, "a clip references a missing media asset"),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<avbridge::EncodeError> for RenderError {
    fn from(e: avbridge::EncodeError) -> Self {
        use avbridge::EncodeError as E;
        match e {
            E::InvalidPath(_) => RenderError::OpenInput,
            E::OpenInput => RenderError::OpenInput,
            E::StreamInfo => RenderError::StreamInfo,
            E::AllocOutput => RenderError::AllocOutput,
            E::NewStream => RenderError::NewStream,
            E::OpenOutput => RenderError::OpenOutput,
            E::WriteHeader => RenderError::WriteHeader,
            E::WriteFrame => RenderError::WriteFrame,
            E::NoAudioStream => RenderError::NoAudioStream,
            E::Decoder => RenderError::Decoder,
            E::FilterGraph => RenderError::FilterGraph,
            E::Encoder => RenderError::Encoder,
            E::Pipeline => RenderError::Pipeline,
            E::NoVideoStream => RenderError::NoVideoStream,
            E::AudioFormatMismatch => RenderError::AudioFormatMismatch,
            E::EmptyTimeline => RenderError::EmptyTimeline,
            E::Unknown(code) => RenderError::Unknown(code),
        }
    }
}

/// How a render ended: all the way through, or stopped early because `cancel` was set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderOutcome {
    Completed,
    Cancelled,
}

/// Renders `source` to `output`, applying single-pass loudnorm normalization to
/// `target_lufs` and copying video untouched. Blocks the calling thread until the render
/// finishes, is cancelled, or fails — callers on a UI thread should run this from a background
/// thread and use `cancel` to interrupt it instead of blocking on it.
///
/// Calls `on_progress(percent)` as the encode reports progress, and checks `cancel` between
/// packets — if another thread sets it, this returns `Ok(RenderOutcome::Cancelled)` rather
/// than treating the cancellation as a failure. `output` is left truncated/invalid in that
/// case (no trailer written), same as previously killing an in-progress `ffmpeg` subprocess.
pub fn render_export(
    source: &Path,
    output: &Path,
    target_lufs: f32,
    duration_secs: f64,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(u8),
) -> Result<RenderOutcome, RenderError> {
    let outcome = avbridge::encode_export(source, output, target_lufs, cancel, |secs| {
        if duration_secs > 0.0 {
            let percent = ((secs / duration_secs) * 100.0).clamp(0.0, 100.0) as u8;
            on_progress(percent);
        }
    })?;

    Ok(match outcome {
        avbridge::EncodeOutcome::Completed => {
            on_progress(100);
            RenderOutcome::Completed
        }
        avbridge::EncodeOutcome::Cancelled => RenderOutcome::Cancelled,
    })
}

/// Resolves `sequence`'s first non-empty video track into export segments (in `start_secs`
/// order) plus the canvas every segment gets conformed onto — the snapshot
/// [`crate::export::ExportJob`] takes when a job enters the queue, and what "Add Export"
/// button actually captures, replacing a single raw asset. Covers the effect fields
/// [`crate::timeline::ClipInstance::video_filter_chain`] documents (crop, brightness/contrast/
/// saturation, color filter, vignette, sharpen, chroma key, blur) plus per-clip audio gain and
/// freeze frame ([`crate::timeline::ClipInstance::frozen`], resolved to
/// [`avbridge::ClipSegment::frozen`] rather than a filter string); everything else
/// `features/request.md`'s Fase 4 status notes list (speed, zoom, transitions, masks,
/// multi-track compositing) isn't resolved yet.
///
/// The canvas's resolution/frame rate is the first clip's own probed resolution/fps; its
/// bitrate is a duration-weighted average of every clip's own source bitrate — video is
/// re-encoded here (unlike [`render_export`]'s exact passthrough copy), so an exact
/// source-bitrate match isn't possible once effects require decode+filter+encode; this is the
/// closest equivalent for a multi-clip timeline. Only the video track's clips' own embedded
/// audio is included — separate audio-only tracks aren't mixed in.
///
/// Fails with [`RenderError::EmptyTimeline`] if the sequence has no video track with at least
/// one clip, [`RenderError::MissingAsset`] if a clip's `asset_id` isn't in `media_library`.
pub fn resolve_timeline_segments(
    sequence: &Sequence,
    media_library: &[MediaAsset],
) -> Result<(Vec<avbridge::ClipSegment>, Canvas), RenderError> {
    let track = sequence
        .timeline
        .tracks
        .iter()
        .find(|t| t.kind == TrackKind::Video && !t.clips.is_empty())
        .ok_or(RenderError::EmptyTimeline)?;

    let mut clips: Vec<_> = track.clips.iter().collect();
    clips.sort_by(|a, b| a.start_secs.total_cmp(&b.start_secs));

    let mut segments = Vec::with_capacity(clips.len());
    let mut total_duration_secs = 0.0;
    let mut weighted_bitrate_bps_secs = 0.0;
    let mut dimensions_fps = None;
    for clip in &clips {
        let asset = media_library
            .iter()
            .find(|a| a.id == clip.asset_id)
            .ok_or(RenderError::MissingAsset)?;
        if dimensions_fps.is_none() {
            let (width, height) = asset.resolution.ok_or(RenderError::NoVideoStream)?;
            dimensions_fps = Some((width, height, asset.fps.unwrap_or(30.0)));
        }
        let duration_secs = clip.duration_secs();
        total_duration_secs += duration_secs;
        weighted_bitrate_bps_secs += asset.source_bitrate_mbps as f64 * 1_000_000.0 * duration_secs;
        segments.push(avbridge::ClipSegment {
            source_path: asset.source_path.clone(),
            source_in_secs: clip.source_in_secs,
            source_out_secs: clip.source_out_secs,
            gain_db: clip.gain_db,
            video_filter: clip.video_filter_chain(),
            frozen: clip.frozen,
            speed_factor: clip.speed_factor,
            zoom_start: clip.zoom_start,
            zoom_end: clip.zoom_end,
        });
    }
    let (width, height, fps) = dimensions_fps.ok_or(RenderError::EmptyTimeline)?;
    let (fps_num, fps_den) = fps_to_rational(fps);
    let bit_rate_bps = if total_duration_secs > 0.0 {
        (weighted_bitrate_bps_secs / total_duration_secs) as i64
    } else {
        0
    };
    let canvas = Canvas {
        width,
        height,
        fps_num,
        fps_den,
        bit_rate_bps,
    };

    Ok((segments, canvas))
}

/// Renders `segments` (already resolved by [`resolve_timeline_segments`], e.g. from a queued
/// [`crate::export::ExportJob`]) as one continuous file via `avbridge::encode_timeline_export`.
/// `on_progress` receives a 0-100 percent, computed off `segments`' own total trimmed
/// duration, same contract as [`render_export`].
pub fn render_export_job(
    segments: &[avbridge::ClipSegment],
    canvas: Canvas,
    output: &Path,
    target_lufs: f32,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(u8),
) -> Result<RenderOutcome, RenderError> {
    let total_duration_secs: f64 = segments
        .iter()
        .map(|s| {
            let speed = if s.speed_factor > 0.0 { s.speed_factor as f64 } else { 1.0 };
            (s.source_out_secs - s.source_in_secs) / speed
        })
        .sum();

    let outcome =
        avbridge::encode_timeline_export(segments, canvas, output, target_lufs, cancel, |secs| {
            if total_duration_secs > 0.0 {
                let percent = ((secs / total_duration_secs) * 100.0).clamp(0.0, 100.0) as u8;
                on_progress(percent);
            }
        })?;

    Ok(match outcome {
        avbridge::EncodeOutcome::Completed => {
            on_progress(100);
            RenderOutcome::Completed
        }
        avbridge::EncodeOutcome::Cancelled => RenderOutcome::Cancelled,
    })
}

/// Resolves `sequence` into segments ([`resolve_timeline_segments`]) and renders them
/// immediately ([`render_export_job`]) — the one-call convenience most callers want; the
/// export queue instead calls the two steps separately so it can snapshot the resolved
/// segments into an [`crate::export::ExportJob`] before rendering starts.
pub fn render_timeline_export(
    sequence: &Sequence,
    media_library: &[MediaAsset],
    output: &Path,
    target_lufs: f32,
    cancel: &AtomicBool,
    on_progress: impl FnMut(u8),
) -> Result<RenderOutcome, RenderError> {
    let (segments, canvas) = resolve_timeline_segments(sequence, media_library)?;
    render_export_job(&segments, canvas, output, target_lufs, cancel, on_progress)
}

/// Approximates `fps` as a small integer ratio for `avbridge::Canvas` — exact for whole frame
/// rates (`30.0` -> `30/1`), otherwise a fixed-point fraction (`29.97` -> `29970/1000`) rather
/// than a true lowest-terms rational (e.g. NTSC's canonical `30000/1001`), which is precise
/// enough for a canvas frame rate and doesn't need a full rational-approximation algorithm.
fn fps_to_rational(fps: f32) -> (u32, u32) {
    if fps <= 0.0 {
        return (30, 1);
    }
    let rounded = fps.round();
    if (fps - rounded).abs() < 0.001 {
        (rounded as u32, 1)
    } else {
        ((fps * 1000.0).round() as u32, 1000)
    }
}
