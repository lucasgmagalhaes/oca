use super::*;

#[test]
fn format_timecode_zero_seconds() {
    assert_eq!(format_timecode(0.0), "00:00");
}

#[test]
fn format_timecode_under_a_minute() {
    assert_eq!(format_timecode(5.0), "00:05");
    assert_eq!(format_timecode(59.0), "00:59");
}

#[test]
fn format_timecode_under_an_hour_omits_the_hours_field() {
    assert_eq!(format_timecode(134.0), "02:14");
}

#[test]
fn format_timecode_rounds_to_the_nearest_second() {
    assert_eq!(format_timecode(5.4), "00:05");
    assert_eq!(format_timecode(5.6), "00:06");
}

#[test]
fn format_timecode_rounding_can_carry_into_the_next_minute() {
    // 59.6 rounds to 60 whole seconds, which must carry into "01:00", not display as "00:60".
    assert_eq!(format_timecode(59.6), "01:00");
}

#[test]
fn format_timecode_rounding_can_carry_into_the_next_hour() {
    // 3599.6 rounds to 3600, which must show the hours field, not "59:60"/"60:00".
    assert_eq!(format_timecode(3599.6), "1:00:00");
}

#[test]
fn format_timecode_at_exactly_one_hour() {
    assert_eq!(format_timecode(3600.0), "1:00:00");
}

#[test]
fn format_timecode_past_one_hour_pads_minutes_and_seconds() {
    assert_eq!(format_timecode(3723.0), "1:02:03");
}

#[test]
fn format_timecode_past_ten_hours() {
    assert_eq!(format_timecode(36_000.0), "10:00:00");
}
