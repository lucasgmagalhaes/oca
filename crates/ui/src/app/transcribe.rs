use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use avcore::timeline::{Track, TrackKind};
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
        if self.transcribing_asset_id.is_some() {
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

        self.transcribing_asset_id = Some(asset_id);
        let model_path = std::path::PathBuf::from(self.prefs.whisper_model_path.clone());
        let tx = self.transcribe_tx.clone();
        std::thread::spawn(move || {
            transcribe_one(&source_path, &model_path, asset_id, &tx);
        });
    }

    /// Applies a finished transcription to the active project's timeline. Called once per
    /// frame from [`eframe::App::ui`], same as [`App::pump_import_queue`].
    pub(super) fn pump_transcribe(&mut self) {
        while let Ok(event) = self.transcribe_rx.try_recv() {
            match event {
                TranscribeEvent::Done { asset_id, segments } => {
                    self.transcribing_asset_id = None;
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
                    self.apply_transcription(segments);
                }
                TranscribeEvent::Failed { asset_id, message } => {
                    self.transcribing_asset_id = None;
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
                    visible: true,
                });
                id
            }
        };
        let Some(track) = timeline.tracks.iter_mut().find(|t| t.id == track_id) else {
            return;
        };
        for (id, seg) in (first_id..).zip(segments) {
            track.text_clips.push(avcore::TextClip {
                id,
                start_secs: playhead_secs + seg.start_secs,
                duration_secs: (seg.end_secs - seg.start_secs).max(0.3),
                text: seg.text,
                font_size: 48.0,
                color_rgba: [255, 255, 255, 255],
                pos_x: 0.1,
                pos_y: 0.85,
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
        Ok(TranscribeOutcome::Completed(segments)) => {
            let _ = tx.send(TranscribeEvent::Done { asset_id, segments });
        }
        Ok(TranscribeOutcome::Cancelled) => {
            // spawn_transcribe never sets cancel - unreachable in practice, but handled rather
            // than silently dropped.
            let _ = tx.send(TranscribeEvent::Done {
                asset_id,
                segments: Vec::new(),
            });
        }
        Err(e) => {
            let _ = tx.send(TranscribeEvent::Failed {
                asset_id,
                message: e.to_string(),
            });
        }
    }
}
