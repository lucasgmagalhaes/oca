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

//! Layer-group templates (`request.md`'s Fase 4 "Templates de grupo de camadas") — save the
//! per-layer settings of a multi-selected group of clips (position, scale, crop, effects, via
//! [`avcore::ClipFormatting`]) as a named, reusable [`avcore::LayerTemplate`], then apply it to
//! fresh footage later by picking one source asset per saved layer. Persisted app-wide in
//! `prefs.saved_layer_templates`, not per-project — the whole point is reusing the same layer
//! group (e.g. "webcam recortada + fundo com blur + jogo centralizado") across different shorts.

use super::timeline_ops::{create_new_track, next_clip_id};
use super::App;

impl App {
    /// Snapshots `multi_selected_clip_ids`' per-layer `(TrackKind, ClipFormatting)`, ordered by
    /// track position then start time, and stages it in [`App::saving_layer_template`] for the
    /// naming modal — what the toolbar's "Salvar como template" button does. A no-op if nothing
    /// is multi-selected.
    pub fn begin_save_layer_template(&mut self) {
        if self.multi_selected_clip_ids.is_empty() {
            return;
        }
        let ids = self.multi_selected_clip_ids.clone();
        let timeline = self.active_project().timeline();
        let mut layers: Vec<(
            usize,
            f64,
            avcore::timeline::TrackKind,
            avcore::ClipFormatting,
        )> = Vec::new();
        for (track_idx, track) in timeline.tracks.iter().enumerate() {
            for clip in &track.clips {
                if ids.contains(&clip.id) {
                    layers.push((track_idx, clip.start_secs, track.kind, clip.formatting()));
                }
            }
        }
        if layers.is_empty() {
            return;
        }
        layers.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let layers = layers
            .into_iter()
            .map(|(_, _, kind, f)| (kind, f))
            .collect();
        self.saving_layer_template = Some((layers, String::new()));
    }

    /// Commits [`App::saving_layer_template`] as a new entry in `prefs.saved_layer_templates`
    /// and persists it — what the save-template modal's confirm button does. A no-op (leaving
    /// the modal open) if the name is blank; the caller is expected to have already validated
    /// this before offering the confirm button, but this re-checks rather than trust that.
    pub fn commit_save_layer_template(&mut self) {
        let Some((layers, name)) = self.saving_layer_template.clone() else {
            return;
        };
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        self.prefs
            .saved_layer_templates
            .push(avcore::LayerTemplate { name, layers });
        self.saving_layer_template = None;
        self.save_prefs();
    }

    /// Removes `prefs.saved_layer_templates[index]` and persists the change — what the
    /// templates list popup's delete button does. A no-op if `index` is out of range.
    pub fn delete_layer_template(&mut self, index: usize) {
        if index >= self.prefs.saved_layer_templates.len() {
            return;
        }
        self.prefs.saved_layer_templates.remove(index);
        self.save_prefs();
    }

    /// Opens the apply-template modal for `prefs.saved_layer_templates[template_index]`, with
    /// one empty asset slot per saved layer for the user to fill in. A no-op if `template_index`
    /// is out of range.
    pub fn begin_apply_layer_template(&mut self, template_index: usize) {
        let Some(template) = self.prefs.saved_layer_templates.get(template_index) else {
            return;
        };
        self.applying_layer_template = Some((template_index, vec![None; template.layers.len()]));
        self.layer_templates_menu_open = false;
    }

    /// Applies [`App::applying_layer_template`]: for every layer with an asset chosen, creates a
    /// brand-new track of that layer's kind ([`create_new_track`] — never reuses an existing
    /// track, so a multi-video-layer template doesn't collapse its layers onto the same track),
    /// adds a clip for the chosen asset at the current playhead, then applies the layer's saved
    /// [`avcore::ClipFormatting`] onto it. Layers left without an asset are silently skipped —
    /// the modal's own "Aplicar" button is expected to gate on every slot being filled, but this
    /// degrades gracefully (partial apply) rather than refusing outright if it isn't. A no-op if
    /// nothing was staged or the referenced template no longer exists (deleted mid-modal).
    pub fn confirm_apply_layer_template(&mut self) {
        let Some((template_index, layer_asset_ids)) = self.applying_layer_template.take() else {
            return;
        };
        let Some(template) = self
            .prefs
            .saved_layer_templates
            .get(template_index)
            .cloned()
        else {
            return;
        };
        let playhead_secs = self.active_project().timeline().playhead_secs;
        for (layer_formatting, asset_id) in template
            .layers
            .iter()
            .map(|(_, formatting)| formatting)
            .zip(layer_asset_ids.iter())
        {
            let Some(asset_id) = asset_id else { continue };
            let Some((kind, duration_secs)) = self.asset_kind_and_duration(*asset_id) else {
                continue;
            };
            let timeline = self.active_project_mut().timeline_mut();
            let track_index = create_new_track(timeline, kind);
            let clip_id = next_clip_id(timeline);
            timeline.tracks[track_index]
                .clips
                .push(avcore::timeline::ClipInstance {
                    id: clip_id,
                    asset_id: *asset_id,
                    start_secs: playhead_secs,
                    source_in_secs: 0.0,
                    source_out_secs: duration_secs,
                    composite_id: None,
                    color_label: None,
                    gain_db: 0.0,
                    frozen: false,
                    speed_factor: 1.0,
                    speed_ramp_end_factor: None,
                    nested_sequence_id: None,
                    crop_x: 0.0,
                    crop_y: 0.0,
                    crop_w: 1.0,
                    crop_h: 1.0,
                    mask_shape: avcore::timeline::MaskShape::None,
                    mask_corner_radius: 0.0,
                    flipped_h: false,
                    color_filter: avcore::timeline::ColorFilter::None,
                    vignette_intensity: 0.0,
                    brightness: 0.0,
                    contrast: 1.0,
                    saturation: 1.0,
                    sharpen: 0.0,
                    chroma_key_enabled: false,
                    chroma_key_color: [0, 255, 0],
                    chroma_key_tolerance: 0.4,
                    blur_intensity: 0.0,
                    shake_intensity: 0.0,
                    glitch_intensity: 0.0,
                    pixelize_intensity: 0.0,
                    transition_in: avcore::timeline::TransitionType::None,
                    transition_duration_secs: 0.5,
                    position_keyframes: vec![],
                    scale_keyframes: vec![],
                    rotation_keyframes: vec![],
                    opacity_keyframes: vec![],
                    gain_keyframes: vec![],
                    voice_cleanup_enabled: false,
                    voice_cleanup_noise_floor_db: -30.0,
                    voice_cleanup_compressor_threshold_db: -18.0,
                    voice_cleanup_compressor_ratio: 3.0,
                    voice_cleanup_ceiling_linear: 0.95,
                    brightness_keyframes: vec![],
                    contrast_keyframes: vec![],
                    saturation_keyframes: vec![],
                    crop_x_keyframes: vec![],
                    crop_y_keyframes: vec![],
                    crop_w_keyframes: vec![],
                    crop_h_keyframes: vec![],
                    deflicker_enabled: false,
                    lut_path: String::new(),
                    layer_scale_x: 1.0,
                    layer_scale_y: 1.0,
                    stabilization_intensity: 0.0,
                    background_removal_enabled: false,
                    background_removal_mask_path: String::new(),
                    blend_mode: avcore::timeline::BlendMode::Normal,
                    anchor_x: 0.5,
                    anchor_y: 0.5,
                });
            timeline.tracks[track_index]
                .clips
                .last_mut()
                .expect("just pushed")
                .apply_formatting(layer_formatting);
        }
    }
}
