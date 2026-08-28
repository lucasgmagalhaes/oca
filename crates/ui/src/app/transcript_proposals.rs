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

//! CF-01 (`spec/architecture/competitive-feature-plan.md`) slices 4-5: the review/apply half of
//! the speech-edit proposal list. [`App::begin_transcript_proposals`] runs the pure detection
//! from `avcore::detect_proposals` over the previewed clip's asset transcript (slice 1) and
//! stages the result in [`App::transcript_review`] for a modal to list per-proposal
//! accept/reject — never a silent auto-apply, the same "every automatic edit must produce a
//! reviewable proposal" rule D1's silence review and the rest of CF-01 follow. Only
//! [`App::apply_transcript_proposals`] mutates the timeline, ripple-deleting the accepted
//! proposals' source ranges (merged, so overlapping proposals can't double-cut) as **one** undo
//! step, reusing the exact same `Track::ripple_delete_range` primitive D1's own apply uses.

use avcore::{
    detect_proposals, load_transcript_document, map_source_range_to_timeline, merge_source_ranges,
    TranscriptProposal,
};

use crate::app::timeline_ops::next_clip_id;
use crate::i18n::Text;

use super::App;

/// One detected speech edit staged for review, plus whether the user has it checked for removal.
#[derive(Clone)]
pub struct TranscriptProposalEntry {
    pub proposal: TranscriptProposal,
    /// Defaults to accepted — "yes to all, uncheck the exceptions", the same batch-review
    /// convention `SilenceReviewGap` uses.
    pub accepted: bool,
}

/// The staged result of [`App::begin_transcript_proposals`], live while the review modal is
/// open. `clip_id`/`track_id` pin *which* clip the proposals were computed for — the apply step
/// re-resolves the clip by id (and falls back gracefully if it's since been deleted) to map the
/// proposals' media-relative ranges onto the timeline.
pub struct TranscriptReview {
    pub clip_id: u64,
    pub track_id: u64,
    pub proposals: Vec<TranscriptProposalEntry>,
}

impl App {
    /// Detects speech-edit proposals over the previewed clip's asset transcript and opens the
    /// review modal with them, earliest-first. A no-op (with a toast) when there's no previewed
    /// video clip or the asset has no transcript yet, and when nothing was found (no modal).
    pub fn begin_transcript_proposals(&mut self) {
        let Some(clip) = self.current_preview_video_clip() else {
            self.push_toast(
                Text::TranscriptProposalsSelectClipFirst
                    .tr(self.locale)
                    .to_string(),
            );
            return;
        };
        let dir = avcore::transcript_cache_dir_for_project(self.active_project());
        let Some(document) = load_transcript_document(&dir, clip.asset_id).ok().flatten() else {
            self.push_toast(
                Text::TranscriptProposalsNoTranscript
                    .tr(self.locale)
                    .to_string(),
            );
            return;
        };
        let proposals = detect_proposals(&document.words, document.language.as_deref());
        if proposals.is_empty() {
            self.push_toast(Text::TranscriptProposalsEmpty.tr(self.locale).to_string());
            return;
        }
        let track_id = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .find(|t| t.clips.iter().any(|c| c.id == clip.id))
            .map(|t| t.id);

        let mut entries: Vec<TranscriptProposalEntry> = proposals
            .into_iter()
            .map(|proposal| TranscriptProposalEntry {
                proposal,
                accepted: true,
            })
            .collect();
        entries.sort_by(|a, b| a.proposal.start_secs.total_cmp(&b.proposal.start_secs));

        self.transcript_review = Some(TranscriptReview {
            clip_id: clip.id,
            track_id: track_id.unwrap_or(0),
            proposals: entries,
        });
    }

    /// Flips one staged proposal's `accepted` flag — what a checkbox in the review modal does.
    pub fn toggle_transcript_proposal(&mut self, index: usize) {
        if let Some(review) = &mut self.transcript_review {
            if let Some(entry) = review.proposals.get_mut(index) {
                entry.accepted = !entry.accepted;
            }
        }
    }

    /// Closes the review modal without applying anything.
    pub fn close_transcript_review(&mut self) {
        self.transcript_review = None;
    }

    /// Ripple-deletes every accepted proposal from the previewed clip's track and closes the
    /// modal — what the review modal's "Apply" button does. All accepted proposals, however
    /// overlapping, collapse into **one undo step** (a single `push_undo_snapshot` before any
    /// mutation). Overlapping source ranges are merged first via [`avcore::merge_source_ranges`]
    /// and processed latest-start-first, matching D1's silence review. A no-op if the review
    /// isn't open or nothing is accepted.
    pub fn apply_transcript_proposals(&mut self) {
        let Some(review) = self.transcript_review.take() else {
            return;
        };
        let accepted: Vec<TranscriptProposal> = review
            .proposals
            .iter()
            .filter(|e| e.accepted)
            .map(|e| e.proposal.clone())
            .collect();
        if accepted.is_empty() {
            return;
        }

        // Resolve the clip by id before mutating anything, so we can map every accepted proposal
        // against its (possibly-trimmed-since) source range while the borrow is still immutable.
        let Some(clip) = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .find(|t| t.id == review.track_id)
            .and_then(|t| t.clips.iter().find(|c| c.id == review.clip_id))
        else {
            // The clip the review was built for no longer exists — nothing meaningful to delete.
            return;
        };
        let clip = clip.clone();

        let mut timeline_ranges: Vec<(f64, f64)> = merge_source_ranges(&accepted)
            .into_iter()
            .filter_map(|range| {
                map_source_range_to_timeline(&clip, range.start_secs, range.end_secs)
            })
            .collect();
        if timeline_ranges.is_empty() {
            return;
        }
        // Rightmost-first so an earlier removal's leftward ripple never invalidates a
        // not-yet-applied range's coordinates (same reasoning as `apply_silence_review`).
        timeline_ranges.sort_by(|a, b| b.0.total_cmp(&a.0));

        self.push_undo_snapshot();

        let timeline = self.active_project_mut().timeline_mut();
        let mut next_id = next_clip_id(timeline);
        let Some(track) = timeline.tracks.iter_mut().find(|t| t.id == review.track_id) else {
            return;
        };
        for (start, end) in timeline_ranges {
            track.ripple_delete_range(start, end, &mut next_id);
        }

        self.invalidate_preview_rendering();
    }
}
