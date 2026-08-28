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

//! D4 (`spec/architecture/differentiators.md`): background-thread scene-cut detection for the
//! selected clip, following the same split `motion_tracking.rs` already established — `core`
//! (`avcore::scene_detection`) owns the pure frame-difference scoring, this module owns sampling
//! frames against the clip's asset via [`avcore::FrameSampler`] (same primitive, same reasoning:
//! `spec/architecture/performance-and-caching.md` §5) and turning detected cuts into
//! [`avcore::MarkerKind::Chapter`] markers on the timeline. Markers, not a modal review list —
//! unlike D1's ripple-delete, adding a marker is non-destructive, and the Timeline Index panel
//! (P2 item 9) already gives per-marker rename/delete, which *is* the review step here.

use std::path::{Path, PathBuf};
use std::time::Duration;

use avcore::timeline::MarkerKind;
use avcore::{FrameSampler, SceneCut};

use crate::i18n::Text;

use super::{App, SceneCutEvent};

/// Frames sampled per second of the clip's own trimmed source duration — mirrors
/// `motion_tracking`'s own rate. Scene cuts are abrupt, so this doesn't need to be denser than
/// motion tracking's cadence to catch them; it only needs to land within roughly a frame-times-
/// this-interval of the true cut, which a Timeline Index rename/nudge can correct regardless.
const SAMPLES_PER_SEC: f64 = 4.0;
/// Upper bound on sampled frames, regardless of `SAMPLES_PER_SEC * duration` — bounds worst-case
/// decode time for a long VOD, same reasoning as `motion_tracking::MAX_SAMPLES`.
const MAX_SAMPLES: usize = 600;

impl App {
    /// Runs scene-cut detection against `selected_clip_id` on a background thread — what the
    /// Editor toolbar's "🎬 Detect Chapters" button does. Samples across the clip's own trimmed
    /// source range, and on completion adds a [`MarkerKind::Chapter`] marker (labeled "Chapter
    /// N") at each detected cut's timeline position. A no-op if nothing is selected or a run is
    /// already in flight.
    pub fn spawn_detect_scene_cuts_for_selected_clip(&mut self) {
        if self
            .scene_cut_detection_state
            .scene_cut_detection_clip_id
            .is_some()
        {
            return;
        }
        let Some(clip) = self.selected_clip() else {
            return;
        };
        let clip_id = clip.id;
        let asset_id = clip.asset_id;
        let source_in_secs = clip.source_in_secs;
        let source_out_secs = clip.source_out_secs;
        let Some(asset) = self
            .active_project()
            .media_library
            .iter()
            .find(|a| a.id == asset_id)
        else {
            return;
        };
        let source_path = asset.source_path.clone();

        self.scene_cut_detection_state.scene_cut_detection_clip_id = Some(clip_id);
        let tx = self
            .scene_cut_detection_state
            .scene_cut_detection_tx
            .clone();
        std::thread::spawn(move || {
            let cuts = detect_scene_cuts_one(&source_path, source_in_secs, source_out_secs);
            let _ = tx.send(SceneCutEvent::Done { clip_id, cuts });
        });
    }

    /// Applies a finished scene-cut-detection run to the timeline. Called once per frame from
    /// [`eframe::App::ui`], same as [`App::pump_motion_tracking`].
    pub(super) fn pump_scene_cut_detection(&mut self) {
        while let Ok(event) = self
            .scene_cut_detection_state
            .scene_cut_detection_rx
            .try_recv()
        {
            match event {
                SceneCutEvent::Done { clip_id, cuts } => {
                    self.scene_cut_detection_state.scene_cut_detection_clip_id = None;
                    if cuts.is_empty() {
                        self.push_toast(
                            crate::i18n::Text::SceneCutDetectionNone
                                .tr(self.locale)
                                .to_string(),
                        );
                        continue;
                    }
                    self.apply_detected_scene_cuts(clip_id, cuts);
                }
            }
        }
    }

    /// Maps each detected cut's source-relative time onto the timeline (same
    /// `start_secs + (source_secs - source_in_secs) / speed_factor` formula
    /// [`avcore::clip_silence_gaps`] uses) and adds a numbered Chapter marker at each — a no-op
    /// for a cut whose clip no longer exists (e.g. deleted while detection was running).
    pub(super) fn apply_detected_scene_cuts(&mut self, clip_id: u64, cuts: Vec<SceneCut>) {
        let Some(clip) = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .find(|c| c.id == clip_id)
            .cloned()
        else {
            return;
        };
        if clip.speed_factor <= 0.0 {
            return;
        }

        self.push_undo_snapshot();
        let existing_chapters = self
            .active_project()
            .timeline()
            .markers
            .iter()
            .filter(|m| m.kind == MarkerKind::Chapter)
            .count();
        let locale = self.locale;
        let timeline = self.active_project_mut().timeline_mut();
        for (i, cut) in cuts.iter().enumerate() {
            let timeline_secs =
                clip.start_secs + (cut.at_secs - clip.source_in_secs) / clip.speed_factor as f64;
            let marker_id = timeline.add_marker(timeline_secs, MarkerKind::Chapter);
            if let Some(marker) = timeline.marker_mut(marker_id) {
                marker.label = Text::ChapterDefaultLabel
                    .tr(locale)
                    .replace("{n}", &(existing_chapters + i + 1).to_string());
            }
        }
    }

    /// Writes every [`MarkerKind::Chapter`] marker on the active sequence's timeline, sorted by
    /// position, as a plain-text `H:MM:SS Label` list — YouTube's own chapter-timestamp format —
    /// to `output_path`. Toasts instead of writing an empty file if there are no chapter markers
    /// yet. What the "Export chapters (.txt)" button does once its save-file dialog picks a
    /// destination.
    pub fn export_chapters_txt(&mut self, output_path: PathBuf) {
        let mut chapters: Vec<&avcore::timeline::Marker> = self
            .active_project()
            .timeline()
            .markers
            .iter()
            .filter(|m| m.kind == MarkerKind::Chapter)
            .collect();
        if chapters.is_empty() {
            self.push_toast(Text::ExportChaptersNone.tr(self.locale).to_string());
            return;
        }
        chapters.sort_by(|a, b| a.position_secs.total_cmp(&b.position_secs));

        let mut contents = String::new();
        for chapter in chapters {
            contents.push_str(&avcore::media::format_timecode(chapter.position_secs));
            contents.push(' ');
            contents.push_str(&chapter.label);
            contents.push('\n');
        }

        match std::fs::write(&output_path, contents) {
            Ok(()) => self.push_toast(Text::ExportChaptersDone.tr(self.locale).to_string()),
            Err(e) => self.push_toast(format!("Failed to export chapters: {e}")),
        }
    }
}

/// Runs on [`App::spawn_detect_scene_cuts_for_selected_clip`]'s background thread — decodes
/// frames sampled across `[source_in_secs, source_out_secs)`, converts each to grayscale, and
/// scores consecutive pairs via [`avcore::detect_scene_cuts`]. Returns an empty `Vec` if fewer
/// than 2 frames could be decoded (nothing to compare) or the source can't be opened.
fn detect_scene_cuts_one(
    source_path: &Path,
    source_in_secs: f64,
    source_out_secs: f64,
) -> Vec<SceneCut> {
    let duration = (source_out_secs - source_in_secs).max(0.0);
    if duration <= 0.0 {
        return Vec::new();
    }
    let sample_times = FrameSampler::even_sample_times(
        source_in_secs,
        source_out_secs,
        SAMPLES_PER_SEC,
        2,
        MAX_SAMPLES,
    );

    let Ok(sampler) = FrameSampler::open(source_path, Duration::from_millis(10)) else {
        return Vec::new();
    };

    let samples: Vec<(f64, avcore::GrayFrame)> = sample_times
        .iter()
        .filter_map(|&t| {
            let frame = sampler.sample(t, Duration::from_millis(500))?;
            Some((
                t,
                avcore::rgba_to_gray(&frame.rgba, frame.width, frame.height),
            ))
        })
        .collect();

    if samples.len() < 2 {
        return Vec::new();
    }

    avcore::detect_scene_cuts(&samples, avcore::DEFAULT_SCENE_CUT_THRESHOLD)
}
