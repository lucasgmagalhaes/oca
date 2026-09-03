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

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use avcore::timeline::{AudioRole, Track, TrackKind};
use avcore::TranscribeOutcome;

use super::{timeline_ops::next_clip_id, App, TranscribeEvent};

impl App {
    /// Transcribes `asset_id`'s audio via `prefs.whisper_model_path` on a background thread —
    /// what the Mídia screen's "Transcrever" button does. A no-op if a transcription is already
    /// running (only one at a time, unlike imports) or no model is configured.
    ///
    /// Segments land as `TextClip`s on the active sequence's first `Text` track (created if
    /// none exists), starting at the current playhead — each segment's own `start_secs` within
    /// the source audio is preserved as an offset from there, so the block of generated
    /// subtitles keeps its internal timing/spacing, just anchored wherever the user parked the
    /// playhead before transcribing (same placement convention as [`App::add_text_clip`]).
    pub fn spawn_transcribe(&mut self, asset_id: u64) {
        if self.transcribe_state.transcribing_asset_id.is_some() {
            return;
        }
        if self.prefs.whisper_model_path.trim().is_empty() {
            self.push_toast(
                crate::i18n::Text::TranscribeNoModelConfigured
                    .tr(self.locale)
                    .to_string(),
            );
            return;
        }
        let Some(source_path) = self
            .active_project()
            .media_library
            .iter()
            .find(|a| a.id == asset_id)
            .map(|a| a.source_path.clone())
        else {
            return;
        };

        self.transcribe_state.transcribing_asset_id = Some(asset_id);
        let model_path = std::path::PathBuf::from(self.prefs.whisper_model_path.clone());
        let tx = self.transcribe_state.transcribe_tx.clone();
        std::thread::spawn(move || {
            transcribe_one(&source_path, &model_path, asset_id, &tx);
        });
    }

    /// Applies a finished transcription to the active project's timeline. Called once per
    /// frame from [`eframe::App::ui`], same as [`App::pump_import_queue`].
    pub(super) fn pump_transcribe(&mut self) {
        while let Ok(event) = self.transcribe_state.transcribe_rx.try_recv() {
            match event {
                TranscribeEvent::Done { asset_id, segments } => {
                    self.transcribe_state.transcribing_asset_id = None;
                    if segments.is_empty() {
                        self.push_toast(
                            crate::i18n::Text::TranscribeNoSpeechFound
                                .tr(self.locale)
                                .to_string(),
                        );
                        continue;
                    }
                    tracing::info!(
                        asset_id,
                        segment_count = segments.len(),
                        "transcription complete"
                    );
                    self.save_transcript_document_for(asset_id, &segments);
                    self.apply_transcription(segments);
                }
                TranscribeEvent::Failed { asset_id, message } => {
                    self.transcribe_state.transcribing_asset_id = None;
                    tracing::error!(asset_id, error = %message, "transcription failed");
                    self.push_toast(format!("Transcription failed: {message}"));
                }
            }
        }
    }

    /// Inserts one `TextClip` per segment onto the active sequence's first `Text` track
    /// (creating one if none exists yet), anchored at the current playhead.
    fn apply_transcription(&mut self, segments: Vec<avcore::TranscribeSegment>) {
        let playhead_secs = self.active_project().timeline().playhead_secs;
        let first_id = next_clip_id(self.active_project().timeline());

        let timeline = self.active_project_mut().timeline_mut();
        let track_id = match timeline.tracks.iter().find(|t| t.kind == TrackKind::Text) {
            Some(track) => track.id,
            None => {
                let id = timeline.tracks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                timeline.tracks.push(Track {
                    id,
                    name: "Legendas".to_string(),
                    kind: TrackKind::Text,
                    clips: Vec::new(),
                    text_clips: Vec::new(),
                    shape_clips: Vec::new(),
                    visible: true,
                    audio_role: AudioRole::Unspecified,
                    locked: false,
                    color_label: None,
                });
                id
            }
        };
        let Some(track) = timeline.tracks.iter_mut().find(|t| t.id == track_id) else {
            return;
        };
        for (id, seg) in (first_id..).zip(segments) {
            // seg.words' start_secs/end_secs are relative to the segment itself (same origin as
            // seg.start_secs, both ultimately anchored to source audio time zero) - TextClip::
            // words wants them relative to the *clip's* start_secs, so subtract seg.start_secs
            // to rebase.
            let words = seg
                .words
                .into_iter()
                .map(|w| avcore::timeline::WordTiming {
                    text: w.text,
                    start_secs: w.start_secs - seg.start_secs,
                    end_secs: w.end_secs - seg.start_secs,
                })
                .collect::<Vec<_>>();
            let highlight_enabled = !words.is_empty();
            track.text_clips.push(avcore::TextClip {
                id,
                start_secs: playhead_secs + seg.start_secs,
                duration_secs: (seg.end_secs - seg.start_secs).max(0.3),
                text: seg.text,
                font_size: 48.0,
                font_family: Default::default(),
                font_style: Default::default(),
                color_rgba: [255, 255, 255, 255],
                background_rgba: [0, 0, 0, 0],
                background_padding: 8.0,
                background_corner_radius: 8.0,
                pos_x: 0.1,
                pos_y: 0.85,
                words,
                highlight_enabled,
                highlight_color_rgba: [255, 220, 0, 255],
                opacity_keyframes: vec![],
                pos_x_keyframes: vec![],
                pos_y_keyframes: vec![],
                scale_keyframes: vec![],
                rotation_keyframes: vec![],
                direction: Default::default(),
                language: None,
            });
        }
    }
}

/// Runs on [`App::spawn_transcribe`]'s background thread.
fn transcribe_one(
    source_path: &Path,
    model_path: &Path,
    asset_id: u64,
    tx: &tokio::sync::mpsc::UnboundedSender<TranscribeEvent>,
) {
    let cancel = Arc::new(AtomicBool::new(false));
    match avcore::transcribe(source_path, model_path, None, &cancel, |_percent| {}) {
        // spawn_transcribe never cancels, so Cancelled is unreachable in practice here - but
        // still applies whatever partial segments came back rather than silently dropping them.
        Ok(TranscribeOutcome::Completed(segments) | TranscribeOutcome::Cancelled(segments)) => {
            let _ = tx.send(TranscribeEvent::Done { asset_id, segments });
        }
        Err(e) => {
            let _ = tx.send(TranscribeEvent::Failed {
                asset_id,
                message: e.to_string(),
            });
        }
    }
}
