use super::*;

#[test]
fn origin_shifts_every_glyph_by_exactly_that_offset() {
    let mut engine = TextLayoutEngine::new_from_locked_catalog();
    let at_zero = engine.shape(
        "Hi",
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
        None,
        (0.0, 0.0),
    );
    let shifted = engine.shape(
        "Hi",
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
        None,
        (10.0, 5.0),
    );
    let g0 = &at_zero.lines[0].glyphs[0];
    let g1 = &shifted.lines[0].glyphs[0];
    assert!((g1.x - g0.x - 10.0).abs() < 0.01);
    assert!((g1.y - g0.y - 5.0).abs() < 0.01);
    // The physical (rasterization-ready) integer position must also reflect the shift, since
    // overlay_render.rs paints from `physical.x`/`physical.y`, not from the float `x`/`y` fields.
    assert!(g1.physical.x > g0.physical.x);
}

#[test]
fn with_shared_engine_reuses_the_same_engine_across_calls() {
    let count_a = with_shared_engine(|engine, _cache| engine.loaded_face_count());
    let count_b = with_shared_engine(|engine, _cache| engine.loaded_face_count());
    assert_eq!(count_a, count_b);
    assert_eq!(count_a, font_catalog::locked_face_bytes().len());
}

#[test]
fn loads_exactly_the_locked_catalog_faces_and_nothing_else() {
    let engine = TextLayoutEngine::new_from_locked_catalog();
    assert_eq!(
        engine.loaded_face_count(),
        font_catalog::locked_face_bytes().len(),
        "a system font leaked into the database, or a locked face failed to load"
    );
}

#[test]
fn every_bundled_family_shapes_ordinary_latin_text_without_missing_glyphs() {
    let mut engine = TextLayoutEngine::new_from_locked_catalog();
    for family in TextFontFamily::ALL {
        let shaped = engine.shape(
            "Ação, coração, você — 0123456789",
            family,
            TextFontStyle::Regular,
            32.0,
            None,
            (0.0, 0.0),
        );
        assert!(
            shaped.glyph_count() > 0,
            "{family:?} produced no glyphs at all"
        );
        // glyph_id 0 is the .notdef glyph in every OpenType font -- a hit here means this
        // family's bundled file is missing coverage for the GF Latin Core acceptance corpus
        // characters the font_catalog doc requires (spec/architecture/built-in-font-catalog.md).
        let notdef_hits = shaped
            .lines
            .iter()
            .flat_map(|line| line.glyphs.iter())
            .filter(|g| g.glyph_id == 0)
            .count();
        assert_eq!(notdef_hits, 0, "{family:?} hit .notdef on the Latin corpus");
    }
}

#[test]
fn fi_and_fl_ligatures_form_a_single_multi_character_cluster() {
    let mut engine = TextLayoutEngine::new_from_locked_catalog();
    let text = "difficult waffle";
    let shaped = engine.shape(
        text,
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
        None,
        (0.0, 0.0),
    );
    let has_multi_char_cluster = shaped
        .lines
        .iter()
        .flat_map(|line| line.glyphs.iter())
        .any(|g| g.cluster.end - g.cluster.start > 1);
    assert!(
        has_multi_char_cluster,
        "expected at least one ligature cluster spanning more than one source character"
    );
    // Fewer glyphs than characters is the real proof a ligature merged two characters into one
    // glyph, not just that a multi-byte cluster range exists.
    let glyph_count = shaped.glyph_count();
    assert!(
        glyph_count < text.chars().count(),
        "glyph_count={glyph_count} did not shrink below char_count={}",
        text.chars().count()
    );
}

#[test]
fn bold_style_selects_a_different_font_face_than_regular_for_a_two_weight_family() {
    let mut engine = TextLayoutEngine::new_from_locked_catalog();
    assert!(TextFontFamily::Lato.supports_bold());
    let regular = engine.shape(
        "Test",
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
        None,
        (0.0, 0.0),
    );
    let bold = engine.shape(
        "Test",
        TextFontFamily::Lato,
        TextFontStyle::Bold,
        32.0,
        None,
        (0.0, 0.0),
    );
    let regular_font_id = regular.lines[0].glyphs[0].font_id;
    let bold_font_id = bold.lines[0].glyphs[0].font_id;
    assert_ne!(
        regular_font_id, bold_font_id,
        "Lato Bold should resolve to a different loaded face than Lato Regular"
    );
}

#[test]
fn single_weight_family_ignores_bold_request_without_producing_notdef() {
    // Bebas Neue only ships one designed weight -- requesting Bold must still shape cleanly by
    // falling back to the one available face, not fail or emit .notdef.
    assert!(!TextFontFamily::BebasNeue.supports_bold());
    let mut engine = TextLayoutEngine::new_from_locked_catalog();
    let shaped = engine.shape(
        "Title",
        TextFontFamily::BebasNeue,
        TextFontStyle::Bold,
        32.0,
        None,
        (0.0, 0.0),
    );
    assert!(shaped.glyph_count() > 0);
    assert!(shaped
        .lines
        .iter()
        .flat_map(|line| line.glyphs.iter())
        .all(|g| g.glyph_id != 0));
}

#[test]
fn text_width_px_is_zero_for_empty_text_and_positive_for_nonempty_text() {
    let mut engine = TextLayoutEngine::new_from_locked_catalog();
    let empty = engine.text_width_px("", TextFontFamily::Lato, TextFontStyle::Regular, 32.0);
    let nonempty = engine.text_width_px(
        "PacoPaçoca",
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
    );
    assert_eq!(empty, 0.0);
    assert!(nonempty > 0.0);
}

#[test]
fn text_width_px_grows_with_longer_text() {
    let mut engine = TextLayoutEngine::new_from_locked_catalog();
    let short = engine.text_width_px("Paco", TextFontFamily::Lato, TextFontStyle::Regular, 32.0);
    let long = engine.text_width_px(
        "PacoPaçoca gameplay highlights",
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
    );
    assert!(long > short);
}

#[test]
fn wrapping_at_a_narrow_width_produces_more_than_one_line() {
    let mut engine = TextLayoutEngine::new_from_locked_catalog();
    let shaped = engine.shape(
        "This is a fairly long caption that should wrap across several lines",
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        24.0,
        Some(120.0),
        (0.0, 0.0),
    );
    assert!(
        shaped.lines.len() > 1,
        "expected wrapping to produce multiple lines, got {}",
        shaped.lines.len()
    );
    for line in &shaped.lines {
        assert!(
            line.width <= 130.0,
            "line width {} exceeded the requested wrap width by more than a reasonable slop",
            line.width
        );
    }
}

#[test]
fn unwrapped_shaping_produces_exactly_one_line_for_single_line_text() {
    let mut engine = TextLayoutEngine::new_from_locked_catalog();
    let shaped = engine.shape(
        "One short line",
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
        None,
        (0.0, 0.0),
    );
    assert_eq!(shaped.lines.len(), 1);
}
