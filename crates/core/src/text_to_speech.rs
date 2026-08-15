//! Text-to-speech — per `request.md`'s Fase 4 "Texto-pra-fala": synthesizes narration from
//! typed text. Runs a Piper VITS ONNX voice model (`rhasspy/piper-voices` on Hugging Face, MIT)
//! against phonemes produced by `espeak-rs` (statically-linked espeak-ng, no runtime DLL — see
//! `build.rs`'s doc comment for how its `espeak-ng-data` directory gets discovered at runtime).
//!
//! Pipeline: [`espeak_rs::text_to_phonemes`] (text -> one IPA string per sentence) ->
//! [`phonemes_to_ids`] (IPA string -> phoneme IDs, via the voice's own `phoneme_id_map`) ->
//! [`synthesize`] (ONNX VITS inference -> raw `f32` samples) -> [`write_wav`].
//!
//! **Phoneme ID scheme:** BOS (`"^"`) at the start, EOS (`"$"`) at the end, and PAD (`"_"`)
//! interspersed after every phoneme symbol — this exact scheme (not guessed: confirmed against
//! a real downloaded voice's `.onnx.json`) is what every published Piper voice was trained on,
//! matching the reference Python implementation's `phonemes_to_ids`.
//!
//! **Model I/O contract:** `input` (`int64[1,N]` phoneme ids), `input_lengths` (`int64[1]`),
//! `scales` (`float32[3]` = `[noise_scale, length_scale, noise_w]`), optionally `sid`
//! (`int64[1]`, only for multi-speaker voices) -> `output` (`float32[1,1,T]` samples in
//! `-1.0..=1.0` at the voice's `sample_rate`). This is Piper's well-established, publicly
//! documented ONNX export contract, not guessed — and confirmed empirically end-to-end on this
//! dev machine (see the module's smoke-tested history in `CLAUDE.md`).

use std::collections::HashMap;
use std::path::Path;

use unicode_segmentation::UnicodeSegmentation;

const PAD_SYMBOL: &str = "_";
const BOS_SYMBOL: &str = "^";
const EOS_SYMBOL: &str = "$";

/// A Piper voice's `.onnx.json` sidecar — only the fields `text_to_speech` actually needs.
/// `#[serde(default)]` throughout since Piper's config schema has grown fields over the years
/// (e.g. `phoneme_map`) that not every voice's config includes.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PiperVoiceConfig {
    pub audio: PiperAudioConfig,
    pub espeak: PiperEspeakConfig,
    #[serde(default)]
    pub inference: PiperInferenceConfig,
    #[serde(default)]
    pub num_speakers: u32,
    /// Character substitutions applied before phoneme-ID lookup (e.g. some voices map `"c"` to
    /// `["k"]`) — rare in practice but silently wrong output (falls through to "unknown
    /// phoneme, skip") if ignored on a voice that has one.
    #[serde(default)]
    pub phoneme_map: HashMap<String, Vec<String>>,
    pub phoneme_id_map: HashMap<String, Vec<i64>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PiperAudioConfig {
    pub sample_rate: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PiperEspeakConfig {
    /// espeak-ng voice/language code (e.g. `"pt-br"`) — passed straight to
    /// [`espeak_rs::text_to_phonemes`].
    pub voice: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PiperInferenceConfig {
    #[serde(default = "default_noise_scale")]
    pub noise_scale: f32,
    #[serde(default = "default_length_scale")]
    pub length_scale: f32,
    #[serde(default = "default_noise_w")]
    pub noise_w: f32,
}

impl Default for PiperInferenceConfig {
    fn default() -> Self {
        Self {
            noise_scale: default_noise_scale(),
            length_scale: default_length_scale(),
            noise_w: default_noise_w(),
        }
    }
}

fn default_noise_scale() -> f32 {
    0.667
}
fn default_length_scale() -> f32 {
    1.0
}
fn default_noise_w() -> f32 {
    0.8
}

#[derive(Debug)]
pub enum TtsError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Espeak(String),
    Onnx(String),
    /// `synthesize` produced no audio at all — e.g. `text` phonemized to nothing (all
    /// whitespace/punctuation with no speakable content).
    NoAudio,
}

impl std::fmt::Display for TtsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TtsError::Io(e) => write!(f, "I/O error: {e}"),
            TtsError::Json(e) => write!(f, "voice config parse error: {e}"),
            TtsError::Espeak(e) => write!(f, "phonemization failed: {e}"),
            TtsError::Onnx(e) => write!(f, "ONNX inference failed: {e}"),
            TtsError::NoAudio => write!(f, "no speakable content in the given text"),
        }
    }
}

impl std::error::Error for TtsError {}

/// Loads and parses a Piper voice's `.onnx.json` sidecar file.
pub fn load_voice_config(config_path: &Path) -> Result<PiperVoiceConfig, TtsError> {
    let bytes = std::fs::read(config_path).map_err(TtsError::Io)?;
    serde_json::from_slice(&bytes).map_err(TtsError::Json)
}

/// Splits an espeak IPA phoneme string into individual symbols by Unicode grapheme cluster
/// (keeps a base letter and any combining stress/length diacritic together as one symbol,
/// matching how Piper voices were trained — a plain `char` split would separate them).
fn split_phoneme_symbols(ipa: &str) -> Vec<String> {
    ipa.graphemes(true).map(str::to_string).collect()
}

/// Converts one sentence's IPA phoneme symbols into a Piper phoneme-ID sequence: BOS, then for
/// each symbol its ID(s) followed by PAD, then EOS — the exact scheme every published Piper
/// voice was trained on (see module docs). Applies `config.phoneme_map` substitutions first.
/// A symbol with no entry in `phoneme_id_map` (after substitution) is silently skipped — same
/// behavior as the reference implementation (unknown/untrainable symbols, e.g. some punctuation).
pub fn phonemes_to_ids(ipa: &str, config: &PiperVoiceConfig) -> Vec<i64> {
    let bos = config
        .phoneme_id_map
        .get(BOS_SYMBOL)
        .cloned()
        .unwrap_or_default();
    let eos = config
        .phoneme_id_map
        .get(EOS_SYMBOL)
        .cloned()
        .unwrap_or_default();
    let pad = config
        .phoneme_id_map
        .get(PAD_SYMBOL)
        .cloned()
        .unwrap_or_default();

    let mut ids = bos;
    for symbol in split_phoneme_symbols(ipa) {
        let mapped: Vec<String> = config
            .phoneme_map
            .get(&symbol)
            .cloned()
            .unwrap_or_else(|| vec![symbol]);
        for m in mapped {
            if let Some(symbol_ids) = config.phoneme_id_map.get(&m) {
                ids.extend(symbol_ids);
                ids.extend(&pad);
            }
        }
    }
    ids.extend(eos);
    ids
}

/// Phonemizes `text` with espeak-ng (voice `config.espeak.voice`) and runs Piper VITS inference
/// (`model_path`) sentence-by-sentence, concatenating the results with a short silence gap
/// between sentences. Returns raw `f32` samples in `-1.0..=1.0` at `config.audio.sample_rate`.
pub fn synthesize(
    model_path: &Path,
    config: &PiperVoiceConfig,
    text: &str,
) -> Result<Vec<f32>, TtsError> {
    use ort::session::Session;
    use ort::value::Value;

    let sentences = espeak_rs::text_to_phonemes(text, &config.espeak.voice, None)
        .map_err(|e| TtsError::Espeak(e.0))?;

    let mut session = Session::builder()
        .map_err(|e| TtsError::Onnx(e.to_string()))?
        .commit_from_file(model_path)
        .map_err(|e| TtsError::Onnx(e.to_string()))?;

    // A quarter-second of silence between sentences reads more naturally than butting them
    // together with no pause at all.
    let gap = vec![0.0f32; (config.audio.sample_rate as usize) / 4];
    let mut samples = Vec::new();

    for sentence in &sentences {
        let ids = phonemes_to_ids(sentence, config);
        if ids.is_empty() {
            continue;
        }
        let ids_len = ids.len();

        let input =
            Value::from_array(([1, ids_len], ids)).map_err(|e| TtsError::Onnx(e.to_string()))?;
        let input_lengths = Value::from_array(([1], vec![ids_len as i64]))
            .map_err(|e| TtsError::Onnx(e.to_string()))?;
        let scales = Value::from_array((
            [3],
            vec![
                config.inference.noise_scale,
                config.inference.length_scale,
                config.inference.noise_w,
            ],
        ))
        .map_err(|e| TtsError::Onnx(e.to_string()))?;

        let mut inputs = vec![
            ("input", input.into_dyn()),
            ("input_lengths", input_lengths.into_dyn()),
            ("scales", scales.into_dyn()),
        ];
        if config.num_speakers > 1 {
            let sid =
                Value::from_array(([1], vec![0i64])).map_err(|e| TtsError::Onnx(e.to_string()))?;
            inputs.push(("sid", sid.into_dyn()));
        }

        let outputs = session
            .run(inputs)
            .map_err(|e| TtsError::Onnx(e.to_string()))?;
        let (_name, value) = outputs
            .iter()
            .next()
            .ok_or_else(|| TtsError::Onnx("model produced no output tensors".into()))?;
        let (_shape, data) = value
            .try_extract_tensor::<f32>()
            .map_err(|e| TtsError::Onnx(e.to_string()))?;

        if !samples.is_empty() {
            samples.extend_from_slice(&gap);
        }
        samples.extend_from_slice(data);
    }

    if samples.is_empty() {
        return Err(TtsError::NoAudio);
    }
    Ok(samples)
}

/// Writes `samples` (mono, `-1.0..=1.0`) to `out_path` as a 16-bit PCM WAV file at `sample_rate`.
pub fn write_wav(out_path: &Path, sample_rate: u32, samples: &[f32]) -> Result<(), TtsError> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer =
        hound::WavWriter::create(out_path, spec).map_err(|e| TtsError::Io(io_from_hound(e)))?;
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        writer
            .write_sample((clamped * i16::MAX as f32) as i16)
            .map_err(|e| TtsError::Io(io_from_hound(e)))?;
    }
    writer
        .finalize()
        .map_err(|e| TtsError::Io(io_from_hound(e)))?;
    Ok(())
}

fn io_from_hound(e: hound::Error) -> std::io::Error {
    match e {
        hound::Error::IoError(io) => io,
        other => std::io::Error::other(other.to_string()),
    }
}

#[cfg(test)]
#[path = "text_to_speech/text_to_speech_test.rs"]
mod tests;
