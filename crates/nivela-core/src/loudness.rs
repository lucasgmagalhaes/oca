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
mod tests;
