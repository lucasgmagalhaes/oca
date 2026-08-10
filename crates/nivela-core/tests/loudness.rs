use std::path::{Path, PathBuf};

use nivela_core::loudness::{measure_loudness, parse_loudnorm_stderr, LoudnessError};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

const FFMPEG_STDERR_FIXTURE: &str = r#"
ffmpeg version 6.0 Copyright (c) 2000-2023 the FFmpeg developers
  built with gcc 12.2.0
Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'boss03_ribby_croaks.mp4':
  Duration: 00:02:14.23, start: 0.000000, bitrate: 42021 kb/s
[Parsed_loudnorm_0 @ 0000020a1b2c3d40]
{
	"input_i" : "-19.40",
	"input_tp" : "-3.20",
	"input_lra" : "8.10",
	"input_thresh" : "-29.80",
	"output_i" : "-16.02",
	"output_tp" : "-1.50",
	"output_lra" : "7.00",
	"output_thresh" : "-26.40",
	"normalization_type" : "dynamic",
	"target_offset" : "0.00"
}
frame=  8043 fps=812 q=-1.0 Lsize=N/A time=00:02:14.20 bitrate=N/A speed=101x
video:0kB audio:0kB subtitle:0kB other streams:0kB global headers:0kB muxing overhead: unknown
"#;

#[test]
fn extracts_the_measurement_fields_from_a_realistic_stderr_dump() {
    let metrics = parse_loudnorm_stderr(FFMPEG_STDERR_FIXTURE).unwrap();
    assert!((metrics.integrated_lufs - (-19.40)).abs() < 0.001);
    assert!((metrics.true_peak_dbtp - (-3.20)).abs() < 0.001);
    assert!((metrics.loudness_range_lu - 8.10).abs() < 0.001);
}

#[test]
fn errors_when_there_is_no_json_block() {
    let result = parse_loudnorm_stderr("just a log line, no braces here");
    assert!(matches!(result, Err(LoudnessError::NoReportFound)));
}

#[test]
fn errors_when_the_json_block_is_not_a_loudnorm_report() {
    let result = parse_loudnorm_stderr("preamble\n{\"unrelated\": true}\ntrailer");
    assert!(matches!(result, Err(LoudnessError::Json(_))));
}

#[test]
fn measures_a_real_file_via_the_ffi_bridge() {
    // video.mp4's audio track is a synthesized 1000Hz sine tone (see
    // oca-avbridge/tests/fixtures) — quiet relative to full scale, so a real, deeply negative
    // LUFS reading here (not the 0.0 fallback parse_loudnorm_stderr uses for unparseable
    // fields) is itself evidence the FFI round-trip worked, not just that it didn't error.
    let metrics = measure_loudness(&fixture("video.mp4")).unwrap();
    assert!(metrics.integrated_lufs < -10.0);
    assert!(metrics.integrated_lufs > -60.0);
}

#[test]
fn errors_on_a_missing_file() {
    assert!(measure_loudness(&fixture("does_not_exist.mp4")).is_err());
}
