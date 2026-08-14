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
