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

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{App, MatteGenerationEvent};

/// Frames sampled per second of the clip's own trimmed source duration — coarser than motion-
/// tracking's own `SAMPLES_PER_SEC` (4.0): each sample re-loads and re-runs a whole ONNX
/// session (`avcore::segment_person` builds a fresh `Session` per call, see its doc comment),
/// so segmentation inference is much more expensive per frame than block-matching.
const SAMPLES_PER_SEC: f64 = 2.0;
/// Upper bound on sampled frames, regardless of `SAMPLES_PER_SEC * duration` — bounds
/// worst-case inference time (and the in-memory luma buffer's size —
/// `avcore::encode_matte_video` holds every sampled frame's alpha matte at once) for a very
/// long clip.
const MAX_SAMPLES: usize = 40;

impl App {
    /// Runs AI background-removal matte generation against `selected_clip_id` on a background
    /// thread — what the properties panel's "Gerar máscara" button does. Samples frames across
    /// the clip's own trimmed source duration, runs `avcore::segment_person` against each, and
    /// encodes the resulting per-frame alpha mattes into a small grayscale-as-luma H.264 video
    /// (`avcore::encode_matte_video`) cached next to the project
    /// (`avcore::background_removal::mask_cache_dir_for_project`). A no-op if nothing is
    /// selected, no model is configured, or a run is already in flight.
    pub fn spawn_generate_matte_for_selected_clip(&mut self) {
        if self
            .matte_generation_state
            .matte_generating_clip_id
            .is_some()
        {
            return;
        }
        if self.prefs.background_removal_model_path.trim().is_empty() {
            self.push_toast(
                crate::i18n::Text::BackgroundRemovalNoModelConfigured
                    .tr(self.locale)
                    .to_string(),
            );
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
        let model_path = PathBuf::from(self.prefs.background_removal_model_path.clone());
        let mask_dir =
            avcore::background_removal::mask_cache_dir_for_project(self.active_project());
        let mask_path = avcore::background_removal::mask_path_for_clip(clip_id, &mask_dir);

        self.matte_generation_state.matte_generating_clip_id = Some(clip_id);
        let tx = self.matte_generation_state.matte_generation_tx.clone();
        std::thread::spawn(move || {
            let event = match generate_matte_one(
                &source_path,
                source_in_secs,
                source_out_secs,
                &model_path,
                &mask_dir,
                &mask_path,
            ) {
                Ok(mask_path) => MatteGenerationEvent::Done { clip_id, mask_path },
                Err(message) => MatteGenerationEvent::Failed { message },
            };
            let _ = tx.send(event);
        });
    }

    /// Applies a finished matte-generation run to the timeline. Called once per frame from
    /// [`eframe::App::ui`], same as [`App::pump_auto_reframe`].
    pub(super) fn pump_matte_generation(&mut self) {
        while let Ok(event) = self.matte_generation_state.matte_generation_rx.try_recv() {
            match event {
                MatteGenerationEvent::Done { clip_id, mask_path } => {
                    self.matte_generation_state.matte_generating_clip_id = None;
                    if self.selected_clip_id == Some(clip_id) {
                        self.set_selected_clip_background_removal_mask_path(
                            mask_path.display().to_string(),
                        );
                    }
                }
                MatteGenerationEvent::Failed { message } => {
                    self.matte_generation_state.matte_generating_clip_id = None;
                    tracing::error!(error = %message, "matte generation failed");
                    self.push_toast(format!("Matte generation failed: {message}"));
                }
            }
        }
    }
}

/// Runs on [`App::spawn_generate_matte_for_selected_clip`]'s background thread — decodes frames
/// sampled across `[source_in_secs, source_out_secs)`, runs `avcore::segment_person` against
/// each, and encodes the resulting per-frame alpha mattes into a matte video at `mask_path`
/// (creating `mask_dir` first if needed). Returns `mask_path` back on success, or an error
/// message describing what failed.
fn generate_matte_one(
    source_path: &Path,
    source_in_secs: f64,
    source_out_secs: f64,
    model_path: &Path,
    mask_dir: &Path,
    mask_path: &Path,
) -> Result<PathBuf, String> {
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

    // (luma matte per sample, only for samples that actually decoded within the deadline and
    // matched the first decoded frame's resolution) — same "a dropped sample just shrinks the
    // list" tolerance motion-tracking's own decode loop uses, rather than failing the whole run
    // over one slow/missing frame.
    let mut luma_frames: Vec<Vec<u8>> = Vec::with_capacity(sample_times.len());
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
            // A source shouldn't change resolution mid-clip; skip a sample that somehow
            // doesn't match rather than feeding a mismatched-size buffer into the encoder.
            continue;
        }
        let alpha = avcore::segment_person(model_path, &frame.rgba, frame.width, frame.height)
            .map_err(|e| format!("segmentation failed: {e}"))?;
        luma_frames.push(
            alpha
                .iter()
                .map(|a| (a.clamp(0.0, 1.0) * 255.0).round() as u8)
                .collect(),
        );
    }

    if luma_frames.len() < 2 {
        return Err("couldn't decode enough frames to generate a matte".to_string());
    }

    // The matte's own declared fps: however many frames actually got sampled per second of the
    // clip's real duration, so the encoded video's total length approximately matches the
    // clip's own — close enough for the export-side compositor, which already tolerates a
    // shorter/longer overlay input by holding the last known frame (same mechanism used when a
    // track-1 clip's own duration doesn't exactly match track 0's). A fixed 1000 denominator
    // keeps this a plain integer ratio without needing real rational reduction.
    let effective_fps = luma_frames.len() as f64 / duration;
    let fps_num = ((effective_fps * 1000.0).round() as u32).max(1);
    let fps_den = 1000u32;

    std::fs::create_dir_all(mask_dir)
        .map_err(|e| format!("failed to create matte cache dir: {e}"))?;
    avcore::encode_matte_video(&luma_frames, frame_w, frame_h, fps_num, fps_den, mask_path)
        .map_err(|e| format!("failed to encode matte video: {e}"))?;

    Ok(mask_path.to_path_buf())
}
