// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui;

use crate::components;
use crate::i18n::Text;
use crate::theme;

use super::{tts_output_dir, App, TtsEvent};

impl App {
    /// Shows the "Texto-pra-fala" modal when [`App::tts_modal_text`] is `Some`. "Gerar" starts
    /// the background synthesis ([`App::spawn_generate_tts`]) and closes the modal immediately
    /// (a no-op, leaving the modal open, while the buffer is blank or no voice model is
    /// configured); Escape/Cancel discards without generating anything.
    pub(super) fn show_tts_modal(&mut self, ctx: &egui::Context) {
        if self.tts_state.tts_modal_text.is_none() {
            return;
        }
        let locale = self.locale;
        let model_configured = !self.prefs.tts_model_path.is_empty();
        let modal = egui::Modal::new(egui::Id::new("tts_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(420.0);
            components::modal_title(ui, Text::TtsModalTitle.tr(locale));
            ui.add_space(8.0);
            if !model_configured {
                ui.label(
                    egui::RichText::new(Text::TtsNoModelConfigured.tr(locale))
                        .color(theme::TEXT_DISABLED),
                );
                ui.add_space(8.0);
            }
            let buf = self.tts_state.tts_modal_text.as_mut().unwrap();
            ui.add(
                egui::TextEdit::multiline(buf)
                    .desired_width(f32::INFINITY)
                    .desired_rows(4),
            );
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let text_blank = self
                    .tts_state
                    .tts_modal_text
                    .as_ref()
                    .is_some_and(|t| t.trim().is_empty());
                if ui
                    .add_enabled(
                        model_configured && !text_blank,
                        egui::Button::new(Text::TtsGenerate.tr(locale)),
                    )
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.tts_state.tts_modal_text = None;
            return;
        }
        if confirmed {
            self.spawn_generate_tts();
        }
    }
    /// Opens the "Texto-pra-fala" modal with an empty text buffer — what the Mídia screen's
    /// "Texto-pra-fala" button does. A no-op if the modal is already open or a generation is
    /// already running.
    pub fn open_tts_modal(&mut self) {
        if self.tts_state.tts_modal_text.is_some() || self.tts_state.tts_generating {
            return;
        }
        self.tts_state.tts_modal_text = Some(String::new());
    }

    /// Synthesizes [`App::tts_modal_text`] on a background thread — what the modal's "Gerar"
    /// button does. Closes the modal immediately (the generation itself keeps running); on
    /// success the resulting WAV is imported into the active project's media library the same
    /// way any other file drop is ([`App::spawn_import`], via [`App::pump_text_to_speech`]). A
    /// no-op if the buffer is blank, the voice model isn't configured yet, or a generation is
    /// already in flight.
    pub fn spawn_generate_tts(&mut self) {
        if self.tts_state.tts_generating {
            return;
        }
        let Some(text) = self.tts_state.tts_modal_text.take() else {
            return;
        };
        let text = text.trim().to_string();
        if text.is_empty() || self.prefs.tts_model_path.is_empty() {
            return;
        }

        self.tts_state.tts_generating = true;
        let model_path = PathBuf::from(&self.prefs.tts_model_path);
        let out_dir = tts_output_dir();
        let tx = self.tts_state.tts_tx.clone();
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
        while let Ok(event) = self.tts_state.tts_rx.try_recv() {
            self.tts_state.tts_generating = false;
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
/// `model_path` (`<model_path>.json`, the layout shipped in the release bundle),
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
