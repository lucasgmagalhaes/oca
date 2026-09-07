use super::*;

#[test]
fn every_marker_kind_has_a_distinct_non_empty_icon() {
    let kinds = [
        avcore::MarkerKind::Standard,
        avcore::MarkerKind::ToDo,
        avcore::MarkerKind::Chapter,
        avcore::MarkerKind::Highlight,
    ];
    let mut icons: Vec<&str> = kinds.iter().map(|&k| marker_kind_icon(k)).collect();
    for icon in &icons {
        assert!(!icon.is_empty());
    }
    icons.sort_unstable();
    icons.dedup();
    assert_eq!(
        icons.len(),
        kinds.len(),
        "two marker kinds must never share the same icon"
    );
}

#[test]
fn icons_are_plain_ascii_not_emoji_or_private_use_glyphs() {
    // These render as a plain &str with no font-family override, so a non-ASCII glyph would
    // risk showing as tofu (see this function's own doc comment on why "*"/"M"/"[]"/"C" were
    // chosen over emoji/Lucide icons here).
    for kind in [
        avcore::MarkerKind::Standard,
        avcore::MarkerKind::ToDo,
        avcore::MarkerKind::Chapter,
        avcore::MarkerKind::Highlight,
    ] {
        assert!(marker_kind_icon(kind).is_ascii());
    }
}
