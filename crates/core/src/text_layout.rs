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
//! value plus a bundled-only [`TextLayoutEngine`]), **not yet the swap** (step 3: moving
//! [`crate::text_metrics`]'s width/word-offset measurement and [`crate::overlay_render`]'s glyph
//! rasterization onto this module's output). Both of those still run on `fontdue` exactly as
//! before — this module has no caller yet outside its own tests. Wiring them over is a real,
//! separate, deliberately deferred next step: it touches every preview/export text pixel at once
//! and needs the golden-image verification this headless sandbox can't perform, unlike the pure
//! shaping-correctness properties this module's own tests already check for real.
//!
//! Loads only [`crate::font_catalog`]'s locked bytes into a `fontdb::Database` via
//! `FontSystem::new_with_locale_and_db` — never `FontSystem::new()`, which the architecture doc
//! explicitly forbids because it scans installed system fonts. `cosmic-text`'s own
//! `PlatformFallback` fallback-name list can never resolve to anything outside that locked set
//! for the same reason the acceptance spike relied on: it only ever matches families already
//! present in the `fontdb::Database` it was built with.

use std::ops::Range;

use cosmic_text::{fontdb, Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Weight};

use crate::font_catalog;
use crate::timeline::{TextFontFamily, TextFontStyle};

/// One positioned glyph within a [`ShapedLine`]. Coordinates are relative to the line's own
/// origin (`x = 0` at the paragraph's leading edge), matching `cosmic-text`'s own `LayoutGlyph`
/// convention so a future rasterizer can consume this directly.
#[derive(Debug, Clone)]
pub struct ShapedGlyph {
    pub font_id: fontdb::ID,
    pub glyph_id: u16,
    /// Left edge of this glyph's advance box, in pixels from the line origin.
    pub x: f32,
    /// Top edge of this glyph's advance box, in pixels from the line origin.
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
    /// otherwise). `font_size_px` is treated the same way `overlay_render`'s existing fontdue
    /// path already treats [`crate::timeline::TextClip::font_size`] — as a plain pixel size, not
    /// converted from points — so this stays numerically comparable to the current renderer.
    pub fn shape(
        &mut self,
        text: &str,
        family: TextFontFamily,
        style: TextFontStyle,
        font_size_px: f32,
        max_width_px: Option<f32>,
    ) -> ShapedText {
        let metrics = Metrics::new(font_size_px, font_size_px * 1.25);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        let attrs = Attrs::new()
            .family(Family::Name(Self::cosmic_family_name(family)))
            .weight(Self::cosmic_weight(style));
        {
            let mut buffer = buffer.borrow_with(&mut self.font_system);
            buffer.set_size(max_width_px, None);
            buffer.set_text(text, &attrs, Shaping::Advanced, None);
            buffer.shape_until_scroll(true);
        }

        let mut lines = Vec::new();
        let mut max_width = 0.0f32;
        for run in buffer.layout_runs() {
            let mut glyphs = Vec::with_capacity(run.glyphs.len());
            let mut line_width = 0.0f32;
            for g in run.glyphs.iter() {
                let level_is_rtl = g.level.number() % 2 == 1;
                let right_edge = g.x + g.w;
                if right_edge > line_width {
                    line_width = right_edge;
                }
                glyphs.push(ShapedGlyph {
                    font_id: g.font_id,
                    glyph_id: g.glyph_id,
                    x: g.x,
                    y: g.y,
                    w: g.w,
                    cluster: g.start..g.end,
                    rtl: level_is_rtl,
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
        self.shape(text, family, style, font_size_px, None).width
    }
}

#[cfg(test)]
#[path = "text_layout/text_layout_test.rs"]
mod tests;
