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

//! Pure-Rust font metrics for positioning word-highlight subtitle overlays — measures each
//! word's advance width at a given font/size using the same font file
//! `avbridge_apply_text_overlays`'s `drawtext` filters render with (`DEFAULT_FONT` in
//! `text_overlay.c`), so a highlighted word's overlay lands on top of the plain-text word it's
//! highlighting rather than drifting out of alignment. Verified empirically against a real
//! `drawtext` render (fontdue's advance width and FreeType's ink bounding box differ by only
//! the glyph's right-side bearing, as expected — not a coincidence, both read the same font
//! file's `hmtx` table).
//!
//! Font shaping is deliberately simple: per-character advance widths summed left to right, no
//! kerning, no ligatures, no bidi/complex script shaping. Fine for the short Latin-script
//! captions this feature targets; wrong for e.g. Arabic or tightly-kerned display faces.

use std::path::Path;
use std::sync::OnceLock;

/// The platform default font path `avbridge`'s `text_overlay.c` renders `drawtext` with —
/// duplicated here (not read from the C side) since it's three lines and pulling in an FFI call
/// just to fetch a constant string isn't worth it. Keep in sync with `DEFAULT_FONT` in
/// `crates/avbridge/csrc/text_overlay.c` if either one changes.
#[cfg(target_os = "macos")]
const DEFAULT_FONT_PATH: &str = "/System/Library/Fonts/Helvetica.ttc";
#[cfg(target_os = "windows")]
const DEFAULT_FONT_PATH: &str = "C:/Windows/Fonts/arial.ttf";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const DEFAULT_FONT_PATH: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf";

fn default_font() -> Option<&'static fontdue::Font> {
    static FONT: OnceLock<Option<fontdue::Font>> = OnceLock::new();
    FONT.get_or_init(|| {
        let bytes = std::fs::read(Path::new(DEFAULT_FONT_PATH)).ok()?;
        fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok()
    })
    .as_ref()
}

/// Sums each character's advance width (no kerning) to get `text`'s total rendered width in
/// pixels at `font_size_px`, matching drawtext's `fontsize` option (both already treat this
/// value as pixels — see `text_clip_to_segment`). Falls back to a rough `0.6 * font_size_px`
/// per character if the platform default font can't be loaded (missing file, unreadable,
/// unparseable) — better than refusing to position word highlights at all, though visibly less
/// accurate.
pub fn text_width_px(text: &str, font_size_px: f32) -> f32 {
    match default_font() {
        Some(font) => text
            .chars()
            .map(|c| font.metrics(c, font_size_px).advance_width)
            .sum(),
        None => text.chars().count() as f32 * font_size_px * 0.6,
    }
}

/// Splits `text` on ASCII spaces and returns each word's left-edge x offset in pixels from the
/// start of the line (word 0 is always at offset `0.0`), for positioning per-word highlight
/// overlays under [`crate::render::resolve_text_segments`]. A single space's width is included
/// between words but not before the first or after the last. Multi-space runs collapse to one
/// gap, matching how the words were presumably joined when transcribed.
pub fn word_x_offsets_px(words: &[&str], font_size_px: f32) -> Vec<f32> {
    let space_width = text_width_px(" ", font_size_px);
    let mut offsets = Vec::with_capacity(words.len());
    let mut x = 0.0f32;
    for (i, word) in words.iter().enumerate() {
        if i > 0 {
            x += space_width;
        }
        offsets.push(x);
        x += text_width_px(word, font_size_px);
    }
    offsets
}

#[cfg(test)]
#[path = "text_metrics/text_metrics_test.rs"]
mod tests;
