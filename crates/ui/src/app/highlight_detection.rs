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

//! D2 (`spec/architecture/differentiators.md`): highlight-candidate detection over the active
//! sequence's whole timeline. Runs synchronously on the main thread, unlike D4's scene-cut
//! detection — this only needs `MediaAsset::waveform_peaks`, already decoded and cached in
//! memory (same reasoning D1's `begin_silence_review` already relies on), not a fresh video
//! decode via `FrameSampler`.

use avcore::timeline::{AudioRole, MarkerKind};

use crate::i18n::Text;

use super::App;

impl App {
    /// Scans every clip on a [`AudioRole::GameAudio`]-tagged track against every clip on a
    /// [`AudioRole::Mic`]-tagged track (see the timeline track header's role picker) for
    /// simultaneous amplitude spikes, and adds a numbered [`MarkerKind::Highlight`] marker at
    /// each detected candidate's start — what the Editor toolbar's "Detect Highlights" button
    /// does. Toasts instead if no track is tagged with both roles yet, or if scanning finds
    /// nothing. Non-destructive, same as D4's Chapter markers — the existing Timeline Index
    /// panel's rename/delete is the review step, not a separate accept/reject modal.
    pub fn detect_highlights(&mut self) {
        let (game_audio_samples, mic_samples) = {
            let project = self.active_project();
            let mut game_audio_samples = Vec::new();
            let mut mic_samples = Vec::new();
            for track in &project.timeline().tracks {
                let bucket = match track.audio_role {
                    AudioRole::GameAudio => &mut game_audio_samples,
                    AudioRole::Mic => &mut mic_samples,
                    AudioRole::Unspecified | AudioRole::Music => continue,
                };
                for clip in &track.clips {
                    let Some(asset) = project.media_library.iter().find(|a| a.id == clip.asset_id)
                    else {
                        continue;
                    };
                    let Some(peaks) = &asset.waveform_peaks else {
                        continue;
                    };
                    bucket.extend(avcore::clip_amplitude_samples(
                        peaks,
                        asset.duration_secs,
                        clip,
                    ));
                }
            }
            (game_audio_samples, mic_samples)
        };

        if game_audio_samples.is_empty() || mic_samples.is_empty() {
            self.push_toast(
                Text::HighlightDetectionNeedsBothRoles
                    .tr(self.locale)
                    .to_string(),
            );
            return;
        }

        let candidates = avcore::detect_highlight_candidates(
            &game_audio_samples,
            &mic_samples,
            avcore::DEFAULT_HIGHLIGHT_THRESHOLD_LINEAR,
            avcore::DEFAULT_HIGHLIGHT_GRID_SECS,
            avcore::DEFAULT_HIGHLIGHT_MIN_DURATION_SECS,
        );

        if candidates.is_empty() {
            self.push_toast(Text::HighlightDetectionNone.tr(self.locale).to_string());
            return;
        }

        self.push_undo_snapshot();
        let locale = self.locale;
        let existing_highlights = self
            .active_project()
            .timeline()
            .markers
            .iter()
            .filter(|m| m.kind == MarkerKind::Highlight)
            .count();
        let timeline = self.active_project_mut().timeline_mut();
        for (i, candidate) in candidates.iter().enumerate() {
            let marker_id = timeline.add_marker(candidate.start_secs, MarkerKind::Highlight);
            if let Some(marker) = timeline.marker_mut(marker_id) {
                marker.label = Text::HighlightDefaultLabel
                    .tr(locale)
                    .replace("{n}", &(existing_highlights + i + 1).to_string());
            }
        }
    }
}
