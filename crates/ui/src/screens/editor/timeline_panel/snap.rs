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

use avcore::MediaAsset;

/// A clip body drag in progress: which clip, where it started from, and where the pointer
/// currently is — resolved into a same-track reposition or a cross-track move once every
/// track's row rect has been computed (see the loop in `timeline_panel`).
pub(super) struct ClipDrag {
    pub(super) clip_id: u64,
    pub(super) source_track_id: u64,
    pub(super) kind: avcore::timeline::TrackKind,
    pub(super) new_start_secs: f64,
    pub(super) pointer_y: f32,
}

/// Immutable snapping data collected once per timeline frame. Keeping this projection separate
/// from rendering prevents every drag handler from re-traversing tracks and markers.
pub(super) struct SnapTargets {
    clip_edges: Vec<(u64, f64, f64)>,
    marker_secs: Vec<f64>,
    waveform_secs: Vec<f64>,
}

impl SnapTargets {
    pub(super) fn from_project(project: &avcore::Project) -> Self {
        Self {
            clip_edges: project
                .timeline()
                .tracks
                .iter()
                .flat_map(|track| &track.clips)
                .map(|clip| {
                    (
                        clip.id,
                        clip.start_secs,
                        clip.start_secs + clip.duration_secs(),
                    )
                })
                .collect(),
            marker_secs: project
                .timeline()
                .markers
                .iter()
                .map(|marker| marker.position_secs)
                .collect(),
            waveform_secs: project
                .timeline()
                .tracks
                .iter()
                .flat_map(|track| &track.clips)
                .flat_map(|clip| {
                    project
                        .media_library
                        .iter()
                        .find(|asset| asset.id == clip.asset_id)
                        .map(|asset| waveform_snap_points_for_clip(asset, clip))
                        .unwrap_or_default()
                })
                .collect(),
        }
    }

    pub(super) fn excluding(&self, clip_id: u64) -> Vec<f64> {
        self.clip_edges
            .iter()
            .filter(|(id, _, _)| *id != clip_id)
            .flat_map(|(_, start, end)| [*start, *end])
            .chain(self.marker_secs.iter().copied())
            .collect()
    }

    pub(super) fn waveform_points(&self) -> &[f64] {
        &self.waveform_secs
    }
}

/// How close (in pixels, at the current zoom) a dragged position must land to a snap target
/// (another clip's edge, or the playhead) before it magnetically snaps to it — `ROADMAP.md` P0
/// item 2. Small enough to stay unobtrusive at high zoom, large enough to actually catch a
/// deliberate drag at low zoom (see `MIN_PX_PER_SEC`/`MAX_PX_PER_SEC` above).
const SNAP_THRESHOLD_PX: f32 = 8.0;

/// The value in `targets` nearest `candidate`, if within [`SNAP_THRESHOLD_PX`] pixels at
/// `px_per_sec` — `candidate` unchanged otherwise (including when `targets` is empty).
pub(super) fn snap_to_nearest(candidate: f64, targets: &[f64], px_per_sec: f32) -> f64 {
    let threshold_secs = (SNAP_THRESHOLD_PX / px_per_sec) as f64;
    targets
        .iter()
        .copied()
        .min_by(|a, b| (a - candidate).abs().total_cmp(&(b - candidate).abs()))
        .filter(|nearest| (nearest - candidate).abs() <= threshold_secs)
        .unwrap_or(candidate)
}

/// Snaps a clip body drag's candidate start position — tries snapping either the clip's start
/// edge or its end edge (`candidate_start + duration_secs`) to the nearest target, whichever
/// needs the smaller adjustment, so a dragged clip can magnetically dock by either edge, not
/// just its leading one. Falls back to `candidate_start` unchanged if neither edge is within
/// snapping range.
pub(super) fn snap_move_start(
    candidate_start: f64,
    duration_secs: f64,
    targets: &[f64],
    px_per_sec: f32,
) -> f64 {
    let snapped_by_start = snap_to_nearest(candidate_start, targets, px_per_sec);
    let candidate_end = candidate_start + duration_secs;
    let snapped_by_end = snap_to_nearest(candidate_end, targets, px_per_sec) - duration_secs;
    match (
        snapped_by_start != candidate_start,
        snapped_by_end != candidate_start,
    ) {
        (true, true) => {
            if (snapped_by_start - candidate_start).abs()
                <= (snapped_by_end - candidate_start).abs()
            {
                snapped_by_start
            } else {
                snapped_by_end
            }
        }
        (true, false) => snapped_by_start,
        (false, true) => snapped_by_end,
        (false, false) => candidate_start,
    }
}

/// Minimum gap `waveform_snap_points_for_clip` looks for — much shorter than D1's own
/// cuttable-gap threshold (`avcore::DEFAULT_MIN_SILENCE_SECS`, 0.5s): a brief natural pause
/// between words is exactly the kind of moment a cut should snap to, not just a length worth
/// actually cutting.
const WAVEFORM_SNAP_MIN_GAP_SECS: f64 = 0.05;

/// D5 (`spec/architecture/differentiators.md`): every low-energy-moment snap point for `clip`,
/// in timeline-relative seconds — the midpoint of each gap `avcore::clip_silence_gaps` detects
/// against `asset`'s waveform. Reuses `clip_silence_gaps` (built for D1's silence-cut detection)
/// purely as a "quiet moment finder" here — a snap target, not something to cut. Empty if
/// `asset` has no cached waveform yet.
pub(super) fn waveform_snap_points_for_clip(
    asset: &MediaAsset,
    clip: &avcore::timeline::ClipInstance,
) -> Vec<f64> {
    let Some(peaks) = &asset.waveform_peaks else {
        return Vec::new();
    };
    avcore::clip_silence_gaps(
        peaks,
        asset.duration_secs,
        clip,
        avcore::DEFAULT_SILENCE_THRESHOLD_LINEAR,
        WAVEFORM_SNAP_MIN_GAP_SECS,
    )
    .into_iter()
    .map(|gap| (gap.start_secs + gap.end_secs) / 2.0)
    .collect()
}
