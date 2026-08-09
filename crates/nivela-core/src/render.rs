//! Renders a normalized export from a source file — Fase 4's render step. Applies loudness
//! normalization (`loudnorm` + a true-peak safety limiter, per Fase 2) while copying video
//! untouched (`-c:v copy`), which is what makes the export's bitrate equal the source's
//! bitrate exactly, one of this editor's two headline differentiators.
//!
//! Progress reporting mirrors `Watch-Gameplay.ps1`'s `Invoke-FfmpegWithProgress`: ffmpeg is
//! asked for machine-readable progress on stdout (`-progress pipe:1`) and each
//! `out_time_us=<n>` line is turned into a percentage against the clip's known duration.
//! [`parse_progress_line`] is the pure, unit-tested half of that; [`render_export`] is the
//! thin wrapper that actually spawns `ffmpeg` and streams its output through it.
//!
//! `ffmpeg` has no notion of pausing a render, so the only mid-render control this module
//! offers is cancellation: [`render_export`] checks `cancel` between progress lines (ffmpeg
//! emits one every fraction of a second) and kills the process if it's been set, rather than
//! needing the caller to juggle the [`std::process::Child`] handle across threads.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub enum RenderError {
    /// Couldn't spawn the `ffmpeg` process.
    Spawn(std::io::Error),
    /// `ffmpeg` ran but exited with a non-zero status.
    ExitStatus { code: Option<i32> },
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::Spawn(e) => write!(f, "failed to run ffmpeg: {e}"),
            RenderError::ExitStatus { code } => write!(f, "ffmpeg exited with code {code:?}"),
        }
    }
}

impl std::error::Error for RenderError {}

/// How a render ended: all the way through, or stopped early because `cancel` was set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderOutcome {
    Completed,
    Cancelled,
}

fn export_command(source: &Path, output: &Path, target_lufs: f32) -> Command {
    let audio_filter =
        format!("loudnorm=I={target_lufs}:TP=-1.0:LRA=11,alimiter=limit=0.95:attack=5:release=50");

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-nostdin", "-hide_banner", "-loglevel", "warning"])
        .arg("-i")
        .arg(source)
        .args(["-af", &audio_filter])
        .args(["-c:v", "copy", "-c:a", "aac", "-b:a", "192k"])
        .args(["-progress", "pipe:1"])
        .arg(output)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    cmd
}

/// Renders `source` to `output`, applying single-pass loudnorm normalization to
/// `target_lufs` and copying video untouched. Blocks the calling thread until the render
/// finishes, is cancelled, or fails — callers on a UI thread should run this from a background
/// thread and use [`render_export`]'s `cancel` flag to interrupt it instead of blocking on it.
///
/// Calls `on_progress(percent)` as ffmpeg reports progress, and polls `cancel` between
/// progress lines — if another thread sets it, the `ffmpeg` child is killed and this returns
/// `Ok(RenderOutcome::Cancelled)` rather than treating the kill as a failure.
pub fn render_export(
    source: &Path,
    output: &Path,
    target_lufs: f32,
    duration_secs: f64,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(u8),
) -> Result<RenderOutcome, RenderError> {
    let mut child = export_command(source, output, target_lufs)
        .spawn()
        .map_err(RenderError::Spawn)?;

    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if cancel.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(RenderOutcome::Cancelled);
            }
            if let Some(percent) = parse_progress_line(&line, duration_secs) {
                on_progress(percent);
            }
        }
    }

    if cancel.load(Ordering::Relaxed) {
        let _ = child.kill();
        let _ = child.wait();
        return Ok(RenderOutcome::Cancelled);
    }

    let status = child.wait().map_err(RenderError::Spawn)?;
    if !status.success() {
        return Err(RenderError::ExitStatus {
            code: status.code(),
        });
    }

    on_progress(100);
    Ok(RenderOutcome::Completed)
}

/// Parses one line of `ffmpeg -progress pipe:1` output, returning a 0-100 percentage once it
/// finds an `out_time_us=<microseconds>` line (the other lines — `frame=`, `fps=`, `bitrate=`,
/// `progress=` — aren't needed for a percentage and are ignored).
pub fn parse_progress_line(line: &str, duration_secs: f64) -> Option<u8> {
    let out_time_us: f64 = line.strip_prefix("out_time_us=")?.trim().parse().ok()?;
    if duration_secs <= 0.0 {
        return None;
    }
    let percent = (out_time_us / 1_000_000.0 / duration_secs) * 100.0;
    Some(percent.clamp(0.0, 100.0) as u8)
}
