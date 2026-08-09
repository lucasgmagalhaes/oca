//! Measures perceptual loudness (LUFS), true peak and loudness range via a single-pass
//! `ffmpeg` `loudnorm` filter run, mirroring the technique in `Watch-Gameplay.ps1`'s
//! `Get-AudioAnalysis` — that script measures the same way today, just from PowerShell.
//!
//! Like [`crate::probe`], the binary invocation ([`measure_loudness`]) is a thin wrapper
//! around a pure, unit-testable parser ([`parse_loudnorm_stderr`]).

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::media::LoudnessMetrics;

#[derive(Debug)]
pub enum LoudnessError {
    /// Couldn't spawn the `ffmpeg` process.
    Spawn(std::io::Error),
    /// `ffmpeg`'s stderr didn't contain a `loudnorm` JSON report — usually means the input
    /// had no audio stream, or `ffmpeg` failed before the filter ran.
    NoReportFound,
    /// Found a `{...}` block but it wasn't a valid `loudnorm` report.
    Json(serde_json::Error),
}

impl std::fmt::Display for LoudnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoudnessError::Spawn(e) => write!(f, "failed to run ffmpeg: {e}"),
            LoudnessError::NoReportFound => {
                write!(f, "no loudnorm report found in ffmpeg output")
            }
            LoudnessError::Json(e) => write!(f, "failed to parse loudnorm report: {e}"),
        }
    }
}

impl std::error::Error for LoudnessError {}

/// Runs a single-pass `loudnorm` measurement against `path` and returns the result.
///
/// This is analysis only — nothing is written or re-encoded. The default targets
/// (`I=-16:TP=-1.5:LRA=11`) match `Watch-Gameplay.ps1`; the two-pass normalization described
/// in Fase 2 of the execution plan (measure, then apply a target like -14 LUFS for YouTube)
/// builds on top of this measurement.
pub fn measure_loudness(path: &Path) -> Result<LoudnessMetrics, LoudnessError> {
    let output = Command::new("ffmpeg")
        .args(["-y", "-nostdin", "-hide_banner", "-loglevel", "info"])
        .arg("-i")
        .arg(path)
        .args(["-af", "loudnorm=I=-16:TP=-1.5:LRA=11:print_format=json"])
        .args(["-f", "null", "-"])
        .output()
        .map_err(LoudnessError::Spawn)?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    parse_loudnorm_stderr(&stderr)
}

#[derive(Debug, Deserialize)]
struct LoudnormReport {
    input_i: String,
    input_tp: String,
    input_lra: String,
}

/// Extracts and parses the `loudnorm` JSON report ffmpeg prints to stderr, ignoring the log
/// lines before and after it.
pub fn parse_loudnorm_stderr(stderr: &str) -> Result<LoudnessMetrics, LoudnessError> {
    let json_block = extract_first_json_object(stderr).ok_or(LoudnessError::NoReportFound)?;
    let report: LoudnormReport = serde_json::from_str(json_block).map_err(LoudnessError::Json)?;

    Ok(LoudnessMetrics {
        integrated_lufs: report.input_i.parse().unwrap_or(0.0),
        true_peak_dbtp: report.input_tp.parse().unwrap_or(0.0),
        loudness_range_lu: report.input_lra.parse().unwrap_or(0.0),
    })
}

/// Finds the first balanced `{...}` substring, scanning byte-by-byte and tracking brace
/// depth so it isn't confused by any earlier unbalanced braces in ffmpeg's log preamble.
fn extract_first_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let mut depth = 0i32;
    for (offset, ch) in text[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let end = start + offset + 1;
                    return Some(&text[start..end]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn extract_first_json_object_ignores_unbalanced_braces_before_it() {
        let text = "log } line\n{\"a\":1}\nmore text";
        assert_eq!(extract_first_json_object(text), Some("{\"a\":1}"));
    }
}
