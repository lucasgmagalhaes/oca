// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! D6 (`spec/architecture/differentiators.md`): one-click shorts pack. Batch-queues one vertical
//! (9:16) export per [`avcore::MarkerKind::Highlight`] marker (D2) already on the active
//! sequence. Deliberately reuses whatever framing/captions the windowed clips already carry
//! (auto-reframe's `position_keyframes`, transcribed `TextClip`s) rather than running either
//! pipeline fresh for each short — a short whose source was never auto-reframed exports
//! centered, and one never transcribed exports without subtitles; both are real, visible gaps
//! (a toast-free, silent partial result would be worse) rather than this feature quietly forcing
//! a face-detection or Whisper run the user didn't ask for.

use std::path::PathBuf;

use avcore::timeline::MarkerKind;
use avcore::{ExportAspectRatio, Sequence, SequenceExportSettings};

use crate::i18n::Text;

use super::App;

/// Seconds of lead-in before a detected highlight's marker position — D2 only stores a single
/// timestamp per highlight, not a range, so D6 needs its own fixed window-size heuristic.
const SHORTS_PACK_LEAD_IN_SECS: f64 = 5.0;
/// Seconds of "reaction" kept after the highlight's marker position.
const SHORTS_PACK_TRAIL_SECS: f64 = 10.0;

impl App {
    /// Batch-exports one vertical short per `MarkerKind::Highlight` marker on the active
    /// sequence into `output_dir` — what the Editor toolbar's "Shorts Pack" button does once its
    /// folder-picker dialog resolves. Each short is a `SHORTS_PACK_LEAD_IN_SECS` +
    /// `SHORTS_PACK_TRAIL_SECS` window around one highlight (clamped to the sequence's own
    /// bounds), extracted via [`avcore::extract_timeline_window`] and queued the same way
    /// "Adicionar exportação" queues the whole sequence. Toasts if there are no Highlight markers
    /// yet (run "Detect Highlights" first) or reports how many candidates queued vs. were
    /// skipped (e.g. a window landing entirely in a gap with no clips to resolve).
    pub fn spawn_shorts_pack(&mut self, output_dir: PathBuf) {
        let project = self.active_project();
        let sequence = project.active_sequence();

        let mut highlight_positions: Vec<f64> = sequence
            .timeline
            .markers
            .iter()
            .filter(|m| m.kind == MarkerKind::Highlight)
            .map(|m| m.position_secs)
            .collect();
        if highlight_positions.is_empty() {
            self.push_toast(Text::ShortsPackNoHighlights.tr(self.locale).to_string());
            return;
        }
        highlight_positions.sort_by(f64::total_cmp);

        let sequence_duration = sequence.timeline.duration_secs();
        let timeline = sequence.timeline.clone();
        let sequence_name = sequence.name.clone();
        let target_lufs = sequence.export_settings.target_lufs;
        let media_library = project.media_library.clone();
        let mut next_id = timeline
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .map(|c| c.id)
            .max()
            .unwrap_or(0)
            + 1;

        let mut queued = 0u32;
        let mut skipped = 0u32;
        for (i, position_secs) in highlight_positions.iter().enumerate() {
            let window_start = (position_secs - SHORTS_PACK_LEAD_IN_SECS).max(0.0);
            let window_end = (position_secs + SHORTS_PACK_TRAIL_SECS).min(sequence_duration);

            let windowed_timeline =
                avcore::extract_timeline_window(&timeline, window_start, window_end, &mut next_id);
            let short_name = format!("{sequence_name}_short_{}", i + 1);
            let windowed_sequence = Sequence {
                id: i as u64 + 1,
                name: short_name.clone(),
                timeline: windowed_timeline,
                export_settings: SequenceExportSettings {
                    aspect_ratio: ExportAspectRatio::Portrait,
                    target_lufs,
                },
            };

            let Ok((track_segments, canvas)) =
                avcore::resolve_timeline_segments_multi(&windowed_sequence, &media_library)
            else {
                skipped += 1;
                continue;
            };
            let Ok(audio_segments) =
                avcore::resolve_audio_segments(&windowed_sequence, &media_library)
            else {
                skipped += 1;
                continue;
            };
            let canvas = avcore::apply_export_aspect_ratio(canvas, ExportAspectRatio::Portrait);
            let text_segments =
                avcore::resolve_text_segments(&windowed_sequence, canvas.width, canvas.height);
            let shape_segments =
                avcore::resolve_shape_segments(&windowed_sequence, canvas.width, canvas.height);
            let output_path = output_dir.join(format!("{short_name}.mp4"));

            self.queue_export(
                short_name,
                track_segments,
                audio_segments,
                text_segments,
                shape_segments,
                canvas,
                target_lufs,
                output_path.display().to_string(),
            );
            queued += 1;
        }

        self.push_toast(
            Text::ShortsPackQueued
                .tr(self.locale)
                .replace("{queued}", &queued.to_string())
                .replace("{skipped}", &skipped.to_string()),
        );
    }
}
