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

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{App, PrivacyBlurGenerationEvent};

/// Frames sampled per second of the clip's own trimmed source duration — motion-tracking's own
/// cadence (block-matching is cheap per frame, unlike background-removal's fresh-ONNX-session-
/// per-sample cost, which uses a coarser 2.0), reused here since [`avcore::mask_propagation`]
/// tracks the seed rectangle with the same block-matching primitive.
const SAMPLES_PER_SEC: f64 = 4.0;
/// Upper bound on sampled frames, regardless of `SAMPLES_PER_SEC * duration` — same "bounds
/// worst-case memory/time for a very long clip" reasoning `background_removal.rs`'s own
/// `MAX_SAMPLES` gives.
const MAX_SAMPLES: usize = 400;
/// How far the tracked seed rectangle is allowed to move between consecutive sampled frames, as
/// a fraction of the frame's shorter dimension — `avcore::mask_propagation::
/// propagate_mask_by_translation`'s own `search_radius_frac`. Not yet exposed as its own slider
/// (a real, separate follow-up) — `MotionTrackRegionState::motion_track_search_radius`'s own
/// default is the same value.
const SEARCH_RADIUS_FRAC: f32 = 0.08;

impl App {
    /// Runs CF-09 privacy-blur mask propagation against `selected_clip_id` on a background
    /// thread — what the properties panel's "Aplicar blur" button does. Samples frames across
    /// the clip's own trimmed source duration, seeds a rectangle from
    /// [`App::privacy_blur_region`]'s current center/width/height, propagates it via
    /// `avcore::mask_propagation::propagate_mask_by_translation`, rasterizes the result
    /// (`avcore::mask_propagation::rasterize_to_matte_frames`), and encodes it into a matte
    /// video (`avcore::encode_matte_video`) cached next to the project, same directory
    /// `App::spawn_generate_matte_for_selected_clip` already uses for its own (background-
    /// removal) matte. A no-op if nothing is selected or a run is already in flight.
    pub fn spawn_apply_privacy_blur_for_selected_clip(&mut self) {
        if self
            .privacy_blur_generation_state
            .privacy_blur_generating_clip_id
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
        let mask_dir =
            avcore::background_removal::mask_cache_dir_for_project(self.active_project());
        let mask_path = mask_dir.join(format!("clip_{clip_id}_privacy_blur_matte.mp4"));
        let center_x = self.privacy_blur_region.privacy_blur_center_x;
        let center_y = self.privacy_blur_region.privacy_blur_center_y;
        let width_frac = self.privacy_blur_region.privacy_blur_width;
        let height_frac = self.privacy_blur_region.privacy_blur_height;

        self.privacy_blur_generation_state
            .privacy_blur_generating_clip_id = Some(clip_id);
        let tx = self
            .privacy_blur_generation_state
            .privacy_blur_generation_tx
            .clone();
        std::thread::spawn(move || {
            let event = match apply_privacy_blur_one(
                &source_path,
                source_in_secs,
                source_out_secs,
                center_x,
                center_y,
                width_frac,
                height_frac,
                &mask_dir,
                &mask_path,
            ) {
                Ok((mask_path, seed_vertices)) => PrivacyBlurGenerationEvent::Done {
                    clip_id,
                    mask_path,
                    seed_vertices,
                },
                Err(message) => PrivacyBlurGenerationEvent::Failed { message },
            };
            let _ = tx.send(event);
        });
    }

    /// Applies a finished privacy-blur matte-generation run to the timeline. Called once per
    /// frame from [`eframe::App::ui`], same as [`App::pump_matte_generation`].
    pub(super) fn pump_privacy_blur_generation(&mut self) {
        while let Ok(event) = self
            .privacy_blur_generation_state
            .privacy_blur_generation_rx
            .try_recv()
        {
            match event {
                PrivacyBlurGenerationEvent::Done {
                    clip_id,
                    mask_path,
                    seed_vertices,
                } => {
                    self.privacy_blur_generation_state
                        .privacy_blur_generating_clip_id = None;
                    if self.selected_clip_id == Some(clip_id) {
                        self.set_selected_clip_privacy_blur_mask(
                            mask_path.display().to_string(),
                            seed_vertices,
                        );
                    }
                }
                PrivacyBlurGenerationEvent::Failed { message } => {
                    self.privacy_blur_generation_state
                        .privacy_blur_generating_clip_id = None;
                    tracing::error!(error = %message, "privacy blur matte generation failed");
                    self.push_toast(format!("Privacy blur generation failed: {message}"));
                }
            }
        }
    }
}

/// Runs on [`App::spawn_apply_privacy_blur_for_selected_clip`]'s background thread — decodes
/// frames sampled across `[source_in_secs, source_out_secs)`, seeds a rectangle centered at
/// `(center_x, center_y)` (each a fraction of the *first sampled frame's* own pixel dimensions),
/// propagates it via `avcore::mask_propagation::propagate_mask_by_translation`, rasterizes and
/// encodes the result into a matte video at `mask_path` (creating `mask_dir` first if needed).
/// Returns `(mask_path, seed_vertices)` on success — `seed_vertices` is the exact rectangle this
/// run seeded from (computed here since only this function decodes a real frame to know its
/// actual pixel dimensions), which the caller persists onto the clip alongside the mask path.
#[allow(clippy::too_many_arguments)]
fn apply_privacy_blur_one(
    source_path: &Path,
    source_in_secs: f64,
    source_out_secs: f64,
    center_x: f32,
    center_y: f32,
    width_frac: f32,
    height_frac: f32,
    mask_dir: &Path,
    mask_path: &Path,
) -> Result<(PathBuf, Vec<(f32, f32)>), String> {
    let duration = (source_out_secs - source_in_secs).max(0.0);
    if duration <= 0.0 {
        return Err("clip has zero duration".to_string());
    }
    let sample_times = avcore::FrameSampler::even_sample_times(
        source_in_secs,
        source_out_secs,
        SAMPLES_PER_SEC,
        2,
        MAX_SAMPLES,
    );

    let sampler = avcore::FrameSampler::open(source_path, Duration::from_millis(20))
        .map_err(|e| format!("failed to open source for decoding: {e}"))?;

    let mut gray_frames: Vec<avcore::GrayFrame> = Vec::with_capacity(sample_times.len());
    let mut frame_w = 0u32;
    let mut frame_h = 0u32;
    for &t in &sample_times {
        let Some(frame) = sampler.sample(t, Duration::from_millis(1500)) else {
            continue;
        };
        if frame_w == 0 {
            frame_w = frame.width;
            frame_h = frame.height;
        } else if frame.width != frame_w || frame.height != frame_h {
            continue;
        }
        gray_frames.push(avcore::rgba_to_gray(&frame.rgba, frame.width, frame.height));
    }

    if gray_frames.len() < 2 {
        return Err("couldn't decode enough frames to propagate a mask".to_string());
    }

    // The seed rectangle's own corners, in full-frame-fraction coordinates — width_frac/
    // height_frac are each a fraction of the shorter dimension (matching avcore::track_region's
    // own convention), so they're scaled by the shorter side in pixels, then converted back to a
    // fraction of the *own* axis (width divided by frame_w, height by frame_h) since a polygon
    // vertex here is always in that convention (crate::timeline::ShapeKind::Polygon, avcore::
    // mask_propagation::PropagatedMask::vertices).
    let short_side = frame_w.min(frame_h) as f32;
    let half_w_frac = (width_frac * short_side) / 2.0 / frame_w as f32;
    let half_h_frac = (height_frac * short_side) / 2.0 / frame_h as f32;
    let seed_vertices = vec![
        (center_x - half_w_frac, center_y - half_h_frac),
        (center_x + half_w_frac, center_y - half_h_frac),
        (center_x + half_w_frac, center_y + half_h_frac),
        (center_x - half_w_frac, center_y + half_h_frac),
    ];

    let propagated = avcore::mask_propagation::propagate_mask_by_translation(
        &seed_vertices,
        center_x,
        center_y,
        &gray_frames,
        width_frac,
        height_frac,
        SEARCH_RADIUS_FRAC,
        avcore::mask_propagation::DEFAULT_LOW_CONFIDENCE_THRESHOLD,
    );
    let luma_frames =
        avcore::mask_propagation::rasterize_to_matte_frames(&propagated, frame_w, frame_h);

    let effective_fps = luma_frames.len() as f64 / duration;
    let fps_num = ((effective_fps * 1000.0).round() as u32).max(1);
    let fps_den = 1000u32;

    std::fs::create_dir_all(mask_dir)
        .map_err(|e| format!("failed to create matte cache dir: {e}"))?;
    avcore::encode_matte_video(&luma_frames, frame_w, frame_h, fps_num, fps_den, mask_path)
        .map_err(|e| format!("failed to encode matte video: {e}"))?;

    Ok((mask_path.to_path_buf(), seed_vertices))
}
