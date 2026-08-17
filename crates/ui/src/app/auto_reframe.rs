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
use std::time::{Duration, Instant};

use super::{App, AutoReframeEvent};

impl App {
    /// Runs auto-reframe against `selected_clip_id` on a background thread — what the
    /// properties panel's "Reenquadramento automático" button does. Targets the current
    /// `export_aspect_ratio` (falling back to the source's own resolution for `Original`, same
    /// as export itself — see [`avcore::ExportAspectRatio::dims_or`]). A no-op if nothing is
    /// selected, no model is configured, or a run is already in flight.
    pub fn spawn_auto_reframe_selected_clip(&mut self) {
        if self.auto_reframing_clip_id.is_some() {
            return;
        }
        if self.prefs.reframe_model_path.trim().is_empty() {
            self.push_toast(
                crate::i18n::Text::AutoReframeNoModelConfigured
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
        let (target_w, target_h) = self.export_aspect_ratio.dims_or((source_w, source_h));
        let model_path = PathBuf::from(self.prefs.reframe_model_path.clone());

        self.auto_reframing_clip_id = Some(clip_id);
        let tx = self.auto_reframe_tx.clone();
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
                &tx,
            );
        });
    }

    /// Applies a finished auto-reframe run to the timeline. Called once per frame from
    /// [`eframe::App::ui`], same as [`App::pump_transcribe`].
    pub(super) fn pump_auto_reframe(&mut self) {
        while let Ok(event) = self.auto_reframe_rx.try_recv() {
            match event {
                AutoReframeEvent::Done {
                    clip_id,
                    crop,
                    subject_found,
                } => {
                    self.auto_reframing_clip_id = None;
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
                AutoReframeEvent::Failed { message } => {
                    self.auto_reframing_clip_id = None;
                    tracing::error!(error = %message, "auto-reframe failed");
                    self.push_toast(format!("Auto-reframe failed: {message}"));
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
    tx: &tokio::sync::mpsc::UnboundedSender<AutoReframeEvent>,
) {
    let frame = extract_frame(source_path, at_secs);
    let subject_center =
        frame.and_then(
            |(w, h, rgba)| match avcore::detect_faces(model_path, &rgba, w, h) {
                Ok(faces) => avcore::main_subject_center(&faces),
                Err(e) => {
                    tracing::warn!(error = %e, "auto-reframe face detection failed");
                    None
                }
            },
        );
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
/// Same open/seek/poll shape as `import.rs`'s `extract_thumbnail`, just a different size cap and
/// a different destination (a detector, not a UI texture).
const REFRAME_FRAME_MAX_DIM: u32 = 960;

fn extract_frame(path: &Path, at_secs: f64) -> Option<(u32, u32, Vec<u8>)> {
    let preview = avcore::preview::Preview::open(path, None).ok()?;
    let _ = preview.seek(at_secs.max(0.0));

    let deadline = Instant::now() + Duration::from_millis(1500);
    let frame = loop {
        if let Some(frame) = preview.current_frame() {
            break frame;
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    let scale = (REFRAME_FRAME_MAX_DIM as f32 / frame.width.max(frame.height) as f32).min(1.0);
    if scale >= 1.0 {
        return Some((frame.width, frame.height, frame.rgba));
    }
    let new_width = ((frame.width as f32 * scale) as u32).max(1);
    let new_height = ((frame.height as f32 * scale) as u32).max(1);
    let mut rgba = vec![0u8; (new_width * new_height * 4) as usize];
    for y in 0..new_height {
        let src_y = (y * frame.height / new_height).min(frame.height - 1);
        for x in 0..new_width {
            let src_x = (x * frame.width / new_width).min(frame.width - 1);
            let src = ((src_y * frame.width + src_x) * 4) as usize;
            let dst = ((y * new_width + x) * 4) as usize;
            rgba[dst..dst + 4].copy_from_slice(&frame.rgba[src..src + 4]);
        }
    }
    Some((new_width, new_height, rgba))
}
