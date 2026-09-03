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

//! CF-04 dynamic auto-reframe orchestration — the `ui`-side counterpart to
//! [`avcore::dynamic_reframe`]. Samples a clip's subject position at several points across its
//! own duration, same detector [`super::auto_reframe`]'s static version already uses
//! ([`avcore::detect_faces`]/[`avcore::main_subject_center`]), then turns the resulting
//! trajectory into sparse crop keyframes via [`avcore::fill_reframe_gaps`]/
//! [`avcore::smooth_subject_centers`]/[`avcore::sparse_crop_keyframes`].

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::auto_reframe::{downscale_frame_rgba, REFRAME_FRAME_MAX_DIM};
use super::{App, DynamicReframeEvent};

/// Sampling rate for dynamic reframe — lower than motion tracking's 4/sec
/// ([`super::motion_tracking`]) since face detection's ONNX inference is costlier per-sample
/// than block matching.
const DYNAMIC_REFRAME_SAMPLES_PER_SEC: f64 = 2.0;
/// Always sample at least this many points, even for a very short clip, so there's something to
/// interpolate between.
const DYNAMIC_REFRAME_MIN_SAMPLES: usize = 4;
/// Bounds a long clip's sample count — each sample is one seek+decode+ONNX-inference, not free.
const DYNAMIC_REFRAME_MAX_SAMPLES: usize = 30;
/// Moving-average window (in samples) passed to [`avcore::smooth_subject_centers`].
const DYNAMIC_REFRAME_SMOOTH_WINDOW: usize = 5;

impl App {
    /// Runs dynamic auto-reframe against `selected_clip_id` on a background thread — what the
    /// properties panel's "Reenquadramento dinâmico" button does. Unlike
    /// [`App::spawn_auto_reframe_selected_clip`] (one detection at the clip's midpoint, one
    /// static crop), this samples the subject's position across the whole clip and produces
    /// crop keyframes so a moving subject stays framed throughout. A no-op if nothing is
    /// selected, no model is configured, or a run (static or dynamic) is already in flight.
    pub fn spawn_dynamic_reframe_selected_clip(&mut self) {
        let Some(clip_id) = self.selected_clip_id else {
            return;
        };
        self.spawn_dynamic_reframe_for_clip(clip_id);
    }

    /// The clip-id-parametrized core [`App::spawn_dynamic_reframe_selected_clip`] delegates to —
    /// also [`App::spawn_shorts_pack`]'s own sequential pre-pass calls this directly for a clip
    /// that isn't necessarily `selected_clip_id` (batch-processing every un-reframed clip a
    /// highlight window touches, one at a time, not just whatever's selected in the properties
    /// panel). Returns whether a background run was actually spawned — `false` on any of the
    /// same no-op conditions [`App::spawn_dynamic_reframe_selected_clip`] already had (already
    /// in flight, no model configured, `clip_id` unknown, its asset missing/without a known
    /// resolution), which [`App::spawn_next_shorts_pack_reframe`] uses to skip a clip it can't
    /// actually process rather than getting stuck.
    pub(super) fn spawn_dynamic_reframe_for_clip(&mut self, clip_id: u64) -> bool {
        if self
            .dynamic_reframe_state
            .dynamic_reframing_clip_id
            .is_some()
        {
            return false;
        }
        if self.prefs.reframe_model_path.trim().is_empty() {
            self.push_toast(
                crate::i18n::Text::AutoReframeNoModelConfigured
                    .tr(self.locale)
                    .to_string(),
            );
            return false;
        }
        let Some(clip) = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .find(|c| c.id == clip_id)
        else {
            return false;
        };
        let asset_id = clip.asset_id;
        let source_in_secs = clip.source_in_secs;
        let source_out_secs = clip.source_out_secs;
        let Some(asset) = self
            .active_project()
            .media_library
            .iter()
            .find(|a| a.id == asset_id)
        else {
            return false;
        };
        let Some((source_w, source_h)) = asset.resolution else {
            return false;
        };
        let source_path = asset.source_path.clone();
        let (target_w, target_h) = self
            .active_sequence_export_settings()
            .aspect_ratio
            .dims_or((source_w, source_h));
        let model_path = PathBuf::from(self.prefs.reframe_model_path.clone());

        self.dynamic_reframe_state.dynamic_reframing_clip_id = Some(clip_id);
        let tx = self.dynamic_reframe_state.dynamic_reframe_tx.clone();
        std::thread::spawn(move || {
            dynamic_reframe_one(
                &source_path,
                source_in_secs,
                source_out_secs,
                source_w,
                source_h,
                target_w,
                target_h,
                &model_path,
                clip_id,
                &tx,
            );
        });
        true
    }

    /// Applies a finished dynamic-reframe run to the timeline. Called once per frame from
    /// [`eframe::App::ui`], same as [`App::pump_auto_reframe`]. A run [`App::spawn_shorts_pack`]'s
    /// own sequential pre-pass kicked off (`clip_id` is the head of `shorts_pack_reframe_state`'s
    /// pending queue) is applied directly to that clip — regardless of the current
    /// `selected_clip_id` — and advances the queue instead of going through the ordinary
    /// selected-clip path; every other run still only applies when its clip is still selected,
    /// same as before this queue existed.
    pub(super) fn pump_dynamic_reframe(&mut self) {
        while let Ok(event) = self.dynamic_reframe_state.dynamic_reframe_rx.try_recv() {
            match event {
                DynamicReframeEvent::Done {
                    clip_id,
                    crop_x_keyframes,
                    crop_y_keyframes,
                    crop_w_keyframes,
                    crop_h_keyframes,
                    subject_found,
                } => {
                    self.dynamic_reframe_state.dynamic_reframing_clip_id = None;
                    if self.is_shorts_pack_reframe_target(clip_id) {
                        self.apply_shorts_pack_reframe_result(
                            clip_id,
                            crop_x_keyframes,
                            crop_y_keyframes,
                            crop_w_keyframes,
                            crop_h_keyframes,
                        );
                    } else {
                        if self.selected_clip_id == Some(clip_id) {
                            self.set_selected_clip_crop_keyframes(
                                crop_x_keyframes,
                                crop_y_keyframes,
                                crop_w_keyframes,
                                crop_h_keyframes,
                            );
                        }
                        if !subject_found {
                            self.push_toast(
                                crate::i18n::Text::AutoReframeNoSubjectFound
                                    .tr(self.locale)
                                    .to_string(),
                            );
                        }
                    }
                }
                DynamicReframeEvent::Failed { clip_id, message } => {
                    self.dynamic_reframe_state.dynamic_reframing_clip_id = None;
                    tracing::error!(error = %message, clip_id, "dynamic auto-reframe failed");
                    if self.is_shorts_pack_reframe_target(clip_id) {
                        // Leave this clip without crop keyframes -- it exports centered, the
                        // same documented fallback an un-reframed clip already had -- and move
                        // on to the next pending one rather than aborting the whole pack.
                        self.advance_shorts_pack_reframe_queue();
                    } else {
                        self.push_toast(format!("Dynamic auto-reframe failed: {message}"));
                    }
                }
            }
        }
    }
}

/// Runs on [`App::spawn_dynamic_reframe_selected_clip`]'s background thread — opens
/// [`avcore::FrameSampler`] once, samples the subject center at each of
/// [`avcore::FrameSampler::even_sample_times`]'s evenly-spaced points, then pipes the resulting
/// trajectory through `avcore`'s gap-filling/smoothing/sparsification to produce the four crop
/// keyframe lists.
#[allow(clippy::too_many_arguments)]
fn dynamic_reframe_one(
    source_path: &Path,
    source_in_secs: f64,
    source_out_secs: f64,
    source_w: u32,
    source_h: u32,
    target_w: u32,
    target_h: u32,
    model_path: &Path,
    clip_id: u64,
    tx: &tokio::sync::mpsc::UnboundedSender<DynamicReframeEvent>,
) {
    let sample_times = avcore::FrameSampler::even_sample_times(
        source_in_secs,
        source_out_secs,
        DYNAMIC_REFRAME_SAMPLES_PER_SEC,
        DYNAMIC_REFRAME_MIN_SAMPLES,
        DYNAMIC_REFRAME_MAX_SAMPLES,
    );
    let Ok(sampler) = avcore::FrameSampler::open(source_path, Duration::from_millis(20)) else {
        let _ = tx.send(DynamicReframeEvent::Failed {
            clip_id,
            message: "could not open source video for sampling".to_string(),
        });
        return;
    };
    let duration = (source_out_secs - source_in_secs).max(1e-6);

    // Threaded across samples (in time order, hence a plain loop rather than `.map()`) so
    // `select_subject_center` can prefer whichever face continues the *previous* sample's own
    // chosen subject over just whichever has the highest raw confidence this sample — otherwise
    // two people trading the higher per-frame score would visibly re-target the crop between
    // samples even though neither actually moved. Only updated on an actual detection: a
    // gap-filled/centered sample carries no new evidence about where the subject is now.
    let mut previous_center: Option<(f32, f32)> = None;
    let mut samples: Vec<avcore::ReframeSample> = Vec::with_capacity(sample_times.len());
    for &t in &sample_times {
        let time_fraction = (((t - source_in_secs) / duration) as f32).clamp(0.0, 1.0);
        let subject_center = sampler
            .sample(t, Duration::from_millis(800))
            .and_then(|frame| {
                let (w, h, rgba) = downscale_frame_rgba(
                    frame.width,
                    frame.height,
                    frame.rgba,
                    REFRAME_FRAME_MAX_DIM,
                );
                match avcore::detect_faces(model_path, &rgba, w, h) {
                    Ok(faces) => avcore::select_subject_center(
                        &faces,
                        previous_center,
                        avcore::DEFAULT_CONTINUITY_MAX_DISTANCE,
                    ),
                    Err(e) => {
                        tracing::warn!(error = %e, "dynamic auto-reframe face detection failed");
                        None
                    }
                }
            });
        if subject_center.is_some() {
            previous_center = subject_center;
        }
        samples.push(avcore::ReframeSample {
            time_fraction,
            subject_center,
        });
    }

    let subject_found = samples.iter().any(|s| s.subject_center.is_some());
    let filled = avcore::fill_reframe_gaps(&samples);
    let smoothed = avcore::smooth_subject_centers(&filled, DYNAMIC_REFRAME_SMOOTH_WINDOW);

    let crop_samples: Vec<(f32, avcore::CropRect)> = samples
        .iter()
        .zip(smoothed.iter())
        .map(|(sample, center)| {
            let crop =
                avcore::compute_reframe_crop(source_w, source_h, target_w, target_h, *center);
            (sample.time_fraction, crop)
        })
        .collect();

    let (crop_x_keyframes, crop_y_keyframes, crop_w_keyframes, crop_h_keyframes) =
        avcore::sparse_crop_keyframes(&crop_samples, avcore::DEFAULT_SPARSIFY_EPSILON);

    let _ = tx.send(DynamicReframeEvent::Done {
        clip_id,
        crop_x_keyframes,
        crop_y_keyframes,
        crop_w_keyframes,
        crop_h_keyframes,
        subject_found,
    });
}
