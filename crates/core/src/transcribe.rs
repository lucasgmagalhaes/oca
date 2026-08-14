//! Automatic speech transcription — Fase 4's "Legendas automáticas". Runs a local Whisper
//! model (whisper.cpp via the `whisper-rs` crate) fully offline: audio is decoded/resampled to
//! the 16kHz mono float PCM whisper.cpp requires via `avbridge::extract_pcm_16k_mono` (no
//! subprocess, same FFI-not-shell-out approach as every other `avbridge` call), then run
//! through Whisper's inference to produce timestamped segments.
//!
//! The model file itself isn't bundled — see `request.md`'s Fase 8 packaging notes ("Modelo do
//! Whisper... incluídos no instalador"), not yet implemented. Callers pass a path to an
//! already-downloaded GGML model (e.g. `ggml-base.bin` from
//! <https://huggingface.co/ggerganov/whisper.cpp>).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// One transcribed segment: a `[start_secs, end_secs)` time range plus its recognized text.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscribeSegment {
    pub start_secs: f64,
    pub end_secs: f64,
    pub text: String,
}

#[derive(Debug)]
pub enum TranscribeError {
    /// Failed to decode/resample `source_path`'s audio — see [`avbridge::PcmError`].
    Audio(avbridge::PcmError),
    /// The source decoded successfully but produced zero PCM samples — nothing to transcribe.
    NoAudio,
    /// `model_path` doesn't exist, isn't a valid GGML Whisper model, or otherwise failed to
    /// load.
    ModelLoad(std::path::PathBuf),
    /// Failed to allocate a whisper.cpp inference state for the loaded model.
    StateInit,
    /// Whisper inference itself failed (not a cancellation — see [`TranscribeOutcome::Cancelled`]
    /// for that).
    Inference,
}

impl std::fmt::Display for TranscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranscribeError::Audio(e) => write!(f, "failed to extract audio: {e}"),
            TranscribeError::NoAudio => write!(f, "source has no audio to transcribe"),
            TranscribeError::ModelLoad(p) => {
                write!(f, "failed to load whisper model at {}", p.display())
            }
            TranscribeError::StateInit => write!(f, "failed to create whisper inference state"),
            TranscribeError::Inference => write!(f, "whisper inference failed"),
        }
    }
}

impl std::error::Error for TranscribeError {}

impl From<avbridge::PcmError> for TranscribeError {
    fn from(e: avbridge::PcmError) -> Self {
        TranscribeError::Audio(e)
    }
}

/// How a transcription ended.
#[derive(Debug, Clone, PartialEq)]
pub enum TranscribeOutcome {
    Completed(Vec<TranscribeSegment>),
    /// `cancel` was already set before inference started — nothing was transcribed. See
    /// [`transcribe`]'s docs for why cancellation can't currently interrupt inference already
    /// in progress.
    Cancelled,
}

/// Transcribes `source_path`'s audio using the Whisper model at `model_path`.
///
/// `language` is a Whisper language code (e.g. `Some("pt")`); `None` lets Whisper
/// auto-detect the spoken language from the first ~30 seconds of audio.
///
/// Calls `on_progress(percent)` (0-100) as inference proceeds. `cancel` is checked only once,
/// immediately before inference starts — if already set, this returns
/// `Ok(TranscribeOutcome::Cancelled)` without running whisper.cpp at all, same convention as
/// [`crate::render::render_export`]'s early-cancel case.
///
/// Cancellation can't currently interrupt inference once it's running: `whisper-rs` 0.16.0's
/// `set_abort_callback_safe` has a real upstream bug — its trampoline casts the callback's
/// double-boxed `Box<dyn FnMut() -> bool>` pointer back to the *original* concrete closure type
/// instead of `Box<dyn FnMut() -> bool>` (compare its `trampoline::<F>` to
/// `set_progress_callback_safe`'s correct `trampoline::<Box<dyn FnMut(i32)>>`), which is
/// undefined behavior — confirmed empirically: any closure capturing state (even a
/// non-atomic `bool` local) makes whisper.cpp abort on the very first internal check
/// regardless of what the closure actually returns, immediately failing with "failed to
/// encode" (`GenericError(-6)`). A closure with zero captures (`|| false`) happens not to
/// trigger it, but that's not something safe to build cancellation on. Not used here until
/// upstream fixes it or this crate vendors a corrected trampoline.
///
/// `cancel` is an `Arc<AtomicBool>` rather than a plain reference (unlike the `render`/`encode`
/// functions' `&AtomicBool`) because `set_progress_callback_safe` (which IS used, and IS
/// correct) requires a `'static` closure.
pub fn transcribe(
    source_path: &Path,
    model_path: &Path,
    language: Option<&str>,
    cancel: &Arc<AtomicBool>,
    mut on_progress: impl FnMut(i32) + 'static,
) -> Result<TranscribeOutcome, TranscribeError> {
    if cancel.load(Ordering::Relaxed) {
        return Ok(TranscribeOutcome::Cancelled);
    }

    let samples = avbridge::extract_pcm_16k_mono(source_path)?;
    if samples.is_empty() {
        return Err(TranscribeError::NoAudio);
    }

    let model_path_str = model_path.to_string_lossy().into_owned();
    let ctx = WhisperContext::new_with_params(&model_path_str, WhisperContextParameters::default())
        .map_err(|_| TranscribeError::ModelLoad(model_path.to_path_buf()))?;
    let mut state = ctx.create_state().map_err(|_| TranscribeError::StateInit)?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(language);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_progress_callback_safe(on_progress);

    state
        .full(params, &samples)
        .map_err(|_| TranscribeError::Inference)?;

    let num_segments = state.full_n_segments();
    let mut segments = Vec::with_capacity(num_segments.max(0) as usize);
    for i in 0..num_segments {
        let Some(segment) = state.get_segment(i) else {
            continue;
        };
        let text = segment.to_str().unwrap_or_default().trim().to_string();
        if text.is_empty() {
            continue;
        }
        // whisper.cpp timestamps are in centiseconds (units of 10ms).
        segments.push(TranscribeSegment {
            start_secs: segment.start_timestamp() as f64 * 0.01,
            end_secs: segment.end_timestamp() as f64 * 0.01,
            text,
        });
    }

    Ok(TranscribeOutcome::Completed(segments))
}
