//! Renders a normalized export from a source file — Fase 4's render step. Applies loudness
//! normalization (`loudnorm` + a true-peak safety limiter, per Fase 2) via `oca-avbridge`'s
//! encode FFI while copying video untouched, which is what makes the export's bitrate equal
//! the source's bitrate exactly, one of this editor's two headline differentiators.

use std::path::Path;
use std::sync::atomic::AtomicBool;

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
