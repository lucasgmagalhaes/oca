use avcore::timeline::{ColorFilter, MaskShape, TransitionType};

use super::{
    OcaApp, BLUR_INTENSITY_RANGE, BRIGHTNESS_RANGE, CHROMA_KEY_TOLERANCE_RANGE, CONTRAST_RANGE,
    CROP_MIN_SIZE, GAIN_DB_RANGE, GLITCH_INTENSITY_RANGE, MASK_CORNER_RADIUS_RANGE,
    PIXELIZE_INTENSITY_RANGE, SATURATION_RANGE, SHAKE_INTENSITY_RANGE, SHARPEN_RANGE,
    SPEED_FACTOR_RANGE, TRANSITION_DURATION_RANGE, VIGNETTE_INTENSITY_RANGE, ZOOM_RANGE,
};

impl OcaApp {
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

    /// Sets `selected_clip_id`'s [`avcore::timeline::ClipInstance::speed_factor`], clamped to
    /// [`SPEED_FACTOR_RANGE`] — what dragging the properties panel's speed slider does. A no-op
    /// if nothing is selected.
    pub fn set_selected_clip_speed(&mut self, speed_factor: f32) {
        let speed_factor =
            speed_factor.clamp(*SPEED_FACTOR_RANGE.start(), *SPEED_FACTOR_RANGE.end());
        self.with_selected_clip_mut(|clip| clip.speed_factor = speed_factor);
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

    /// Sets `selected_clip_id`'s zoom ([`avcore::timeline::ClipInstance::zoom_start`]/
    /// `zoom_end`), each independently clamped to [`ZOOM_RANGE`] — what dragging the properties
    /// panel's zoom sliders does. A no-op if nothing is selected.
    pub fn set_selected_clip_zoom(&mut self, zoom_start: f32, zoom_end: f32) {
        let zoom_start = zoom_start.clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
        let zoom_end = zoom_end.clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
        self.with_selected_clip_mut(|clip| {
            clip.zoom_start = zoom_start;
            clip.zoom_end = zoom_end;
        });
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
