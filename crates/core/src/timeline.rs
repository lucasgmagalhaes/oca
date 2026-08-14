use serde::{Deserialize, Serialize};

use crate::keyframe::{self, Keyframe, Position};

/// What a [`Track`] carries. Determines how the timeline widget renders its clips
/// (thumbnails for video, waveforms for audio) and which asset kind can be dropped onto it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    Video,
    Audio,
    /// A text-overlay track: holds [`TextClip`]s rendered as drawtext overlays on export.
    /// No media assets are placed here — only `text_clips`.
    Text,
}

/// One placed text overlay on a [`Track`] whose [`TrackKind`] is [`TrackKind::Text`].
/// Rendered into the exported video via the `drawtext` avfilter in a post-processing pass
/// after the main timeline encode — see `avbridge::apply_text_overlays`.
///
/// Preview is not yet implemented — see the TODO in `core::preview`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextClip {
    pub id: u64,
    /// Start time on the timeline, in seconds.
    pub start_secs: f64,
    /// How long the text stays visible, in seconds.
    pub duration_secs: f64,
    /// The text string to render.
    pub text: String,
    /// Font size in points.
    pub font_size: f32,
    /// RGBA color: `[r, g, b, a]`, each 0–255. Alpha 255 = fully opaque.
    pub color_rgba: [u8; 4],
    /// Horizontal anchor as a 0.0–1.0 fraction of the canvas width (0.0 = left edge).
    pub pos_x: f32,
    /// Vertical anchor as a 0.0–1.0 fraction of the canvas height (0.0 = top edge).
    pub pos_y: f32,
    /// Per-word timestamps within this clip's own text, `start_secs`/`end_secs` relative to
    /// this clip's *own* start (not the timeline) — per `request.md`'s Fase 4 "Legenda com
    /// destaque de palavra (estilo shorts)". Populated when this clip was generated from
    /// [`crate::transcribe::transcribe`]'s word-level output (`ui`'s "Transcrever" flow); empty
    /// for a manually-typed text block, which has no per-word timing to highlight against.
    /// `#[serde(default)]` so older saved projects load with no word highlighting.
    #[serde(default)]
    pub words: Vec<WordTiming>,
    /// Whether [`TextClip::words`] should be rendered as in-place word highlighting
    /// (`resolve_text_segments` emits one highlighted overlay per word, on top of the base
    /// text) rather than as plain static text. Meaningless while `words` is empty.
    /// `#[serde(default)]` so older saved projects load with highlighting off.
    #[serde(default)]
    pub highlight_enabled: bool,
    /// Color a word is drawn in while it's the one being spoken, `[r, g, b, a]` — meaningless
    /// while `highlight_enabled` is `false`. `#[serde(default = ..)]` so older saved projects
    /// load at a reasonable default (bright yellow, matching the popular shorts-caption look)
    /// rather than an invisible/transparent black.
    #[serde(default = "default_highlight_color")]
    pub highlight_color_rgba: [u8; 4],
}

/// One word within a [`TextClip`]'s [`TextClip::words`] — see
/// [`crate::transcribe::TranscribeWord`], which this mirrors (kept as a separate type since
/// `core::transcribe`'s output is a transient transcription result, not project-saved state).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordTiming {
    pub text: String,
    /// Relative to the owning [`TextClip::start_secs`], not the timeline.
    pub start_secs: f64,
    /// Relative to the owning [`TextClip::start_secs`], not the timeline.
    pub end_secs: f64,
}

fn default_highlight_color() -> [u8; 4] {
    [255, 220, 0, 255]
}

/// Layer mask shape for a block, per `request.md`'s Fase 4 "Máscaras" spec — clips a layer to a
/// shape instead of the plain rectangular crop ([`ClipInstance::crop_x`] etc.), e.g. for webcam
/// frames. `Custom` (a user-drawn shape) isn't supported yet — only the two fixed shapes the
/// spec names alongside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MaskShape {
    #[default]
    None,
    Circle,
    RoundedRect,
}

/// Color filter for a block, per `request.md`'s Fase 4 "Efeitos visuais" spec ("Preto e branco
/// e sépia"). A bounded subset of the eventual "Filtros de cor e LUTs" library — just these two
/// fixed looks, no adjustable LUT yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColorFilter {
    #[default]
    None,
    BlackAndWhite,
    Sepia,
}

/// Transition style for a block's incoming edge, per `request.md`'s Fase 4 "Efeitos visuais"
/// spec ("Transições entre clipes (fade, corte seco, slide, zoom)"). `HardCut` is the spec's
/// "corte seco" spelled out as an explicit choice, distinct from `None` meaning "no transition
/// configured yet" — both currently render identically (nothing renders either), but they mean
/// different things to the user's edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TransitionType {
    #[default]
    None,
    Fade,
    HardCut,
    Slide,
    Zoom,
}

impl TransitionType {
    /// Maps this transition to the integer code used in [`avbridge::ClipSegment::transition_in`]
    /// and the `ClipSegment.transition_in` C field. `None` and `HardCut` both produce `0`
    /// (no-op filter), since both mean an instant cut at the C level.
    pub fn to_export_code(self) -> u8 {
        match self {
            Self::None | Self::HardCut => 0,
            Self::Fade => 1,
            Self::Slide => 2,
            Self::Zoom => 3,
        }
    }
}

/// One placed instance of a `MediaAsset` on the timeline. `source_in_secs`/`source_out_secs`
/// mark the trimmed range within the source asset; `start_secs` is its position on the track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipInstance {
    pub id: u64,
    pub asset_id: u64,
    pub start_secs: f64,
    pub source_in_secs: f64,
    pub source_out_secs: f64,
    /// `Some(group_id)` if this clip is a member of a composite block (per `request.md`'s
    /// Fase 3 "blocos compostos" spec) — every clip sharing the same id, always on the same
    /// track (composite blocks don't span tracks yet), moves/splits/deletes together as a
    /// unit (see `ui`'s `App::merge_into_composite` and the timeline panel's drag/delete
    /// handling). `#[serde(default)]` so a project saved before this field existed still
    /// loads, every clip in it just standalone (`None`).
    #[serde(default)]
    pub composite_id: Option<u64>,
    /// Volume adjustment in decibels applied to this block's audio, independent of every other
    /// clip — per `request.md`'s Fase 4 "ganho de volume por bloco" spec. `0.0` is unity gain.
    /// Currently only feeds the timeline waveform display (`ui`'s `draw_waveform`, scaled by
    /// [`ClipInstance::gain_linear`]); export doesn't mix the timeline yet (it still
    /// passthrough-renders a single source file per job, see `core::render`), so this doesn't
    /// affect exported audio yet. `#[serde(default)]` so older saved projects load at unity
    /// gain.
    #[serde(default)]
    pub gain_db: f32,
    /// `true` if this block is frozen — holds a single still frame
    /// ([`ClipInstance::source_in_secs`]) for its whole displayed duration instead of playing
    /// through the trimmed source range, per `request.md`'s Fase 4 "Congelar" spec ("segura um
    /// quadro específico por uma duração configurável"). The held frame is picked by trimming
    /// `source_in_secs` to it; the hold duration is just the block's existing on-timeline
    /// length ([`ClipInstance::duration_secs`]), adjustable the same way as any other clip via
    /// the existing trim handles — freeze doesn't need its own duration field. Changes how the
    /// timeline draws the block (`ui`'s timeline panel shows a single repeated poster frame
    /// instead of a filmstrip, per-position thumbnails) and is now wired into export — resolved
    /// straight to [`avbridge::ClipSegment::frozen`] (`crate::render::resolve_timeline_segments`)
    /// rather than through [`ClipInstance::video_filter_chain`], since holding a frame needs the
    /// segment's own decode loop to synthesize duplicate frames, not a static avfilter string.
    /// Audio is unaffected — a frozen block's audio still plays across its full trimmed range.
    /// Still doesn't affect preview playback (`avcore::preview::Preview` always plays the real
    /// decoded source) — the same kind of preview gap as [`ClipInstance::gain_db`].
    /// `#[serde(default)]` so older saved projects load unfrozen.
    #[serde(default)]
    pub frozen: bool,
    /// Playback speed multiplier for this block — `2.0` plays twice as fast, `0.5` half speed,
    /// per `request.md`'s Fase 4 "Velocidade" spec. `1.0` is normal speed. Currently only shown
    /// as a badge on the timeline block (`ui`'s timeline panel); it doesn't yet resample audio,
    /// change the block's on-timeline duration, or affect preview playback or export — the same
    /// kind of gap as [`ClipInstance::gain_db`]/[`ClipInstance::frozen`], just earlier: speed
    /// needs the timeline to support a block whose on-screen length differs from
    /// `source_out_secs - source_in_secs`, which nothing here does yet. `#[serde(default = ..)]`
    /// so older saved projects load at normal speed.
    #[serde(default = "default_speed_factor")]
    pub speed_factor: f32,
    /// Normalized crop rectangle within the source frame — `(crop_x, crop_y)` is the visible
    /// sub-rectangle's top-left corner, `(crop_w, crop_h)` its size, all fractions of the full
    /// frame (`0.0..=1.0`). Defaults to `(0.0, 0.0, 1.0, 1.0)` — the whole frame, uncropped —
    /// per `request.md`'s Fase 4 "Recorte (crop)" spec: reframing separate from the time-based
    /// split already covered in Fase 3. Currently only shown as a badge on the timeline block
    /// (`ui`'s timeline panel, via [`ClipInstance::is_cropped`]); doesn't yet affect preview
    /// playback or export — the same kind of gap as [`ClipInstance::gain_db`]. Independently
    /// clamped to `[0.0, 1.0]` when set (`ui`'s `App::set_selected_clip_crop`); a crop rect
    /// extending past the frame edge (`crop_x + crop_w > 1.0`) isn't rejected — a known
    /// simplification with no visible effect yet since nothing renders the crop.
    /// `#[serde(default = ..)]` so older saved projects load uncropped.
    #[serde(default)]
    pub crop_x: f32,
    #[serde(default)]
    pub crop_y: f32,
    #[serde(default = "default_crop_extent")]
    pub crop_w: f32,
    #[serde(default = "default_crop_extent")]
    pub crop_h: f32,
    /// Layer mask shape ([`MaskShape::None`] by default — unmasked). Independent of the
    /// rectangular crop above; a block can be both cropped and masked. `#[serde(default)]` so
    /// older saved projects load unmasked.
    #[serde(default)]
    pub mask_shape: MaskShape,
    /// Corner radius for [`MaskShape::RoundedRect`], as a fraction (`0.0..=1.0`) of the block's
    /// shorter frame dimension — meaningless for the other shapes. `#[serde(default)]` so older
    /// saved projects load at `0.0` (square corners).
    #[serde(default)]
    pub mask_corner_radius: f32,
    /// `true` if this block's frame is mirrored horizontally, per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Espelhar (flip horizontal)"). Currently only shown as a badge
    /// on the timeline block (`ui`'s timeline panel); doesn't yet affect preview playback or
    /// export — the same kind of gap as [`ClipInstance::gain_db`]. `#[serde(default)]` so
    /// older saved projects load unflipped.
    #[serde(default)]
    pub flipped_h: bool,
    /// Color filter applied to this block ([`ColorFilter::None`] by default). Currently only
    /// shown as a tinted timeline-block fill (`ui`'s timeline panel); doesn't yet affect
    /// preview playback or export — the same kind of gap as [`ClipInstance::gain_db`].
    /// `#[serde(default)]` so older saved projects load unfiltered.
    #[serde(default)]
    pub color_filter: ColorFilter,
    /// Vignette strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Vinheta"). Currently only shown as a darkened border stroke
    /// around the timeline block, scaled by intensity (`ui`'s timeline panel); doesn't yet
    /// affect preview playback or export — the same kind of gap as [`ClipInstance::gain_db`].
    /// `#[serde(default)]` so older saved projects load with no vignette.
    #[serde(default)]
    pub vignette_intensity: f32,
    /// Brightness adjustment for this block, `-1.0..=1.0` (`0.0` is unchanged) — per
    /// `request.md`'s Fase 4 "Efeitos visuais" spec ("Brilho, contraste e saturação").
    /// Currently has no visible effect anywhere (`ui`'s properties panel just exposes the
    /// slider); doesn't yet affect preview playback or export — the same kind of gap as
    /// [`ClipInstance::gain_db`]. `#[serde(default)]` so older saved projects load unchanged.
    #[serde(default)]
    pub brightness: f32,
    /// Contrast multiplier for this block, `0.0..=2.0` (`1.0` is unchanged) — same spec and gap
    /// as [`ClipInstance::brightness`]. `#[serde(default = ..)]` so older saved projects load
    /// unchanged.
    #[serde(default = "default_unity_multiplier")]
    pub contrast: f32,
    /// Saturation multiplier for this block, `0.0..=2.0` (`1.0` is unchanged, `0.0` is
    /// grayscale) — same spec and gap as [`ClipInstance::brightness`]. `#[serde(default = ..)]`
    /// so older saved projects load unchanged.
    #[serde(default = "default_unity_multiplier")]
    pub saturation: f32,
    /// Sharpen strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Nitidez (sharpen)"). Currently has no visible effect anywhere
    /// (`ui`'s properties panel just exposes the slider); doesn't yet affect preview playback or
    /// export — the same kind of gap as [`ClipInstance::gain_db`]. `#[serde(default)]` so older
    /// saved projects load unsharpened.
    #[serde(default)]
    pub sharpen: f32,
    /// `true` if chroma key (green-screen removal) is enabled for this block, per
    /// `request.md`'s Fase 4 "Efeitos visuais" spec ("Chroma key"). Currently has no visible
    /// effect anywhere (`ui`'s properties panel just exposes the toggle/color/tolerance
    /// controls); doesn't yet affect preview playback or export — the same kind of gap as
    /// [`ClipInstance::gain_db`]. `#[serde(default)]` so older saved projects load disabled.
    #[serde(default)]
    pub chroma_key_enabled: bool,
    /// The key color to remove, as `[r, g, b]` (`0..=255` each) — meaningless while
    /// [`ClipInstance::chroma_key_enabled`] is `false`. `#[serde(default = ..)]` so older saved
    /// projects load at the conventional chroma-green `#00FF00`.
    #[serde(default = "default_chroma_key_color")]
    pub chroma_key_color: [u8; 3],
    /// How close a pixel's color must be to [`ClipInstance::chroma_key_color`] to be keyed out,
    /// `0.0..=1.0` (`0.0` is exact-match-only, `1.0` keys everything) — meaningless while
    /// `chroma_key_enabled` is `false`. `#[serde(default = ..)]` so older saved projects load at
    /// a reasonable default tolerance.
    #[serde(default = "default_chroma_key_tolerance")]
    pub chroma_key_tolerance: f32,
    /// Blur strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Blur"). Currently has no visible effect anywhere (`ui`'s
    /// properties panel just exposes the slider); doesn't yet affect preview playback or export
    /// — the same kind of gap as [`ClipInstance::gain_db`]. `#[serde(default)]` so older saved
    /// projects load unblurred.
    #[serde(default)]
    pub blur_intensity: f32,
    /// Camera-shake strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s
    /// Fase 4 "Efeitos visuais" spec ("Shake"), the deliberate counterpart of the eventual video
    /// stabilization feature. Same gap as [`ClipInstance::blur_intensity`]. `#[serde(default)]`
    /// so older saved projects load unshaken.
    #[serde(default)]
    pub shake_intensity: f32,
    /// Glitch strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Glitch"). Same gap as [`ClipInstance::blur_intensity`].
    /// `#[serde(default)]` so older saved projects load unglitched.
    #[serde(default)]
    pub glitch_intensity: f32,
    /// Pixelize/mosaic-censor strength for this block, `0.0..=1.0` (`0.0` is off) — per
    /// `request.md`'s Fase 4 "Efeitos visuais" spec ("Pixelizar/censura (mosaico)"). Same gap as
    /// [`ClipInstance::blur_intensity`]. `#[serde(default)]` so older saved projects load
    /// unpixelized.
    #[serde(default)]
    pub pixelize_intensity: f32,
    /// Transition style for this block's incoming edge ([`TransitionType::None`] by default) —
    /// per `request.md`'s Fase 4 "Efeitos visuais" spec ("Transições entre clipes"). Models
    /// only the transition entering this clip, not a real cross-blend between two adjacent
    /// clips — that would need a relationship between this clip and the one before it, not a
    /// field on a single `ClipInstance`. A deliberately smaller first cut, same shape as the
    /// rest of this struct's effect fields. Currently has no visible effect anywhere (`ui`'s
    /// properties panel just exposes the picker); doesn't yet affect preview playback or export
    /// — the same kind of gap as [`ClipInstance::gain_db`]. `#[serde(default)]` so older saved
    /// projects load with no transition.
    #[serde(default)]
    pub transition_in: TransitionType,
    /// Duration in seconds of [`ClipInstance::transition_in`], meaningless while it's
    /// `TransitionType::None`. `#[serde(default = ..)]` so older saved projects load at a
    /// reasonable default duration.
    #[serde(default = "default_transition_duration")]
    pub transition_duration_secs: f32,
    /// General keyframe animation for this block's position (translate offset, normalized as a
    /// fraction of canvas width/height), per `features/request.md`'s Fase 4 "Keyframes" spec.
    /// Empty = no offset. Wired into export (`crate::keyframe::position_overlay_xy_expr`) —
    /// only has a visible effect on an overlay-track clip, since a single/background track has
    /// no compositing stage to translate into (same caveat as `chroma_key_enabled`'s alpha).
    /// Not yet wired into live preview. `#[serde(default)]` so older saved projects (or a
    /// project saved before this field existed) load with no position animation.
    #[serde(default)]
    pub position_keyframes: Vec<Keyframe<Position>>,
    /// General keyframe animation for this block's scale, per `features/request.md`'s Fase 4
    /// "Keyframes" spec — supersedes the old two-endpoint `zoom_start`/`zoom_end` Ken-Burns
    /// fields (a 2-keyframe list reproduces that same behavior as a degenerate case). Empty =
    /// no scaling (`1.0`). Wired into export (`crate::keyframe::scale_filter_expr`); not yet
    /// wired into live preview. `#[serde(default)]` so older saved projects load unscaled — a
    /// project that had real `zoom_start`/`zoom_end` values loses that animation on load, since
    /// this field replaces rather than migrates it (no back-compat promised for this format).
    #[serde(default)]
    pub scale_keyframes: Vec<Keyframe<f32>>,
    /// General keyframe animation for this block's rotation, in degrees, per
    /// `features/request.md`'s Fase 4 "Keyframes" spec. Empty = no rotation (`0.0`). Wired into
    /// export (`crate::keyframe::rotation_filter_angle_expr`); not yet wired into live preview.
    /// `#[serde(default)]` so older saved projects load unrotated.
    #[serde(default)]
    pub rotation_keyframes: Vec<Keyframe<f32>>,
    /// General keyframe animation for this block's opacity, `0.0..=1.0`, per
    /// `features/request.md`'s Fase 4 "Keyframes" spec. Empty = fully opaque (`1.0`). Wired into
    /// export (`crate::keyframe::opacity_alpha_ramp_expr`) — only has a visible effect on an
    /// overlay-track clip, same caveat as position above. Not yet wired into live preview.
    /// `#[serde(default)]` so older saved projects load fully opaque.
    #[serde(default)]
    pub opacity_keyframes: Vec<Keyframe<f32>>,
    /// `true` if temporal luminance-flicker removal is enabled for this block, per
    /// `request.md`'s Fase 4 "Efeitos visuais" spec ("Remoção de flicker") — common in
    /// screen/gameplay captures at certain refresh rates. Wired to export via `video_filter_chain`
    /// (`deflicker`); no equivalent stage in `core::preview`'s `build_video_filter_bin` yet, the
    /// same preview gap several other effects here have. `#[serde(default)]` so older saved
    /// projects load with it off.
    #[serde(default)]
    pub deflicker_enabled: bool,
}

/// The rendering/display settings of a [`ClipInstance`] that can be copied onto a different
/// block without touching its structural fields (`id`, `asset_id`, start/trim, composite
/// membership). Used by `ui`'s "copiar formatação" feature (`Ctrl+Shift+C`/`V`).
#[derive(Debug, Clone, PartialEq)]
pub struct ClipFormatting {
    pub gain_db: f32,
    pub frozen: bool,
    pub speed_factor: f32,
    pub crop_x: f32,
    pub crop_y: f32,
    pub crop_w: f32,
    pub crop_h: f32,
    pub mask_shape: MaskShape,
    pub mask_corner_radius: f32,
    pub flipped_h: bool,
    pub color_filter: ColorFilter,
    pub vignette_intensity: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub sharpen: f32,
    pub chroma_key_enabled: bool,
    pub chroma_key_color: [u8; 3],
    pub chroma_key_tolerance: f32,
    pub blur_intensity: f32,
    pub shake_intensity: f32,
    pub glitch_intensity: f32,
    pub pixelize_intensity: f32,
    pub transition_in: TransitionType,
    pub transition_duration_secs: f32,
    pub position_keyframes: Vec<Keyframe<Position>>,
    pub scale_keyframes: Vec<Keyframe<f32>>,
    pub rotation_keyframes: Vec<Keyframe<f32>>,
    pub opacity_keyframes: Vec<Keyframe<f32>>,
    pub deflicker_enabled: bool,
}

fn default_speed_factor() -> f32 {
    1.0
}

fn default_transition_duration() -> f32 {
    0.5
}

fn default_crop_extent() -> f32 {
    1.0
}

fn default_unity_multiplier() -> f32 {
    1.0
}

fn default_chroma_key_color() -> [u8; 3] {
    [0, 255, 0]
}

fn default_chroma_key_tolerance() -> f32 {
    0.4
}

impl ClipInstance {
    /// How long this instance plays for, i.e. its trimmed length — not the source asset's
    /// full duration.
    pub fn duration_secs(&self) -> f64 {
        (self.source_out_secs - self.source_in_secs) / self.speed_factor as f64
    }

    /// Linear amplitude multiplier for [`ClipInstance::gain_db`] — e.g. `+6.0` dB roughly
    /// doubles amplitude, `-6.0` dB roughly halves it. `0.0` dB gives `1.0` (unity).
    pub fn gain_linear(&self) -> f32 {
        10f32.powf(self.gain_db / 20.0)
    }

    /// `true` if the crop rect isn't the full, uncropped frame.
    pub fn is_cropped(&self) -> bool {
        self.crop_x != 0.0 || self.crop_y != 0.0 || self.crop_w != 1.0 || self.crop_h != 1.0
    }

    /// `true` if a layer mask ([`ClipInstance::mask_shape`]) is applied.
    pub fn is_masked(&self) -> bool {
        self.mask_shape != MaskShape::None
    }

    /// `true` if a color filter ([`ClipInstance::color_filter`]) is applied.
    pub fn is_color_filtered(&self) -> bool {
        self.color_filter != ColorFilter::None
    }

    /// `true` if [`ClipInstance::vignette_intensity`] is above zero.
    pub fn has_vignette(&self) -> bool {
        self.vignette_intensity > 0.0
    }

    /// `true` if chroma key ([`ClipInstance::chroma_key_enabled`]) is on.
    pub fn is_chroma_keyed(&self) -> bool {
        self.chroma_key_enabled
    }

    /// `true` if a transition ([`ClipInstance::transition_in`]) is configured on this block's
    /// incoming edge.
    pub fn has_transition(&self) -> bool {
        self.transition_in != TransitionType::None
    }

    /// `true` if this block has any position keyframes.
    pub fn has_position_keyframes(&self) -> bool {
        !self.position_keyframes.is_empty()
    }

    /// `true` if this block has any scale keyframes (the general-keyframe replacement for the
    /// old `is_zoomed`).
    pub fn has_scale_keyframes(&self) -> bool {
        !self.scale_keyframes.is_empty()
    }

    /// `true` if this block has any rotation keyframes.
    pub fn has_rotation_keyframes(&self) -> bool {
        !self.rotation_keyframes.is_empty()
    }

    /// `true` if this block has any opacity keyframes.
    pub fn has_opacity_keyframes(&self) -> bool {
        !self.opacity_keyframes.is_empty()
    }

    /// Builds this clip's scale/rotation/opacity keyframe avfilter fragment, spliced into the
    /// per-clip chain before [`ClipInstance::video_filter_chain`]'s own stages — the same
    /// position the old `zoom` stage used to occupy. `None` if none of the three are animated.
    /// Position keyframes aren't part of this — they apply to the *overlay* compositing stage,
    /// not a per-clip filter (see [`keyframe::position_overlay_xy_expr`] and `crate::render`).
    pub fn keyframe_video_filter_chain(
        &self,
        fps_num: u32,
        fps_den: u32,
        timeline_duration_secs: f64,
    ) -> Option<String> {
        let mut stages = Vec::new();
        if let Some(scale) = keyframe::scale_filter_expr(
            &self.scale_keyframes,
            fps_num,
            fps_den,
            timeline_duration_secs,
        ) {
            stages.push(scale);
        }
        if let Some(angle_expr) =
            keyframe::rotation_filter_angle_expr(&self.rotation_keyframes, timeline_duration_secs)
        {
            stages.push(format!(
                "rotate=angle='{angle_expr}':ow=rotw('{angle_expr}'):oh=roth('{angle_expr}')"
            ));
        }
        if let Some(alpha_expr) = keyframe::opacity_alpha_ramp_expr(
            &self.opacity_keyframes,
            fps_num,
            fps_den,
            timeline_duration_secs,
        ) {
            stages.push(format!(
                "format=yuva420p,geq=lum='p(X,Y)':cb='cb(X,Y)':cr='cr(X,Y)':a='alpha(X,Y)*({alpha_expr})'"
            ));
        }
        if stages.is_empty() {
            None
        } else {
            Some(stages.join(","))
        }
    }

    /// Builds this clip's avfilter chain description for `core::render::render_timeline_export`
    /// — the subset of effect fields expressible as a static per-clip video filter (see
    /// `features/request.md`'s Fase 4 "Efeitos visuais" list): crop, deflicker, brightness/
    /// contrast/saturation, the black-and-white/sepia color filter, chroma key, mask shape,
    /// blur, sharpen, pixelize, shake, glitch, vignette, and horizontal flip. `gain_db` is audio, not
    /// video, and isn't part of this chain. `speed_factor` and `zoom_start`/`zoom_end` are
    /// handled in `bridge.c` (not here). `mask_shape`'s alpha only survives to the rendered
    /// output on an overlay track — see the caveat on its stage below. `transition_in` needs a
    /// materially different mechanism (cross-clip blending) and isn't covered here yet.
    /// `frozen` also needs a different mechanism (frame duplication) but is covered elsewhere —
    /// see [`ClipInstance::frozen`]'s doc.
    ///
    /// Returns `""` (no-op) if none of the covered effects deviate from neutral. Filters are
    /// comma-joined in a fixed order — crop first (so later filters see the cropped frame,
    /// not un-cropped coordinates), flip last (so it doesn't mirror crop/vignette geometry) —
    /// which is a judgment call, not something derivable from the field values themselves.
    pub fn video_filter_chain(&self) -> String {
        let mut stages = Vec::new();

        if self.is_cropped() {
            stages.push(format!(
                "crop=iw*{}:ih*{}:iw*{}:ih*{}",
                self.crop_w, self.crop_h, self.crop_x, self.crop_y
            ));
        }
        if self.deflicker_enabled {
            // Arithmetic-mean mode over a 5-frame temporal window (both FFmpeg's own
            // defaults) — smooths out frame-to-frame luminance variation from screen/gameplay
            // capture at certain refresh rates, before any other stage reshapes that luminance.
            stages.push("deflicker=mode=am:size=5".to_string());
        }
        if self.brightness != 0.0 || self.contrast != 1.0 || self.saturation != 1.0 {
            stages.push(format!(
                "eq=brightness={}:contrast={}:saturation={}",
                self.brightness, self.contrast, self.saturation
            ));
        }
        match self.color_filter {
            ColorFilter::None => {}
            ColorFilter::BlackAndWhite => stages.push("hue=s=0".to_string()),
            ColorFilter::Sepia => stages.push(
                "colorchannelmixer=.393:.769:.189:0:.349:.686:.168:0:.272:.534:.131:0".to_string(),
            ),
        }
        if self.is_chroma_keyed() {
            let [r, g, b] = self.chroma_key_color;
            stages.push(format!(
                "colorkey=0x{r:02x}{g:02x}{b:02x}:{:.3}:0.1",
                self.chroma_key_tolerance
            ));
        }
        match self.mask_shape {
            MaskShape::None => {}
            MaskShape::Circle | MaskShape::RoundedRect => {
                // Promotes to an alpha-having pixel format, then geq's per-pixel expression
                // clips it to the mask shape by zeroing alpha() outside it — multiplying by
                // (rather than overwriting) the existing alpha(X,Y) so a chroma-keyed clip's
                // own transparency composes with the mask instead of being clobbered by it.
                // Note the same caveat as chroma_key's below: the final `format=yuv420p`
                // conform on a single (non-overlay) track drops this alpha again — the mask
                // only has a visible effect on a clip placed on an overlay track (see
                // `build_overlay_vfilter` in timeline_export_multi.c, which has no such conform between
                // its two per-track chains and the `overlay` filter that composites them).
                //
                // avfilter's filtergraph-level parser only treats a comma as a stage separator
                // outside quotes — every comma below sits inside the geq option's own `'...'`
                // quoting, so `pow`/`min`/`max`/`lte`'s comma-separated arguments are safe
                // (verified against a real ffmpeg build, not just read off the docs).
                let minwh = "min(W,H)";
                let alpha_expr = match self.mask_shape {
                    MaskShape::Circle => {
                        format!("alpha(X,Y)*lte(pow(X-W/2,2)+pow(Y-H/2,2),pow({minwh}/2,2))")
                    }
                    MaskShape::RoundedRect => {
                        // Rounded-rect signed-distance field: shrink the half-extents by the
                        // corner radius, measure how far outside that inner rect (X,Y) falls,
                        // then subtract the radius back out — <=0 is inside the rounded shape.
                        let radius = format!(
                            "min({:.4}*{minwh},{minwh}/2)",
                            self.mask_corner_radius.clamp(0.0, 1.0)
                        );
                        format!(
                            "alpha(X,Y)*lte(sqrt(pow(max(abs(X-W/2)-(W/2-{radius}),0),2)+pow(max(abs(Y-H/2)-(H/2-{radius}),0),2))-{radius},0)"
                        )
                    }
                    MaskShape::None => unreachable!("outer match already excludes None"),
                };
                stages.push(format!(
                    "format=yuva420p,geq=lum='p(X,Y)':cb='cb(X,Y)':cr='cr(X,Y)':a='{alpha_expr}'"
                ));
            }
        }
        if self.blur_intensity > 0.0 {
            stages.push(format!("boxblur={:.2}", self.blur_intensity * 10.0));
        }
        if self.sharpen > 0.0 {
            stages.push(format!("unsharp=5:5:{:.2}:5:5:0.0", self.sharpen * 3.0));
        }
        if self.pixelize_intensity > 0.0 {
            // Scale down to block_size-pixel grid then back up — nearest-neighbor gives the
            // hard mosaic look. block ranges from 2px (subtle) to 50px (heavy censorship).
            let block = (2.0 + self.pixelize_intensity * 48.0).round() as u32;
            stages.push(format!(
                "scale=iw/{b}:ih/{b}:flags=neighbor,scale=iw*{b}:ih*{b}:flags=neighbor",
                b = block
            ));
        }
        if self.shake_intensity > 0.0 {
            // Crop away a margin on all sides (giving room to "shake" into), oscillate the crop
            // origin with two independent sinusoids, then scale back to the original frame size.
            // margin at full intensity: 8% of each dimension per side.
            let margin = self.shake_intensity * 0.08_f32;
            let keep = 1.0_f32 - 2.0 * margin;
            let scale_back = 1.0 / keep;
            stages.push(format!(
                "crop=iw*{keep:.4}:ih*{keep:.4}:iw*{m:.4}*(1+sin(n*0.31)):ih*{m:.4}*(1+cos(n*0.23)),scale=iw*{sb:.4}:ih*{sb:.4}",
                m = margin,
                sb = scale_back,
            ));
        }
        if self.glitch_intensity > 0.0 {
            // Temporal luma + chroma noise approximates digital glitch corruption.
            let ls = (self.glitch_intensity * 60.0).round() as u32;
            let cs = (self.glitch_intensity * 25.0).round() as u32;
            stages.push(format!(
                "noise=c0s={ls}:c0f=t:c1s={cs}:c1f=t:c2s={cs}:c2f=t"
            ));
        }
        if self.has_vignette() {
            stages.push(format!("vignette=PI/4*{:.3}", self.vignette_intensity));
        }
        if self.flipped_h {
            stages.push("hflip".to_string());
        }

        stages.join(",")
    }

    /// True if `at_secs` (timeline-relative) falls strictly inside this clip's placed range.
    /// Boundary-exact positions return `false` — splitting exactly on an edge would just
    /// produce a zero-length half.
    pub fn contains(&self, at_secs: f64) -> bool {
        at_secs > self.start_secs && at_secs < self.start_secs + self.duration_secs()
    }

    /// Drags the clip's left edge to `new_start_secs`, keeping its end point
    /// (`source_out_secs`) fixed — `source_in_secs` shifts by the same delta as `start_secs`,
    /// since trimming the start plays a later point in the source. No-op (`false`) if that
    /// would put `new_start_secs` or the resulting `source_in_secs` below zero, or shrink the
    /// clip below `min_duration_secs`.
    pub fn trim_start(&mut self, new_start_secs: f64, min_duration_secs: f64) -> bool {
        let timeline_delta = new_start_secs - self.start_secs;
        let new_source_in_secs = self.source_in_secs + timeline_delta * self.speed_factor as f64;
        let new_source_duration = self.source_out_secs - new_source_in_secs;
        let new_timeline_duration = new_source_duration / self.speed_factor as f64;
        if new_start_secs < 0.0
            || new_source_in_secs < 0.0
            || new_timeline_duration < min_duration_secs
        {
            return false;
        }
        self.start_secs = new_start_secs;
        self.source_in_secs = new_source_in_secs;
        true
    }

    /// Snapshots every rendering/display field as a [`ClipFormatting`] value — used by
    /// `ui`'s "copiar formatação" feature to copy settings that can be pasted onto a
    /// different block without duplicating the clip itself.
    pub fn formatting(&self) -> ClipFormatting {
        ClipFormatting {
            gain_db: self.gain_db,
            frozen: self.frozen,
            speed_factor: self.speed_factor,
            crop_x: self.crop_x,
            crop_y: self.crop_y,
            crop_w: self.crop_w,
            crop_h: self.crop_h,
            mask_shape: self.mask_shape,
            mask_corner_radius: self.mask_corner_radius,
            flipped_h: self.flipped_h,
            color_filter: self.color_filter,
            vignette_intensity: self.vignette_intensity,
            brightness: self.brightness,
            contrast: self.contrast,
            saturation: self.saturation,
            sharpen: self.sharpen,
            chroma_key_enabled: self.chroma_key_enabled,
            chroma_key_color: self.chroma_key_color,
            chroma_key_tolerance: self.chroma_key_tolerance,
            blur_intensity: self.blur_intensity,
            shake_intensity: self.shake_intensity,
            glitch_intensity: self.glitch_intensity,
            pixelize_intensity: self.pixelize_intensity,
            transition_in: self.transition_in,
            transition_duration_secs: self.transition_duration_secs,
            position_keyframes: self.position_keyframes.clone(),
            scale_keyframes: self.scale_keyframes.clone(),
            rotation_keyframes: self.rotation_keyframes.clone(),
            opacity_keyframes: self.opacity_keyframes.clone(),
            deflicker_enabled: self.deflicker_enabled,
        }
    }

    /// Applies every field from `f` onto this clip, leaving structural fields (`id`,
    /// `asset_id`, `start_secs`, `source_in_secs`/`source_out_secs`, `composite_id`)
    /// unchanged — the "colar formatação" counterpart to [`Self::formatting`].
    pub fn apply_formatting(&mut self, f: &ClipFormatting) {
        self.gain_db = f.gain_db;
        self.frozen = f.frozen;
        self.speed_factor = f.speed_factor;
        self.crop_x = f.crop_x;
        self.crop_y = f.crop_y;
        self.crop_w = f.crop_w;
        self.crop_h = f.crop_h;
        self.mask_shape = f.mask_shape;
        self.mask_corner_radius = f.mask_corner_radius;
        self.flipped_h = f.flipped_h;
        self.color_filter = f.color_filter;
        self.vignette_intensity = f.vignette_intensity;
        self.brightness = f.brightness;
        self.contrast = f.contrast;
        self.saturation = f.saturation;
        self.sharpen = f.sharpen;
        self.chroma_key_enabled = f.chroma_key_enabled;
        self.chroma_key_color = f.chroma_key_color;
        self.chroma_key_tolerance = f.chroma_key_tolerance;
        self.blur_intensity = f.blur_intensity;
        self.shake_intensity = f.shake_intensity;
        self.glitch_intensity = f.glitch_intensity;
        self.pixelize_intensity = f.pixelize_intensity;
        self.transition_in = f.transition_in;
        self.transition_duration_secs = f.transition_duration_secs;
        self.position_keyframes = f.position_keyframes.clone();
        self.scale_keyframes = f.scale_keyframes.clone();
        self.rotation_keyframes = f.rotation_keyframes.clone();
        self.opacity_keyframes = f.opacity_keyframes.clone();
        self.deflicker_enabled = f.deflicker_enabled;
    }

    /// Drags the clip's right edge to `new_end_secs` (timeline-relative), keeping `start_secs`
    /// and `source_in_secs` fixed. No-op (`false`) if that would shrink the clip below
    /// `min_duration_secs`, or (when `max_source_out_secs` is known — the source asset's own
    /// duration) push `source_out_secs` past the end of the actual source media.
    pub fn trim_end(
        &mut self,
        new_end_secs: f64,
        min_duration_secs: f64,
        max_source_out_secs: Option<f64>,
    ) -> bool {
        let new_timeline_duration = new_end_secs - self.start_secs;
        let new_source_out_secs =
            self.source_in_secs + new_timeline_duration * self.speed_factor as f64;
        if new_timeline_duration < min_duration_secs {
            return false;
        }
        if let Some(max) = max_source_out_secs {
            if new_source_out_secs > max {
                return false;
            }
        }
        self.source_out_secs = new_source_out_secs;
        true
    }
}

fn default_true() -> bool {
    true
}

/// One row of the timeline (e.g. `V1`, `A1`, `A2` in the mockup), holding an ordered list of
/// clips. Tracks don't overlap-check their own clips — that's an editing-time concern.
///
/// Text tracks (`kind == TrackKind::Text`) hold [`TextClip`]s in `text_clips` instead of
/// [`ClipInstance`]s in `clips` — `clips` is always empty for a text track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: u64,
    pub name: String,
    pub kind: TrackKind,
    pub clips: Vec<ClipInstance>,
    /// Text overlays on this track. Only populated when `kind == TrackKind::Text`; always
    /// empty for `Video`/`Audio` tracks. `#[serde(default)]` so projects saved before this
    /// field existed load without error.
    #[serde(default)]
    pub text_clips: Vec<TextClip>,
    /// Whether this track contributes to export and preview. Toggled from the timeline track
    /// header. Defaults to `true`; missing in project files saved before this field was added
    /// deserializes as `true` via the serde default so existing projects are unaffected.
    #[serde(default = "default_true")]
    pub visible: bool,
}

impl Track {
    /// The clip covering `at_secs` (timeline-relative), inclusive of `start_secs` — distinct
    /// from [`ClipInstance::contains`]'s strict-interior semantics (which exists for split
    /// safety, so splitting exactly on an edge doesn't produce a zero-length half). This one is
    /// for "what's playing right now" queries (preview), where the clip starting exactly at
    /// the playhead should count. `None` if nothing on this track covers `at_secs`.
    pub fn clip_at(&self, at_secs: f64) -> Option<&ClipInstance> {
        self.clips
            .iter()
            .find(|c| at_secs >= c.start_secs && at_secs < c.start_secs + c.duration_secs())
    }

    /// Splits the clip covering `at_secs` (timeline-relative) into two: the original keeps
    /// `id` and has its `source_out_secs` trimmed to the split point; a new clip starting at
    /// `at_secs`, with `new_clip_id` and the rest of the original's source range, is inserted
    /// right after it. No-op (`false`) if no clip on this track covers `at_secs`.
    pub fn split_clip_at(&mut self, at_secs: f64, new_clip_id: u64) -> bool {
        let Some(index) = self.clips.iter().position(|c| c.contains(at_secs)) else {
            return false;
        };

        let clip = &mut self.clips[index];
        let split_source_secs =
            clip.source_in_secs + (at_secs - clip.start_secs) * clip.speed_factor as f64;
        let split_frac = ((at_secs - clip.start_secs) / clip.duration_secs()) as f32;
        let (position_first, position_second) = keyframe::split_keyframes_at(
            &clip.position_keyframes,
            split_frac,
            Position { x: 0.0, y: 0.0 },
        );
        let (scale_first, scale_second) =
            keyframe::split_keyframes_at(&clip.scale_keyframes, split_frac, 1.0);
        let (rotation_first, rotation_second) =
            keyframe::split_keyframes_at(&clip.rotation_keyframes, split_frac, 0.0);
        let (opacity_first, opacity_second) =
            keyframe::split_keyframes_at(&clip.opacity_keyframes, split_frac, 1.0);
        let second_half = ClipInstance {
            id: new_clip_id,
            asset_id: clip.asset_id,
            start_secs: at_secs,
            source_in_secs: split_source_secs,
            source_out_secs: clip.source_out_secs,
            // Splitting a composite member must not silently ungroup it from the rest of the
            // block.
            composite_id: clip.composite_id,
            gain_db: clip.gain_db,
            frozen: clip.frozen,
            speed_factor: clip.speed_factor,
            crop_x: clip.crop_x,
            crop_y: clip.crop_y,
            crop_w: clip.crop_w,
            crop_h: clip.crop_h,
            mask_shape: clip.mask_shape,
            mask_corner_radius: clip.mask_corner_radius,
            flipped_h: clip.flipped_h,
            color_filter: clip.color_filter,
            vignette_intensity: clip.vignette_intensity,
            brightness: clip.brightness,
            contrast: clip.contrast,
            saturation: clip.saturation,
            sharpen: clip.sharpen,
            chroma_key_enabled: clip.chroma_key_enabled,
            chroma_key_color: clip.chroma_key_color,
            chroma_key_tolerance: clip.chroma_key_tolerance,
            blur_intensity: clip.blur_intensity,
            shake_intensity: clip.shake_intensity,
            glitch_intensity: clip.glitch_intensity,
            pixelize_intensity: clip.pixelize_intensity,
            // Arguably a freshly-split second half shouldn't inherit an "incoming transition"
            // meant for the original clip's start, but every other field here is propagated
            // unconditionally on split, so this stays consistent with that rather than special-
            // casing it.
            transition_in: clip.transition_in,
            transition_duration_secs: clip.transition_duration_secs,
            position_keyframes: position_second,
            scale_keyframes: scale_second,
            rotation_keyframes: rotation_second,
            opacity_keyframes: opacity_second,
            deflicker_enabled: clip.deflicker_enabled,
        };
        clip.source_out_secs = split_source_secs;
        clip.position_keyframes = position_first;
        clip.scale_keyframes = scale_first;
        clip.rotation_keyframes = rotation_first;
        clip.opacity_keyframes = opacity_first;

        self.clips.insert(index + 1, second_half);
        true
    }

    /// Mutable access to the clip with this id, if it's on this track.
    pub fn clip_mut(&mut self, clip_id: u64) -> Option<&mut ClipInstance> {
        self.clips.iter_mut().find(|c| c.id == clip_id)
    }

    /// Repositions the clip with `clip_id` to `new_start_secs` on this same track — what
    /// dragging a clip's body (not one of its edges) does. Doesn't check for overlap with
    /// neighboring clips (matches this struct's existing no-overlap-checking policy, see
    /// above) — dragging one clip onto another just lets them overlap for now. No-op
    /// (`false`) if the clip isn't on this track or `new_start_secs` is negative.
    pub fn move_clip(&mut self, clip_id: u64, new_start_secs: f64) -> bool {
        if new_start_secs < 0.0 {
            return false;
        }
        let Some(clip) = self.clip_mut(clip_id) else {
            return false;
        };
        clip.start_secs = new_start_secs;
        true
    }

    /// The position, in seconds, where this track's last clip ends. `0.0` for an empty track —
    /// the natural "append here" position for a clip added to this track. Accounts for both
    /// [`ClipInstance`]s and [`TextClip`]s so text tracks report their own length correctly.
    pub fn duration_secs(&self) -> f64 {
        let clips_end = self
            .clips
            .iter()
            .map(|c| c.start_secs + c.duration_secs())
            .fold(0.0, f64::max);
        let text_end = self
            .text_clips
            .iter()
            .map(|t| t.start_secs + t.duration_secs)
            .fold(0.0, f64::max);
        clips_end.max(text_end)
    }
}

/// A project's full set of tracks plus the current playhead position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    pub tracks: Vec<Track>,
    pub playhead_secs: f64,
}

impl Timeline {
    /// Mutable access to the clip with `clip_id` across all tracks, if it exists. Searches
    /// tracks in order and returns the first match — clip ids are unique within a timeline.
    pub fn clip_mut(&mut self, clip_id: u64) -> Option<&mut ClipInstance> {
        self.tracks.iter_mut().find_map(|t| t.clip_mut(clip_id))
    }

    /// The position, in seconds, where the last clip on any track ends — i.e. how long the
    /// edited sequence runs for. `0.0` for an empty timeline.
    pub fn duration_secs(&self) -> f64 {
        self.tracks
            .iter()
            .map(Track::duration_secs)
            .fold(0.0, f64::max)
    }

    /// Moves the clip with `clip_id` onto `target_track_id` at `new_start_secs`, removing it
    /// from wherever it currently lives (which may itself be `target_track_id`, for a same-
    /// track reposition — [`Track::move_clip`] is the cheaper path for that specific case, but
    /// this stays correct for it too). No-op (`false`) if the clip or target track don't
    /// exist, `new_start_secs` is negative, or the target track's `kind` doesn't match the
    /// clip's current track's — a video clip can't land on an audio track and vice versa.
    pub fn move_clip_to_track(
        &mut self,
        clip_id: u64,
        target_track_id: u64,
        new_start_secs: f64,
    ) -> bool {
        if new_start_secs < 0.0 {
            return false;
        }
        let Some(source_index) = self
            .tracks
            .iter()
            .position(|t| t.clips.iter().any(|c| c.id == clip_id))
        else {
            return false;
        };
        let Some(target_index) = self.tracks.iter().position(|t| t.id == target_track_id) else {
            return false;
        };
        if self.tracks[source_index].kind != self.tracks[target_index].kind {
            return false;
        }

        let clip_index = self.tracks[source_index]
            .clips
            .iter()
            .position(|c| c.id == clip_id)
            .expect("source_index was found by locating a track containing this clip_id");
        let mut clip = self.tracks[source_index].clips.remove(clip_index);
        clip.start_secs = new_start_secs;
        self.tracks[target_index].clips.push(clip);
        true
    }
}
