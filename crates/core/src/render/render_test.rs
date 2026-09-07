use super::*;

#[test]
fn fps_to_rational_uses_whole_number_over_one_for_an_integer_fps() {
    assert_eq!(fps_to_rational(30.0), (30, 1));
    assert_eq!(fps_to_rational(60.0), (60, 1));
    assert_eq!(fps_to_rational(24.0), (24, 1));
}

#[test]
fn fps_to_rational_uses_a_fixed_point_fraction_for_a_non_integer_fps() {
    // 29.97 is not within 0.001 of a whole number, so it takes the /1000 fraction path.
    assert_eq!(fps_to_rational(29.97), (29970, 1000));
    assert_eq!(fps_to_rational(23.976), (23976, 1000));
}

#[test]
fn fps_to_rational_treats_a_value_within_0_001_of_whole_as_whole() {
    assert_eq!(fps_to_rational(30.0004), (30, 1));
    assert_eq!(fps_to_rational(29.9996), (30, 1));
}

#[test]
fn fps_to_rational_falls_back_to_30_1_for_zero_or_negative_fps() {
    assert_eq!(fps_to_rational(0.0), (30, 1));
    assert_eq!(fps_to_rational(-5.0), (30, 1));
}

#[test]
fn fps_to_rational_never_panics_on_extreme_input() {
    let _ = fps_to_rational(f32::INFINITY);
    let _ = fps_to_rational(f32::NAN);
}
