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
        TextDirection::Auto,
    );
    let shifted = engine.shape(
        "Hi",
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
        None,
        (10.0, 5.0),
        TextDirection::Auto,
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
            TextDirection::Auto,
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
        TextDirection::Auto,
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
        TextDirection::Auto,
    );
    let bold = engine.shape(
        "Test",
        TextFontFamily::Lato,
        TextFontStyle::Bold,
        32.0,
        None,
        (0.0, 0.0),
        TextDirection::Auto,
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
        TextDirection::Auto,
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
        TextDirection::Auto,
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
        TextDirection::Auto,
    );
    assert_eq!(shaped.lines.len(), 1);
}

#[test]
fn forced_direction_never_leaks_the_bidi_mark_into_the_glyph_list_or_cluster_ranges() {
    // Same real proof `fi_and_fl_ligatures_form_a_single_multi_character_cluster` uses for
    // ligatures: comparing shaped output against Auto for the exact same source text is a real
    // check the internal LRM/RLM prefix this override uses is fully invisible to callers, not
    // just that the byte-offset arithmetic type-checks.
    //
    // Real finding from running this against actual cosmic-text: an RTL-leveled run's glyphs
    // come back from `layout_runs()` in HarfBuzz's own RTL shaping-buffer order (rightmost pen
    // position first, advancing leftward) — the exact reverse of Auto/Ltr's iteration order for
    // the same text, even though every glyph's *final pixel position* still reads correctly left
    // to right. So this compares cluster *sets* (order-independent), not iteration order, and
    // separately proves final positions stay in reading order below.
    let mut engine = TextLayoutEngine::new_from_locked_catalog();
    let text = "PacoPaçoca";
    let auto = engine.shape(
        text,
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
        None,
        (0.0, 0.0),
        TextDirection::Auto,
    );
    let ltr = engine.shape(
        text,
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
        None,
        (0.0, 0.0),
        TextDirection::Ltr,
    );
    let rtl = engine.shape(
        text,
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
        None,
        (0.0, 0.0),
        TextDirection::Rtl,
    );
    for shaped in [&auto, &ltr, &rtl] {
        assert_eq!(
            shaped.glyph_count(),
            auto.glyph_count(),
            "forcing a direction must not drop or add a glyph for the caller's own text"
        );
    }
    let mut auto_clusters: Vec<_> = auto.lines[0]
        .glyphs
        .iter()
        .map(|g| g.cluster.clone())
        .collect();
    let mut ltr_clusters: Vec<_> = ltr.lines[0]
        .glyphs
        .iter()
        .map(|g| g.cluster.clone())
        .collect();
    let mut rtl_clusters: Vec<_> = rtl.lines[0]
        .glyphs
        .iter()
        .map(|g| g.cluster.clone())
        .collect();
    for clusters in [&mut auto_clusters, &mut ltr_clusters, &mut rtl_clusters] {
        clusters.sort_by_key(|c| c.start);
    }
    assert_eq!(
        auto_clusters, ltr_clusters,
        "the set of cluster byte ranges must be relative to the caller's text, not the internal LRM-prefixed one"
    );
    assert_eq!(
        auto_clusters, rtl_clusters,
        "the set of cluster byte ranges must be relative to the caller's text, not the internal RLM-prefixed one"
    );
    // No glyph's cluster should ever reach beyond the caller's own text length.
    for shaped in [&auto, &ltr, &rtl] {
        for glyph in &shaped.lines[0].glyphs {
            assert!(glyph.cluster.end <= text.len());
        }
    }
    // Forcing Rtl still must not scramble the actual rendered pixels: sorted by logical cluster
    // start, each glyph's final `x` position must still increase monotonically, proving a pure
    // left-to-right script reads correctly left to right even when the paragraph itself is
    // anchored/ordered as RTL (it lands as one right-anchored block, not character soup).
    let mut by_cluster_start: Vec<_> = rtl.lines[0].glyphs.iter().collect();
    by_cluster_start.sort_by_key(|g| g.cluster.start);
    for pair in by_cluster_start.windows(2) {
        assert!(
            pair[1].x > pair[0].x,
            "RTL-forced Latin text must still render each character left-to-right in final pixel position"
        );
    }
}

#[test]
fn forced_direction_does_not_panic_on_empty_text() {
    let mut engine = TextLayoutEngine::new_from_locked_catalog();
    for direction in [TextDirection::Auto, TextDirection::Ltr, TextDirection::Rtl] {
        let shaped = engine.shape(
            "",
            TextFontFamily::Lato,
            TextFontStyle::Regular,
            32.0,
            None,
            (0.0, 0.0),
            direction,
        );
        assert_eq!(shaped.glyph_count(), 0);
    }
}

#[test]
fn auto_detects_a_pure_hebrew_paragraph_as_rtl() {
    // No Hebrew face is bundled yet (TEXT-01C) so this only checks the computed bidi level, not
    // glyph coverage -- `unicode-bidi` resolves levels straight from the Unicode text,
    // independent of which font ends up rasterizing (or missing) the glyph.
    let mut engine = TextLayoutEngine::new_from_locked_catalog();
    let hebrew = "שלום";
    let auto = engine.shape(
        hebrew,
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
        None,
        (0.0, 0.0),
        TextDirection::Auto,
    );
    assert!(
        !auto.lines[0].glyphs.is_empty(),
        "expected at least one glyph even without a bundled Hebrew face"
    );
    assert!(
        auto.lines[0].glyphs.iter().all(|g| g.rtl),
        "Hebrew text should auto-detect as an RTL paragraph"
    );
}

#[test]
fn forced_rtl_direction_flips_which_side_of_the_line_each_scripts_run_lands_on() {
    // A real proof the override actually changes paragraph-level *run ordering* (TEXT-01B's own
    // goal 2), not just that it compiles or that it's a no-op on a single-script string: a
    // strong-R Hebrew character (bidi class R) forces the paragraph to Auto-detect RTL on its
    // own already whenever it leads, so this instead starts with a Latin-leading mixed string --
    // Auto picks LTR from the leading "AB", forcing Rtl overrides that same P2/P3 detection.
    // Individual per-character bidi levels correctly stay intrinsic to each script either way
    // (UAX #9 I1/I2 -- a strong-R character is always an odd/RTL level regardless of paragraph
    // base direction), so what actually flips between the two runs is *which side of the line*
    // each script's run lands on.
    let mut engine = TextLayoutEngine::new_from_locked_catalog();
    let text = "AB שב"; // Latin "AB", a space, two Hebrew letters
    let auto = engine.shape(
        text,
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
        None,
        (0.0, 0.0),
        TextDirection::Auto,
    );
    let rtl = engine.shape(
        text,
        TextFontFamily::Lato,
        TextFontStyle::Regular,
        32.0,
        None,
        (0.0, 0.0),
        TextDirection::Rtl,
    );

    let mean_x = |shaped: &ShapedText, rtl: bool| -> f32 {
        let xs: Vec<f32> = shaped.lines[0]
            .glyphs
            .iter()
            .filter(|g| g.rtl == rtl)
            .map(|g| g.x)
            .collect();
        assert!(!xs.is_empty());
        xs.iter().sum::<f32>() / xs.len() as f32
    };

    let auto_latin_x = mean_x(&auto, false);
    let auto_hebrew_x = mean_x(&auto, true);
    let rtl_latin_x = mean_x(&rtl, false);
    let rtl_hebrew_x = mean_x(&rtl, true);

    assert!(
        auto_latin_x < auto_hebrew_x,
        "Auto (LTR-detected from the leading Latin run) should place Latin left of Hebrew"
    );
    assert!(
        rtl_latin_x > rtl_hebrew_x,
        "forcing Rtl should place the Latin run right of the Hebrew run instead"
    );
}
