use super::*;

#[test]
fn every_fixed_preset_round_trips_through_its_own_shape_kind() {
    use avcore::timeline::ShapeKind;
    assert_eq!(
        shape_kind_to_preset(&ShapeKind::Ellipse),
        ShapePreset::Ellipse
    );
    assert_eq!(
        shape_kind_to_preset(&ShapeKind::rectangle()),
        ShapePreset::Rectangle
    );
    assert_eq!(
        shape_kind_to_preset(&ShapeKind::triangle()),
        ShapePreset::Triangle
    );
    assert_eq!(
        shape_kind_to_preset(&ShapeKind::trapezoid()),
        ShapePreset::Trapezoid
    );
    assert_eq!(
        shape_kind_to_preset(&ShapeKind::arrow()),
        ShapePreset::Arrow
    );
}

#[test]
fn an_arbitrary_polygon_that_matches_no_preset_is_custom() {
    use avcore::timeline::ShapeKind;
    let hand_drawn = ShapeKind::Polygon(vec![(0.1, 0.1), (0.2, 0.3), (-0.4, 0.1)]);
    assert_eq!(shape_kind_to_preset(&hand_drawn), ShapePreset::Custom);
}

#[test]
fn a_polygon_with_the_same_vertex_count_but_different_positions_is_still_custom() {
    // Guards against a preset match that's really just checking vertex *count* -- the
    // classifier must compare positions too, not just polygon shape.
    use avcore::timeline::ShapeKind;
    let rectangle_like_but_not_quite =
        ShapeKind::Polygon(vec![(-0.4, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)]);
    assert_eq!(
        shape_kind_to_preset(&rectangle_like_but_not_quite),
        ShapePreset::Custom
    );
}

#[test]
fn every_preset_has_a_distinct_non_empty_label_per_locale() {
    let presets = [
        ShapePreset::Ellipse,
        ShapePreset::Rectangle,
        ShapePreset::Triangle,
        ShapePreset::Trapezoid,
        ShapePreset::Arrow,
        ShapePreset::Custom,
    ];
    for locale in [crate::i18n::Locale::PtBr, crate::i18n::Locale::En] {
        let mut labels: Vec<String> = presets
            .iter()
            .map(|&p| shape_preset_label(p, locale))
            .collect();
        for label in &labels {
            assert!(!label.is_empty());
        }
        labels.sort();
        labels.dedup();
        assert_eq!(labels.len(), presets.len());
    }
}
