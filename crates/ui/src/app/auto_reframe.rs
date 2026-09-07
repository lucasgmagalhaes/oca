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

use super::{App, AutoReframeEvent};

impl App {
    /// Runs auto-reframe against `selected_clip_id` on a background thread — what the
    /// properties panel's "Reenquadramento automático" button does. Targets the current
    /// active sequence's export aspect ratio (falling back to the source's own resolution for
    /// `Original`, same as export itself — see [`avcore::ExportAspectRatio::dims_or`]). A no-op
    /// if nothing is selected or a run is already in flight; a model must be configured *unless*
    /// the clip already carries a [`avcore::timeline::ClipInstance::reframe_seed_point`] — a
    /// manually pinned anchor needs no face-detection model at all.
    pub fn spawn_auto_reframe_selected_clip(&mut self) {
        if self.auto_reframe_state.auto_reframing_clip_id.is_some() {
            return;
        }
        let Some(clip) = self.selected_clip() else {
            return;
        };
        let seed_point = clip.reframe_seed_point;
        if seed_point.is_none() && self.prefs.reframe_model_path.trim().is_empty() {
            self.push_toast(
                crate::i18n::Text::AutoReframeNoModelConfigured
                    .tr(self.locale)
                    .to_string(),
            );
            return;
        }
        let clip_id = clip.id;
        let asset_id = clip.asset_id;
        let midpoint_secs = (clip.source_in_secs + clip.source_out_secs) / 2.0;
        let Some(asset) = self
            .active_project()
            .media_library
            .iter()
            .find(|a| a.id == asset_id)
        else {
            return;
        };
        let Some((source_w, source_h)) = asset.resolution else {
            return;
        };
        let source_path = asset.source_path.clone();
        let (target_w, target_h) = self
            .active_sequence_export_settings()
            .aspect_ratio
            .dims_or((source_w, source_h));
        let model_path = PathBuf::from(self.prefs.reframe_model_path.clone());

        self.auto_reframe_state.auto_reframing_clip_id = Some(clip_id);
        let tx = self.auto_reframe_state.auto_reframe_tx.clone();
        std::thread::spawn(move || {
            auto_reframe_one(
                &source_path,
                midpoint_secs,
                source_w,
                source_h,
                target_w,
                target_h,
                &model_path,
                clip_id,
                seed_point,
                &tx,
            );
        });
    }

    /// Applies a finished auto-reframe run to the timeline. Called once per frame from
    /// [`eframe::App::ui`], same as [`App::pump_transcribe`].
    pub(super) fn pump_auto_reframe(&mut self) {
        while let Ok(event) = self.auto_reframe_state.auto_reframe_rx.try_recv() {
            match event {
                AutoReframeEvent::Done {
                    clip_id,
                    crop,
                    subject_found,
                } => {
                    self.auto_reframe_state.auto_reframing_clip_id = None;
                    if self.selected_clip_id == Some(clip_id) {
                        self.set_selected_clip_crop(crop.x, crop.y, crop.w, crop.h);
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
        }
    }
}

/// Runs on [`App::spawn_auto_reframe_selected_clip`]'s background thread — decodes one frame at
/// `at_secs`, runs face detection against it, and computes the resulting crop rect. Detection
/// failures (missing/broken model, no frame decoded) fall back to a centered crop rather than
/// leaving the clip untouched — `crop_x`/`crop_y`/`crop_w`/`crop_h` is a plain data field with
/// no "unset" state, so this always has *something* reasonable to apply, matching
/// [`avcore::auto_reframe::compute_reframe_crop`]'s own `None`-subject fallback.
///
/// `seed_point`, when `Some` (`ClipInstance::reframe_seed_point`, CF-04's own "optional
/// user-provided seed point"), is used directly as the crop's anchor and skips face detection
/// entirely — no frame decode, no ONNX inference — since the user has already pinned exactly
/// where this clip should stay framed, overriding whatever face detection would have found.
#[allow(clippy::too_many_arguments)]
fn auto_reframe_one(
    source_path: &Path,
    at_secs: f64,
    source_w: u32,
    source_h: u32,
    target_w: u32,
    target_h: u32,
    model_path: &Path,
    clip_id: u64,
    seed_point: Option<(f32, f32)>,
    tx: &tokio::sync::mpsc::UnboundedSender<AutoReframeEvent>,
) {
    let subject_center = match seed_point {
        Some(seed) => Some(seed),
        None => {
            let frame = extract_frame(source_path, at_secs);
            frame.and_then(
                |(w, h, rgba)| match avcore::detect_faces(model_path, &rgba, w, h) {
                    Ok(faces) => avcore::main_subject_center(&faces),
                    Err(e) => {
                        tracing::warn!(error = %e, "auto-reframe face detection failed");
                        None
                    }
                },
            )
        }
    };
    let subject_found = subject_center.is_some();
    let crop = avcore::compute_reframe_crop(source_w, source_h, target_w, target_h, subject_center);
    let _ = tx.send(AutoReframeEvent::Done {
        clip_id,
        crop,
        subject_found,
    });
}

/// Decodes one frame from `path` at `at_secs`, undownscaled beyond a generous cap — face
/// detection resizes to its own fixed 320x240 input regardless, so this only needs to be large
/// enough that a small/distant face survives that resize as more than a couple of pixels.
/// Uses [`avcore::FrameSampler`], same as `import.rs`'s `extract_thumbnail`, just a different
/// size cap and a different destination (a detector, not a UI texture).
pub(super) const REFRAME_FRAME_MAX_DIM: u32 = 960;

fn extract_frame(path: &Path, at_secs: f64) -> Option<(u32, u32, Vec<u8>)> {
    let sampler = avcore::FrameSampler::open(path, Duration::from_millis(20)).ok()?;
    let frame = sampler.sample(at_secs, Duration::from_millis(1500))?;
    Some(downscale_frame_rgba(
        frame.width,
        frame.height,
        frame.rgba,
        REFRAME_FRAME_MAX_DIM,
    ))
}

/// Nearest-neighbor downscale to fit within `max_dim` on the longer side, no-op if the frame is
/// already smaller. Shared by [`extract_frame`] (static auto-reframe) and dynamic auto-reframe's
/// per-sample decoding — both feed the same [`avcore::detect_faces`] detector at the same size
/// cap, so this stays a single implementation rather than two copies drifting apart.
pub(super) fn downscale_frame_rgba(
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    max_dim: u32,
) -> (u32, u32, Vec<u8>) {
    let scale = (max_dim as f32 / width.max(height) as f32).min(1.0);
    if scale >= 1.0 {
        return (width, height, rgba);
    }
    let new_width = ((width as f32 * scale) as u32).max(1);
    let new_height = ((height as f32 * scale) as u32).max(1);
    let mut out = vec![0u8; (new_width * new_height * 4) as usize];
    for y in 0..new_height {
        let src_y = (y * height / new_height).min(height - 1);
        for x in 0..new_width {
            let src_x = (x * width / new_width).min(width - 1);
            let src = ((src_y * width + src_x) * 4) as usize;
            let dst = ((y * new_width + x) * 4) as usize;
            out[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
        }
    }
    (new_width, new_height, out)
}

#[cfg(test)]
#[path = "auto_reframe/auto_reframe_test.rs"]
mod tests;
