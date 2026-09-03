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

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::keyframe::{self, Keyframe, Position};

/// What a [`Track`] carries. Determines how the timeline widget renders its clips
/// (thumbnails for video, waveforms for audio) and which asset kind can be dropped onto it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    Video,
    Audio,
    /// A text-overlay track: holds [`TextClip`]s rasterized into RGBA overlays on export.
    /// No media assets are placed here — only `text_clips`.
    Text,
    /// A shape-overlay track: holds [`ShapeClip`]s (rectangles/ellipses — per `request.md`'s
    /// Fase 4 "Efeitos visuais" umbrella, a graphic-element counterpart to text overlays). No
    /// media assets are placed here — only `shape_clips`.
    Shape,
}

/// What kind of audio source a [`Track`] carries — user-set metadata (a track header picker),
/// not inferred. `Video`/`Audio` tracks both carry real audio (a video capture's own embedded
/// game audio counts) and both can be tagged; `Text`/`Shape` tracks stay `Unspecified` since
/// they never carry audio at all. Exists so a feature that needs to reason about *which* track
/// is which audio source — D2 (`spec/architecture/differentiators.md`, highlight detection
/// needs to correlate simultaneous game-audio + mic spikes) and, later, multicam sync — has an
/// actual answer instead of guessing from track order or name. `#[default]` `Unspecified` so an
/// untagged/older-saved project's tracks are simply invisible to those features rather than
/// misclassified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AudioRole {
    #[default]
    Unspecified,
    GameAudio,
    Mic,
    Music,
}

impl AudioRole {
    /// Maps this role to the integer code used in [`avbridge::AudioSegment::duck_role`] and the
    /// `AudioSegment.duck_role` C field (P2 item 6, "Audio ducking") — `Mic` is the sidechain
    /// trigger, `Music` is what gets ducked under it, everything else mixes in unducked.
    pub fn to_duck_role_code(self) -> u8 {
        match self {
            Self::Unspecified | Self::GameAudio => 0,
            Self::Mic => 1,
            Self::Music => 2,
        }
    }
}

/// Bundled font family used by a [`TextClip`]. Every family is shipped with oca under the
/// SIL Open Font License, so projects render identically even when the host has no fonts
/// installed. The actual font bytes are parsed lazily by [`crate::text_metrics`].
///
/// Persisted in `.ocproj` by variant name (`serde`'s default unit-variant encoding, confirmed
/// against `rmp-serde`'s own `serialize_unit_variant` — it writes the name string unconditionally,
/// never a positional index, so reordering these variants in source has never been able to change
/// on-disk meaning). `Lato` carries `#[serde(other)]` — FONT-01's forwards-compatibility rule
/// (`spec/architecture/built-in-font-catalog.md`: "an unknown future ID... renders with the
/// deterministic fallback rather than corrupting project load") for the one part achievable
/// without a data-carrying persisted type: a project saved by a future build with a family this
/// build doesn't know falls back to Lato instead of failing to load at all. The remaining half of
/// that rule — preserving the unrecognized name itself through a resave, rather than silently
/// normalizing it to `Lato` — needs [`crate::font_catalog`]'s `family_id` to become the actual
/// persisted representation, deliberately deferred (see that module's own doc comment).
/// `#[serde(other)]` requires its variant to be declared last, which is why `Lato` (already
/// `#[default]`) moved to the bottom of this list — [`Self::ALL`]'s own order is a separate,
/// explicit array and keeps Lato first for the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextFontFamily {
    /// Condensed display face suitable for titles.
    BebasNeue,
    /// High-contrast serif display face.
    PlayfairDisplay,
    /// Casual handwritten face.
    PatrickHand,
    /// Monospaced face.
    AnonymousPro,
    /// Heavy, high-contrast face intended for short-form captions.
    ArchivoBlack,
    /// Modern sans-serif suitable for body text and general captions. Also the deterministic
    /// fallback for a font family this build doesn't recognize (see this enum's own doc comment).
    #[default]
    #[serde(other)]
    Lato,
}

impl TextFontFamily {
    pub const ALL: [Self; 6] = [
        Self::Lato,
        Self::BebasNeue,
        Self::PlayfairDisplay,
        Self::PatrickHand,
        Self::AnonymousPro,
        Self::ArchivoBlack,
    ];

    /// Whether this bundled family has a distinct bold file. Single-weight display faces keep
    /// their own designed weight and therefore expose only [`TextFontStyle::Regular`].
    pub const fn supports_bold(self) -> bool {
        matches!(
            self,
            Self::Lato | Self::PlayfairDisplay | Self::AnonymousPro
        )
    }
}

/// Style within a bundled [`TextFontFamily`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextFontStyle {
    #[default]
    Regular,
    Bold,
}

/// Paragraph base direction for a [`TextClip`] (TEXT-01B,
/// `spec/architecture/complex-text-shaping.md`) — UAX #9's own P2/P3 auto-detection by default,
/// or an explicit override for text whose script alone doesn't disambiguate direction (e.g. a
/// caption that is only digits/punctuation, or mixed RTL/LTR text where the author wants the
/// *paragraph's* level pinned regardless of which script happens to lead). Does not force every
/// character's direction like a bidi *override* would — numbers and embedded opposite-script
/// runs still follow ordinary UAX #9 resolution within that pinned paragraph level, same as CSS's
/// `direction` property (as opposed to `unicode-bidi: bidi-override`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextDirection {
    #[default]
    Auto,
    Ltr,
    Rtl,
}

/// Horizontal text alignment for a [`TextClip`] (TEXT-01B, `spec/architecture/
/// complex-text-shaping.md`'s "semantic alignment" goal, scoped down to plain visual Left/
/// Center/Right for this slice — see that doc's own TEXT-01B step 1 note). `Auto` (the default)
/// preserves this clip's exact pre-existing behavior byte-for-byte: [`TextClip::pos_x`] is the
/// text block's *left* edge, wrapping into the remaining space to the canvas's right edge, and
/// per-line alignment within that box falls back to `cosmic-text`'s own direction-aware default
/// (left for an LTR-detected paragraph, right for RTL). Choosing `Left`/`Center`/`Right`
/// explicitly instead **redefines what `pos_x` anchors**: the text block's left edge, horizontal
/// center, or right edge respectively — matching how most caption/design tools bind a position
/// handle to the alignment currently selected, at the cost of `pos_x` meaning something different
/// depending on this field (a real, deliberate trade-off, not an oversight — the alternative of
/// always aligning within a fixed canvas-wide box was rejected because it would make `pos_x`
/// silently stop mattering the moment alignment left `Auto`/`Left`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextAlign {
    #[default]
    Auto,
    Left,
    Center,
    Right,
}

/// One placed text overlay on a [`Track`] whose [`TrackKind`] is [`TrackKind::Text`].
/// Rasterized with the selected bundled font into an RGBA image, then composited in a native
/// post-processing pass after the main timeline encode — see `avbridge::apply_text_overlays`.
///
/// Previewed via [`crate::overlay_render::render_text_clip_rgba`] and
/// [`crate::preview::Preview::open_composited`]'s replaceable overlay branches. During playback,
/// the UI asks the preview to replace its frozen RGBA buffer only when `words`/
/// `highlight_enabled` resolve to another active word. Preview and export filter the same
/// complete-caption `fontdue` layout by UTF-8 byte range, so automatic wrapping, explicit
/// newlines, font geometry, and background geometry remain identical.
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
    /// Bundled font family. Defaults to Lato for projects saved before font selection existed.
    #[serde(default)]
    pub font_family: TextFontFamily,
    /// Font style. Families without a distinct bold file normalize this to `Regular` at render
    /// time, while keeping the persisted value harmless and forwards-compatible.
    #[serde(default)]
    pub font_style: TextFontStyle,
    /// RGBA color: `[r, g, b, a]`, each 0–255. Alpha 255 = fully opaque.
    pub color_rgba: [u8; 4],
    /// RGBA color behind the complete laid-out text block. Alpha 0 disables the background.
    #[serde(default)]
    pub background_rgba: [u8; 4],
    /// Space in pixels between glyph bounds and the background edge.
    #[serde(default = "default_text_background_padding")]
    pub background_padding: f32,
    /// Rounded-corner radius in pixels for the background.
    #[serde(default = "default_text_background_corner_radius")]
    pub background_corner_radius: f32,
    /// Horizontal anchor as a 0.0–1.0 fraction of the canvas width (0.0 = left edge).
    pub pos_x: f32,
    /// Vertical anchor as a 0.0–1.0 fraction of the canvas height (0.0 = top edge).
    pub pos_y: f32,
    /// General keyframe animation for `pos_x`/`pos_y` over this clip's own on-timeline
    /// duration — the position/scale/rotation slice `spec/ROADMAP.md` P4 item 34 explicitly
    /// called out as not yet done, since `TextClip`'s export path pre-rasterizes a full-canvas
    /// RGBA PNG per segment (`crate::overlay_render::render_text_segment_rgba`) rather than
    /// `ShapeClip`'s self-contained `geq` expression, so the same "position is just a `geq`
    /// per-pixel formula" trick doesn't apply here. Ships instead as a pixel-offset `overlay`
    /// stage in `avbridge::apply_text_overlays`: the raster still bakes one constant anchor
    /// (the first keyframe's value when keyframes are present, `pos_x`/`pos_y` otherwise — same
    /// "keyframes win when present" convention `ShapeClip`'s own keyframe fields use) exactly
    /// as before, and `keyframe::text_position_offset_expr` builds the delta from that anchor as
    /// an `overlay=x=<expr>:y=<expr>` fragment instead of the always-`x=0:y=0` this filter graph
    /// used to have — no sprite-cropping restructuring needed, and an unkeyframed `TextClip`
    /// gets the exact same PNG/filter graph it always had. Each axis independently overrides its
    /// own constant when non-empty, same as `ShapeClip::center_x_keyframes`/`center_y_keyframes`
    /// (two separate `f32` lists, not one `Position`-typed list, for the same reason: this is an
    /// absolute `0.0..=1.0` anchor fraction, not `ClipInstance::position_keyframes`' `-1.0..=1.0`
    /// pan-offset convention). `#[serde(default)]` so older saved projects load with no position
    /// animation. Export-only, like `opacity_keyframes` — no live preview effect (`render_text_
    /// clip_rgba` always draws at the plain `pos_x`/`pos_y` anchor).
    #[serde(default)]
    pub pos_x_keyframes: Vec<Keyframe<f32>>,
    #[serde(default)]
    pub pos_y_keyframes: Vec<Keyframe<f32>>,
    /// General keyframe animation for this block's overall size, as a multiplier (`1.0` =
    /// unscaled) — the last piece of P4 item 34's `TextClip` scope, same non-preview,
    /// export-only shape `pos_x_keyframes` has. Unlike position, this needed no raster-baking
    /// change at all: `keyframe::text_scale_sample_exprs` builds a `geq` inverse-sample remap
    /// around the raster's own baked position anchor (`pos_x`/`pos_y`, or `pos_x_keyframes`/
    /// `pos_y_keyframes`' first value when present) — an inverse zoom, not a literal `scale`
    /// filter with `eval=frame`, which reliably corrupted the heap in a real export elsewhere in
    /// this codebase when its output frame size varied per frame (see `CLAUDE.md`). Clamped to
    /// the same `0.1..=20.0` bounds [`crate::timeline::ClipInstance::scale_keyframes`] uses.
    /// `#[serde(default)]` so older saved projects load unscaled.
    #[serde(default)]
    pub scale_keyframes: Vec<Keyframe<f32>>,
    /// General keyframe animation for this block's own clockwise rotation, in degrees — the last
    /// piece of P4 item 34's `TextClip` scope. Same treatment as `scale_keyframes`: no
    /// raster-baking change needed, `keyframe::text_rotation_sample_exprs` builds a `geq`
    /// inverse-sample remap around the same raster-baked position anchor. Scale and rotation
    /// compose independently around that shared anchor with no interaction term needed (an
    /// isotropic scale and a rotation around the same center commute). `#[serde(default)]` so
    /// older saved projects load unrotated.
    #[serde(default)]
    pub rotation_keyframes: Vec<Keyframe<f32>>,
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
    /// General opacity fade over this clip's own on-timeline duration — the first slice of P4
    /// item 34's `TextClip` scope, per `spec/ROADMAP.md` (position, scale, and rotation followed
    /// later, see `pos_x_keyframes`/`pos_y_keyframes`/`scale_keyframes`/`rotation_keyframes`).
    /// The raster already carries a real alpha channel (transparent background around the text/
    /// background box), so a fade is just an alpha *multiplier* applied to the existing pixels in
    /// `avbridge::apply_text_overlays`'s filter graph (`keyframe::text_opacity_alpha_expr`) —
    /// zero changes to the Rust-side rasterization or highlight-layout code this doc comment's
    /// sibling fields depend on. Empty means "no fade, same static visibility window as
    /// before" — the exact same filter graph an unanimated `TextClip` always had.
    /// `#[serde(default)]` so older saved projects load with no fade.
    #[serde(default)]
    pub opacity_keyframes: Vec<Keyframe<f32>>,
    /// Explicit paragraph base-direction override — see [`TextDirection`]. `#[serde(default)]`
    /// so older saved projects load as `Auto` (UAX #9 auto-detection), the exact behavior this
    /// clip already had before `TextDirection` existed.
    #[serde(default)]
    pub direction: TextDirection,
    /// Optional BCP-47-ish language tag (e.g. `"pt-BR"`, `"ar"`), persisted for a future shaping
    /// pass (TEXT-01C's international-fallback face selection) to consume — not yet read by
    /// [`crate::text_layout`] or [`crate::overlay_render`], since `cosmic-text` 0.19's `Attrs`
    /// has no language field to feed it into. A real, deliberately deferred gap, not silently
    /// dropped: kept here now so a project authored with this metadata doesn't need a second
    /// migration once shaping does consume it. `#[serde(default)]` so older saved projects load
    /// with no language hint.
    #[serde(default)]
    pub language: Option<String>,
    /// Horizontal text alignment — see [`TextAlign`]. `#[serde(default)]` so older saved projects
    /// load as `Auto`, the exact behavior this clip already had before `TextAlign` existed
    /// (`pos_x` as a plain left edge, direction-aware per-line fallback).
    #[serde(default)]
    pub text_align: TextAlign,
}

fn default_text_background_padding() -> f32 {
    8.0
}

fn default_text_background_corner_radius() -> f32 {
    8.0
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

/// Which geometric primitive a [`ShapeClip`] draws. `Polygon` covers every straight-edged
/// preset `request.md`'s "geometric forms" ask names (rectangle, square, triangle, trapezoid,
/// arrow) plus a user-drawn custom shape — all just a list of vertices in the shape's own
/// `-0.5..=0.5` local unit square, tested for point-in-polygon via ray casting (handles
/// concave outlines like the arrow's, not just convex ones) — see [`ShapeKind::rectangle`] etc.
/// for the fixed presets and `avcore::shape_render` for the rendering math. `Ellipse` (also
/// used for a locked-aspect "circle") gets its own variant since a quadratic in/out test is far
/// cheaper than ray-casting an approximated polygon would be.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShapeKind {
    Ellipse,
    /// Vertices in order (clockwise or counter-clockwise, either works for the ray-casting
    /// test), each roughly `-0.5..=0.5` — the shape's own local unit square before it's scaled
    /// by [`ShapeClip::width`]/[`ShapeClip::height`], rotated, and translated to
    /// [`ShapeClip::center_x`]/[`ShapeClip::center_y`]. `request.md`'s "opção de desenhar uma
    /// forma personalizada" (custom shape) is this same variant with user-placed vertices — the
    /// properties panel's per-vertex X/Y editor (`ui`'s `polygon_vertex_editor`, shown whenever
    /// a `ShapeClip`'s `shape_kind` is a `Polygon` — every preset included, since they're all
    /// `Polygon` under the hood too, see `ShapeKind::rectangle()` etc.) lets a user hand-edit,
    /// add, or remove vertices starting from any preset or from scratch, so this is no longer
    /// data-model-only.
    Polygon(Vec<(f32, f32)>),
}

impl Default for ShapeKind {
    fn default() -> Self {
        Self::rectangle()
    }
}

impl ShapeKind {
    pub fn rectangle() -> Self {
        Self::Polygon(vec![(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)])
    }

    /// Same vertices as [`Self::rectangle`] — "square" is a UI-level aspect lock (equal
    /// `width`/`height` on the [`ShapeClip`]), not a different shape.
    pub fn square() -> Self {
        Self::rectangle()
    }

    pub fn triangle() -> Self {
        Self::Polygon(vec![(0.0, -0.5), (0.5, 0.5), (-0.5, 0.5)])
    }

    /// Narrower top edge than bottom, per the conventional trapezoid look.
    pub fn trapezoid() -> Self {
        Self::Polygon(vec![(-0.25, -0.5), (0.25, -0.5), (0.5, 0.5), (-0.5, 0.5)])
    }

    /// A right-pointing arrow: a thin shaft plus a wide triangular head, as one seven-vertex
    /// concave polygon (correct under ray casting, unlike a convex-only in/out test).
    pub fn arrow() -> Self {
        Self::Polygon(vec![
            (-0.5, -0.15),
            (0.1, -0.15),
            (0.1, -0.35),
            (0.5, 0.0),
            (0.1, 0.35),
            (0.1, 0.15),
            (-0.5, 0.15),
        ])
    }
}

/// One placed geometric shape on a [`Track`] whose [`TrackKind`] is [`TrackKind::Shape`].
/// Rendered the same way [`TextClip`] is — a `geq`-based post-processing pass after the main
/// timeline encode, see `avcore::shape_render`/`avbridge::apply_shape_overlays`.
///
/// Previewed via [`crate::overlay_render::render_shape_clip_rgba`], same static-overlay-branch
/// mechanism as [`TextClip`]'s.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeClip {
    pub id: u64,
    /// Start time on the timeline, in seconds.
    pub start_secs: f64,
    /// How long the shape stays visible, in seconds.
    pub duration_secs: f64,
    pub shape_kind: ShapeKind,
    /// Center, as a `0.0..=1.0` fraction of canvas width/height.
    pub center_x: f32,
    pub center_y: f32,
    /// General keyframe animation for this shape's center position over its own on-timeline
    /// duration, per the keyframe-expansion gap found while surveying what else the existing
    /// keyframe system could drive (`spec/ROADMAP.md` P4 item 34) — the first slice of that
    /// item to ship, since `ShapeClip`'s export path (a self-contained `geq` expression built
    /// entirely in Rust, see `crate::shape_render`) turned out to already support a `T`-keyed
    /// per-pixel expression without any FFI/C changes, unlike `TextClip`'s pre-rasterized-PNG
    /// overlay approach (see `TextClip::pos_x_keyframes` for how that one ships instead, as a
    /// pixel-offset `overlay` stage rather than a `geq` formula). Each field independently
    /// overrides its own constant (`center_x`/`center_y`) when
    /// non-empty, same "keyframes win when present" relationship every other keyframe field in
    /// this codebase already has. `#[serde(default)]` so older saved projects load with no
    /// position animation.
    #[serde(default)]
    pub center_x_keyframes: Vec<Keyframe<f32>>,
    #[serde(default)]
    pub center_y_keyframes: Vec<Keyframe<f32>>,
    /// Size, as a `0.0..=1.0` fraction of canvas width/height — what dragging a resize handle
    /// changes.
    pub width: f32,
    pub height: f32,
    /// General keyframe animation for width/height over this shape's own on-timeline duration —
    /// the rest of `spec/ROADMAP.md` P4 item 34's `ShapeClip` scope, shipped after position
    /// animation. `shape_render::inside_expr`'s geometry math (`ellipse_inside_expr`/
    /// `polygon_inside_expr`) was reworked to accept `geq`-expression-language sub-expressions
    /// for the half-extents instead of literal `f64`s, so the same multiply-through-avoid-
    /// division trick the static case always used still applies verbatim — an unkeyframed
    /// `width`/`height` degenerates back to the same plain numeric literal
    /// `keyframe::shape_axis_expr` already returns for the unkeyframed position case, so the
    /// static-shape math is unchanged in that (still the common) case. `#[serde(default)]` so
    /// older saved projects load with no size animation.
    #[serde(default)]
    pub width_keyframes: Vec<Keyframe<f32>>,
    #[serde(default)]
    pub height_keyframes: Vec<Keyframe<f32>>,
    /// Clockwise rotation around the shape's own center, in degrees.
    pub rotation_deg: f32,
    /// General keyframe animation for `rotation_deg` over this shape's own on-timeline
    /// duration — the last piece of P4 item 34's `ShapeClip` scope. Unlike width/height, this
    /// doesn't touch `inside_expr` at all: only the local-frame rotation (`rx`/`ry` in
    /// `shape_render::build_shape_filter_desc`) changes, from Rust-precomputed `sin`/`cos`
    /// literals to `geq`'s own `sin(...)`/`cos(...)`/`PI` expression-language functions (all
    /// confirmed present in FFmpeg's expression evaluator, not assumed) evaluated per pixel.
    /// `#[serde(default)]` so older saved projects load with no rotation animation.
    #[serde(default)]
    pub rotation_keyframes: Vec<Keyframe<f32>>,
    /// RGBA fill/stroke color: `[r, g, b, a]`, each 0–255. Alpha 255 = fully opaque.
    pub color_rgba: [u8; 4],
    /// Outline thickness in pixels. `0.0` = filled shape; `> 0.0` = outline only, that thick
    /// (clamped to the shape's own half-extent, so a thickness larger than the shape still
    /// renders as filled rather than vanishing).
    pub stroke_thickness_px: f32,
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
    /// `Some(sequence_id)` if this clip is a **compound clip** (nested sequence) — its content
    /// comes from rendering another [`crate::project::Sequence`] in the same
    /// [`crate::project::Project`], not `asset_id` (ignored when this is set; kept at whatever
    /// stale/placeholder value it had, same as other fields a clip kind doesn't use). Per
    /// `spec/ROADMAP.md`'s "nested sequences / compound clips" item — creates a new `Sequence`
    /// out of a selection ([`App::create_compound_clip_from_selection`] in `ui`), moves the
    /// selected clips into it, and drops this clip in their place. `source_in_secs`/
    /// `source_out_secs` trim into the *rendered* nested timeline (0-based, same convention as
    /// an ordinary asset), not the nested `Sequence`'s own internal timeline positions.
    /// [`crate::nested_sequence::materialize_nested_sequences`] resolves this recursively (a
    /// nested sequence's own clips may themselves be nested) to a temp rendered file before
    /// export/preview, cached and reused unless that sequence's own timeline content changes.
    /// `#[serde(default)]` so older saved projects load with every clip asset-backed, unchanged.
    #[serde(default)]
    pub nested_sequence_id: Option<u64>,
    /// `Some(group_id)` if this clip is a member of a composite block (per `request.md`'s
    /// Fase 3 "blocos compostos" spec) — every clip sharing the same id, always on the same
    /// track (composite blocks don't span tracks yet), moves/splits/deletes together as a
    /// unit (see `ui`'s `App::merge_into_composite` and the timeline panel's drag/delete
    /// handling). `#[serde(default)]` so a project saved before this field existed still
    /// loads, every clip in it just standalone (`None`).
    #[serde(default)]
    pub composite_id: Option<u64>,
    /// Optional RGB color label for this block, per `matrix/competitor-parity.md`'s 2026-08-27
    /// update (`spec/ROADMAP.md` P4 item 27) — a purely cosmetic at-a-glance organization aid
    /// (Premiere's clip labels, DaVinci's clip *and* track color), painted as the timeline
    /// block's fill color in place of its usual kind-based color when set. `None` = use the
    /// usual coloring. `#[serde(default)]` so older saved projects load with no label.
    #[serde(default)]
    pub color_label: Option<[u8; 3]>,
    /// Volume adjustment in decibels applied to this block's audio, independent of every other
    /// clip — per `request.md`'s Fase 4 "ganho de volume por bloco" spec. `0.0` is unity gain.
    /// Feeds the timeline waveform display (`ui`'s `draw_waveform`, scaled by
    /// [`ClipInstance::gain_linear`]) and is wired into export — resolved to
    /// `avbridge::ClipSegment::gain_db` and applied as a `volume=<gain>dB` audio filter stage
    /// (`timeline_export.c`/`timeline_export_multi.c`). Also wired into preview now
    /// (`crate::preview::build_audio_filter_bin`/`Preview::open_composited`'s own audio chain,
    /// both via GStreamer's `volume` element — a linear scale factor, so `gain_db` converts via
    /// `preview::gain_db_to_linear`, unlike avfilter's own `volume=<gain>dB` string option).
    /// `#[serde(default)]` so older saved projects load at unity gain.
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
    /// per `request.md`'s Fase 4 "Velocidade" spec. `1.0` is normal speed. Already factored into
    /// [`ClipInstance::duration_secs`] (the trimmed source range divided by speed, so the
    /// timeline block's own length reflects the sped-up/slowed-down result), shown as a badge
    /// on the timeline block (`ui`'s timeline panel), and wired into export: resolved to
    /// `avbridge::ClipSegment::speed_factor`, applied as `setpts=PTS/<speed>` on video and
    /// `atempo` on audio (`timeline_export.c`/`timeline_export_multi.c`). No live preview effect
    /// yet. `#[serde(default = ..)]` so older saved projects load at normal speed. When
    /// [`ClipInstance::speed_ramp_end_factor`] is `Some`, this field is instead the ramp's
    /// *start* speed — see that field's own doc comment.
    #[serde(default = "default_speed_factor")]
    pub speed_factor: f32,
    /// End speed of a smooth, continuous speed ramp across this clip's whole trimmed duration —
    /// `None` (the default) means plain constant `speed_factor`, unchanged. `Some(end)` means
    /// `speed_factor` is the ramp's *start* speed and this is its end speed, linearly
    /// interpolated in speed (not in output-time) over the clip's own trimmed source duration —
    /// per `spec/ROADMAP.md` P4 item 29's "smooth continuous curve" follow-up to the earlier
    /// stepped approximation. Export resolves this to a `setpts` expression that's the
    /// *integral* of `1/speed(t)` (a natural-log term, since speed is linear in `t`) rather than
    /// splitting the clip into discrete pieces — see
    /// [`crate::keyframe::smooth_speed_ramp_duration_secs`] for the matching duration formula
    /// [`ClipInstance::duration_secs`] uses, and
    /// `avbridge::ClipSegment::smooth_speed_ramp_end_factor`/`timeline_export.c`'s `setpts_str`
    /// construction for the export-side expression. `#[serde(default)]` so older saved projects
    /// load with no ramp (plain `speed_factor`, unchanged behavior).
    #[serde(default)]
    pub speed_ramp_end_factor: Option<f32>,
    /// Normalized crop rectangle within the source frame — `(crop_x, crop_y)` is the visible
    /// sub-rectangle's top-left corner, `(crop_w, crop_h)` its size, all fractions of the full
    /// frame (`0.0..=1.0`). Defaults to `(0.0, 0.0, 1.0, 1.0)` — the whole frame, uncropped —
    /// per `request.md`'s Fase 4 "Recorte (crop)" spec: reframing separate from the time-based
    /// split already covered in Fase 3. Shown as a badge on the timeline block (`ui`'s timeline
    /// panel, via [`ClipInstance::is_cropped`]) and wired into export ([`ClipInstance::
    /// video_filter_chain`]'s `crop=...` stage); no live preview effect yet. Independently
    /// clamped to `[0.0, 1.0]` when set (`ui`'s `App::set_selected_clip_crop`); a crop rect
    /// extending past the frame edge (`crop_x + crop_w > 1.0`) isn't rejected here — a known
    /// simplification, not verified against how `ffmpeg`'s own `crop` filter behaves on an
    /// out-of-bounds rectangle at render time. `#[serde(default = ..)]` so older saved projects
    /// load uncropped.
    #[serde(default)]
    pub crop_x: f32,
    #[serde(default)]
    pub crop_y: f32,
    #[serde(default = "default_crop_extent")]
    pub crop_w: f32,
    #[serde(default = "default_crop_extent")]
    pub crop_h: f32,
    /// General keyframe animation for this block's crop rectangle over time (a moving/resizing
    /// pan window), per the keyframe-expansion gap found while surveying what else the existing
    /// keyframe system could drive (`spec/ROADMAP.md` P4 item 33) — independent of
    /// [`ClipInstance::scale_keyframes`]'s Ken-Burns zoom (a single symmetric zoom factor about
    /// the frame center), this animates all four crop axes independently. Each field is
    /// independent: a non-empty list overrides that axis's own constant field above (same
    /// "keyframes win when present" relationship [`ClipInstance::gain_keyframes`] has with
    /// `gain_db`). Wired into export ([`crate::keyframe::crop_filter_expr`], a `geq`-based
    /// per-pixel approach — see that function's own doc comment for why, over `crop`+
    /// `eval=frame`), spliced into [`ClipInstance::keyframe_video_filter_chain`] alongside
    /// scale/rotation/opacity/color-balance rather than [`ClipInstance::video_filter_chain`]'s
    /// own static `crop` stage, which this field being non-empty on any axis suppresses instead
    /// of double-emitting (same pattern [`ClipInstance::brightness_keyframes`] established for
    /// the static `eq` stage). Not yet wired into live preview. `#[serde(default)]` so older
    /// saved projects load with no crop animation (using the constant fields as before).
    #[serde(default)]
    pub crop_x_keyframes: Vec<Keyframe<f32>>,
    #[serde(default)]
    pub crop_y_keyframes: Vec<Keyframe<f32>>,
    #[serde(default)]
    pub crop_w_keyframes: Vec<Keyframe<f32>>,
    #[serde(default)]
    pub crop_h_keyframes: Vec<Keyframe<f32>>,
    /// Layer mask shape ([`MaskShape::None`] by default — unmasked). Independent of the
    /// rectangular crop above; a block can be both cropped and masked. Wired into export
    /// ([`ClipInstance::video_filter_chain`]'s `geq`-based alpha stage) but, like
    /// [`ClipInstance::chroma_key_enabled`], only has a visible effect on a clip placed on an
    /// **overlay track** — a single/background track's final `format=yuv420p` conform drops
    /// the alpha plane it produces. Also wired into preview
    /// (`crate::preview::build_mask_shape_stage`'s `alphacombine` stage, fed a static
    /// `crate::overlay_render::render_mask_shape_gray8` buffer instead of a decoded file), same
    /// overlay-only gate. `#[serde(default)]` so older saved projects load unmasked.
    #[serde(default)]
    pub mask_shape: MaskShape,
    /// Corner radius for [`MaskShape::RoundedRect`], as a fraction (`0.0..=1.0`) of the block's
    /// shorter frame dimension — meaningless for the other shapes. `#[serde(default)]` so older
    /// saved projects load at `0.0` (square corners).
    #[serde(default)]
    pub mask_corner_radius: f32,
    /// `true` if this block's frame is mirrored horizontally, per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Espelhar (flip horizontal)"). Shown as a badge on the timeline
    /// block (`ui`'s timeline panel) and wired into export ([`ClipInstance::video_filter_chain`]'s
    /// `hflip` stage); no live preview effect yet. `#[serde(default)]` so older saved projects
    /// load unflipped.
    #[serde(default)]
    pub flipped_h: bool,
    /// Color filter applied to this block ([`ColorFilter::None`] by default). Shown as a
    /// tinted timeline-block fill (`ui`'s timeline panel) and wired into export
    /// ([`ClipInstance::video_filter_chain`]'s `hue=s=0`/`colorchannelmixer` stage); no live
    /// preview effect yet. `#[serde(default)]` so older saved projects load unfiltered.
    #[serde(default)]
    pub color_filter: ColorFilter,
    /// Vignette strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Vinheta"). Shown as a darkened border stroke around the timeline
    /// block, scaled by intensity (`ui`'s timeline panel), and wired into export
    /// ([`ClipInstance::video_filter_chain`]'s `vignette` stage); no live preview effect yet.
    /// `#[serde(default)]` so older saved projects load with no vignette.
    #[serde(default)]
    pub vignette_intensity: f32,
    /// Brightness adjustment for this block, `-1.0..=1.0` (`0.0` is unchanged) — per
    /// `request.md`'s Fase 4 "Efeitos visuais" spec ("Brilho, contraste e saturação"). Wired
    /// into export ([`ClipInstance::video_filter_chain`]'s `eq=brightness=...` stage, combined
    /// with [`ClipInstance::contrast`]/[`ClipInstance::saturation`] into one `eq` filter); no
    /// live preview effect yet. `#[serde(default)]` so older saved projects load unchanged.
    #[serde(default)]
    pub brightness: f32,
    /// Contrast multiplier for this block, `0.0..=2.0` (`1.0` is unchanged) — same spec and
    /// export wiring as [`ClipInstance::brightness`]. `#[serde(default = ..)]` so older saved
    /// projects load unchanged.
    #[serde(default = "default_unity_multiplier")]
    pub contrast: f32,
    /// Saturation multiplier for this block, `0.0..=2.0` (`1.0` is unchanged, `0.0` is
    /// grayscale) — same spec and export wiring as [`ClipInstance::brightness`].
    /// `#[serde(default = ..)]` so older saved projects load unchanged.
    #[serde(default = "default_unity_multiplier")]
    pub saturation: f32,
    /// General keyframe animation for this block's brightness/contrast/saturation over time, per
    /// the keyframe-expansion gap found while surveying what else the existing keyframe system
    /// could drive (`spec/ROADMAP.md` P4 item 32). Each field is independent: a non-empty list
    /// overrides that axis's own constant field above (same "keyframes win when present"
    /// relationship [`ClipInstance::gain_keyframes`] has with `gain_db`); an axis left empty
    /// keeps using its constant. Wired into export
    /// ([`crate::keyframe::color_balance_filter_expr`], spliced into
    /// [`ClipInstance::keyframe_video_filter_chain`] alongside scale/rotation/opacity rather
    /// than [`ClipInstance::video_filter_chain`]'s own static `eq` stage, which this field being
    /// non-empty on any axis suppresses instead of double-emitting) — a real, narrow ordering
    /// caveat: the animated `eq` stage runs at the *front* of the per-clip filter chain (with
    /// scale/rotation/opacity) rather than its usual position after crop/deflicker/
    /// stabilization, so a clip combining color-grading keyframes with any of those three sees
    /// its color grading applied to the pre-crop/pre-deflicker/pre-stabilization frame instead.
    /// Not yet wired into live preview. `#[serde(default)]` so older saved projects load with no
    /// color-grading animation (using the constant fields as before).
    #[serde(default)]
    pub brightness_keyframes: Vec<Keyframe<f32>>,
    #[serde(default)]
    pub contrast_keyframes: Vec<Keyframe<f32>>,
    #[serde(default)]
    pub saturation_keyframes: Vec<Keyframe<f32>>,
    /// Sharpen strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Nitidez (sharpen)"). Wired into export
    /// ([`ClipInstance::video_filter_chain`]'s `unsharp` stage); no live preview effect yet.
    /// `#[serde(default)]` so older saved projects load unsharpened.
    #[serde(default)]
    pub sharpen: f32,
    /// `true` if chroma key (green-screen removal) is enabled for this block, per
    /// `request.md`'s Fase 4 "Efeitos visuais" spec ("Chroma key"). Wired into export
    /// ([`ClipInstance::video_filter_chain`]'s `colorkey` stage) but only has a visible effect
    /// on a clip placed on an **overlay track** — a single/background track's final
    /// `format=yuv420p` conform drops the alpha plane this produces (same caveat
    /// [`ClipInstance::mask_shape`] carries above; see CLAUDE.md's "Alpha/overlay-track
    /// caveat"). No live preview effect yet. `#[serde(default)]` so older saved projects load
    /// disabled.
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
    /// "Efeitos visuais" spec ("Blur"). Wired into export ([`ClipInstance::video_filter_chain`]'s
    /// `boxblur` stage); no live preview effect yet. `#[serde(default)]` so older saved projects
    /// load unblurred.
    #[serde(default)]
    pub blur_intensity: f32,
    /// Camera-shake strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s
    /// Fase 4 "Efeitos visuais" spec ("Shake"), the deliberate counterpart of the eventual video
    /// stabilization feature. Wired into export and preview (unlike most of this struct's other
    /// effect fields) — see [`ClipInstance::video_filter_chain`]'s `crop`+`scale` oscillation
    /// stage for export and `build_video_filter_bin` for the preview element. `#[serde(default)]`
    /// so older saved projects load unshaken.
    #[serde(default)]
    pub shake_intensity: f32,
    /// Glitch strength for this block, `0.0..=1.0` (`0.0` is off) — per `request.md`'s Fase 4
    /// "Efeitos visuais" spec ("Glitch"). Wired into export ([`ClipInstance::video_filter_chain`]'s
    /// `noise` stage); no live preview effect yet, unlike [`ClipInstance::shake_intensity`].
    /// `#[serde(default)]` so older saved projects load unglitched.
    #[serde(default)]
    pub glitch_intensity: f32,
    /// Pixelize/mosaic-censor strength for this block, `0.0..=1.0` (`0.0` is off) — per
    /// `request.md`'s Fase 4 "Efeitos visuais" spec ("Pixelizar/censura (mosaico)"). Wired into
    /// both export ([`ClipInstance::video_filter_chain`]'s scale-down/scale-up stage) and
    /// preview (`build_video_filter_bin`). `#[serde(default)]` so older saved projects load
    /// unpixelized.
    #[serde(default)]
    pub pixelize_intensity: f32,
    /// Transition style for this block's incoming edge ([`TransitionType::None`] by default) —
    /// per `request.md`'s Fase 4 "Efeitos visuais" spec ("Transições entre clipes"). Models
    /// only the transition entering this clip, not a real cross-blend between two adjacent
    /// clips — that would need a relationship between this clip and the one before it, not a
    /// field on a single `ClipInstance`. A deliberately smaller first cut, same shape as the
    /// rest of this struct's effect fields. Wired into export — resolved to `avbridge::
    /// ClipSegment::transition_in`, rendered as a fade/slide/zoom applied over
    /// [`ClipInstance::transition_duration_secs`] at the start of the clip's own filter chain
    /// (`timeline_export.c`/`timeline_export_multi.c`); no live preview effect yet.
    /// `#[serde(default)]` so older saved projects load with no transition.
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
    /// no scaling (`1.0`). Wired into export (`crate::keyframe::scale_filter_expr`) and into
    /// live preview (`core::preview::build_video_filter_bin`'s `videocrop`+`videoscale`+
    /// `capsfilter` chain, re-evaluated per buffer off its own PTS). `#[serde(default)]` so
    /// older saved projects load unscaled — a project that had real `zoom_start`/`zoom_end`
    /// values loses that animation on load, since this field replaces rather than migrates it
    /// (no back-compat promised for this format).
    #[serde(default)]
    pub scale_keyframes: Vec<Keyframe<f32>>,
    /// General keyframe animation for this block's rotation, in degrees, per
    /// `features/request.md`'s Fase 4 "Keyframes" spec. Empty = no rotation (`0.0`). Wired into
    /// export (`crate::keyframe::rotation_filter_angle_expr`) and into live preview
    /// (`core::preview::build_video_filter_bin`'s `rotate` element, its `angle` property
    /// re-evaluated per buffer). `#[serde(default)]` so older saved projects load unrotated.
    #[serde(default)]
    pub rotation_keyframes: Vec<Keyframe<f32>>,
    /// General keyframe animation for this block's opacity, `0.0..=1.0`, per
    /// `features/request.md`'s Fase 4 "Keyframes" spec. Empty = fully opaque (`1.0`). Wired into
    /// export (`crate::keyframe::opacity_alpha_ramp_expr`) — only has a visible effect on an
    /// overlay-track clip at export time, same caveat as position above — and into live preview
    /// too (`core::preview::build_video_filter_bin`'s `alpha` element), where it's visible on
    /// any clip regardless of track, since the preview's fixed RGBA output already supports
    /// alpha blending directly rather than needing export's overlay-compositing stage.
    /// `#[serde(default)]` so older saved projects load fully opaque.
    #[serde(default)]
    pub opacity_keyframes: Vec<Keyframe<f32>>,
    /// General keyframe animation for this block's audio gain, in **dB** (same unit as
    /// [`ClipInstance::gain_db`]), per the keyframe-expansion gap found while surveying what
    /// else the existing keyframe system could drive (`spec/ROADMAP.md` P4 item 31). Empty =
    /// use the constant [`ClipInstance::gain_db`] unchanged (this field, when non-empty,
    /// overrides that constant rather than combining with it — same "one or the other, not
    /// both" relationship `scale_keyframes` has with the old `zoom_start`/`zoom_end`). Wired
    /// into export (`crate::keyframe::gain_filter_db_expr`, via `avbridge::AudioSegment::
    /// gain_keyframe_expr`) — FFmpeg's `volume` filter's `eval=frame` expression mode is a
    /// linear multiplier, not dB, so the expression wraps each interpolated dB value in
    /// `pow(10,X/20)`. Not yet wired into live preview — same "export first" shape several
    /// other keyframe fields on this struct started with. `#[serde(default)]` so older saved
    /// projects load with no gain animation (using the constant `gain_db` as before).
    #[serde(default)]
    pub gain_keyframes: Vec<Keyframe<f32>>,
    /// `true` to run this block's audio through CF-02's "Gameplay Voice" cleanup chain before
    /// mixing — `highpass=f=80,afftdn=nf=<noise_floor>,acompressor=threshold=<threshold>dB:
    /// ratio=<ratio>:attack=10:release=250:makeup=1.5,alimiter=limit=<ceiling>` (values from
    /// `scripts/Watch-Gameplay.ps1`'s own proven chain — `spec/architecture/
    /// competitive-feature-plan.md`'s CF-03), applied per clip in `avbridge::audio_mix`'s
    /// per-branch filter graph, ahead of the existing final `afftdn`/`loudnorm`/`alimiter`
    /// mastering pass every export already runs on the finished mix. Meant for `Mic`-role
    /// tracks, but this is a plain per-clip toggle regardless of role — the "Mic by default,
    /// explicit override elsewhere" gate is a `ui` selector concern, not a data constraint.
    /// Non-destructive/reversible like every other effect toggle on this struct. `#[serde(
    /// default)]` so older saved projects load with it off. Not yet wired into live preview —
    /// export only, same "export first" shape several other effect fields here started with.
    #[serde(default)]
    pub voice_cleanup_enabled: bool,
    /// `afftdn`'s `nf` (expected noise floor), in dB — more negative removes less noise floor
    /// (afftdn's own convention: `nf` is where it expects the *noise* to sit, not a cut amount).
    /// `#[serde(default = ..)]` matches the proven script default.
    #[serde(default = "default_voice_cleanup_noise_floor_db")]
    pub voice_cleanup_noise_floor_db: f32,
    /// `acompressor`'s `threshold`, in dB — audio above this level gets compressed.
    #[serde(default = "default_voice_cleanup_compressor_threshold_db")]
    pub voice_cleanup_compressor_threshold_db: f32,
    /// `acompressor`'s `ratio` — how strongly audio above the threshold is compressed (higher =
    /// stronger leveling).
    #[serde(default = "default_voice_cleanup_compressor_ratio")]
    pub voice_cleanup_compressor_ratio: f32,
    /// `alimiter`'s `limit`, linear (not dB) — matches this chain's own `alimiter` stage and the
    /// unrelated final-mastering `alimiter` stage's own convention (both `0.0..=1.0`).
    #[serde(default = "default_voice_cleanup_ceiling_linear")]
    pub voice_cleanup_ceiling_linear: f32,
    /// Path to a `.cube` 3D LUT file applied to this block's color grading, per `request.md`'s
    /// Fase 4 "Filtros de cor e LUTs" spec. Empty string = no LUT (the FFI-friendly analog of
    /// `Option<PathBuf>` this codebase already uses for other optional string fields, since a
    /// plain `String` round-trips through `.ocproj`'s MessagePack struct-map encoding without
    /// needing an `Option` variant on the wire). Wired into export via `video_filter_chain`'s
    /// `lut3d` stage; no equivalent GStreamer element exists on this dev machine's install (no
    /// `lut3d`/`gllut3d`/cube-file element turned up in a real `gst-inspect-1.0` listing), so
    /// preview has no LUT stage — the same "export only" gap several other effects here have,
    /// just for a different reason (missing element, not "not wired yet"). `#[serde(default)]`
    /// so older saved projects load with no LUT applied.
    #[serde(default)]
    pub lut_path: String,
    /// Layer footprint size, as a multiplier of this clip's own native decoded width/height —
    /// `1.0` (both axes) is native size, unchanged. Independent axes allow a deliberate
    /// non-uniform stretch, not just uniform scaling, per `request.md`'s Fase 4 "Transformação
    /// de camadas" spec ("largura, altura... ajustáveis"). A *different* concept from
    /// [`ClipInstance::scale_keyframes`]'s Ken-Burns zoom, which crops into and rescales back to
    /// the *same* frame size (a zoom-in-place on the content) — this instead genuinely resizes
    /// the frame buffer that gets composited, shrinking or growing the clip's on-canvas
    /// footprint. Wired into export via a `scale=iw*x:ih*y` avfilter stage appended after every
    /// other per-clip stage (`crate::render::resolve_clip_filters`) — only meaningful on an
    /// overlay track (track 1+ in `avbridge_encode_timeline_export_multi`): a single/background
    /// track's final canvas-size conform has no pad/fit step, so shrinking there would produce
    /// a mismatched-resolution frame rather than a smaller picture with visible canvas around
    /// it, the same overlay-only caveat `position_keyframes`/`opacity_keyframes` already have,
    /// just for a correctness reason instead of a compositing one. Not yet wired into live
    /// preview. `#[serde(default = ..)]` so older saved projects load at native size.
    #[serde(default = "default_unity_multiplier")]
    pub layer_scale_x: f32,
    #[serde(default = "default_unity_multiplier")]
    pub layer_scale_y: f32,
    /// `true` if temporal luminance-flicker removal is enabled for this block, per
    /// `request.md`'s Fase 4 "Efeitos visuais" spec ("Remoção de flicker") — common in
    /// screen/gameplay captures at certain refresh rates. Wired to export via `video_filter_chain`
    /// (`deflicker`); no equivalent stage in `core::preview`'s `build_video_filter_bin` yet, the
    /// same preview gap several other effects here have. `#[serde(default)]` so older saved
    /// projects load with it off.
    #[serde(default)]
    pub deflicker_enabled: bool,
    /// Video stabilization strength for this block, `0.0..=1.0` (`0.0` is off) — per
    /// `request.md`'s Fase 4 "Efeitos visuais" spec ("Estabilização de vídeo"), the deliberate
    /// opposite of [`ClipInstance::shake_intensity`] (which adds tremido on purpose; this
    /// removes it from footage that already has it). Wired into export via `video_filter_chain`'s
    /// `deshake` stage — `libavfilter`'s built-in single-pass stabilizer, not the more capable
    /// two-pass `vidstabdetect`/`vidstabtransform` pair (`libvidstab`), which this project's
    /// pinned FFmpeg build doesn't have compiled in (confirmed via a real `ffmpeg -buildconf`,
    /// not assumed — it explicitly lists `--disable-libvidstab`). No GStreamer element for this
    /// exists on this dev machine's install either (confirmed via `gst-inspect-1.0`, same as
    /// `lut_path`'s caveat), so this is export-only, same shape as LUTs. `#[serde(default)]` so
    /// older saved projects load unstabilized.
    #[serde(default)]
    pub stabilization_intensity: f32,
    /// `true` once [`ClipInstance::background_removal_mask_path`] holds a matte generated for
    /// this exact clip — per `request.md`'s Fase 4 "Remoção de fundo por IA" spec. Not a plain
    /// style toggle like the effect fields above: the matte in `background_removal_mask_path`
    /// is generated per-clip (tied to this instance's own `source_in_secs`/`source_out_secs`
    /// range, see `crate::background_removal`), so this field is deliberately excluded from
    /// [`ClipFormatting`] — pasting it onto a different block would point that block at another
    /// clip's matte video. Wired into export (`crate::render::resolve_timeline_segments_multi`
    /// -> `avbridge::ClipSegment::mask_video_path` -> an `alphamerge` stage in
    /// `timeline_export_multi.c`), gated the same way `mask_shape`/`chroma_key`'s own alpha is:
    /// **only takes effect on an overlay track** (track 1+ in a multi-track export) — a
    /// single/background track's clips never composite, so their alpha (from this or any other
    /// source) is always discarded by the final `format=yuv420p` conform regardless. Also wired
    /// into preview (`crate::preview::build_composite_branch`'s `alphacombine` stage — the
    /// GStreamer counterpart to avfilter's `alphamerge`), same overlay-only gate.
    /// `#[serde(default)]` so older saved projects load with it off.
    #[serde(default)]
    pub background_removal_enabled: bool,
    /// Path to the grayscale-as-luma alpha-matte video `ui`'s "Gerar máscara" flow generates for
    /// this clip (`App::spawn_generate_matte_for_selected_clip`) — meaningless while
    /// [`ClipInstance::background_removal_enabled`] is `false`. Empty string = not yet
    /// generated. `#[serde(default)]` so older saved projects load with no matte.
    #[serde(default)]
    pub background_removal_mask_path: String,
}

/// The rendering/display settings of a [`ClipInstance`] that can be copied onto a different
/// block without touching its structural fields (`id`, `asset_id`, start/trim, composite
/// membership). Used by `ui`'s "copiar formatação" feature (`Ctrl+Shift+C`/`V`) and by its
/// layer-template feature (`request.md`'s Fase 4 "Templates de grupo de camadas") — a saved
/// template is a named `Vec<(TrackKind, ClipFormatting)>` persisted to `ui`'s `PrefsState`,
/// hence `Serialize`/`Deserialize` here (every field type already round-trips through
/// `.ocproj`'s project persistence via `ClipInstance` itself, so this is no new surface).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub gain_keyframes: Vec<Keyframe<f32>>,
    pub voice_cleanup_enabled: bool,
    pub voice_cleanup_noise_floor_db: f32,
    pub voice_cleanup_compressor_threshold_db: f32,
    pub voice_cleanup_compressor_ratio: f32,
    pub voice_cleanup_ceiling_linear: f32,
    pub brightness_keyframes: Vec<Keyframe<f32>>,
    pub contrast_keyframes: Vec<Keyframe<f32>>,
    pub saturation_keyframes: Vec<Keyframe<f32>>,
    pub crop_x_keyframes: Vec<Keyframe<f32>>,
    pub crop_y_keyframes: Vec<Keyframe<f32>>,
    pub crop_w_keyframes: Vec<Keyframe<f32>>,
    pub crop_h_keyframes: Vec<Keyframe<f32>>,
    pub deflicker_enabled: bool,
    pub lut_path: String,
    pub layer_scale_x: f32,
    pub layer_scale_y: f32,
    pub stabilization_intensity: f32,
}

/// A named, reusable group of layers (per `request.md`'s Fase 4 "Templates de grupo de
/// camadas" spec, e.g. "webcam recortada + fundo com blur + jogo centralizado") — each entry
/// pairs a [`TrackKind`] with the [`ClipFormatting`] to apply to whatever source clip the user
/// picks for that layer when the template is applied. Order matters (it's the layer stacking
/// order the template was saved with) but nothing else about the *source* clips (asset,
/// timing, track identity) is captured — those are supplied fresh each time the template is
/// applied, which is the whole point: reapply the same look to different footage without
/// reconfiguring effect-by-effect. Persisted in `ui`'s `PrefsState` (app-wide, not per-project,
/// since a template is meant to be reused across projects/shorts), not `.ocproj` — plain
/// top-level `Vec<LayerTemplate>` there, not nested in `Project`/`Sequence`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerTemplate {
    pub name: String,
    pub layers: Vec<(TrackKind, ClipFormatting)>,
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

fn default_voice_cleanup_noise_floor_db() -> f32 {
    -30.0
}

fn default_voice_cleanup_compressor_threshold_db() -> f32 {
    -18.0
}

fn default_voice_cleanup_compressor_ratio() -> f32 {
    3.0
}

fn default_voice_cleanup_ceiling_linear() -> f32 {
    0.95
}

impl ClipInstance {
    /// How long this instance plays for, i.e. its trimmed length — not the source asset's
    /// full duration. Uses [`keyframe::smooth_speed_ramp_duration_secs`]'s log-based integral
    /// when [`ClipInstance::speed_ramp_end_factor`] is `Some` (a smooth ramp doesn't compress
    /// time by a plain constant divisor), plain division otherwise.
    pub fn duration_secs(&self) -> f64 {
        let source_duration = self.source_out_secs - self.source_in_secs;
        match self.speed_ramp_end_factor {
            Some(end_speed) => keyframe::smooth_speed_ramp_duration_secs(
                source_duration,
                self.speed_factor,
                end_speed,
            ),
            None => source_duration / self.speed_factor as f64,
        }
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

    /// `true` if this is a compound clip (nested sequence) — see
    /// [`ClipInstance::nested_sequence_id`]'s own doc comment.
    pub fn is_nested_sequence(&self) -> bool {
        self.nested_sequence_id.is_some()
    }

    /// `true` if a layer mask ([`ClipInstance::mask_shape`]) is applied.
    pub fn is_masked(&self) -> bool {
        self.mask_shape != MaskShape::None
    }

    /// `true` if a color filter ([`ClipInstance::color_filter`]) is applied.
    pub fn is_color_filtered(&self) -> bool {
        self.color_filter != ColorFilter::None
    }

    /// `true` if a 3D LUT ([`ClipInstance::lut_path`]) is applied.
    pub fn has_lut(&self) -> bool {
        !self.lut_path.is_empty()
    }

    /// `true` if this block's layer footprint ([`ClipInstance::layer_scale_x`]/`_y`) differs
    /// from native size on either axis.
    pub fn has_layer_scale(&self) -> bool {
        (self.layer_scale_x - 1.0).abs() > 1e-4 || (self.layer_scale_y - 1.0).abs() > 1e-4
    }

    /// `true` if [`ClipInstance::stabilization_intensity`] is above zero.
    pub fn has_stabilization(&self) -> bool {
        self.stabilization_intensity > 0.0
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

    /// `true` if this block has any audio gain keyframes (overriding the constant `gain_db`).
    pub fn has_gain_keyframes(&self) -> bool {
        !self.gain_keyframes.is_empty()
    }

    /// `true` if this block has color-grading keyframes on any of brightness/contrast/
    /// saturation — gates whether [`Self::video_filter_chain`]'s static `eq` stage should defer
    /// to [`Self::keyframe_video_filter_chain`]'s animated one instead.
    pub fn has_color_keyframes(&self) -> bool {
        !self.brightness_keyframes.is_empty()
            || !self.contrast_keyframes.is_empty()
            || !self.saturation_keyframes.is_empty()
    }

    /// `true` if this block has crop/pan keyframes on any of x/y/width/height — gates whether
    /// [`Self::video_filter_chain`]'s static `crop` stage should defer to
    /// [`Self::keyframe_video_filter_chain`]'s animated one instead.
    pub fn has_crop_keyframes(&self) -> bool {
        !self.crop_x_keyframes.is_empty()
            || !self.crop_y_keyframes.is_empty()
            || !self.crop_w_keyframes.is_empty()
            || !self.crop_h_keyframes.is_empty()
    }

    /// Builds this clip's crop/scale/rotation/opacity/color-balance keyframe avfilter fragment,
    /// spliced into the per-clip chain before [`ClipInstance::video_filter_chain`]'s own stages
    /// — the same position the old `zoom` stage used to occupy. `None` if none of the five are
    /// animated. Position keyframes aren't part of this — they apply to the *overlay*
    /// compositing stage, not a per-clip filter (see [`keyframe::position_overlay_xy_expr`] and
    /// `crate::render`). Crop keyframes run first (mirroring the static `crop` stage's own
    /// traditional "runs before every other effect" position in `video_filter_chain`, so a
    /// crop/pan animation reframes the source before scale/rotate/color-balance operate on it).
    /// Color-balance keyframes running here at all (rather than in their usual post-crop/
    /// deflicker/stabilization spot in `video_filter_chain`) is a real, narrow ordering caveat —
    /// see [`ClipInstance::brightness_keyframes`]'s doc comment.
    pub fn keyframe_video_filter_chain(
        &self,
        fps_num: u32,
        fps_den: u32,
        timeline_duration_secs: f64,
    ) -> Option<String> {
        let mut stages = Vec::new();
        if self.has_crop_keyframes() {
            if let Some(crop) = keyframe::crop_filter_expr(
                &self.crop_x_keyframes,
                &self.crop_y_keyframes,
                &self.crop_w_keyframes,
                &self.crop_h_keyframes,
                self.crop_x,
                self.crop_y,
                self.crop_w,
                self.crop_h,
                fps_num,
                fps_den,
                timeline_duration_secs,
            ) {
                stages.push(crop);
            }
        }
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
        if self.has_color_keyframes() {
            if let Some(eq) = keyframe::color_balance_filter_expr(
                &self.brightness_keyframes,
                &self.contrast_keyframes,
                &self.saturation_keyframes,
                self.brightness,
                self.contrast,
                self.saturation,
                timeline_duration_secs,
            ) {
                stages.push(eq);
            }
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
    /// video, and isn't part of this chain. `speed_factor` is handled in the native bridge;
    /// general keyframes are compiled separately by [`ClipInstance::keyframe_video_filter_chain`].
    /// `mask_shape`'s alpha only survives to the rendered
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

        if self.is_cropped() && !self.has_crop_keyframes() {
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
        if self.has_stabilization() {
            // rx/ry (search radius in pixels, deshake's valid range 0..64) scale with
            // intensity rather than a fixed radius — a small radius only corrects gentle
            // handheld wobble, a large one can also absorb bigger jolts, at the cost of more
            // aggressive cropping into the frame at the edges (deshake's own trade-off, not
            // something this stage compensates for). Runs right after deflicker, before any
            // color/stylistic stage reshapes the pixel data motion estimation reads.
            let radius = (4.0 + self.stabilization_intensity.clamp(0.0, 1.0) * 60.0).round() as i32;
            stages.push(format!("deshake=rx={radius}:ry={radius}:edge=mirror"));
        }
        if !self.has_color_keyframes()
            && (self.brightness != 0.0 || self.contrast != 1.0 || self.saturation != 1.0)
        {
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
        if self.has_lut() {
            // Forward slashes even on Windows sidesteps avfilter's own backslash-escaping rules
            // inside a quoted option value (ffmpeg accepts `/`-separated paths on any platform);
            // a literal single quote in the path (the one character `'...'` quoting can't pass
            // through unescaped) is escaped avfilter-style.
            let escaped = self.lut_path.replace('\\', "/").replace('\'', "'\\''");
            stages.push(format!("lut3d=file='{escaped}'"));
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
            gain_keyframes: self.gain_keyframes.clone(),
            voice_cleanup_enabled: self.voice_cleanup_enabled,
            voice_cleanup_noise_floor_db: self.voice_cleanup_noise_floor_db,
            voice_cleanup_compressor_threshold_db: self.voice_cleanup_compressor_threshold_db,
            voice_cleanup_compressor_ratio: self.voice_cleanup_compressor_ratio,
            voice_cleanup_ceiling_linear: self.voice_cleanup_ceiling_linear,
            brightness_keyframes: self.brightness_keyframes.clone(),
            contrast_keyframes: self.contrast_keyframes.clone(),
            saturation_keyframes: self.saturation_keyframes.clone(),
            crop_x_keyframes: self.crop_x_keyframes.clone(),
            crop_y_keyframes: self.crop_y_keyframes.clone(),
            crop_w_keyframes: self.crop_w_keyframes.clone(),
            crop_h_keyframes: self.crop_h_keyframes.clone(),
            deflicker_enabled: self.deflicker_enabled,
            lut_path: self.lut_path.clone(),
            layer_scale_x: self.layer_scale_x,
            layer_scale_y: self.layer_scale_y,
            stabilization_intensity: self.stabilization_intensity,
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
        self.gain_keyframes = f.gain_keyframes.clone();
        self.voice_cleanup_enabled = f.voice_cleanup_enabled;
        self.voice_cleanup_noise_floor_db = f.voice_cleanup_noise_floor_db;
        self.voice_cleanup_compressor_threshold_db = f.voice_cleanup_compressor_threshold_db;
        self.voice_cleanup_compressor_ratio = f.voice_cleanup_compressor_ratio;
        self.voice_cleanup_ceiling_linear = f.voice_cleanup_ceiling_linear;
        self.brightness_keyframes = f.brightness_keyframes.clone();
        self.contrast_keyframes = f.contrast_keyframes.clone();
        self.saturation_keyframes = f.saturation_keyframes.clone();
        self.crop_x_keyframes = f.crop_x_keyframes.clone();
        self.crop_y_keyframes = f.crop_y_keyframes.clone();
        self.crop_w_keyframes = f.crop_w_keyframes.clone();
        self.crop_h_keyframes = f.crop_h_keyframes.clone();
        self.deflicker_enabled = f.deflicker_enabled;
        self.lut_path = f.lut_path.clone();
        self.layer_scale_x = f.layer_scale_x;
        self.layer_scale_y = f.layer_scale_y;
        self.stabilization_intensity = f.stabilization_intensity;
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

    /// Shifts which part of the source media this clip shows by `delta_secs`, without moving
    /// it on the timeline or changing its duration — Premiere/DaVinci/FCP's "Slip" tool
    /// (`ROADMAP.md` P2 item 11). `source_in_secs` and `source_out_secs` move together. No-op
    /// (`false`) if that would push `source_in_secs` below zero, or (when
    /// `max_source_out_secs` is known — the source asset's own duration) `source_out_secs`
    /// past the end of the actual source media.
    pub fn slip(&mut self, delta_secs: f64, max_source_out_secs: Option<f64>) -> bool {
        let new_source_in_secs = self.source_in_secs + delta_secs;
        let new_source_out_secs = self.source_out_secs + delta_secs;
        if new_source_in_secs < 0.0 {
            return false;
        }
        if let Some(max) = max_source_out_secs {
            if new_source_out_secs > max {
                return false;
            }
        }
        self.source_in_secs = new_source_in_secs;
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
    /// Shape overlays on this track. Only populated when `kind == TrackKind::Shape`; always
    /// empty for other track kinds. `#[serde(default)]` so projects saved before this field
    /// existed load without error.
    #[serde(default)]
    pub shape_clips: Vec<ShapeClip>,
    /// Whether this track contributes to export and preview. Toggled from the timeline track
    /// header. Defaults to `true`; missing in project files saved before this field was added
    /// deserializes as `true` via the serde default so existing projects are unaffected.
    #[serde(default = "default_true")]
    pub visible: bool,
    /// Which audio source this track carries, if the user has said — see [`AudioRole`].
    /// `#[serde(default)]` so a project saved before this field existed loads with every track
    /// `Unspecified`, same as a never-tagged track in a new project.
    #[serde(default)]
    pub audio_role: AudioRole,
    /// Optional RGB color label for this track, per `matrix/competitor-parity.md`'s 2026-08-27
    /// update (`spec/ROADMAP.md` P4 item 27) — DaVinci Resolve's track color, called out by
    /// users as something Premiere still lacks. Purely cosmetic, same "at-a-glance
    /// organization" role as [`ClipInstance::color_label`]. `None` = use the usual track-header
    /// coloring. `#[serde(default)]` so older saved projects load with no label.
    #[serde(default)]
    pub color_label: Option<[u8; 3]>,
    /// Whether this track refuses clip moves/trims from its header's lock toggle — matches
    /// `oca-editor-mock.html`'s `.tl-track-tool` lock icon. Distinct from `visible`: a locked
    /// track still renders/exports, it just can't be edited by accident while working on other
    /// tracks nearby. `#[serde(default)]` so older saved projects load with every track
    /// unlocked.
    #[serde(default)]
    pub locked: bool,
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
        let source_duration = clip.source_out_secs - clip.source_in_secs;
        let timeline_elapsed = at_secs - clip.start_secs;
        // A smooth ramp's source time isn't linear in timeline time (that's the whole point of
        // the ramp), so the plain `elapsed * speed_factor` a constant-speed clip uses would land
        // the split at the wrong source frame — use the ramp's own inverse instead. This is
        // mathematically identical to the old formula in the non-ramped (overwhelmingly common)
        // case, not a behavior change there: keyframe::smooth_speed_ramp_source_secs_at's own
        // `v1 == v0` fallback is exactly `timeline_elapsed * speed_factor`.
        let split_source_secs = clip.source_in_secs
            + match clip.speed_ramp_end_factor {
                Some(end_speed) => keyframe::smooth_speed_ramp_source_secs_at(
                    timeline_elapsed,
                    source_duration,
                    clip.speed_factor,
                    end_speed,
                ),
                None => timeline_elapsed * clip.speed_factor as f64,
            };
        // Fraction of the *source* range consumed by the split — what keyframes (stored in
        // source-relative `0.0..=1.0`, see evaluate_keyframes' callers) actually need. Equal to
        // the old `(at_secs - start) / clip.duration_secs()` timeline-fraction in the
        // non-ramped case (both reduce to the same ratio when speed is constant), so this is a
        // safe drop-in even though it's phrased differently.
        let split_frac = if source_duration > 1e-9 {
            ((split_source_secs - clip.source_in_secs) / source_duration) as f32
        } else {
            0.0
        };
        // A ramp doesn't survive a split as one continuous curve — each half becomes its own
        // ramp instead, sharing the same speed-at-the-cut boundary so there's no audible/visual
        // jump right at the split (same "no jump at the cut" goal split_keyframes_at's own doc
        // comment describes for keyframes).
        let speed_at_split = clip.speed_ramp_end_factor.map(|end_speed| {
            let b = (end_speed as f64 - clip.speed_factor as f64) / source_duration.max(1e-9);
            (clip.speed_factor as f64 + b * (split_source_secs - clip.source_in_secs)) as f32
        });
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
        let (gain_first, gain_second) =
            keyframe::split_keyframes_at(&clip.gain_keyframes, split_frac, 0.0);
        let (brightness_first, brightness_second) =
            keyframe::split_keyframes_at(&clip.brightness_keyframes, split_frac, 0.0);
        let (contrast_first, contrast_second) =
            keyframe::split_keyframes_at(&clip.contrast_keyframes, split_frac, 1.0);
        let (saturation_first, saturation_second) =
            keyframe::split_keyframes_at(&clip.saturation_keyframes, split_frac, 1.0);
        let (crop_x_first, crop_x_second) =
            keyframe::split_keyframes_at(&clip.crop_x_keyframes, split_frac, 0.0);
        let (crop_y_first, crop_y_second) =
            keyframe::split_keyframes_at(&clip.crop_y_keyframes, split_frac, 0.0);
        let (crop_w_first, crop_w_second) =
            keyframe::split_keyframes_at(&clip.crop_w_keyframes, split_frac, 1.0);
        let (crop_h_first, crop_h_second) =
            keyframe::split_keyframes_at(&clip.crop_h_keyframes, split_frac, 1.0);
        let second_half = ClipInstance {
            id: new_clip_id,
            asset_id: clip.asset_id,
            start_secs: at_secs,
            source_in_secs: split_source_secs,
            source_out_secs: clip.source_out_secs,
            // Splitting a composite member must not silently ungroup it from the rest of the
            // block.
            composite_id: clip.composite_id,
            color_label: clip.color_label,
            gain_db: clip.gain_db,
            frozen: clip.frozen,
            // Second half's own ramp starts at the split boundary's speed and rides out to the
            // original clip's end speed, unchanged.
            speed_factor: speed_at_split.unwrap_or(clip.speed_factor),
            speed_ramp_end_factor: clip.speed_ramp_end_factor,
            nested_sequence_id: clip.nested_sequence_id,
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
            gain_keyframes: gain_second,
            voice_cleanup_enabled: clip.voice_cleanup_enabled,
            voice_cleanup_noise_floor_db: clip.voice_cleanup_noise_floor_db,
            voice_cleanup_compressor_threshold_db: clip.voice_cleanup_compressor_threshold_db,
            voice_cleanup_compressor_ratio: clip.voice_cleanup_compressor_ratio,
            voice_cleanup_ceiling_linear: clip.voice_cleanup_ceiling_linear,
            brightness_keyframes: brightness_second,
            contrast_keyframes: contrast_second,
            saturation_keyframes: saturation_second,
            crop_x_keyframes: crop_x_second,
            crop_y_keyframes: crop_y_second,
            crop_w_keyframes: crop_w_second,
            crop_h_keyframes: crop_h_second,
            deflicker_enabled: clip.deflicker_enabled,
            lut_path: clip.lut_path.clone(),
            layer_scale_x: clip.layer_scale_x,
            layer_scale_y: clip.layer_scale_y,
            stabilization_intensity: clip.stabilization_intensity,
            // Not carried over: the matte at `clip.background_removal_mask_path` (if any) was
            // generated for the pre-split `source_in_secs..source_out_secs` range, which no
            // longer matches either half after the split — a stale matte pointing at the wrong
            // frame range, silently wrong. `false`/empty until re-generated for this half.
            background_removal_enabled: false,
            background_removal_mask_path: String::new(),
        };
        clip.source_out_secs = split_source_secs;
        // First half's own ramp rides from the original start speed to the split boundary's
        // speed — mirrors the second half's own comment above.
        if let Some(speed_at_split) = speed_at_split {
            clip.speed_ramp_end_factor = Some(speed_at_split);
        }
        clip.position_keyframes = position_first;
        clip.scale_keyframes = scale_first;
        clip.rotation_keyframes = rotation_first;
        clip.opacity_keyframes = opacity_first;
        clip.gain_keyframes = gain_first;
        clip.brightness_keyframes = brightness_first;
        clip.contrast_keyframes = contrast_first;
        clip.saturation_keyframes = saturation_first;
        clip.crop_x_keyframes = crop_x_first;
        clip.crop_y_keyframes = crop_y_first;
        clip.crop_w_keyframes = crop_w_first;
        clip.crop_h_keyframes = crop_h_first;
        // Same staleness reasoning as the second half above — the original clip's own trimmed
        // range changed too.
        clip.background_removal_enabled = false;
        clip.background_removal_mask_path = String::new();

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

    /// The id of the clip immediately before `clip_id` on this track (the one with the
    /// greatest `start_secs` that's still less than `clip_id`'s own) — `None` if `clip_id`
    /// isn't on this track or is already the earliest one. Shared by the roll/slide edits
    /// below to find which neighbor a shared edge/absorbed gap belongs to.
    pub fn previous_clip_id(&self, clip_id: u64) -> Option<u64> {
        let this_start = self.clips.iter().find(|c| c.id == clip_id)?.start_secs;
        self.clips
            .iter()
            .filter(|c| c.id != clip_id && c.start_secs < this_start)
            .max_by(|a, b| a.start_secs.total_cmp(&b.start_secs))
            .map(|c| c.id)
    }

    /// The id of the clip immediately after `clip_id` on this track, by the mirror-image rule
    /// [`Self::previous_clip_id`] uses.
    pub fn next_clip_id(&self, clip_id: u64) -> Option<u64> {
        let this_start = self.clips.iter().find(|c| c.id == clip_id)?.start_secs;
        self.clips
            .iter()
            .filter(|c| c.id != clip_id && c.start_secs > this_start)
            .min_by(|a, b| a.start_secs.total_cmp(&b.start_secs))
            .map(|c| c.id)
    }

    /// Trims `clip_id`'s start to `new_start_secs`, then shifts every clip whose `start_secs`
    /// is past `clip_id`'s own (pre-trim) start by the same delta — Premiere/DaVinci/FCP's
    /// "Ripple" tool (`ROADMAP.md` P2 item 11): no gap is left behind, later clips slide to
    /// fill it. No-op (`false`) if the clip isn't on this track or the underlying
    /// [`ClipInstance::trim_start`] refuses the edit — nothing is shifted in that case.
    pub fn ripple_trim_start(
        &mut self,
        clip_id: u64,
        new_start_secs: f64,
        min_duration_secs: f64,
    ) -> bool {
        let Some(index) = self.clips.iter().position(|c| c.id == clip_id) else {
            return false;
        };
        let old_start_secs = self.clips[index].start_secs;
        if !self.clips[index].trim_start(new_start_secs, min_duration_secs) {
            return false;
        }
        let delta = new_start_secs - old_start_secs;
        for clip in &mut self.clips {
            if clip.id != clip_id && clip.start_secs > old_start_secs {
                clip.start_secs = (clip.start_secs + delta).max(0.0);
            }
        }
        true
    }

    /// [`Self::ripple_trim_start`]'s mirror for the clip's *end* edge: trims `clip_id`'s end to
    /// `new_end_secs`, then shifts every clip whose `start_secs` is past `clip_id`'s own by the
    /// resulting duration delta.
    pub fn ripple_trim_end(
        &mut self,
        clip_id: u64,
        new_end_secs: f64,
        min_duration_secs: f64,
        max_source_out_secs: Option<f64>,
    ) -> bool {
        let Some(index) = self.clips.iter().position(|c| c.id == clip_id) else {
            return false;
        };
        let this_start = self.clips[index].start_secs;
        let old_end_secs = this_start + self.clips[index].duration_secs();
        if !self.clips[index].trim_end(new_end_secs, min_duration_secs, max_source_out_secs) {
            return false;
        }
        let delta = new_end_secs - old_end_secs;
        for clip in &mut self.clips {
            if clip.id != clip_id && clip.start_secs > this_start {
                clip.start_secs = (clip.start_secs + delta).max(0.0);
            }
        }
        true
    }

    /// Moves the cut point between `clip_id` and its immediate next neighbor
    /// ([`Self::next_clip_id`]) to `new_boundary_secs` — Premiere/DaVinci/FCP's "Roll" tool
    /// (`ROADMAP.md` P2 item 11): `clip_id`'s end and the neighbor's start move together, so
    /// the pair's combined timeline span (and every other clip's position) stays unchanged.
    /// No-op (`false`) if `clip_id` has no next neighbor on this track, or the edit would
    /// violate either clip's own trim bounds — applied atomically: a rejection on either side
    /// leaves both clips exactly as they were, never a half-rolled pair.
    pub fn roll_edit(
        &mut self,
        clip_id: u64,
        new_boundary_secs: f64,
        min_duration_secs: f64,
        this_max_source_out_secs: Option<f64>,
    ) -> bool {
        let Some(index) = self.clips.iter().position(|c| c.id == clip_id) else {
            return false;
        };
        let Some(next_id) = self.next_clip_id(clip_id) else {
            return false;
        };
        let next_index = self
            .clips
            .iter()
            .position(|c| c.id == next_id)
            .expect("next_clip_id only ever returns an id present on this track");

        let this_snapshot = self.clips[index].clone();
        let next_snapshot = self.clips[next_index].clone();
        let next_old_start = self.clips[next_index].start_secs;

        let this_ok = self.clips[index].trim_end(
            new_boundary_secs,
            min_duration_secs,
            this_max_source_out_secs,
        );
        let next_delta = new_boundary_secs - next_old_start;
        let next_new_start = next_old_start + next_delta;
        // trim_start never needs a max-source-out bound: it only moves source_in toward
        // source_out (which stays fixed), never past it.
        let next_ok =
            this_ok && self.clips[next_index].trim_start(next_new_start, min_duration_secs);

        if !next_ok {
            self.clips[index] = this_snapshot;
            self.clips[next_index] = next_snapshot;
            return false;
        }
        true
    }

    /// Moves `clip_id` to `new_start_secs` on this track, keeping its own duration/source
    /// range unchanged — Premiere/DaVinci/FCP's "Slide" tool (`ROADMAP.md` P2 item 11): the
    /// immediate previous and next clips ([`Self::previous_clip_id`]/[`Self::next_clip_id`],
    /// resolved against `clip_id`'s *pre-move* position) absorb the movement by adjusting their
    /// own out/in points to meet `clip_id`'s new position, so nothing else on the track shifts.
    /// No-op (`false`) if `new_start_secs` is negative, `clip_id` isn't on this track, or
    /// either affected neighbor's own trim bounds would be violated — applied atomically, same
    /// as [`Self::roll_edit`]. A neighbor that doesn't exist (`clip_id` is first/last on the
    /// track) simply isn't adjusted on that side.
    pub fn slide_clip(
        &mut self,
        clip_id: u64,
        new_start_secs: f64,
        min_duration_secs: f64,
        prev_max_source_out_secs: Option<f64>,
    ) -> bool {
        if new_start_secs < 0.0 {
            return false;
        }
        let Some(index) = self.clips.iter().position(|c| c.id == clip_id) else {
            return false;
        };
        let this_duration = self.clips[index].duration_secs();
        let new_end_secs = new_start_secs + this_duration;

        let prev_index = self
            .previous_clip_id(clip_id)
            .and_then(|id| self.clips.iter().position(|c| c.id == id));
        let next_index = self
            .next_clip_id(clip_id)
            .and_then(|id| self.clips.iter().position(|c| c.id == id));

        let snapshots: Vec<(usize, ClipInstance)> = [Some(index), prev_index, next_index]
            .into_iter()
            .flatten()
            .map(|i| (i, self.clips[i].clone()))
            .collect();

        let prev_ok = match prev_index {
            Some(i) => {
                self.clips[i].trim_end(new_start_secs, min_duration_secs, prev_max_source_out_secs)
            }
            None => true,
        };
        // next's trim_start never needs a max-source-out bound, same reasoning as roll_edit.
        let next_ok = prev_ok
            && match next_index {
                Some(i) => self.clips[i].trim_start(new_end_secs, min_duration_secs),
                None => true,
            };

        if !next_ok {
            for (i, snapshot) in snapshots {
                self.clips[i] = snapshot;
            }
            return false;
        }
        self.clips[index].start_secs = new_start_secs;
        true
    }

    /// Removes the timeline range `[start_secs, end_secs)` from this track and ripples every
    /// later clip left to close the gap — "ripple delete," what D1's silence-gap review
    /// (`spec/architecture/differentiators.md`) applies to each accepted
    /// [`crate::silence_detection::SilenceGap`]. Any clip straddling either boundary is split
    /// first via [`Self::split_clip_at`] (each split, if performed, consumes one id from
    /// `next_clip_id` and increments it — a no-op split at an exact boundary leaves it
    /// untouched), then every clip now falling fully inside the range is dropped, and every
    /// clip starting at or after `end_secs` shifts left by the removed span. Only `self.clips`
    /// is affected, matching [`Self::ripple_trim_start`]/[`Self::ripple_trim_end`]'s existing
    /// scope (text/shape overlay tracks are never video/audio tracks, so this never applies to
    /// them). No-op (`false`, `next_clip_id` untouched) if `end_secs <= start_secs`.
    pub fn ripple_delete_range(
        &mut self,
        start_secs: f64,
        end_secs: f64,
        next_clip_id: &mut u64,
    ) -> bool {
        if end_secs <= start_secs {
            return false;
        }

        if self.split_clip_at(start_secs, *next_clip_id) {
            *next_clip_id += 1;
        }
        if self.split_clip_at(end_secs, *next_clip_id) {
            *next_clip_id += 1;
        }

        const EPSILON: f64 = 1e-6;
        let removed_span = end_secs - start_secs;
        self.clips.retain(|c| {
            let c_end = c.start_secs + c.duration_secs();
            !(c.start_secs >= start_secs - EPSILON && c_end <= end_secs + EPSILON)
        });
        for clip in &mut self.clips {
            if clip.start_secs >= end_secs - EPSILON {
                clip.start_secs = (clip.start_secs - removed_span).max(0.0);
            }
        }
        true
    }

    /// The position, in seconds, where this track's last clip ends. `0.0` for an empty track —
    /// the natural "append here" position for a clip added to this track. Accounts for
    /// [`ClipInstance`]s, [`TextClip`]s, and [`ShapeClip`]s so every track kind reports its own
    /// length correctly.
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
        let shape_end = self
            .shape_clips
            .iter()
            .map(|s| s.start_secs + s.duration_secs)
            .fold(0.0, f64::max);
        clips_end.max(text_end).max(shape_end)
    }
}

/// A review/comment marker's category — Final Cut Pro's typed-marker model (per `ROADMAP.md`
/// P2 item 9), not just a plain unstyled note: `ToDo` tracks a `completed` state a searchable
/// Timeline Index panel can filter on, `Chapter` marks a navigable section boundary, `Standard`
/// is a plain annotation. `Highlight` (D2, `spec/architecture/differentiators.md`) marks an
/// auto-detected candidate moment — same non-destructive "add a marker, let the existing
/// Timeline Index panel's rename/delete be the review step" shape D4's Chapter markers already
/// established, rather than a separate accept/reject modal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MarkerKind {
    #[default]
    Standard,
    ToDo,
    Chapter,
    Highlight,
}

impl MarkerKind {
    pub const ALL: &'static [MarkerKind] = &[
        MarkerKind::Standard,
        MarkerKind::ToDo,
        MarkerKind::Chapter,
        MarkerKind::Highlight,
    ];
}

/// One review/comment marker on the timeline — a point in time (not a clip, not tied to any
/// particular track) with a short label and a [`MarkerKind`]. `id`s are unique within a
/// [`Timeline`], same convention [`ClipInstance::id`]/[`TextClip::id`] already use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    pub id: u64,
    pub position_secs: f64,
    pub label: String,
    pub kind: MarkerKind,
    /// Only meaningful for [`MarkerKind::ToDo`] — a searchable Timeline Index panel can filter
    /// these out once resolved without deleting the marker (the review history stays visible).
    #[serde(default)]
    pub completed: bool,
}

/// A project's full set of tracks plus the current playhead position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    pub tracks: Vec<Track>,
    pub playhead_secs: f64,
    /// Review/comment markers — `#[serde(default)]` so a project saved before this field
    /// existed still loads (empty marker list), per this crate's struct-map `.ocproj` format
    /// (see `CLAUDE.md`).
    #[serde(default)]
    pub markers: Vec<Marker>,
    /// Multicam groups (P2 item 10, "Multicam editing") — `#[serde(default)]` so a project saved
    /// before this field existed still loads (empty group list), same convention as `markers`.
    #[serde(default)]
    pub multicam_groups: Vec<MulticamGroup>,
}

/// A set of `Video` tracks recorded simultaneously from different sources (e.g. game capture,
/// webcam, mic-facecam) that have been synced against each other by [`crate::multicam_sync`], so
/// switching which one plays at a given moment ("switch to angle 2") is a matter of retargeting
/// a clip on `program_track_id` to the right other member's asset at the right (offset-adjusted)
/// source time — see [`Timeline::switch_multicam_angle`]. Angles are ordinary [`Track`]s, not a
/// new [`TrackKind`]: this group is purely a sidecar grouping + offset record, same non-invasive
/// shape [`Marker`] uses, so no existing per-track/per-clip code needs to know about multicam at
/// all except `program_track_id`'s visibility (every non-program member is hidden — see
/// [`Timeline::add_multicam_group`] — so only the currently active angle actually renders/
/// exports; the others stay in the project purely as switchable source material).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MulticamGroup {
    pub id: u64,
    pub name: String,
    /// Every track in this group, `program_track_id` included, in a stable, user-meaningful
    /// order — index 0 is "angle 1", index 1 is "angle 2", etc., what number-key angle switching
    /// refers to.
    pub member_track_ids: Vec<u64>,
    /// Which member is currently the one visible/exported track. Always one of
    /// `member_track_ids`.
    pub program_track_id: u64,
    /// Each member track's sync offset in seconds, relative to every other member: for the same
    /// real-world moment, `member_track_time = other_member_track_time + (offset[member] -
    /// offset[other_member])`. The track chosen as the sync reference when the group was created
    /// has offset `0.0`; every other member's offset is `crate::multicam_sync::
    /// compute_sync_offset_secs`'s result against that reference. Absent an entry (shouldn't
    /// happen for a member_track_ids member, but keeps lookups total) is treated as `0.0`.
    pub sync_offsets_secs: HashMap<u64, f64>,
}

impl MulticamGroup {
    /// This member's sync offset, or `0.0` if `track_id` isn't in `sync_offsets_secs` (should
    /// only happen for a `track_id` that isn't actually a member of this group).
    pub fn offset_secs(&self, track_id: u64) -> f64 {
        self.sync_offsets_secs
            .get(&track_id)
            .copied()
            .unwrap_or(0.0)
    }
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

    /// Adds a new [`Marker`] at `position_secs` (clamped to `0.0`) with `kind`, `label` empty
    /// and `completed: false`, and returns its freshly assigned id — one past the highest
    /// existing marker id, `1` if there are none yet, same "max + 1" convention every other
    /// timeline entity's id assignment already uses (see `next_clip_id` in `ui`).
    pub fn add_marker(&mut self, position_secs: f64, kind: MarkerKind) -> u64 {
        let id = self.markers.iter().map(|m| m.id).max().unwrap_or(0) + 1;
        self.markers.push(Marker {
            id,
            position_secs: position_secs.max(0.0),
            label: String::new(),
            kind,
            completed: false,
        });
        id
    }

    /// Removes the marker with `marker_id`, if any. `true` if a marker was actually removed.
    pub fn remove_marker(&mut self, marker_id: u64) -> bool {
        let before = self.markers.len();
        self.markers.retain(|m| m.id != marker_id);
        self.markers.len() != before
    }

    /// Mutable access to the marker with `marker_id`, if it exists.
    pub fn marker_mut(&mut self, marker_id: u64) -> Option<&mut Marker> {
        self.markers.iter_mut().find(|m| m.id == marker_id)
    }

    /// Every marker sorted by `position_secs` ascending — what a searchable Timeline Index
    /// panel (and the ruler's own left-to-right tick rendering) both want, rather than
    /// insertion order.
    pub fn markers_sorted(&self) -> Vec<&Marker> {
        let mut markers: Vec<&Marker> = self.markers.iter().collect();
        markers.sort_by(|a, b| a.position_secs.total_cmp(&b.position_secs));
        markers
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

impl Timeline {
    /// Adds a new [`MulticamGroup`] over `member_track_ids` (must have at least 2 members, all
    /// existing `Video` tracks, or this is a no-op returning `None`) with `program_track_id`'s
    /// offset implicitly `0.0` unless overridden in `sync_offsets_secs`, and hides every other
    /// member (`Track::visible = false`) so only the program track actually renders/exports —
    /// the rest stay in the project purely as switchable source material for
    /// [`Timeline::switch_multicam_angle`]. Returns the freshly assigned group id ("max + 1",
    /// same convention every other timeline entity's id assignment uses), or `None` if
    /// `program_track_id` isn't one of `member_track_ids`, fewer than 2 members are given, or
    /// any member id doesn't name an existing `Video` track.
    pub fn add_multicam_group(
        &mut self,
        name: String,
        member_track_ids: Vec<u64>,
        program_track_id: u64,
        sync_offsets_secs: HashMap<u64, f64>,
    ) -> Option<u64> {
        if member_track_ids.len() < 2 || !member_track_ids.contains(&program_track_id) {
            return None;
        }
        if !member_track_ids.iter().all(|id| {
            self.tracks
                .iter()
                .any(|t| t.id == *id && t.kind == TrackKind::Video)
        }) {
            return None;
        }

        for track_id in &member_track_ids {
            if *track_id == program_track_id {
                continue;
            }
            if let Some(track) = self.tracks.iter_mut().find(|t| t.id == *track_id) {
                track.visible = false;
            }
        }

        let id = self.multicam_groups.iter().map(|g| g.id).max().unwrap_or(0) + 1;
        self.multicam_groups.push(MulticamGroup {
            id,
            name,
            member_track_ids,
            program_track_id,
            sync_offsets_secs,
        });
        Some(id)
    }

    /// Mutable access to the multicam group with `group_id`, if it exists.
    pub fn multicam_group_mut(&mut self, group_id: u64) -> Option<&mut MulticamGroup> {
        self.multicam_groups.iter_mut().find(|g| g.id == group_id)
    }

    /// Removes the multicam group with `group_id` and re-shows every one of its member tracks
    /// (undoing the hide [`Timeline::add_multicam_group`] applied) — the tracks and their clips
    /// themselves are left alone, only the grouping/offset record and the non-program members'
    /// visibility are undone. `true` if a group was actually removed.
    pub fn remove_multicam_group(&mut self, group_id: u64) -> bool {
        let Some(index) = self.multicam_groups.iter().position(|g| g.id == group_id) else {
            return false;
        };
        let group = self.multicam_groups.remove(index);
        for track_id in &group.member_track_ids {
            if let Some(track) = self.tracks.iter_mut().find(|t| t.id == *track_id) {
                track.visible = true;
            }
        }
        true
    }

    /// Switches `group_id`'s active angle, at `at_secs` (timeline-relative), to
    /// `member_track_ids[angle_index]`: splits the program track's clip covering `at_secs` (via
    /// [`Track::split_clip_at`], `new_clip_id`) so the switch takes effect exactly at that point,
    /// then retargets the resulting piece's `asset_id`/`source_in_secs`/`source_out_secs` to
    /// whatever the target angle's own track was showing at the sync-offset-adjusted equivalent
    /// time — same source-window duration, different source. Resets the retargeted piece's
    /// `speed_factor` to `1.0` (a speed-ramped multicam switch is out of scope for this pass) and
    /// clears its stale `background_removal_mask_path`/keyframes the same way an ordinary split
    /// already does for its second half, since they're generated for/apply to the pre-switch
    /// source.
    ///
    /// No-op (`false`) if: the group or `angle_index` don't exist; the target angle is already
    /// the program track; the program track has no clip covering `at_secs`; or the target angle's
    /// track has no clip covering the offset-adjusted time (e.g. that source hadn't started
    /// recording yet at this moment).
    pub fn switch_multicam_angle(
        &mut self,
        group_id: u64,
        angle_index: usize,
        at_secs: f64,
        new_clip_id: u64,
    ) -> bool {
        let Some(group) = self.multicam_groups.iter().find(|g| g.id == group_id) else {
            return false;
        };
        let Some(&target_track_id) = group.member_track_ids.get(angle_index) else {
            return false;
        };
        if target_track_id == group.program_track_id {
            return false;
        }
        let program_track_id = group.program_track_id;
        let target_time =
            at_secs - group.offset_secs(program_track_id) + group.offset_secs(target_track_id);

        let Some(target_track) = self.tracks.iter().find(|t| t.id == target_track_id) else {
            return false;
        };
        let Some(target_clip) = target_track.clip_at(target_time) else {
            return false;
        };
        let new_asset_id = target_clip.asset_id;
        let new_source_in_secs = target_clip.source_in_secs
            + (target_time - target_clip.start_secs) * target_clip.speed_factor as f64;

        let Some(program_track) = self.tracks.iter_mut().find(|t| t.id == program_track_id) else {
            return false;
        };
        if program_track.clip_at(at_secs).is_none() {
            return false;
        }
        let already_at_boundary = program_track.clips.iter().any(|c| c.start_secs == at_secs);
        if !already_at_boundary && !program_track.split_clip_at(at_secs, new_clip_id) {
            return false;
        }
        // Whether pre-existing or freshly created by the split above, the piece this switch
        // targets is the one starting exactly at `at_secs`.
        let Some(switched_clip) = program_track
            .clips
            .iter_mut()
            .find(|c| c.start_secs == at_secs)
        else {
            return false;
        };
        let switched_duration_secs = (switched_clip.source_out_secs - switched_clip.source_in_secs)
            as f64
            / switched_clip.speed_factor as f64;
        switched_clip.asset_id = new_asset_id;
        switched_clip.speed_factor = 1.0;
        switched_clip.source_in_secs = new_source_in_secs;
        switched_clip.source_out_secs = new_source_in_secs + switched_duration_secs;
        switched_clip.background_removal_enabled = false;
        switched_clip.background_removal_mask_path = String::new();
        switched_clip.position_keyframes.clear();
        switched_clip.scale_keyframes.clear();
        switched_clip.rotation_keyframes.clear();
        switched_clip.opacity_keyframes.clear();
        true
    }
}
