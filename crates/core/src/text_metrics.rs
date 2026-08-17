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

//! Pure-Rust access to oca's bundled fonts and metrics for positioning word-highlight subtitle
//! overlays. Font files are compiled into the application, but each face is parsed only on its
//! first render or measurement through a dedicated [`OnceLock`] — opening the text-properties
//! panel therefore doesn't eagerly parse every bundled family.
//!
//! Font shaping is deliberately simple: per-character advance widths summed left to right, no
//! kerning, no ligatures, no bidi/complex script shaping. Fine for the short Latin-script
//! captions this feature targets; wrong for e.g. Arabic or tightly-kerned display faces.

use std::sync::OnceLock;

use crate::timeline::{TextFontFamily, TextFontStyle};

static LATO_REGULAR: OnceLock<Option<fontdue::Font>> = OnceLock::new();
static LATO_BOLD: OnceLock<Option<fontdue::Font>> = OnceLock::new();
static BEBAS_NEUE: OnceLock<Option<fontdue::Font>> = OnceLock::new();
static PLAYFAIR_REGULAR: OnceLock<Option<fontdue::Font>> = OnceLock::new();
static PLAYFAIR_BOLD: OnceLock<Option<fontdue::Font>> = OnceLock::new();
static PATRICK_HAND: OnceLock<Option<fontdue::Font>> = OnceLock::new();
static ANONYMOUS_PRO_REGULAR: OnceLock<Option<fontdue::Font>> = OnceLock::new();
static ANONYMOUS_PRO_BOLD: OnceLock<Option<fontdue::Font>> = OnceLock::new();
static ARCHIVO_BLACK: OnceLock<Option<fontdue::Font>> = OnceLock::new();

fn parse_font(
    cache: &'static OnceLock<Option<fontdue::Font>>,
    bytes: &'static [u8],
) -> Option<&'static fontdue::Font> {
    cache
        .get_or_init(|| fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default()).ok())
        .as_ref()
}

/// Returns one selected bundled face, parsing it on first use and reusing it thereafter.
/// Families with one designed weight ignore `Bold` and return that same face.
pub(crate) fn bundled_font(
    family: TextFontFamily,
    style: TextFontStyle,
) -> Option<&'static fontdue::Font> {
    match (family, style) {
        (TextFontFamily::Lato, TextFontStyle::Regular) => parse_font(
            &LATO_REGULAR,
            include_bytes!("../assets/fonts/lato/Lato-Regular.ttf"),
        ),
        (TextFontFamily::Lato, TextFontStyle::Bold) => parse_font(
            &LATO_BOLD,
            include_bytes!("../assets/fonts/lato/Lato-Bold.ttf"),
        ),
        (TextFontFamily::BebasNeue, _) => parse_font(
            &BEBAS_NEUE,
            include_bytes!("../assets/fonts/bebas-neue/BebasNeue-Regular.ttf"),
        ),
        (TextFontFamily::PlayfairDisplay, TextFontStyle::Regular) => parse_font(
            &PLAYFAIR_REGULAR,
            include_bytes!("../assets/fonts/playfair-display-sc/PlayfairDisplaySC-Regular.ttf"),
        ),
        (TextFontFamily::PlayfairDisplay, TextFontStyle::Bold) => parse_font(
            &PLAYFAIR_BOLD,
            include_bytes!("../assets/fonts/playfair-display-sc/PlayfairDisplaySC-Bold.ttf"),
        ),
        (TextFontFamily::PatrickHand, _) => parse_font(
            &PATRICK_HAND,
            include_bytes!("../assets/fonts/patrick-hand/PatrickHand-Regular.ttf"),
        ),
        (TextFontFamily::AnonymousPro, TextFontStyle::Regular) => parse_font(
            &ANONYMOUS_PRO_REGULAR,
            include_bytes!("../assets/fonts/anonymous-pro/AnonymousPro-Regular.ttf"),
        ),
        (TextFontFamily::AnonymousPro, TextFontStyle::Bold) => parse_font(
            &ANONYMOUS_PRO_BOLD,
            include_bytes!("../assets/fonts/anonymous-pro/AnonymousPro-Bold.ttf"),
        ),
        (TextFontFamily::ArchivoBlack, _) => parse_font(
            &ARCHIVO_BLACK,
            include_bytes!("../assets/fonts/archivo-black/ArchivoBlack-Regular.ttf"),
        ),
    }
}

/// Sums each character's advance width (no kerning) to get `text`'s total rendered width in
/// pixels at `font_size_px`. Falls back to a rough `0.6 * font_size_px` per character if a
/// bundled font is unexpectedly unparseable — better than refusing to position word highlights
/// at all, though visibly less accurate.
pub fn text_width_px(text: &str, font_size_px: f32) -> f32 {
    text_width_px_with_font(
        text,
        font_size_px,
        TextFontFamily::default(),
        TextFontStyle::default(),
    )
}

/// Font-selecting counterpart of [`text_width_px`].
pub fn text_width_px_with_font(
    text: &str,
    font_size_px: f32,
    family: TextFontFamily,
    style: TextFontStyle,
) -> f32 {
    match bundled_font(family, style) {
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
    word_x_offsets_px_with_font(
        words,
        font_size_px,
        TextFontFamily::default(),
        TextFontStyle::default(),
    )
}

/// Font-selecting counterpart of [`word_x_offsets_px`].
pub fn word_x_offsets_px_with_font(
    words: &[&str],
    font_size_px: f32,
    family: TextFontFamily,
    style: TextFontStyle,
) -> Vec<f32> {
    let space_width = text_width_px_with_font(" ", font_size_px, family, style);
    let mut offsets = Vec::with_capacity(words.len());
    let mut x = 0.0f32;
    for (i, word) in words.iter().enumerate() {
        if i > 0 {
            x += space_width;
        }
        offsets.push(x);
        x += text_width_px_with_font(word, font_size_px, family, style);
    }
    offsets
}

#[cfg(test)]
#[path = "text_metrics/text_metrics_test.rs"]
mod tests;
