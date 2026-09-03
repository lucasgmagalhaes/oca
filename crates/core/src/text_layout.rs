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
//! [`crate::overlay_render`] now shapes and paints through [`with_shared_engine`] instead of
//! `fontdue`. `crate::text_metrics`'s own per-character advance-summing measurement functions are
//! untouched for now (still `fontdue`-backed) since nothing outside `overlay_render` calls them
//! for pixel-affecting work; see that module's own doc comment for the remaining gap.
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
    /// right for RTL) exactly as before this parameter existed.
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
        let cosmic_align = match align {
            TextAlign::Auto => None,
            TextAlign::Left => Some(Align::Left),
            TextAlign::Center => Some(Align::Center),
            TextAlign::Right => Some(Align::Right),
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

    /// Measures `text`'s rendered width in pixels at `font_size_px`, unwrapped — a `cosmic-text`
    /// counterpart to [`crate::text_metrics::text_width_px_with_font`], not yet wired as its
    /// replacement (see this module's own doc comment).
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
/// built once on first use and reused after — mirrors [`crate::text_metrics`]'s own
/// per-face `OnceLock` caching, just one shared engine instead of one lock per face, since
/// `cosmic-text`'s own shaping/rasterization caches already key internally by face/size/glyph.
/// Guarded by a `Mutex` (not `Sync` on its own — `FontSystem` mutates internal caches on every
/// shape/rasterize call) since callers span the UI thread (`preview.rs`, refreshed on each timed
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
