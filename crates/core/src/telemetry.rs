// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Structured runtime telemetry — `request.md`'s Fase 7 "Telemetria de runtime": local,
//! append-only JSON-lines records of usage events (import/export duration, preview frame
//! time, error events). Stays on-device only — nothing here ever leaves the machine or talks
//! to a network, matching `request.md`'s explicit "fica no dispositivo por padrão" wording —
//! `ui` decides *where* the file lives (see `App::telemetry_path`, next to the platform log
//! directory Fase 6's `tracing` file appender already writes to) and whether the user has
//! opted out; this module only knows how to append one record given an already-resolved path.
//!
//! **Not yet covered:** CPU/RAM/GPU resource sampling from `request.md`'s full wishlist — no
//! resource-sampling dependency (e.g. `sysinfo`) is wired in yet, only the event-shaped
//! metrics below (import/export duration, preview frame time, error events).

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Above this size, [`record_event`] rotates the file to a `.1`-suffixed backup (replacing any
/// previous one) before appending further — bounds total disk usage to roughly this size times
/// two (the live file plus one backup) for an install that runs for a very long time. Simple
/// size-based rotation rather than the daily rotation Fase 6's `tracing` log uses, since this
/// crate has no date/time dependency to compute calendar boundaries with, and a plain size cap
/// needs none.
const ROTATE_AT_BYTES: u64 = 10 * 1024 * 1024;

/// One structured usage or error event, appended as a single JSON-lines record by
/// [`record_event`]. `#[serde(tag = "event")]` makes each line self-describing without a
/// separate schema/wrapper type to keep in sync.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TelemetryEvent {
    /// A file finished importing — `duration_ms` covers `App::spawn_import`'s whole
    /// background-thread pass for that file (probe, then loudness/proxy/waveform
    /// enrichment), not just one sub-step.
    ImportCompleted { duration_ms: u64 },
    /// An export job's render worker finished, successfully or not. `duration_ms` is
    /// wall-clock encode time; `output_duration_secs` is the exported timeline's own length —
    /// a different quantity (encoding speed vs. footage length), both useful to have side by
    /// side when looking at real-world render throughput later.
    ExportCompleted {
        duration_ms: u64,
        output_duration_secs: f64,
        success: bool,
    },
    /// A sampled preview frame time (milliseconds), taken periodically while the preview is
    /// playing — not a record of every single render frame, which would be far too high a
    /// volume for a plain JSON-lines file.
    PreviewFrameTime { frame_time_ms: f32 },
    /// A recoverable error already surfaced to the user or logged via `tracing::error!` —
    /// `context` is a short machine-readable tag (e.g. `"import"`, `"export"`), not a full
    /// sentence, so records group cleanly if this file is ever aggregated across sessions.
    Error { context: String, message: String },
}

#[derive(Debug)]
pub enum TelemetryError {
    Io(std::io::Error),
    Serialize(serde_json::Error),
}

impl std::fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelemetryError::Io(e) => write!(f, "failed to write telemetry record: {e}"),
            TelemetryError::Serialize(e) => write!(f, "failed to serialize telemetry record: {e}"),
        }
    }
}

impl std::error::Error for TelemetryError {}

/// One line of `telemetry.jsonl` — a Unix-epoch-seconds timestamp plus the event itself,
/// flattened so the JSON stays flat (`{"timestamp": ..., "event": "import_completed", ...}`)
/// rather than nesting the event under its own key.
#[derive(Serialize)]
struct TelemetryRecord<'a> {
    timestamp: u64,
    #[serde(flatten)]
    event: &'a TelemetryEvent,
}

/// Appends one [`TelemetryEvent`] as a single JSON line to `telemetry_path`, creating the file
/// (and any missing parent directories) if it doesn't exist yet, rotating it first via
/// [`rotate_if_oversized`] if it's grown past [`ROTATE_AT_BYTES`]. Never truncates or rewrites
/// existing lines otherwise — still a plain append-only log within one rotation.
pub fn record_event(telemetry_path: &Path, event: &TelemetryEvent) -> Result<(), TelemetryError> {
    if let Some(parent) = telemetry_path.parent() {
        std::fs::create_dir_all(parent).map_err(TelemetryError::Io)?;
    }
    rotate_if_oversized(telemetry_path);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = serde_json::to_string(&TelemetryRecord { timestamp, event })
        .map_err(TelemetryError::Serialize)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(telemetry_path)
        .map_err(TelemetryError::Io)?;
    writeln!(file, "{line}").map_err(TelemetryError::Io)?;
    Ok(())
}

/// Renames `telemetry_path` to a `.1`-suffixed backup (replacing any previous one via
/// `fs::rename`'s own overwrite-on-rename behavior) if it exists and is at least
/// [`ROTATE_AT_BYTES`] large. Best-effort: a missing file, or a rename that fails (e.g. no
/// permission), is silently treated as "nothing to rotate" — [`record_event`] must still be
/// able to append a record either way, rotation is a bonus, not a precondition.
fn rotate_if_oversized(telemetry_path: &Path) {
    let Ok(metadata) = std::fs::metadata(telemetry_path) else {
        return;
    };
    if metadata.len() < ROTATE_AT_BYTES {
        return;
    }
    let mut backup_name = telemetry_path.as_os_str().to_os_string();
    backup_name.push(".1");
    let _ = std::fs::rename(telemetry_path, PathBuf::from(backup_name));
}
