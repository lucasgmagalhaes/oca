// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use super::*;

#[test]
fn parses_short_and_long_hex_colors() {
    assert_eq!(parse_color_value("#0f8"), Ok([0, 255, 136, 255]));
    assert_eq!(parse_color_value("#0f8c"), Ok([0, 255, 136, 204]));
    assert_eq!(parse_color_value("#12A0ff"), Ok([18, 160, 255, 255]));
    assert_eq!(parse_color_value("0x12A0ff80"), Ok([18, 160, 255, 128]));
}

#[test]
fn parses_rgb_and_rgba_colors() {
    assert_eq!(parse_color_value("rgb(12, 34, 56)"), Ok([12, 34, 56, 255]));
    assert_eq!(
        parse_color_value("RGBA(12, 34, 56, 0.5)"),
        Ok([12, 34, 56, 128])
    );
    assert_eq!(parse_color_value("12, 34, 56, 25%"), Ok([12, 34, 56, 64]));
    assert_eq!(parse_color_value("12,34,56,128"), Ok([12, 34, 56, 128]));
}

#[test]
fn rejects_malformed_or_out_of_range_colors() {
    for value in [
        "",
        "#12",
        "#GGGGGG",
        "rgb(1,2)",
        "rgb(256,0,0)",
        "rgba(1,2,3,300)",
    ] {
        assert_eq!(parse_color_value(value), Err(()), "{value}");
    }
}

#[test]
fn formatter_preserves_every_channel() {
    assert_eq!(format_color_hex([18, 160, 255, 128]), "#12A0FF80");
}
