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

use super::*;

#[test]
fn text_width_px_is_positive_for_nonempty_text() {
    assert!(text_width_px("BEEP", 48.0) > 0.0);
}

#[test]
fn text_width_px_is_zero_for_empty_text() {
    assert_eq!(text_width_px("", 48.0), 0.0);
}

#[test]
fn text_width_px_scales_up_with_font_size() {
    let small = text_width_px("BEEP", 24.0);
    let large = text_width_px("BEEP", 48.0);
    assert!(large > small);
}

#[test]
fn text_width_px_grows_with_more_characters() {
    let short = text_width_px("BE", 48.0);
    let long = text_width_px("BEEEEEP", 48.0);
    assert!(long > short);
}

#[test]
fn word_x_offsets_px_starts_the_first_word_at_zero() {
    let offsets = word_x_offsets_px(&["Hello", "World"], 48.0);
    assert_eq!(offsets[0], 0.0);
}

#[test]
fn word_x_offsets_px_is_strictly_increasing() {
    let offsets = word_x_offsets_px(&["Hello", "World", "Again"], 48.0);
    assert!(offsets[1] > offsets[0]);
    assert!(offsets[2] > offsets[1]);
}

#[test]
fn word_x_offsets_px_matches_word_count() {
    let offsets = word_x_offsets_px(&["one", "two", "three", "four"], 32.0);
    assert_eq!(offsets.len(), 4);
}

#[test]
fn word_x_offsets_px_is_empty_for_no_words() {
    let offsets: Vec<f32> = word_x_offsets_px(&[], 48.0);
    assert!(offsets.is_empty());
}

#[test]
fn word_byte_ranges_follow_repeated_words_across_whitespace_and_lines() {
    assert_eq!(
        word_byte_ranges("go  go\nnow", &["go", "go", "now"]),
        vec![Some([0, 2]), Some([4, 6]), Some([7, 10])]
    );
}

#[test]
fn word_byte_ranges_leave_stale_timing_words_unmatched() {
    assert_eq!(
        word_byte_ranges("edited caption", &["edited", "old", "caption"]),
        vec![Some([0, 6]), None, Some([7, 14])]
    );
}

#[test]
fn every_bundled_font_face_parses_and_is_cached() {
    for family in TextFontFamily::ALL {
        let regular = bundled_font(family, TextFontStyle::Regular)
            .unwrap_or_else(|| panic!("failed to parse {family:?} regular"));
        assert!(std::ptr::eq(
            regular,
            bundled_font(family, TextFontStyle::Regular).unwrap()
        ));
        if family.supports_bold() {
            bundled_font(family, TextFontStyle::Bold)
                .unwrap_or_else(|| panic!("failed to parse {family:?} bold"));
        }
    }
}

#[test]
fn selecting_another_family_changes_text_metrics() {
    let lato = text_width_px_with_font(
        "Paco Paçoca",
        48.0,
        TextFontFamily::Lato,
        TextFontStyle::Regular,
    );
    let mono = text_width_px_with_font(
        "Paco Paçoca",
        48.0,
        TextFontFamily::AnonymousPro,
        TextFontStyle::Regular,
    );
    assert_ne!(lato, mono);
}
