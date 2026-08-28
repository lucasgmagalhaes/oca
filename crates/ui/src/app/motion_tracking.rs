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

use std::path::Path;
use std::time::Duration;

use avcore::{Keyframe, Position};

use super::{App, MotionTrackEvent, MOTION_TRACK_SEARCH_RADIUS_RANGE, MOTION_TRACK_SIZE_RANGE};

/// Frames sampled per second of the clip's own trimmed source duration — dense enough to follow
/// ordinary motion, coarse enough that a several-second clip still tracks in roughly a second or
/// two on a background thread. Capped by [`MAX_SAMPLES`] regardless of duration.
const SAMPLES_PER_SEC: f64 = 4.0;
/// Upper bound on sampled frames, regardless of `SAMPLES_PER_SEC * duration` — bounds worst-case
/// decode time for a very long clip.
const MAX_SAMPLES: usize = 60;

impl App {
    /// Enters the preview panel's drag-to-select motion-tracking region picker (see
    /// [`crate::screens::editor::draw_motion_track_region_picker`]) — a no-op if no preview
    /// frame is loaded, since the picker needs the preview's texture/aspect to draw against;
    /// callers should toast [`crate::i18n::Text::MotionTrackRegionPickNeedsPreview`] instead in
    /// that case, same as [`App::start_drawing_custom_shape`]'s precondition.
    pub fn start_picking_motion_track_region(&mut self) {
        if self.preview_state.preview_texture.is_none() {
            return;
        }
        self.motion_track_region.picking_motion_track_region = true;
    }

    pub fn stop_picking_motion_track_region(&mut self) {
        self.motion_track_region.picking_motion_track_region = false;
    }

    /// Runs motion tracking against `selected_clip_id` on a background thread — what the
    /// properties panel's "Rastrear movimento" button does. Tracks the region set by
    /// `motion_track_center_x`/`_y`/`motion_track_width`/`_height`/`motion_track_search_radius`
    /// (the properties panel's region controls, editable before clicking the button — defaults
    /// to a centered square region the same size the button always used before those controls
    /// existed) across the clip's own trimmed source duration, then rewrites
    /// `position_keyframes` as that motion applied on top of whatever single position (or the
    /// default centered-at-origin placement) was already set. A no-op if nothing is selected or
    /// a run is already in flight.
    pub fn spawn_motion_track_selected_clip(&mut self) {
        if self.motion_tracking_state.motion_tracking_clip_id.is_some() {
            return;
        }
        let Some(clip) = self.selected_clip() else {
            return;
        };
        let clip_id = clip.id;
        let asset_id = clip.asset_id;
        let source_in_secs = clip.source_in_secs;
        let source_out_secs = clip.source_out_secs;
        let base_position = clip
            .position_keyframes
            .first()
            .map(|k| k.value)
            .unwrap_or(Position { x: 0.0, y: 0.0 });
        let Some(asset) = self
            .active_project()
            .media_library
            .iter()
            .find(|a| a.id == asset_id)
        else {
            return;
        };
        let source_path = asset.source_path.clone();
        let center_x = self
            .motion_track_region
            .motion_track_center_x
            .clamp(0.0, 1.0);
        let center_y = self
            .motion_track_region
            .motion_track_center_y
            .clamp(0.0, 1.0);
        let template_width = self.motion_track_region.motion_track_width.clamp(
            *MOTION_TRACK_SIZE_RANGE.start(),
            *MOTION_TRACK_SIZE_RANGE.end(),
        );
        let template_height = self.motion_track_region.motion_track_height.clamp(
            *MOTION_TRACK_SIZE_RANGE.start(),
            *MOTION_TRACK_SIZE_RANGE.end(),
        );
        let search_radius = self.motion_track_region.motion_track_search_radius.clamp(
            *MOTION_TRACK_SEARCH_RADIUS_RANGE.start(),
            *MOTION_TRACK_SEARCH_RADIUS_RANGE.end(),
        );

        self.motion_tracking_state.motion_tracking_clip_id = Some(clip_id);
        let tx = self.motion_tracking_state.motion_tracking_tx.clone();
        std::thread::spawn(move || {
            let keyframes = motion_track_one(
                &source_path,
                source_in_secs,
                source_out_secs,
                base_position,
                center_x,
                center_y,
                template_width,
                template_height,
                search_radius,
            );
            let _ = tx.send(MotionTrackEvent::Done { clip_id, keyframes });
        });
    }

    /// Applies a finished motion-tracking run to the timeline. Called once per frame from
    /// [`eframe::App::ui`], same as [`App::pump_auto_reframe`].
    pub(super) fn pump_motion_tracking(&mut self) {
        while let Ok(event) = self.motion_tracking_state.motion_tracking_rx.try_recv() {
            match event {
                MotionTrackEvent::Done { clip_id, keyframes } => {
                    self.motion_tracking_state.motion_tracking_clip_id = None;
                    if keyframes.is_empty() {
                        self.push_toast(
                            crate::i18n::Text::MotionTrackNoFramesDecoded
                                .tr(self.locale)
                                .to_string(),
                        );
                        continue;
                    }
                    if self.selected_clip_id == Some(clip_id) {
                        self.set_selected_clip_position_keyframes(keyframes);
                    }
                }
            }
        }
    }
}

/// Runs on [`App::spawn_motion_track_selected_clip`]'s background thread — decodes frames
/// sampled across `[source_in_secs, source_out_secs)`, tracks the region centered at
/// `(center_x, center_y)` (source-frame fractions, per `avcore::track_region`) across them, and
/// converts the result into a `position_keyframes` list. Returns an empty `Vec` if fewer than 2
/// frames could be decoded (nothing meaningful to track — [`App::pump_motion_tracking`] leaves
/// the clip's existing keyframes untouched in that case rather than replacing them with a
/// single-point "animation").
#[allow(clippy::too_many_arguments)]
fn motion_track_one(
    source_path: &Path,
    source_in_secs: f64,
    source_out_secs: f64,
    base_position: Position,
    center_x: f32,
    center_y: f32,
    template_width: f32,
    template_height: f32,
    search_radius: f32,
) -> Vec<Keyframe<Position>> {
    let duration = (source_out_secs - source_in_secs).max(0.0);
    if duration <= 0.0 {
        return Vec::new();
    }
    let sample_times = avcore::FrameSampler::even_sample_times(
        source_in_secs,
        source_out_secs,
        SAMPLES_PER_SEC,
        2,
        MAX_SAMPLES,
    );

    let Some(sampler) = avcore::FrameSampler::open(source_path, Duration::from_millis(10)).ok()
    else {
        return Vec::new();
    };

    // (time_fraction, gray frame) — only for samples that actually decoded within the
    // deadline, so a dropped frame just shrinks the keyframe list rather than desyncing the
    // remaining ones' timing (using each decoded frame's own real timestamp, not its index,
    // keeps that correct even when some samples are skipped).
    let mut decoded = Vec::with_capacity(sample_times.len());
    for &t in &sample_times {
        let Some(frame) = sampler.sample(t, Duration::from_millis(800)) else {
            continue;
        };
        let time_fraction = ((t - source_in_secs) / duration) as f32;
        decoded.push((
            time_fraction,
            avcore::rgba_to_gray(&frame.rgba, frame.width, frame.height),
        ));
    }

    if decoded.len() < 2 {
        return Vec::new();
    }

    let (time_fractions, frames): (Vec<f32>, Vec<avcore::GrayFrame>) = decoded.into_iter().unzip();
    let tracked = avcore::track_region(
        &frames,
        center_x,
        center_y,
        template_width,
        template_height,
        search_radius,
    );
    avcore::tracked_positions_to_keyframes(&tracked, &time_fractions, base_position)
}
