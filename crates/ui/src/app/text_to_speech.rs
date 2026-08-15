use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{tts_output_dir, App, TtsEvent};

impl App {
    /// Opens the "Texto-pra-fala" modal with an empty text buffer — what the Mídia screen's
    /// "Texto-pra-fala" button does. A no-op if the modal is already open or a generation is
    /// already running.
    pub fn open_tts_modal(&mut self) {
        if self.tts_modal_text.is_some() || self.tts_generating {
            return;
        }
        self.tts_modal_text = Some(String::new());
    }

    /// Synthesizes [`App::tts_modal_text`] on a background thread — what the modal's "Gerar"
    /// button does. Closes the modal immediately (the generation itself keeps running); on
    /// success the resulting WAV is imported into the active project's media library the same
    /// way any other file drop is ([`App::spawn_import`], via [`App::pump_text_to_speech`]). A
    /// no-op if the buffer is blank, the voice model isn't configured yet, or a generation is
    /// already in flight.
    pub fn spawn_generate_tts(&mut self) {
        if self.tts_generating {
            return;
        }
        let Some(text) = self.tts_modal_text.take() else {
            return;
        };
        let text = text.trim().to_string();
        if text.is_empty() || self.prefs.tts_model_path.is_empty() {
            return;
        }

        self.tts_generating = true;
        let model_path = PathBuf::from(&self.prefs.tts_model_path);
        let out_dir = tts_output_dir();
        let tx = self.tts_tx.clone();
        std::thread::spawn(move || {
            let event = match generate_tts_one(&model_path, &text, &out_dir) {
                Ok(wav_path) => TtsEvent::Done { wav_path },
                Err(message) => TtsEvent::Failed { message },
            };
            let _ = tx.send(event);
        });
    }

    /// Applies a finished (or failed) TTS run. Called once per frame from
    /// [`eframe::App::ui`], same as [`App::pump_auto_reframe`]. A successful run is handed to
    /// [`App::spawn_import`] so it goes through the exact same probe/loudness/waveform pipeline
    /// as any imported file.
    pub(super) fn pump_text_to_speech(&mut self) {
        while let Ok(event) = self.tts_rx.try_recv() {
            self.tts_generating = false;
            match event {
                TtsEvent::Done { wav_path } => {
                    self.spawn_import(vec![wav_path]);
                }
                TtsEvent::Failed { message } => {
                    self.push_toast(format!(
                        "{}: {message}",
                        crate::i18n::Text::TtsGenerationFailed.tr(self.locale)
                    ));
                }
            }
        }
    }
}

/// Runs on [`App::spawn_generate_tts`]'s background thread — loads the voice config next to
/// `model_path` (`<model_path>.json`, the layout [`avcore::download_tts_voice`] downloads into),
/// synthesizes `text`, and writes the result as a uniquely-named WAV under `out_dir`.
fn generate_tts_one(model_path: &Path, text: &str, out_dir: &Path) -> Result<PathBuf, String> {
    let config_path = PathBuf::from(format!("{}.json", model_path.display()));
    let config = avcore::load_voice_config(&config_path).map_err(|e| e.to_string())?;
    let samples = avcore::synthesize(model_path, &config, text).map_err(|e| e.to_string())?;

    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let out_path = out_dir.join(format!("tts_{timestamp}.wav"));
    avcore::write_wav(&out_path, config.audio.sample_rate, &samples).map_err(|e| e.to_string())?;
    Ok(out_path)
}
