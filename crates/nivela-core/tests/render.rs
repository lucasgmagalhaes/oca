use nivela_core::render::parse_progress_line;

#[test]
fn parses_out_time_us_into_a_percentage() {
    // 30s into a 120s clip = 25%.
    assert_eq!(parse_progress_line("out_time_us=30000000", 120.0), Some(25));
}

#[test]
fn clamps_above_100_percent() {
    // ffmpeg can report slightly past the nominal duration near the end of encoding.
    assert_eq!(
        parse_progress_line("out_time_us=999000000", 120.0),
        Some(100)
    );
}

#[test]
fn ignores_unrelated_progress_lines() {
    assert_eq!(parse_progress_line("frame=8043", 120.0), None);
    assert_eq!(parse_progress_line("fps=60.00", 120.0), None);
    assert_eq!(parse_progress_line("progress=continue", 120.0), None);
}

#[test]
fn returns_none_for_an_unknown_duration() {
    assert_eq!(parse_progress_line("out_time_us=30000000", 0.0), None);
    assert_eq!(parse_progress_line("out_time_us=30000000", -1.0), None);
}

#[test]
fn returns_none_for_a_malformed_value() {
    assert_eq!(parse_progress_line("out_time_us=not_a_number", 120.0), None);
}
