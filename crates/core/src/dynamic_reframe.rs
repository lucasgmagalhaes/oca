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

//! CF-04 (`spec/architecture/competitive-feature-plan.md`): dynamic auto-reframe — instead of
//! [`crate::auto_reframe`]'s single centered-on-one-detected-frame static crop, sample the
//! clip's subject position at several points across its own duration and turn the resulting
//! trajectory into sparse [`crate::timeline::ClipInstance::crop_x_keyframes`]/`_y`/`_w`/`_h`, so
//! a subject that moves across the frame stays framed throughout instead of only at the single
//! moment the static version happened to sample.
//!
//! This module is deliberately just the *trajectory* half — turning a series of per-sample
//! subject centers into keyframes. Detection itself is unchanged, reusing
//! [`crate::auto_reframe::detect_faces`]/[`crate::auto_reframe::main_subject_center`]/
//! [`crate::auto_reframe::compute_reframe_crop`] exactly as the static version already does, one
//! call per sample — no second detection mechanism, no new ONNX surface.
//!
//! **Scope cuts from the full CF-04 spec, deliberate and documented, not silent gaps:**
//! - "Bounded adaptive cadence" is bounded (fixed max sample count) but not content-adaptive —
//!   cadence never adjusts based on scene motion/cuts. Sampling itself reuses
//!   [`crate::frame_sampler::FrameSampler::even_sample_times`] (same primitive
//!   [`crate::motion_tracking`]'s tracker already uses), called by the `ui`-side orchestration
//!   that drives this module, not duplicated here.
//! - "Select the primary subject using continuity, size, confidence" — each sample
//!   independently reuses [`crate::auto_reframe::main_subject_center`]'s existing
//!   highest-confidence-wins rule; there's no cross-sample identity tracking (e.g. two people
//!   trading which one has the highest per-frame confidence would visibly re-target between
//!   samples, smoothed but not corrected). A real, separate future improvement, not attempted
//!   here — the doc's own "optional user seed" is also not implemented.
//! - "Show the proposed path and allow correction before applying" — no new review UI. The
//!   emitted crop keyframes land directly in the properties panel's existing Crop X/Y/W/H
//!   Keyframes sections (already editable there), the same "existing UI is the review step"
//!   precedent D4's chapter-marker detection already established instead of a bespoke
//!   accept/reject modal.

use crate::auto_reframe::CropRect;
use crate::keyframe::Keyframe;

/// How many *consecutive* samples with no subject detected still hold the last-known center
/// before falling back to a centered crop. Matches CF-04's own acceptance criteria: a run this
/// long or shorter is a "short detection gap" (held, no visible center jump); a longer run is
/// "long subject loss" (falls back to centered framing).
pub const MAX_HOLD_GAP_SAMPLES: usize = 3;

/// One sample's detection result — paired with its own `time_fraction` so the caller doesn't
/// need to keep a separate parallel array in sync.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReframeSample {
    pub time_fraction: f32,
    /// `None` when no subject was detected at this sample (e.g. gameplay footage, or the
    /// subject briefly turned away/left frame).
    pub subject_center: Option<(f32, f32)>,
}

/// Fills gaps in a per-sample subject-center series: a run of up to [`MAX_HOLD_GAP_SAMPLES`]
/// consecutive missing detections holds the last known center (no visible jump); a longer run —
/// or missing detections at the very start, before any subject has ever been seen at all — falls
/// back to `None` (centered framing, via [`crate::auto_reframe::compute_reframe_crop`]'s own
/// `None`-subject handling) for every sample in that run.
pub fn fill_reframe_gaps(samples: &[ReframeSample]) -> Vec<Option<(f32, f32)>> {
    let mut out = vec![None; samples.len()];
    let mut i = 0;
    while i < samples.len() {
        if let Some(center) = samples[i].subject_center {
            out[i] = Some(center);
            i += 1;
            continue;
        }
        let gap_start = i;
        while i < samples.len() && samples[i].subject_center.is_none() {
            i += 1;
        }
        let gap_len = i - gap_start;
        // Nothing to hold if the gap starts at sample 0 -- there's no prior center yet.
        let hold_value = if gap_start == 0 {
            None
        } else {
            out[gap_start - 1]
        };
        if gap_len <= MAX_HOLD_GAP_SAMPLES {
            for slot in &mut out[gap_start..i] {
                *slot = hold_value;
            }
        }
        // A longer gap is left as None for its whole span -- compute_reframe_crop's own
        // None-subject fallback (centered) applies to each of those samples independently.
    }
    out
}

/// Simple centered moving-average smoothing over `window` samples — damps per-sample detection
/// jitter (a face box's reported center wobbling by a few pixels frame to frame) without a full
/// motion model. `None` entries pass through unchanged: once framing has already fallen back to
/// centered there's no position left to smooth toward, and averaging a `None` run's neighbors
/// into it would silently resurrect a "detected" center that was never actually seen at that
/// sample.
pub fn smooth_subject_centers(
    centers: &[Option<(f32, f32)>],
    window: usize,
) -> Vec<Option<(f32, f32)>> {
    let half = window / 2;
    centers
        .iter()
        .enumerate()
        .map(|(i, center)| {
            if center.is_none() {
                return None;
            }
            let start = i.saturating_sub(half);
            let end = (i + half + 1).min(centers.len());
            let mut sum_x = 0.0f32;
            let mut sum_y = 0.0f32;
            let mut count = 0.0f32;
            for c in &centers[start..end] {
                if let Some((x, y)) = c {
                    sum_x += x;
                    sum_y += y;
                    count += 1.0;
                }
            }
            if count == 0.0 {
                *center
            } else {
                Some((sum_x / count, sum_y / count))
            }
        })
        .collect()
}

/// Sparsifies one axis of a per-sample crop-rect trajectory into a `Keyframe<f32>` list, only
/// emitting a point when it differs from the last *emitted* value by more than `epsilon` — a
/// near-static subject shouldn't produce one keyframe per sample. Always emits the first and
/// last sample so the keyframe range covers the whole clip even when nothing ever moves enough
/// to cross the threshold in between. Returns an empty list for an empty `samples` input, same
/// "nothing to animate" meaning every other `*_keyframes` field's empty-list default already
/// has.
fn sparse_axis_keyframes(
    samples: &[(f32, CropRect)],
    epsilon: f32,
    axis: impl Fn(CropRect) -> f32,
) -> Vec<Keyframe<f32>> {
    if samples.is_empty() {
        return Vec::new();
    }
    let mut out = vec![Keyframe {
        time_fraction: samples[0].0,
        value: axis(samples[0].1),
    }];
    if samples.len() == 1 {
        return out;
    }
    for &(time_fraction, crop) in &samples[1..samples.len() - 1] {
        let value = axis(crop);
        if (value - out.last().unwrap().value).abs() > epsilon {
            out.push(Keyframe {
                time_fraction,
                value,
            });
        }
    }
    let (last_time, last_crop) = samples[samples.len() - 1];
    let last_value = axis(last_crop);
    if (last_value - out.last().unwrap().value).abs() > epsilon || out.len() == 1 {
        out.push(Keyframe {
            time_fraction: last_time,
            value: last_value,
        });
    }
    out
}

/// Default sparsification threshold — crop coordinates are canvas fractions (`0.0..=1.0`), so
/// `0.01` is roughly a 1%-of-frame change, small enough to track real movement, large enough to
/// absorb the residual jitter [`smooth_subject_centers`] doesn't fully remove.
pub const DEFAULT_SPARSIFY_EPSILON: f32 = 0.01;

/// Converts a per-sample crop-rect trajectory into the four sparse keyframe lists
/// [`crate::timeline::ClipInstance::crop_x_keyframes`]/`_y`/`_w`/`_h` expect, each axis
/// sparsified independently via [`sparse_axis_keyframes`] (a sample where only `x` moved
/// shouldn't force a redundant keyframe onto `y`/`w`/`h` too).
pub fn sparse_crop_keyframes(
    samples: &[(f32, CropRect)],
    epsilon: f32,
) -> (
    Vec<Keyframe<f32>>,
    Vec<Keyframe<f32>>,
    Vec<Keyframe<f32>>,
    Vec<Keyframe<f32>>,
) {
    (
        sparse_axis_keyframes(samples, epsilon, |c| c.x),
        sparse_axis_keyframes(samples, epsilon, |c| c.y),
        sparse_axis_keyframes(samples, epsilon, |c| c.w),
        sparse_axis_keyframes(samples, epsilon, |c| c.h),
    )
}

#[cfg(test)]
#[path = "dynamic_reframe/dynamic_reframe_test.rs"]
mod tests;
