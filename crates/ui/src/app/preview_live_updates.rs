// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Live property updates for an already-open preview pipeline.

use super::{App, LiveUpdateKey};

impl App {
    fn preview_includes_video_clip(&self, clip_id: u64) -> bool {
        self.preview_state.preview_clip_id == Some(clip_id)
            || self
                .preview_state
                .preview_overlay_clip_ids
                .contains(&clip_id)
    }

    /// Pushes brightness, contrast, and saturation into a loaded video branch.
    pub(super) fn push_live_balance_update(
        &mut self,
        clip_id: u64,
        brightness: f32,
        contrast: f32,
        effective_saturation: f32,
    ) {
        if !self.preview_includes_video_clip(clip_id) {
            return;
        }
        if let Some(worker) = &self.preview_state.worker {
            worker.live(
                LiveUpdateKey::for_clip("balance", clip_id),
                move |preview| {
                    preview.set_live_balance(clip_id, brightness, contrast, effective_saturation);
                },
            );
        }
    }

    /// Pushes blur or sharpen strength into a loaded video branch.
    pub(super) fn push_live_blur_update(&mut self, clip_id: u64, net_sigma: f64) {
        if !self.preview_includes_video_clip(clip_id) {
            return;
        }
        if let Some(worker) = &self.preview_state.worker {
            worker.live(LiveUpdateKey::for_clip("blur", clip_id), move |preview| {
                preview.set_live_blur(clip_id, net_sigma);
            });
        }
    }

    /// Pushes chroma-key settings into a loaded video branch.
    pub(super) fn push_live_chroma_key_update(
        &mut self,
        clip_id: u64,
        color: [u8; 3],
        tolerance: f32,
    ) {
        if !self.preview_includes_video_clip(clip_id) {
            return;
        }
        if let Some(worker) = &self.preview_state.worker {
            worker.live(
                LiveUpdateKey::for_clip("chroma-key", clip_id),
                move |preview| {
                    preview.set_live_chroma_key(clip_id, color, tolerance);
                },
            );
        }
    }

    /// Pushes crop settings into a loaded video branch.
    pub(super) fn push_live_crop_update(
        &mut self,
        clip_id: u64,
        crop_x: f32,
        crop_y: f32,
        crop_w: f32,
        crop_h: f32,
    ) {
        if !self.preview_includes_video_clip(clip_id) {
            return;
        }
        if let Some(worker) = &self.preview_state.worker {
            worker.live(LiveUpdateKey::for_clip("crop", clip_id), move |preview| {
                preview.set_live_crop(clip_id, crop_x, crop_y, crop_w, crop_h);
            });
        }
    }

    /// Pushes pixelize strength into a loaded video branch.
    pub(super) fn push_live_pixelize_update(&mut self, clip_id: u64, pixelize_intensity: f32) {
        if !self.preview_includes_video_clip(clip_id) {
            return;
        }
        if let Some(worker) = &self.preview_state.worker {
            worker.live(
                LiveUpdateKey::for_clip("pixelize", clip_id),
                move |preview| {
                    preview.set_live_pixelize(clip_id, pixelize_intensity);
                },
            );
        }
    }

    /// Pushes shake strength into a loaded video branch.
    pub(super) fn push_live_shake_update(&mut self, clip_id: u64, shake_intensity: f32) {
        if !self.preview_includes_video_clip(clip_id) {
            return;
        }
        if let Some(worker) = &self.preview_state.worker {
            worker.live(LiveUpdateKey::for_clip("shake", clip_id), move |preview| {
                preview.set_live_shake(clip_id, shake_intensity);
            });
        }
    }

    /// Pushes the overlay-only mask settings into a loaded composited branch.
    pub(super) fn push_live_mask_update(
        &mut self,
        clip_id: u64,
        mask_shape: avcore::timeline::MaskShape,
        mask_corner_radius: f32,
    ) {
        if !self
            .preview_state
            .preview_overlay_clip_ids
            .contains(&clip_id)
        {
            return;
        }
        if let Some(worker) = &self.preview_state.worker {
            worker.live(LiveUpdateKey::for_clip("mask", clip_id), move |preview| {
                let _ = preview.set_live_mask(clip_id, mask_shape, mask_corner_radius);
            });
        }
    }
}
