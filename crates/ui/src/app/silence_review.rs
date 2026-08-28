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

//! D1 (`spec/architecture/differentiators.md`): the review/apply half of automatic silence
//! detection. [`App::begin_silence_review`] scans one track and stages the result in
//! [`App::silence_review`] for a modal (`ui`'s `modals.rs`) to list per-gap accept/reject —
//! never a silent auto-apply, per the differentiators doc's explicit requirement. Only
//! [`App::apply_silence_review`] actually mutates the timeline.

use avcore::{
    clip_silence_gaps, SilenceGap, DEFAULT_MIN_SILENCE_SECS, DEFAULT_SILENCE_THRESHOLD_LINEAR,
};

use crate::i18n::Text;

use super::App;

/// One detected gap staged for review, plus whether the user has it checked for removal.
/// Defaults to accepted — matches this app's other batch-review UIs (e.g. the export queue's
/// conflict list), where the common case is "yes to all, uncheck the exceptions."
pub struct SilenceReviewGap {
    pub clip_id: u64,
    pub gap: SilenceGap,
    pub accepted: bool,
}

/// The staged result of [`App::begin_silence_review`], live while the review modal is open.
pub struct SilenceReview {
    pub track_id: u64,
    pub gaps: Vec<SilenceReviewGap>,
}

impl App {
    /// Scans every clip on the track holding `selected_clip_id` for silence gaps (via
    /// [`avcore::clip_silence_gaps`] against each clip's asset waveform) and opens the review
    /// modal with the result, sorted earliest-first. A clip whose asset has no cached waveform
    /// yet (`MediaAsset::waveform_peaks` is `None` — background enrichment hasn't reached it, or
    /// it's video-only/no-audio) contributes no gaps rather than erroring. Does nothing but show
    /// a toast if no clip is selected — this reuses the selection to pick *which* track to scan,
    /// same as most other track-scoped actions in this app.
    pub fn begin_silence_review(&mut self) {
        let Some(selected_id) = self.selected_clip_id else {
            self.push_toast(
                Text::SilenceReviewSelectClipFirst
                    .tr(self.locale)
                    .to_string(),
            );
            return;
        };
        let project = self.active_project();
        let Some(track) = project
            .timeline()
            .tracks
            .iter()
            .find(|t| t.clips.iter().any(|c| c.id == selected_id))
        else {
            return;
        };

        let mut gaps = Vec::new();
        for clip in &track.clips {
            let Some(asset) = project.media_library.iter().find(|a| a.id == clip.asset_id) else {
                continue;
            };
            let Some(peaks) = &asset.waveform_peaks else {
                continue;
            };
            for gap in clip_silence_gaps(
                peaks,
                asset.duration_secs,
                clip,
                DEFAULT_SILENCE_THRESHOLD_LINEAR,
                DEFAULT_MIN_SILENCE_SECS,
            ) {
                gaps.push(SilenceReviewGap {
                    clip_id: clip.id,
                    gap,
                    accepted: true,
                });
            }
        }
        gaps.sort_by(|a, b| a.gap.start_secs.total_cmp(&b.gap.start_secs));

        self.silence_review = Some(SilenceReview {
            track_id: track.id,
            gaps,
        });
    }

    /// Flips one staged gap's `accepted` flag — what a checkbox in the review modal does. A
    /// no-op if the review isn't open or `index` is out of range.
    pub fn toggle_silence_gap_accepted(&mut self, index: usize) {
        if let Some(review) = &mut self.silence_review {
            if let Some(entry) = review.gaps.get_mut(index) {
                entry.accepted = !entry.accepted;
            }
        }
    }

    /// Closes the review modal without applying anything.
    pub fn close_silence_review(&mut self) {
        self.silence_review = None;
    }

    /// Ripple-deletes every accepted gap from the reviewed track and closes the modal — what the
    /// review modal's "Apply" button does. Processes gaps latest-start-first so an earlier
    /// removal's leftward ripple never invalidates a not-yet-applied gap's coordinates (each
    /// accepted gap is still expressed in the track's pre-review timeline). A no-op if the
    /// review isn't open.
    pub fn apply_silence_review(&mut self) {
        let Some(review) = self.silence_review.take() else {
            return;
        };
        if !review.gaps.iter().any(|g| g.accepted) {
            return;
        }

        self.push_undo_snapshot();
        // Ids must stay unique across the whole timeline, not just this track -- same source of
        // truth `App::split_at_playhead` uses for its own `next_id` counter.
        let mut next_id = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .map(|c| c.id)
            .max()
            .unwrap_or(0)
            + 1;

        let timeline = self.active_project_mut().timeline_mut();
        let Some(track) = timeline.tracks.iter_mut().find(|t| t.id == review.track_id) else {
            return;
        };

        let mut accepted: Vec<&SilenceGap> = review
            .gaps
            .iter()
            .filter(|g| g.accepted)
            .map(|g| &g.gap)
            .collect();
        accepted.sort_by(|a, b| b.start_secs.total_cmp(&a.start_secs));

        for gap in accepted {
            track.ripple_delete_range(gap.start_secs, gap.end_secs, &mut next_id);
        }
    }
}
