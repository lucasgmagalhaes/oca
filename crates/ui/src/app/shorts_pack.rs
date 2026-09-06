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

//! D6 (`spec/architecture/differentiators.md`): one-click shorts pack. Batch-queues one vertical
//! (9:16) export per [`avcore::MarkerKind::Highlight`] marker (D2) already on the active
//! sequence. Deliberately reuses whatever captions the windowed clips already carry (transcribed
//! `TextClip`s) rather than running Whisper fresh for each short — a short whose source was
//! never transcribed exports without subtitles, a real, visible gap (a toast-free, silent
//! partial result would be worse) rather than this feature quietly forcing a Whisper run the
//! user didn't ask for.
//!
//! Framing is different: CF-04's dynamic auto-reframe (`spec/ROADMAP.md`) now runs automatically
//! for any un-reframed video clip a highlight window touches, one clip at a time, *before* the
//! actual per-short export queueing below — see [`App::spawn_next_shorts_pack_reframe`]. A clip
//! that's already carrying crop keyframes (however they got there — the static or dynamic
//! reframe button, or hand-edited) is left untouched; only a clip with none gets one. If no
//! reframe model is configured at all, this pre-pass is skipped entirely and every un-reframed
//! clip exports centered, the same fallback this feature always had.

use std::path::PathBuf;

use avcore::timeline::{MarkerKind, TrackKind};
use avcore::{ExportAspectRatio, Sequence, SequenceExportSettings, Timeline};

use crate::i18n::Text;

use super::{App, ShortsPackReframeState};

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
    /// bounds). Toasts if there are no Highlight markers yet (run "Detect Highlights" first).
    ///
    /// Before queueing anything, collects every un-reframed video clip any window touches — see
    /// this module's own doc comment — and, if a reframe model is configured and the list isn't
    /// empty, runs [`App::spawn_next_shorts_pack_reframe`]'s sequential pre-pass first. The
    /// actual window-extraction/export-queueing (this function's entire previous body) now lives
    /// in [`App::queue_shorts_pack_exports`], called either immediately (nothing to reframe) or
    /// once that pre-pass finishes.
    pub fn spawn_shorts_pack(&mut self, output_dir: PathBuf) {
        let project = self.active_project();
        let sequence = project.active_sequence();

        let highlight_positions = sorted_highlight_positions(&sequence.timeline);
        if highlight_positions.is_empty() {
            self.push_toast(Text::ShortsPackNoHighlights.tr(self.locale).to_string());
            return;
        }

        let sequence_duration = sequence.timeline.duration_secs();
        let pending_clip_ids =
            clips_needing_reframe(&sequence.timeline, &highlight_positions, sequence_duration);

        if pending_clip_ids.is_empty() || self.prefs.reframe_model_path.trim().is_empty() {
            self.queue_shorts_pack_exports(output_dir);
            return;
        }

        // One undo snapshot for the whole auto-reframe pre-pass, pushed up front rather than
        // relying on push_undo_snapshot_for_drag's pointer-release-gated coalescing (meant for
        // continuous slider drags, not a background-thread-driven batch with no pointer
        // involved at all -- reusing it here would be a fragile abuse of that mechanism).
        self.push_undo_snapshot();
        self.push_toast(
            Text::ShortsPackReframing
                .tr(self.locale)
                .replace("{n}", &pending_clip_ids.len().to_string()),
        );
        self.shorts_pack_reframe_state = Some(ShortsPackReframeState {
            pending_clip_ids,
            output_dir,
        });
        self.spawn_next_shorts_pack_reframe();
    }

    /// Whether `clip_id` is the clip [`App::shorts_pack_reframe_state`]'s pending queue is
    /// currently waiting on — `pump_dynamic_reframe` uses this to route a finished/failed
    /// dynamic-reframe run to the shorts-pack queue-advancing path instead of the ordinary
    /// selected-clip one.
    pub(super) fn is_shorts_pack_reframe_target(&self, clip_id: u64) -> bool {
        self.shorts_pack_reframe_state
            .as_ref()
            .is_some_and(|state| state.pending_clip_ids.first() == Some(&clip_id))
    }

    /// Applies a finished shorts-pack-queued dynamic-reframe run directly to `clip_id` — never
    /// gated on `selected_clip_id` the way [`App::set_selected_clip_crop_keyframes`] is, since a
    /// shorts-pack clip is essentially never the one currently selected in the properties panel.
    /// No live-preview push either (see `App::with_selected_clip_mut`'s own doc comment for what
    /// that's for): a shorts-pack clip isn't part of whatever's currently previewed, so there's
    /// nothing to push. The undo snapshot for this whole pre-pass was already taken once, up
    /// front, in [`App::spawn_shorts_pack`].
    pub(super) fn apply_shorts_pack_reframe_result(
        &mut self,
        clip_id: u64,
        crop_x_keyframes: Vec<avcore::Keyframe<f32>>,
        crop_y_keyframes: Vec<avcore::Keyframe<f32>>,
        crop_w_keyframes: Vec<avcore::Keyframe<f32>>,
        crop_h_keyframes: Vec<avcore::Keyframe<f32>>,
    ) {
        let timeline = self.active_project_mut().timeline_mut();
        if let Some(clip) = timeline.clip_mut(clip_id) {
            clip.crop_x_keyframes = crop_x_keyframes;
            clip.crop_y_keyframes = crop_y_keyframes;
            clip.crop_w_keyframes = crop_w_keyframes;
            clip.crop_h_keyframes = crop_h_keyframes;
        }
        self.advance_shorts_pack_reframe_queue();
    }

    /// Pops the just-finished (or just-failed) clip off the front of the pending queue and moves
    /// on — see [`App::spawn_next_shorts_pack_reframe`].
    pub(super) fn advance_shorts_pack_reframe_queue(&mut self) {
        if let Some(state) = &mut self.shorts_pack_reframe_state {
            if !state.pending_clip_ids.is_empty() {
                state.pending_clip_ids.remove(0);
            }
        }
        self.spawn_next_shorts_pack_reframe();
    }

    /// Spawns a dynamic-reframe run for the clip at the front of
    /// `shorts_pack_reframe_state`'s pending queue, or — once the queue is empty — clears that
    /// state and calls [`App::queue_shorts_pack_exports`] with the output directory it was
    /// carrying. If [`App::spawn_dynamic_reframe_for_clip`] can't actually spawn a run for the
    /// clip currently at the front (e.g. its asset went missing since the pre-pass first scanned
    /// the timeline), skips straight to the next one rather than getting stuck.
    pub(super) fn spawn_next_shorts_pack_reframe(&mut self) {
        let Some(state) = &self.shorts_pack_reframe_state else {
            return;
        };
        let Some(&clip_id) = state.pending_clip_ids.first() else {
            let output_dir = state.output_dir.clone();
            self.shorts_pack_reframe_state = None;
            self.queue_shorts_pack_exports(output_dir);
            return;
        };
        if !self.spawn_dynamic_reframe_for_clip(clip_id) {
            self.advance_shorts_pack_reframe_queue();
        }
    }

    /// The window-extraction/export-queueing half of what [`App::spawn_shorts_pack`] used to do
    /// in one synchronous pass — now also the continuation [`App::spawn_next_shorts_pack_reframe`]
    /// calls once every un-reframed clip a highlight window touches has either been reframed or
    /// given up on. Reports how many candidates queued vs. were skipped (e.g. a window landing
    /// entirely in a gap with no clips to resolve).
    fn queue_shorts_pack_exports(&mut self, output_dir: PathBuf) {
        let project = self.active_project();
        let sequence = project.active_sequence();

        let highlight_positions = sorted_highlight_positions(&sequence.timeline);
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
            let privacy_blur_segments = avcore::resolve_privacy_blur_segments(&windowed_sequence);
            let output_path = output_dir.join(format!("{short_name}.mp4"));

            self.queue_export(
                short_name,
                track_segments,
                audio_segments,
                text_segments,
                shape_segments,
                privacy_blur_segments,
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

fn sorted_highlight_positions(timeline: &Timeline) -> Vec<f64> {
    let mut positions: Vec<f64> = timeline
        .markers
        .iter()
        .filter(|m| m.kind == MarkerKind::Highlight)
        .map(|m| m.position_secs)
        .collect();
    positions.sort_by(f64::total_cmp);
    positions
}

/// Every distinct video clip touched by at least one `[window_start, window_end)` highlight
/// window (per `SHORTS_PACK_LEAD_IN_SECS`/`SHORTS_PACK_TRAIL_SECS`, clamped to
/// `sequence_duration`) that has no crop keyframes of its own yet
/// ([`avcore::timeline::ClipInstance::has_crop_keyframes`]) — the set [`App::spawn_shorts_pack`]
/// runs its sequential auto-reframe pre-pass over. Order matches `highlight_positions`, with
/// duplicates (the same clip spanning more than one window) collapsed to their first occurrence.
fn clips_needing_reframe(
    timeline: &Timeline,
    highlight_positions: &[f64],
    sequence_duration: f64,
) -> Vec<u64> {
    let mut ids = Vec::new();
    for position_secs in highlight_positions {
        let window_start = (position_secs - SHORTS_PACK_LEAD_IN_SECS).max(0.0);
        let window_end = (position_secs + SHORTS_PACK_TRAIL_SECS).min(sequence_duration);
        for track in &timeline.tracks {
            if track.kind != TrackKind::Video {
                continue;
            }
            for clip in &track.clips {
                let clip_end = clip.start_secs + clip.duration_secs();
                let overlaps = clip.start_secs < window_end && clip_end > window_start;
                if overlaps && !clip.has_crop_keyframes() && !ids.contains(&clip.id) {
                    ids.push(clip.id);
                }
            }
        }
    }
    ids
}

#[cfg(test)]
#[path = "shorts_pack/shorts_pack_test.rs"]
mod tests;
