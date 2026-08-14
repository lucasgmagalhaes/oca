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

/// How a transcription ended. A cancelled transcription still carries whatever segments
/// whisper.cpp had already emitted before the abort callback fired — there's no "invalid
/// truncated file" concern here (nothing is written to disk), so partial results are still
/// useful to the caller rather than being discarded.
#[derive(Debug, Clone, PartialEq)]
pub enum TranscribeOutcome {
    Completed(Vec<TranscribeSegment>),
    Cancelled(Vec<TranscribeSegment>),
}

/// Trampoline for whisper.cpp's `ggml_abort_callback` (`unsafe extern "C" fn(*mut c_void) ->
/// bool`), used via `FullParams`'s raw `set_abort_callback`/`set_abort_callback_user_data`
/// instead of the crate's own `set_abort_callback_safe` — see [`transcribe`]'s docs for why.
/// `user_data` must point at a `Box<dyn FnMut() -> bool>` previously leaked via
/// `Box::into_raw(Box::new(Box::new(closure) as Box<dyn FnMut() -> bool>))` (double-boxed: an
/// owned `Box<dyn Trait>` is itself a fat pointer, so it needs its own heap slot to have a
/// stable thin address to hand across the FFI boundary) — the same construction
/// `set_progress_callback_safe` correctly uses, just paired here with a trampoline that agrees
/// with it on the pointee type, which is exactly what `set_abort_callback_safe` gets wrong.
unsafe extern "C" fn abort_trampoline(user_data: *mut std::ffi::c_void) -> bool {
    // SAFETY: caller (this module's `transcribe`) guarantees user_data was produced by the
    // construction described above and remains valid (the Box isn't freed) for as long as
    // whisper.cpp might still call this — i.e. until after `state.full()` returns.
    let closure = unsafe { &mut *(user_data as *mut Box<dyn FnMut() -> bool>) };
    closure()
}

/// Transcribes `source_path`'s audio using the Whisper model at `model_path`.
///
/// `language` is a Whisper language code (e.g. `Some("pt")`); `None` lets Whisper
/// auto-detect the spoken language from the first ~30 seconds of audio.
///
/// Calls `on_progress(percent)` (0-100) as inference proceeds, and checks `cancel` between
/// whisper.cpp's internal steps — if another thread sets it, inference stops early and this
/// returns `Ok(TranscribeOutcome::Cancelled(partial_segments))` rather than treating the
/// cancellation as a failure (same convention as [`crate::render::render_export`]).
///
/// Cancellation is wired through `FullParams`'s raw, `unsafe` `set_abort_callback` API rather
/// than the crate's own safe `set_abort_callback_safe`, which has a real upstream bug in
/// `whisper-rs` 0.16.0: its trampoline casts the callback's double-boxed
/// `Box<dyn FnMut() -> bool>` pointer back to the *original* concrete closure type instead of
/// `Box<dyn FnMut() -> bool>` (compare its `trampoline::<F>` to `set_progress_callback_safe`'s
/// correct `trampoline::<Box<dyn FnMut(i32)>>`) — undefined behavior, confirmed empirically:
/// any closure capturing state made whisper.cpp abort on the very first internal check
/// regardless of what the closure actually returned, immediately failing with "failed to
/// encode" (`GenericError(-6)`). [`abort_trampoline`] above reimplements the same
/// double-box construction with the matching pointee type instead.
///
/// `cancel` is an `Arc<AtomicBool>` rather than a plain reference (unlike the `render`/`encode`
/// functions' `&AtomicBool`) because `set_progress_callback_safe` (which IS used as-is, and IS
/// correct) requires a `'static` closure.
pub fn transcribe(
    source_path: &Path,
    model_path: &Path,
    language: Option<&str>,
    cancel: &Arc<AtomicBool>,
    on_progress: impl FnMut(i32) + 'static,
) -> Result<TranscribeOutcome, TranscribeError> {
    if cancel.load(Ordering::Relaxed) {
        return Ok(TranscribeOutcome::Cancelled(Vec::new()));
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

    let cancel_for_abort = Arc::clone(cancel);
    let abort_closure: Box<dyn FnMut() -> bool> =
        Box::new(move || cancel_for_abort.load(Ordering::Relaxed));
    let abort_user_data = Box::into_raw(Box::new(abort_closure));
    // SAFETY: abort_user_data points at the Box<dyn FnMut() -> bool> abort_trampoline expects,
    // built exactly the way its doc comment requires. It's reclaimed (Box::from_raw) right
    // after state.full() returns below, by which point whisper.cpp is guaranteed to have
    // stopped calling it — the call is synchronous and whisper.cpp doesn't retain the pointer
    // past it.
    unsafe {
        params.set_abort_callback(Some(abort_trampoline));
        params.set_abort_callback_user_data(abort_user_data as *mut std::ffi::c_void);
    }

    let full_result = state.full(params, &samples);
    // SAFETY: state.full() has returned, so whisper.cpp will not call abort_trampoline with
    // this pointer again — safe to reclaim and drop now.
    unsafe {
        drop(Box::from_raw(abort_user_data));
    }

    // whisper.cpp's return code doesn't distinguish "aborted via callback" from a genuine
    // encode/decode failure (both surface as e.g. WhisperError::FailedToEncode) — so once
    // cancel is set, full_result's specific Err (if any) can't be trusted to tell them apart.
    // Treat it as a cancellation unconditionally in that case, using whatever segments were
    // already committed before the abort rather than discarding them as an error.
    if cancel.load(Ordering::Relaxed) {
        return Ok(TranscribeOutcome::Cancelled(collect_segments(&state)));
    }
    full_result.map_err(|_| TranscribeError::Inference)?;

    Ok(TranscribeOutcome::Completed(collect_segments(&state)))
}

fn collect_segments(state: &whisper_rs::WhisperState) -> Vec<TranscribeSegment> {
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
    segments
}
