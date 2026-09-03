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

use avcore::timeline::{ColorFilter, MaskShape, TransitionType};
use avcore::{Keyframe, Position};

use super::{
    App, BLUR_INTENSITY_RANGE, BRIGHTNESS_RANGE, CHROMA_KEY_TOLERANCE_RANGE, CONTRAST_RANGE,
    CROP_MIN_SIZE, GAIN_DB_RANGE, GLITCH_INTENSITY_RANGE, LAYER_SCALE_RANGE,
    MASK_CORNER_RADIUS_RANGE, PIXELIZE_INTENSITY_RANGE, SATURATION_RANGE, SCALE_RANGE,
    SHAKE_INTENSITY_RANGE, SHARPEN_RANGE, SPEED_FACTOR_RANGE, STABILIZATION_INTENSITY_RANGE,
    TRANSITION_DURATION_RANGE, VIGNETTE_INTENSITY_RANGE, VOICE_CLEANUP_CEILING_RANGE,
    VOICE_CLEANUP_COMPRESSOR_RATIO_RANGE, VOICE_CLEANUP_COMPRESSOR_THRESHOLD_RANGE,
    VOICE_CLEANUP_NOISE_FLOOR_RANGE,
};

impl App {
    /// Sets `selected_clip_id`'s [`avcore::timeline::ClipInstance::gain_db`], clamped to
    /// [`GAIN_DB_RANGE`] — what dragging the properties panel's gain slider does. A no-op if
    /// nothing is selected.
    pub fn set_selected_clip_gain(&mut self, gain_db: f32) {
        let gain_db = gain_db.clamp(*GAIN_DB_RANGE.start(), *GAIN_DB_RANGE.end());
        self.with_selected_clip_mut(|clip| clip.gain_db = gain_db);
    }

    /// Sets `selected_clip_id`'s [`avcore::timeline::ClipInstance::frozen`] — what checking the
    /// properties panel's "Congelar quadro" box does. A no-op if nothing is selected.
    pub fn set_selected_clip_frozen(&mut self, frozen: bool) {
        self.with_selected_clip_mut(|clip| clip.frozen = frozen);
    }

    /// Sets `selected_clip_id`'s [`avcore::timeline::ClipInstance::deflicker_enabled`] — what
    /// checking the properties panel's "Remover flicker" box does. A no-op if nothing is
    /// selected.
    pub fn set_selected_clip_deflicker(&mut self, deflicker_enabled: bool) {
        self.with_selected_clip_mut(|clip| clip.deflicker_enabled = deflicker_enabled);
    }

    /// Sets `selected_clip_id`'s
    /// [`avcore::timeline::ClipInstance::background_removal_enabled`] — what checking the
    /// properties panel's "Remover fundo (IA)" box does. A no-op if nothing is selected. Only
    /// takes visible effect on export once a matte has actually been generated (see
    /// [`App::spawn_generate_matte_for_selected_clip`]) — checking the box alone doesn't
    /// generate one.
    pub fn set_selected_clip_background_removal(&mut self, background_removal_enabled: bool) {
        self.with_selected_clip_mut(|clip| {
            clip.background_removal_enabled = background_removal_enabled
        });
    }

    /// Sets `selected_clip_id`'s
    /// [`avcore::timeline::ClipInstance::background_removal_mask_path`] — what a finished
    /// [`App::spawn_generate_matte_for_selected_clip`] run applies. A no-op if nothing is
    /// selected.
    pub fn set_selected_clip_background_removal_mask_path(&mut self, mask_path: String) {
        self.with_selected_clip_mut(|clip| clip.background_removal_mask_path = mask_path);
    }

    /// Sets `selected_clip_id`'s [`avcore::timeline::ClipInstance::speed_factor`], clamped to
    /// [`SPEED_FACTOR_RANGE`] — what dragging the properties panel's speed slider does. A no-op
    /// if nothing is selected.
    pub fn set_selected_clip_speed(&mut self, speed_factor: f32) {
        let Some(clip_id) = self.selected_clip_id else {
            return;
        };
        let speed_factor =
            speed_factor.clamp(*SPEED_FACTOR_RANGE.start(), *SPEED_FACTOR_RANGE.end());
        let mut changed = false;
        self.with_selected_clip_mut(|clip| {
            clip.speed_factor = speed_factor;
            changed = true;
        });
        if changed {
            self.refresh_preview_speed(clip_id);
        }
    }

    /// Sets `selected_clip_id`'s crop rect ([`avcore::timeline::ClipInstance::crop_x`]/`crop_y`/
    /// `crop_w`/`crop_h`), each independently clamped to `[0.0, 1.0]` (`crop_w`/`crop_h` floored
    /// at [`CROP_MIN_SIZE`]) — what dragging the properties panel's crop controls does. A no-op
    /// if nothing is selected.
    pub fn set_selected_clip_crop(&mut self, crop_x: f32, crop_y: f32, crop_w: f32, crop_h: f32) {
        let crop_x = crop_x.clamp(0.0, 1.0);
        let crop_y = crop_y.clamp(0.0, 1.0);
        let crop_w = crop_w.clamp(CROP_MIN_SIZE, 1.0);
        let crop_h = crop_h.clamp(CROP_MIN_SIZE, 1.0);
        self.with_selected_clip_mut(|clip| {
            clip.crop_x = crop_x;
            clip.crop_y = crop_y;
            clip.crop_w = crop_w;
            clip.crop_h = crop_h;
        });
    }

    /// Sets `selected_clip_id`'s layer mask ([`avcore::timeline::ClipInstance::mask_shape`]/
    /// `mask_corner_radius`, the latter clamped to [`MASK_CORNER_RADIUS_RANGE`]) — what picking
    /// a shape/dragging the corner-radius slider in the properties panel does. A no-op if
    /// nothing is selected.
    pub fn set_selected_clip_mask(&mut self, mask_shape: MaskShape, mask_corner_radius: f32) {
        let mask_corner_radius = mask_corner_radius.clamp(
            *MASK_CORNER_RADIUS_RANGE.start(),
            *MASK_CORNER_RADIUS_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| {
            clip.mask_shape = mask_shape;
            clip.mask_corner_radius = mask_corner_radius;
        });
    }

    /// Sets `selected_clip_id`'s horizontal mirroring
    /// ([`avcore::timeline::ClipInstance::flipped_h`]) — what checking the properties panel's
    /// "Espelhar" box does. A no-op if nothing is selected.
    pub fn set_selected_clip_flip_h(&mut self, flipped_h: bool) {
        self.with_selected_clip_mut(|clip| clip.flipped_h = flipped_h);
    }

    /// Sets `selected_clip_id`'s color filter
    /// ([`avcore::timeline::ClipInstance::color_filter`]) — what picking a filter in the
    /// properties panel does. A no-op if nothing is selected.
    pub fn set_selected_clip_color_filter(&mut self, color_filter: ColorFilter) {
        self.with_selected_clip_mut(|clip| clip.color_filter = color_filter);
    }

    /// Sets `selected_clip_id`'s compositing blend mode
    /// ([`avcore::timeline::ClipInstance::blend_mode`]) — what picking a mode in the properties
    /// panel's Composite section does. A no-op if nothing is selected. Only meaningful for a
    /// clip on an overlay track — see that field's own doc comment for the position/PIP scope
    /// limit while a non-`Normal` mode is active.
    pub fn set_selected_clip_blend_mode(&mut self, blend_mode: avcore::timeline::BlendMode) {
        self.with_selected_clip_mut(|clip| clip.blend_mode = blend_mode);
    }

    /// Sets `selected_clip_id`'s 3D LUT path ([`avcore::timeline::ClipInstance::lut_path`]) —
    /// what browsing for a `.cube` file or picking a preset in the properties panel does. Empty
    /// string clears the LUT. A no-op if nothing is selected.
    pub fn set_selected_clip_lut(&mut self, lut_path: String) {
        self.with_selected_clip_mut(|clip| clip.lut_path = lut_path);
    }

    /// Sets `selected_clip_id`'s layer footprint size
    /// ([`avcore::timeline::ClipInstance::layer_scale_x`]/`_y`, each independently clamped to
    /// [`LAYER_SCALE_RANGE`]) — what dragging the properties panel's width/height sliders, or a
    /// corner resize handle on the preview panel's layer box, does. A no-op if nothing is
    /// selected.
    pub fn set_selected_clip_layer_scale(&mut self, scale_x: f32, scale_y: f32) {
        let scale_x = scale_x.clamp(*LAYER_SCALE_RANGE.start(), *LAYER_SCALE_RANGE.end());
        let scale_y = scale_y.clamp(*LAYER_SCALE_RANGE.start(), *LAYER_SCALE_RANGE.end());
        self.with_selected_clip_mut(|clip| {
            clip.layer_scale_x = scale_x;
            clip.layer_scale_y = scale_y;
        });
    }

    /// Sets `selected_clip_id`'s video-stabilization strength
    /// ([`avcore::timeline::ClipInstance::stabilization_intensity`], clamped to
    /// [`STABILIZATION_INTENSITY_RANGE`]) — what dragging the properties panel's stabilization
    /// slider does. A no-op if nothing is selected.
    pub fn set_selected_clip_stabilization(&mut self, stabilization_intensity: f32) {
        let stabilization_intensity = stabilization_intensity.clamp(
            *STABILIZATION_INTENSITY_RANGE.start(),
            *STABILIZATION_INTENSITY_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| clip.stabilization_intensity = stabilization_intensity);
    }

    /// Sets `selected_clip_id`'s vignette strength
    /// ([`avcore::timeline::ClipInstance::vignette_intensity`], clamped to
    /// [`VIGNETTE_INTENSITY_RANGE`]) — what dragging the properties panel's vignette slider
    /// does. A no-op if nothing is selected.
    pub fn set_selected_clip_vignette(&mut self, vignette_intensity: f32) {
        let vignette_intensity = vignette_intensity.clamp(
            *VIGNETTE_INTENSITY_RANGE.start(),
            *VIGNETTE_INTENSITY_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| clip.vignette_intensity = vignette_intensity);
    }

    /// Sets `selected_clip_id`'s sharpen strength
    /// ([`avcore::timeline::ClipInstance::sharpen`], clamped to [`SHARPEN_RANGE`]) — what
    /// dragging the properties panel's sharpen slider does. A no-op if nothing is selected.
    pub fn set_selected_clip_sharpen(&mut self, sharpen: f32) {
        let sharpen = sharpen.clamp(*SHARPEN_RANGE.start(), *SHARPEN_RANGE.end());
        self.with_selected_clip_mut(|clip| clip.sharpen = sharpen);
    }

    /// Sets `selected_clip_id`'s chroma key settings
    /// ([`avcore::timeline::ClipInstance::chroma_key_enabled`]/`chroma_key_color`/
    /// `chroma_key_tolerance`, the tolerance clamped to [`CHROMA_KEY_TOLERANCE_RANGE`]) — what
    /// toggling the checkbox or adjusting the color/tolerance controls in the properties panel
    /// does. A no-op if nothing is selected.
    pub fn set_selected_clip_chroma_key(
        &mut self,
        chroma_key_enabled: bool,
        chroma_key_color: [u8; 3],
        chroma_key_tolerance: f32,
    ) {
        let chroma_key_tolerance = chroma_key_tolerance.clamp(
            *CHROMA_KEY_TOLERANCE_RANGE.start(),
            *CHROMA_KEY_TOLERANCE_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| {
            clip.chroma_key_enabled = chroma_key_enabled;
            clip.chroma_key_color = chroma_key_color;
            clip.chroma_key_tolerance = chroma_key_tolerance;
        });
    }

    /// Sets `selected_clip_id`'s CF-03 voice-cleanup toggle and its four adjustable parameters
    /// ([`avcore::timeline::ClipInstance::voice_cleanup_enabled`]/`voice_cleanup_noise_floor_db`/
    /// `voice_cleanup_compressor_threshold_db`/`voice_cleanup_compressor_ratio`/
    /// `voice_cleanup_ceiling_linear`, each clamped to its own `VOICE_CLEANUP_*_RANGE`) — what
    /// the properties panel's "Limpeza de voz" section does. Export-only (see that field's own
    /// doc comment on `ClipInstance`) — this never touches the live preview pipeline, unlike
    /// most of this file's other setters. A no-op if nothing is selected.
    pub fn set_selected_clip_voice_cleanup(
        &mut self,
        voice_cleanup_enabled: bool,
        voice_cleanup_noise_floor_db: f32,
        voice_cleanup_compressor_threshold_db: f32,
        voice_cleanup_compressor_ratio: f32,
        voice_cleanup_ceiling_linear: f32,
    ) {
        let voice_cleanup_noise_floor_db = voice_cleanup_noise_floor_db.clamp(
            *VOICE_CLEANUP_NOISE_FLOOR_RANGE.start(),
            *VOICE_CLEANUP_NOISE_FLOOR_RANGE.end(),
        );
        let voice_cleanup_compressor_threshold_db = voice_cleanup_compressor_threshold_db.clamp(
            *VOICE_CLEANUP_COMPRESSOR_THRESHOLD_RANGE.start(),
            *VOICE_CLEANUP_COMPRESSOR_THRESHOLD_RANGE.end(),
        );
        let voice_cleanup_compressor_ratio = voice_cleanup_compressor_ratio.clamp(
            *VOICE_CLEANUP_COMPRESSOR_RATIO_RANGE.start(),
            *VOICE_CLEANUP_COMPRESSOR_RATIO_RANGE.end(),
        );
        let voice_cleanup_ceiling_linear = voice_cleanup_ceiling_linear.clamp(
            *VOICE_CLEANUP_CEILING_RANGE.start(),
            *VOICE_CLEANUP_CEILING_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| {
            clip.voice_cleanup_enabled = voice_cleanup_enabled;
            clip.voice_cleanup_noise_floor_db = voice_cleanup_noise_floor_db;
            clip.voice_cleanup_compressor_threshold_db = voice_cleanup_compressor_threshold_db;
            clip.voice_cleanup_compressor_ratio = voice_cleanup_compressor_ratio;
            clip.voice_cleanup_ceiling_linear = voice_cleanup_ceiling_linear;
        });
    }

    /// Sets `selected_clip_id`'s blur strength
    /// ([`avcore::timeline::ClipInstance::blur_intensity`], clamped to [`BLUR_INTENSITY_RANGE`])
    /// — what dragging the properties panel's blur slider does. A no-op if nothing is selected.
    pub fn set_selected_clip_blur(&mut self, blur_intensity: f32) {
        let blur_intensity =
            blur_intensity.clamp(*BLUR_INTENSITY_RANGE.start(), *BLUR_INTENSITY_RANGE.end());
        self.with_selected_clip_mut(|clip| clip.blur_intensity = blur_intensity);
    }

    /// Sets `selected_clip_id`'s camera-shake strength
    /// ([`avcore::timeline::ClipInstance::shake_intensity`], clamped to
    /// [`SHAKE_INTENSITY_RANGE`]) — what dragging the properties panel's shake slider does. A
    /// no-op if nothing is selected.
    pub fn set_selected_clip_shake(&mut self, shake_intensity: f32) {
        let shake_intensity =
            shake_intensity.clamp(*SHAKE_INTENSITY_RANGE.start(), *SHAKE_INTENSITY_RANGE.end());
        self.with_selected_clip_mut(|clip| clip.shake_intensity = shake_intensity);
    }

    /// Sets `selected_clip_id`'s glitch strength
    /// ([`avcore::timeline::ClipInstance::glitch_intensity`], clamped to
    /// [`GLITCH_INTENSITY_RANGE`]) — what dragging the properties panel's glitch slider does. A
    /// no-op if nothing is selected.
    pub fn set_selected_clip_glitch(&mut self, glitch_intensity: f32) {
        let glitch_intensity = glitch_intensity.clamp(
            *GLITCH_INTENSITY_RANGE.start(),
            *GLITCH_INTENSITY_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| clip.glitch_intensity = glitch_intensity);
    }

    /// Sets `selected_clip_id`'s pixelize/mosaic-censor strength
    /// ([`avcore::timeline::ClipInstance::pixelize_intensity`], clamped to
    /// [`PIXELIZE_INTENSITY_RANGE`]) — what dragging the properties panel's pixelize slider
    /// does. A no-op if nothing is selected.
    pub fn set_selected_clip_pixelize(&mut self, pixelize_intensity: f32) {
        let pixelize_intensity = pixelize_intensity.clamp(
            *PIXELIZE_INTENSITY_RANGE.start(),
            *PIXELIZE_INTENSITY_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| clip.pixelize_intensity = pixelize_intensity);
    }

    /// Sets `selected_clip_id`'s transition
    /// ([`avcore::timeline::ClipInstance::transition_in`]/`transition_duration_secs`, the
    /// duration clamped to [`TRANSITION_DURATION_RANGE`]) — what picking a transition type or
    /// dragging the duration slider in the properties panel does. A no-op if nothing is
    /// selected.
    pub fn set_selected_clip_transition(
        &mut self,
        transition_in: TransitionType,
        transition_duration_secs: f32,
    ) {
        let transition_duration_secs = transition_duration_secs.clamp(
            *TRANSITION_DURATION_RANGE.start(),
            *TRANSITION_DURATION_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| {
            clip.transition_in = transition_in;
            clip.transition_duration_secs = transition_duration_secs;
        });
    }

    /// Replaces `selected_clip_id`'s position keyframes
    /// ([`avcore::timeline::ClipInstance::position_keyframes`]) wholesale — the properties
    /// panel clones the current list out, lets the user add/remove/edit rows locally, and
    /// calls this once with the edited list on any change (the same whole-list writeback shape
    /// `TextClip` editing already uses). A no-op if nothing is selected.
    pub fn set_selected_clip_position_keyframes(&mut self, keyframes: Vec<Keyframe<Position>>) {
        self.with_selected_clip_mut(|clip| clip.position_keyframes = keyframes);
    }

    /// Replaces `selected_clip_id`'s scale keyframes
    /// ([`avcore::timeline::ClipInstance::scale_keyframes`]) wholesale — see
    /// [`Self::set_selected_clip_position_keyframes`]. Values are clamped to [`SCALE_RANGE`],
    /// matching the old `zoom_start`/`zoom_end` sliders this field replaces.
    pub fn set_selected_clip_scale_keyframes(&mut self, mut keyframes: Vec<Keyframe<f32>>) {
        for kf in &mut keyframes {
            kf.value = kf.value.clamp(*SCALE_RANGE.start(), *SCALE_RANGE.end());
        }
        self.with_selected_clip_mut(|clip| clip.scale_keyframes = keyframes);
    }

    /// Replaces `selected_clip_id`'s rotation keyframes
    /// ([`avcore::timeline::ClipInstance::rotation_keyframes`]) wholesale — see
    /// [`Self::set_selected_clip_position_keyframes`].
    pub fn set_selected_clip_rotation_keyframes(&mut self, keyframes: Vec<Keyframe<f32>>) {
        self.with_selected_clip_mut(|clip| clip.rotation_keyframes = keyframes);
    }

    /// Replaces `selected_clip_id`'s opacity keyframes
    /// ([`avcore::timeline::ClipInstance::opacity_keyframes`]) wholesale — see
    /// [`Self::set_selected_clip_position_keyframes`]. Values are clamped to `0.0..=1.0`.
    pub fn set_selected_clip_opacity_keyframes(&mut self, mut keyframes: Vec<Keyframe<f32>>) {
        for kf in &mut keyframes {
            kf.value = kf.value.clamp(0.0, 1.0);
        }
        self.with_selected_clip_mut(|clip| clip.opacity_keyframes = keyframes);
    }

    /// Replaces `selected_clip_id`'s audio gain keyframes
    /// ([`avcore::timeline::ClipInstance::gain_keyframes`]) wholesale — see
    /// [`Self::set_selected_clip_position_keyframes`]. Values are clamped to
    /// [`GAIN_DB_RANGE`], matching the constant `gain_db` slider this overrides when non-empty.
    pub fn set_selected_clip_gain_keyframes(&mut self, mut keyframes: Vec<Keyframe<f32>>) {
        for kf in &mut keyframes {
            kf.value = kf.value.clamp(*GAIN_DB_RANGE.start(), *GAIN_DB_RANGE.end());
        }
        self.with_selected_clip_mut(|clip| clip.gain_keyframes = keyframes);
    }

    /// Replaces `selected_clip_id`'s brightness keyframes
    /// ([`avcore::timeline::ClipInstance::brightness_keyframes`]) wholesale — see
    /// [`Self::set_selected_clip_position_keyframes`]. Values are clamped to
    /// [`BRIGHTNESS_RANGE`], matching the constant `brightness` slider this overrides when
    /// non-empty.
    pub fn set_selected_clip_brightness_keyframes(&mut self, mut keyframes: Vec<Keyframe<f32>>) {
        for kf in &mut keyframes {
            kf.value = kf
                .value
                .clamp(*BRIGHTNESS_RANGE.start(), *BRIGHTNESS_RANGE.end());
        }
        self.with_selected_clip_mut(|clip| clip.brightness_keyframes = keyframes);
    }

    /// Replaces `selected_clip_id`'s contrast keyframes
    /// ([`avcore::timeline::ClipInstance::contrast_keyframes`]) wholesale — see
    /// [`Self::set_selected_clip_position_keyframes`]. Values are clamped to
    /// [`CONTRAST_RANGE`].
    pub fn set_selected_clip_contrast_keyframes(&mut self, mut keyframes: Vec<Keyframe<f32>>) {
        for kf in &mut keyframes {
            kf.value = kf
                .value
                .clamp(*CONTRAST_RANGE.start(), *CONTRAST_RANGE.end());
        }
        self.with_selected_clip_mut(|clip| clip.contrast_keyframes = keyframes);
    }

    /// Replaces `selected_clip_id`'s saturation keyframes
    /// ([`avcore::timeline::ClipInstance::saturation_keyframes`]) wholesale — see
    /// [`Self::set_selected_clip_position_keyframes`]. Values are clamped to
    /// [`SATURATION_RANGE`].
    pub fn set_selected_clip_saturation_keyframes(&mut self, mut keyframes: Vec<Keyframe<f32>>) {
        for kf in &mut keyframes {
            kf.value = kf
                .value
                .clamp(*SATURATION_RANGE.start(), *SATURATION_RANGE.end());
        }
        self.with_selected_clip_mut(|clip| clip.saturation_keyframes = keyframes);
    }

    /// Replaces `selected_clip_id`'s crop-x keyframes
    /// ([`avcore::timeline::ClipInstance::crop_x_keyframes`]) wholesale — see
    /// [`Self::set_selected_clip_position_keyframes`]. Values are clamped to `[0.0, 1.0]`,
    /// matching [`Self::set_selected_clip_crop`]'s existing constant clamp.
    pub fn set_selected_clip_crop_x_keyframes(&mut self, mut keyframes: Vec<Keyframe<f32>>) {
        for kf in &mut keyframes {
            kf.value = kf.value.clamp(0.0, 1.0);
        }
        self.with_selected_clip_mut(|clip| clip.crop_x_keyframes = keyframes);
    }

    /// Replaces `selected_clip_id`'s crop-y keyframes
    /// ([`avcore::timeline::ClipInstance::crop_y_keyframes`]) wholesale — see
    /// [`Self::set_selected_clip_crop_x_keyframes`].
    pub fn set_selected_clip_crop_y_keyframes(&mut self, mut keyframes: Vec<Keyframe<f32>>) {
        for kf in &mut keyframes {
            kf.value = kf.value.clamp(0.0, 1.0);
        }
        self.with_selected_clip_mut(|clip| clip.crop_y_keyframes = keyframes);
    }

    /// Replaces `selected_clip_id`'s crop-width keyframes
    /// ([`avcore::timeline::ClipInstance::crop_w_keyframes`]) wholesale — see
    /// [`Self::set_selected_clip_crop_x_keyframes`]. Values are floored at [`CROP_MIN_SIZE`],
    /// matching [`Self::set_selected_clip_crop`]'s existing constant clamp.
    pub fn set_selected_clip_crop_w_keyframes(&mut self, mut keyframes: Vec<Keyframe<f32>>) {
        for kf in &mut keyframes {
            kf.value = kf.value.clamp(CROP_MIN_SIZE, 1.0);
        }
        self.with_selected_clip_mut(|clip| clip.crop_w_keyframes = keyframes);
    }

    /// Replaces `selected_clip_id`'s crop-height keyframes
    /// ([`avcore::timeline::ClipInstance::crop_h_keyframes`]) wholesale — see
    /// [`Self::set_selected_clip_crop_x_keyframes`].
    pub fn set_selected_clip_crop_h_keyframes(&mut self, mut keyframes: Vec<Keyframe<f32>>) {
        for kf in &mut keyframes {
            kf.value = kf.value.clamp(CROP_MIN_SIZE, 1.0);
        }
        self.with_selected_clip_mut(|clip| clip.crop_h_keyframes = keyframes);
    }

    /// Replaces all four crop keyframe lists at once, in a single undo snapshot — what dynamic
    /// auto-reframe applies, as opposed to the four `set_selected_clip_crop_*_keyframes` calls
    /// the properties panel's own keyframe editor makes one at a time.
    pub fn set_selected_clip_crop_keyframes(
        &mut self,
        mut x: Vec<Keyframe<f32>>,
        mut y: Vec<Keyframe<f32>>,
        mut w: Vec<Keyframe<f32>>,
        mut h: Vec<Keyframe<f32>>,
    ) {
        for kf in &mut x {
            kf.value = kf.value.clamp(0.0, 1.0);
        }
        for kf in &mut y {
            kf.value = kf.value.clamp(0.0, 1.0);
        }
        for kf in &mut w {
            kf.value = kf.value.clamp(CROP_MIN_SIZE, 1.0);
        }
        for kf in &mut h {
            kf.value = kf.value.clamp(CROP_MIN_SIZE, 1.0);
        }
        self.with_selected_clip_mut(|clip| {
            clip.crop_x_keyframes = x;
            clip.crop_y_keyframes = y;
            clip.crop_w_keyframes = w;
            clip.crop_h_keyframes = h;
        });
    }

    /// Adds one opacity keyframe at the current timeline playhead position, for the selected
    /// clip — what `Ctrl+O` (`request.md`'s Fase 6 key binding spec, "adicionar marcador de
    /// opacidade") does. The new marker's value is the clip's own current effective opacity at
    /// that instant (`avcore::keyframe::evaluate_keyframes` against a time-sorted copy of the
    /// existing keyframes — the properties panel's own list editor doesn't keep
    /// `opacity_keyframes` sorted as stored, so evaluating needs its own sorted copy), so
    /// placing the marker doesn't itself change how the clip looks; only moving it afterward
    /// does. A no-op if nothing is selected or the playhead isn't within the selected clip's
    /// own timeline span.
    pub fn add_opacity_marker_at_playhead(&mut self) {
        let playhead_secs = self.active_project().timeline().playhead_secs;
        let Some(clip) = self.selected_clip() else {
            return;
        };
        let duration_secs = clip.duration_secs();
        if duration_secs <= 0.0 {
            return;
        }
        let time_fraction = ((playhead_secs - clip.start_secs) / duration_secs) as f32;
        if !(0.0..=1.0).contains(&time_fraction) {
            return;
        }

        let mut sorted = clip.opacity_keyframes.clone();
        sorted.sort_by(|a, b| a.time_fraction.total_cmp(&b.time_fraction));
        let value = avcore::keyframe::evaluate_keyframes(&sorted, time_fraction, 1.0);

        let mut keyframes = clip.opacity_keyframes.clone();
        keyframes.push(Keyframe {
            time_fraction,
            value,
        });
        self.set_selected_clip_opacity_keyframes(keyframes);
    }

    /// Sets `selected_clip_id`'s brightness/contrast/saturation
    /// ([`avcore::timeline::ClipInstance::brightness`]/`contrast`/`saturation`), each
    /// independently clamped to its own range ([`BRIGHTNESS_RANGE`]/[`CONTRAST_RANGE`]/
    /// [`SATURATION_RANGE`]) — what dragging the properties panel's color-adjustment sliders
    /// does. A no-op if nothing is selected.
    pub fn set_selected_clip_color_adjust(
        &mut self,
        brightness: f32,
        contrast: f32,
        saturation: f32,
    ) {
        let brightness = brightness.clamp(*BRIGHTNESS_RANGE.start(), *BRIGHTNESS_RANGE.end());
        let contrast = contrast.clamp(*CONTRAST_RANGE.start(), *CONTRAST_RANGE.end());
        let saturation = saturation.clamp(*SATURATION_RANGE.start(), *SATURATION_RANGE.end());
        self.with_selected_clip_mut(|clip| {
            clip.brightness = brightness;
            clip.contrast = contrast;
            clip.saturation = saturation;
        });
    }
}
