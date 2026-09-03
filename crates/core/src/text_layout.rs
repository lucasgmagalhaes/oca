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

//! TEXT-01A: the bundled-only OpenType shaping adapter
//! ([`spec/architecture/complex-text-shaping.md`](../../../../spec/architecture/complex-text-shaping.md)),
//! built on `cosmic-text` — the candidate the doc's own acceptance spike (2026-08-28, see that
//! doc's "Spike result") proved sound before any production code changed.
//!
//! This module is the adapter (the doc's TEXT-01A steps 1-2: a provider-neutral [`ShapedText`]
//! value plus a bundled-only [`TextLayoutEngine`]) **and** step 3's rasterization consumer —
//! [`crate::overlay_render`] now shapes and paints through [`with_shared_engine`] instead of the
//! old `fontdue`-based per-character advance-summing measurement, which TEXT-01B step 4 removed
//! from `crate::text_metrics` entirely once it had no remaining caller — see that module's own
//! doc comment.
//!
//! Loads only [`crate::font_catalog`]'s locked bytes into a `fontdb::Database` via
//! `FontSystem::new_with_locale_and_db` — never `FontSystem::new()`, which the architecture doc
//! explicitly forbids because it scans installed system fonts. `cosmic-text`'s own
//! `PlatformFallback` fallback-name list can never resolve to anything outside that locked set
//! for the same reason the acceptance spike relied on: it only ever matches families already
//! present in the `fontdb::Database` it was built with.

use std::borrow::Cow;
use std::ops::Range;
use std::sync::{Mutex, OnceLock};

use cosmic_text::{
    fontdb, Align, Attrs, Buffer, Family, FontSystem, Metrics, PhysicalGlyph, Shaping, SwashCache,
    Weight,
};

use crate::font_catalog;
use crate::timeline::{TextAlign, TextDirection, TextFontFamily, TextFontStyle};

/// Left-to-right mark (U+200E) / right-to-left mark (U+200F) — invisible, zero-advance format
/// characters whose own bidi class (`L`/`R`) is "strong" for UAX #9's P2/P3 first-strong-char
/// paragraph-direction detection. Prepending one forces [`TextDirection::Ltr`]/`::Rtl` as the
/// paragraph's base level without acting as a bidi *override*: unlike LRE/RLE/LRO/RLO, embedded
/// runs of the opposite script still resolve normally within that pinned level (matching CSS's
/// `direction` property, not `unicode-bidi: bidi-override`) — see [`TextDirection`]'s own doc
/// comment. Both are 3 bytes in UTF-8.
const LEFT_TO_RIGHT_MARK: char = '\u{200E}';
const RIGHT_TO_LEFT_MARK: char = '\u{200F}';

/// The paragraph base direction [`resolve_paragraph_direction`] resolved, for
/// [`TextAlign::Start`]/[`TextAlign::End`]'s CSS-logical-property semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedDirection {
    Ltr,
    Rtl,
}

/// Resolves [`TextDirection::Auto`]'s own UAX #9 P2/P3 first-strong-character detection (via
/// `unicode_bidi::get_base_direction`, the same algorithm [`TextLayoutEngine::shape`]'s
/// bidi-mark-prefix trick pins `cosmic-text`'s internal detection to) — or the explicit override
/// when `direction` isn't `Auto`. `unicode_bidi::Direction::Mixed` (the crate's own name for "no
/// strong character found at all," despite the name — see its doc comment) resolves to `Ltr`,
/// matching UAX #9 P3's own default-to-LTR rule for an all-neutral paragraph.
pub fn resolve_paragraph_direction(text: &str, direction: TextDirection) -> ResolvedDirection {
    match direction {
        TextDirection::Ltr => ResolvedDirection::Ltr,
        TextDirection::Rtl => ResolvedDirection::Rtl,
        TextDirection::Auto => match unicode_bidi::get_base_direction(text) {
            unicode_bidi::Direction::Rtl => ResolvedDirection::Rtl,
            unicode_bidi::Direction::Ltr | unicode_bidi::Direction::Mixed => ResolvedDirection::Ltr,
        },
    }
}

/// Resolves [`TextAlign::Start`]/[`TextAlign::End`] into the physical [`TextAlign::Left`]/
/// [`TextAlign::Right`] edge their CSS-logical-property semantics mean for `text` under
/// `direction` — see [`TextAlign`]'s own doc comment. `Auto`/`Left`/`Center`/`Right` pass through
/// unchanged. The single source of truth both [`TextLayoutEngine::shape`] (per-line `cosmic-text`
/// alignment) and `crate::overlay_render`'s wrap/alignment box computation call, so the two can
/// never resolve a clip's `Start`/`End` to different physical edges.
pub fn resolve_text_align(text: &str, direction: TextDirection, align: TextAlign) -> TextAlign {
    match align {
        TextAlign::Start | TextAlign::End => {
            let is_rtl = resolve_paragraph_direction(text, direction) == ResolvedDirection::Rtl;
            let is_start = align == TextAlign::Start;
            if is_start != is_rtl {
                TextAlign::Left
            } else {
                TextAlign::Right
            }
        }
        other => other,
    }
}

/// Explicit UAX #9 directional-formatting characters this module scans for in
/// [`scan_bidi_controls`] — invisible codepoints a user's typed or pasted text can contain that
/// change how later text renders, distinct from [`LEFT_TO_RIGHT_MARK`]/[`RIGHT_TO_LEFT_MARK`]
/// (which this module itself only ever inserts internally, never reads back out of caller text).
const LRE: char = '\u{202A}'; // Left-to-Right Embedding
const RLE: char = '\u{202B}'; // Right-to-Left Embedding
const PDF: char = '\u{202C}'; // Pop Directional Formatting (closes LRE/RLE/LRO/RLO)
const LRO: char = '\u{202D}'; // Left-to-Right Override
const RLO: char = '\u{202E}'; // Right-to-Left Override
const LRI: char = '\u{2066}'; // Left-to-Right Isolate
const RLI: char = '\u{2067}'; // Right-to-Left Isolate
const FSI: char = '\u{2068}'; // First Strong Isolate
const PDI: char = '\u{2069}'; // Pop Directional Isolate

/// Whether an opened embedding/override (closed by [`PDF`]) or isolate (closed by [`PDI`]) is
/// waiting on the scan stack in [`scan_bidi_controls`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenControlKind {
    Embedding,
    Isolate,
}

/// Result of [`scan_bidi_controls`] — see the architecture doc's own "The editor should expose
/// invisible directional controls on demand" note
/// (`spec/architecture/complex-text-shaping.md`'s TEXT-01B section): a caller should surface a
/// non-blocking warning, never silently strip anything, since legitimate bidi content must
/// round-trip exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BidiControlWarning {
    /// `true` if the text contains [`LRO`]/[`RLO`] anywhere. These force every character between
    /// the override and its close (or the end of the paragraph, if never closed) to render in
    /// one direction regardless of its own script — visually misleading by design, so this is
    /// worth a warning even when perfectly well-formed.
    pub has_override: bool,
    /// `true` if the text contains an explicit directional control ([`LRE`]/[`RLE`]/[`LRO`]/
    /// [`RLO`]/[`LRI`]/[`RLI`]/[`FSI`]) with no matching close by the end of the string, or a
    /// close ([`PDF`]/[`PDI`]) with nothing open to match — either shape means the text's
    /// rendered direction depends on invisible characters a reader (and this editor's own text
    /// box) can't see.
    pub has_unmatched_control: bool,
}

impl BidiControlWarning {
    /// Whether a caller should show *any* warning at all.
    pub fn any(self) -> bool {
        self.has_override || self.has_unmatched_control
    }
}

/// Scans `text` for explicit UAX #9 directional-formatting characters and reports whether a
/// caller should show a non-blocking warning — see [`BidiControlWarning`]'s own doc comment.
/// Matching is a simple single combined stack (an opener pushes its [`OpenControlKind`]; [`PDF`]
/// pops only when the top is an `Embedding`, [`PDI`] pops only when the top is an `Isolate` —
/// matching UAX #9's own rule X7/X6a "if there is no matching code, do nothing" behavior, so a
/// stray close never incorrectly closes an unrelated open control underneath it) — not full
/// UAX #9 isolate-run resolution, which this only needs to *flag*, not actually apply (that's
/// [`TextLayoutEngine::shape`]'s job via `cosmic-text`'s own `unicode-bidi`-backed
/// implementation).
pub fn scan_bidi_controls(text: &str) -> BidiControlWarning {
    let mut has_override = false;
    let mut has_unmatched_control = false;
    let mut stack: Vec<OpenControlKind> = Vec::new();
    for c in text.chars() {
        match c {
            LRE | RLE => stack.push(OpenControlKind::Embedding),
            LRO | RLO => {
                has_override = true;
                stack.push(OpenControlKind::Embedding);
            }
            LRI | RLI | FSI => stack.push(OpenControlKind::Isolate),
            PDF => {
                if stack.last() == Some(&OpenControlKind::Embedding) {
                    stack.pop();
                } else {
                    has_unmatched_control = true;
                }
            }
            PDI => {
                if stack.last() == Some(&OpenControlKind::Isolate) {
                    stack.pop();
                } else {
                    has_unmatched_control = true;
                }
            }
            _ => {}
        }
    }
    if !stack.is_empty() {
        has_unmatched_control = true;
    }
    BidiControlWarning {
        has_override,
        has_unmatched_control,
    }
}

/// One positioned glyph within a [`ShapedLine`], already placed at its final canvas-relative
/// pixel position (the `origin` passed to [`TextLayoutEngine::shape`] is baked in).
#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    pub font_id: fontdb::ID,
    pub glyph_id: u16,
    /// Left edge of this glyph's advance box, in canvas-relative pixels.
    pub x: f32,
    /// Top edge of this glyph's advance box, in canvas-relative pixels.
    pub y: f32,
    /// Advance width in pixels.
    pub w: f32,
    /// Logical UTF-8 byte range of the cluster this glyph belongs to, within the shaped text.
    /// Multiple glyphs can share one cluster (e.g. an Indic conjunct); one glyph can also
    /// represent multiple source characters (e.g. an `fi` ligature) — this is *not* guaranteed
    /// to be exactly one character wide.
    pub cluster: Range<usize>,
    /// `true` when this glyph's UAX #9 embedding level is odd (right-to-left).
    pub rtl: bool,
    /// Hinting-quantized rasterization handle — pass `physical.cache_key` straight to
    /// `cosmic_text::SwashCache::get_image`/`with_pixels` (see [`with_shared_engine`]).
    /// `physical.x`/`physical.y` are the *integer* pixel component of this glyph's position; a
    /// rasterized image's own `Placement.left`/`.top` are relative to that integer anchor, not to
    /// `(0, 0)` — `overlay_render.rs` adds them together, matching `cosmic-text`'s own
    /// `render.rs` reference implementation. This is the one place this module's "provider-
    /// neutral value" goal bends: rasterization needs cosmic-text's own cache key, and
    /// `overlay_render` is TEXT-01A's own in-crate rasterization consumer, not an unrelated
    /// caller this would leak shaping concerns into.
    pub physical: PhysicalGlyph,
}

/// One visual line of shaped, positioned glyphs.
#[derive(Debug, Clone, Default)]
pub struct ShapedLine {
    pub glyphs: Vec<ShapedGlyph>,
    /// This line's total advance width in pixels.
    pub width: f32,
}

/// Provider-neutral shaped-text output — the only input the doc's eventual rasterization step
/// (`overlay_render`, TEXT-01A step 3) should consume once wired, so oca's own code never
/// depends on `cosmic_text` types directly outside this module.
#[derive(Debug, Clone, Default)]
pub struct ShapedText {
    pub lines: Vec<ShapedLine>,
    /// Bounding width across every line, in pixels.
    pub width: f32,
    /// Bounding height across every line, in pixels (line count * line height).
    pub height: f32,
}

impl ShapedText {
    /// Total glyph count across every line — a cheap proxy for "did shaping produce anything."
    pub fn glyph_count(&self) -> usize {
        self.lines.iter().map(|line| line.glyphs.len()).sum()
    }
}

/// Bundled-only text shaper. Owns one `cosmic_text::FontSystem` built from
/// [`font_catalog::locked_face_bytes`] — construction parses every locked face's metadata up
/// front (small: 9 files today), but glyph outlines are still only rasterized on demand by
/// `cosmic-text` itself.
pub struct TextLayoutEngine {
    font_system: FontSystem,
}

impl TextLayoutEngine {
    /// Builds an engine with a `fontdb::Database` containing exactly
    /// [`font_catalog::locked_face_bytes`]'s bytes — no installed system font ever participates,
    /// per the architecture doc's explicit requirement.
    pub fn new_from_locked_catalog() -> Self {
        let mut db = fontdb::Database::new();
        for (_, _, bytes) in font_catalog::locked_face_bytes() {
            db.load_font_data(bytes.to_vec());
        }
        let font_system = FontSystem::new_with_locale_and_db("en-US".to_string(), db);
        Self { font_system }
    }

    /// Number of faces actually loaded into the underlying `fontdb::Database` — exposed for
    /// tests proving no system font leaked in (should always equal
    /// `font_catalog::locked_face_bytes().len()`).
    pub fn loaded_face_count(&self) -> usize {
        self.font_system.db().faces().count()
    }

    /// Mutable access to the underlying `FontSystem`, needed by `cosmic_text::SwashCache`'s own
    /// rasterization calls (which take `&mut FontSystem`).
    pub fn font_system_mut(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    fn cosmic_family_name(family: TextFontFamily) -> &'static str {
        font_catalog::find_family(family.family_id())
            .map(|entry| entry.display_name)
            .unwrap_or("Lato")
    }

    fn cosmic_weight(style: TextFontStyle) -> Weight {
        match style {
            TextFontStyle::Regular => Weight::NORMAL,
            TextFontStyle::Bold => Weight::BOLD,
        }
    }

    /// Shapes `text` at `font_size_px`, wrapping at `max_width_px` when given (unbounded
    /// otherwise), with every glyph already placed at its final canvas-relative pixel position
    /// (`origin` added in, matching `fontdue::layout::LayoutSettings`'s `x`/`y` convention the
    /// previous `overlay_render.rs` renderer used). `font_size_px` is treated the same way that
    /// renderer already treated [`crate::timeline::TextClip::font_size`] — as a plain pixel size,
    /// not converted from points — so this stays numerically comparable.
    ///
    /// `direction` overrides UAX #9's own auto-detected paragraph base level when not
    /// [`TextDirection::Auto`] — see [`TextDirection`]'s own doc comment. Every returned
    /// [`ShapedGlyph::cluster`] is still a byte range into `text` exactly as passed in: the
    /// bidi-mark prefix this uses internally to pin the direction is stripped back out of both
    /// the glyph list and every cluster offset before returning, so callers never see it.
    ///
    /// `align` selects each line's horizontal alignment *within* the `(origin.0, max_width_px)`
    /// box the caller already computed — see [`TextAlign`]'s own doc comment for how a caller is
    /// expected to pick that box differently per alignment (`Auto`/`Left` anchor the box's own
    /// left edge at `origin.0`; `Center`/`Right` expect the caller to have already centered or
    /// right-anchored the box around its own intended position). `TextAlign::Auto` passes `None`
    /// through unchanged, keeping `cosmic-text`'s own direction-aware default (left for LTR,
    /// right for RTL) exactly as before this parameter existed. `Start`/`End` resolve to `Left`/
    /// `Right` via [`resolve_text_align`] before reaching `cosmic-text` — a caller computing its
    /// own wrap/alignment box (e.g. `crate::overlay_render`) must call the same function to pick
    /// the matching physical edge, since this method never reports back which edge it resolved to.
    #[allow(clippy::too_many_arguments)]
    pub fn shape(
        &mut self,
        text: &str,
        family: TextFontFamily,
        style: TextFontStyle,
        font_size_px: f32,
        max_width_px: Option<f32>,
        origin: (f32, f32),
        direction: TextDirection,
        align: TextAlign,
    ) -> ShapedText {
        let mark = match direction {
            TextDirection::Auto => None,
            TextDirection::Ltr => Some(LEFT_TO_RIGHT_MARK),
            TextDirection::Rtl => Some(RIGHT_TO_LEFT_MARK),
        };
        let prefix_len = mark.map_or(0, char::len_utf8);
        let shaped_text: Cow<str> = match mark {
            Some(mark) => Cow::Owned(format!("{mark}{text}")),
            None => Cow::Borrowed(text),
        };
        let cosmic_align = match resolve_text_align(text, direction, align) {
            TextAlign::Auto => None,
            TextAlign::Left => Some(Align::Left),
            TextAlign::Center => Some(Align::Center),
            TextAlign::Right => Some(Align::Right),
            TextAlign::Start | TextAlign::End => {
                unreachable!("resolve_text_align never returns Start/End")
            }
        };

        let metrics = Metrics::new(font_size_px, font_size_px * 1.25);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        let attrs = Attrs::new()
            .family(Family::Name(Self::cosmic_family_name(family)))
            .weight(Self::cosmic_weight(style));
        {
            let mut buffer = buffer.borrow_with(&mut self.font_system);
            buffer.set_size(max_width_px, None);
            buffer.set_text(&shaped_text, &attrs, Shaping::Advanced, cosmic_align);
            buffer.shape_until_scroll(true);
        }

        let mut lines = Vec::new();
        let mut max_width = 0.0f32;
        for run in buffer.layout_runs() {
            // `LayoutGlyph::y` is relative to *this run's own* baseline, not an absolute
            // position — `LayoutRun::line_y` ("Y offset to baseline of line") is the piece that
            // makes each successive wrapped line land lower than the last. Confirmed by reading
            // `layout_runs()`'s own `LayoutRunIter` construction in `cosmic-text`'s source
            // (`buffer.rs`), not assumed: every glyph in a multi-line shape had `y == 0` until
            // this was added in, which silently collapsed every line onto the same row.
            let line_origin = (origin.0, origin.1 + run.line_y);
            let mut glyphs = Vec::with_capacity(run.glyphs.len());
            let mut line_width = 0.0f32;
            for g in run.glyphs.iter() {
                // The bidi-mark's own cluster never corresponds to any character in the
                // caller's original `text` — drop it rather than emit a cluster range that
                // would underflow when `prefix_len` is subtracted below.
                if g.end <= prefix_len {
                    continue;
                }
                let level_is_rtl = g.level.number() % 2 == 1;
                let right_edge = g.x + g.w;
                if right_edge > line_width {
                    line_width = right_edge;
                }
                let physical = g.physical(line_origin, 1.0);
                glyphs.push(ShapedGlyph {
                    font_id: g.font_id,
                    glyph_id: g.glyph_id,
                    x: line_origin.0 + g.x,
                    y: line_origin.1 + g.y,
                    w: g.w,
                    cluster: g.start.saturating_sub(prefix_len)..g.end.saturating_sub(prefix_len),
                    rtl: level_is_rtl,
                    physical,
                });
            }
            max_width = max_width.max(line_width);
            lines.push(ShapedLine {
                glyphs,
                width: line_width,
            });
        }

        ShapedText {
            height: lines.len() as f32 * metrics.line_height,
            width: max_width,
            lines,
        }
    }

    /// Measures `text`'s rendered width in pixels at `font_size_px`, unwrapped — has no caller of
    /// its own yet outside this module's tests, same "unused until something needs it" shape the
    /// removed `fontdue`-based `crate::text_metrics::text_width_px_with_font` had before TEXT-01B
    /// step 4 deleted it.
    pub fn text_width_px(
        &mut self,
        text: &str,
        family: TextFontFamily,
        style: TextFontStyle,
        font_size_px: f32,
    ) -> f32 {
        self.shape(
            text,
            family,
            style,
            font_size_px,
            None,
            (0.0, 0.0),
            TextDirection::Auto,
            TextAlign::Auto,
        )
        .width
    }
}

static SHARED_ENGINE: OnceLock<Mutex<(TextLayoutEngine, SwashCache)>> = OnceLock::new();

/// Runs `f` against one process-wide [`TextLayoutEngine`] + `cosmic_text::SwashCache` pair,
/// built once on first use and reused after (a single shared `OnceLock`, since `cosmic-text`'s
/// own shaping/rasterization caches already key internally by face/size/glyph — no need for one
/// lock per face the way the removed `fontdue`-based `crate::text_metrics::bundled_font` used to
/// have). Guarded by a `Mutex` (not `Sync` on its own — `FontSystem` mutates internal caches on
/// every shape/rasterize call) since callers span the UI thread (`preview.rs`, refreshed on each timed
/// word change) and export's background render thread (`render.rs`, one call per text segment) —
/// neither is a per-video-frame hot path, so lock contention here is not a real concern.
pub fn with_shared_engine<R>(f: impl FnOnce(&mut TextLayoutEngine, &mut SwashCache) -> R) -> R {
    let cell = SHARED_ENGINE.get_or_init(|| {
        Mutex::new((
            TextLayoutEngine::new_from_locked_catalog(),
            SwashCache::new(),
        ))
    });
    let mut guard = cell.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (engine, swash_cache) = &mut *guard;
    f(engine, swash_cache)
}

#[cfg(test)]
#[path = "text_layout/text_layout_test.rs"]
mod tests;
