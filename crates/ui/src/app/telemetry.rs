// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use eframe::egui;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::{App, PREVIEW_FRAME_TELEMETRY_INTERVAL, RESOURCE_TELEMETRY_INTERVAL};

/// Where `telemetry.jsonl` lives — right next to the rolling daily log files Fase 6's
/// `tracing` file appender already writes into (`crate::platform_log_dir`), since both are
/// "local, on-device, operational data about this install" rather than user configuration
/// (which lives next to `prefs.oc` instead).
pub(super) fn telemetry_path() -> PathBuf {
    crate::platform_log_dir().join("telemetry.jsonl")
}

/// Drains `rx` for the app's whole lifetime, appending each event to `telemetry_path` via
/// `avcore::record_event`. Runs on its own thread — same reasoning as every other background
/// worker in this module (`App::spawn_import`, the export render workers, ...): disk I/O must
/// not run on the UI thread, and this specifically must not block whichever thread queued the
/// event either (a preview frame-time sample is sent from the UI thread itself).
pub(super) fn spawn_telemetry_writer(
    mut rx: UnboundedReceiver<avcore::TelemetryEvent>,
    telemetry_path: PathBuf,
) {
    std::thread::spawn(move || {
        while let Some(event) = rx.blocking_recv() {
            if let Err(e) = avcore::record_event(&telemetry_path, &event) {
                tracing::warn!(error = %e, "failed to write telemetry record");
            }
        }
    });
}

/// Runs for the app's whole lifetime, sampling CPU/RAM (`avcore::ResourceSampler`) once per
/// [`RESOURCE_TELEMETRY_INTERVAL`] and sending the result to `tx` — a separate thread from
/// [`spawn_telemetry_writer`], since resource sampling itself (not just the disk write) must
/// not run on the UI thread either. `enabled` is checked each tick rather than once at spawn
/// time, so toggling the Preferences checkbox takes effect on the very next tick without
/// needing to restart this thread.
pub(super) fn spawn_resource_sampler(
    tx: UnboundedSender<avcore::TelemetryEvent>,
    enabled: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let mut sampler = avcore::ResourceSampler::new();
        loop {
            std::thread::sleep(RESOURCE_TELEMETRY_INTERVAL);
            if !enabled.load(Ordering::Relaxed) {
                continue;
            }
            if tx.send(sampler.sample()).is_err() {
                return;
            }
        }
    });
}

/// Runs for the app's whole lifetime, sampling GPU utilization/VRAM (`avcore::GpuSampler`) once
/// per [`RESOURCE_TELEMETRY_INTERVAL`] and sending results to `tx` — mirrors
/// [`spawn_resource_sampler`] exactly, except this thread exits immediately if
/// `avcore::GpuSampler::new` returns `None` (no NVML-compatible GPU on this machine at all,
/// which won't change mid-session) rather than looping forever just to keep re-checking
/// something that can't become true. A tick where [`avcore::GpuSampler::sample`] itself returns
/// `None` (a transient NVML query failure) is silently skipped, same as a disabled-telemetry
/// tick.
pub(super) fn spawn_gpu_sampler(
    tx: UnboundedSender<avcore::TelemetryEvent>,
    enabled: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let Some(sampler) = avcore::GpuSampler::new() else {
            tracing::info!(
                "no NVML-compatible GPU detected -- GPU usage telemetry disabled for this session"
            );
            return;
        };
        loop {
            std::thread::sleep(RESOURCE_TELEMETRY_INTERVAL);
            if !enabled.load(Ordering::Relaxed) {
                continue;
            }
            let Some(event) = sampler.sample() else {
                continue;
            };
            if tx.send(event).is_err() {
                return;
            }
        }
    });
}

impl App {
    /// Queues a telemetry event for the background writer thread — a cheap, non-blocking
    /// channel send, safe to call from the UI thread. A no-op if telemetry is disabled in
    /// Preferences (checked here so call sites don't need to know about the setting) or if the
    /// writer thread has already shut down (channel closed at exit — `send` just returns an
    /// error `App` ignores, same as every other event channel in this module).
    pub(crate) fn record_telemetry(&self, event: avcore::TelemetryEvent) {
        if !self.prefs.telemetry_enabled {
            return;
        }
        let _ = self.telemetry_state.telemetry_tx.send(event);
    }

    /// Records a `PreviewFrameTime` sample while the preview is playing — throttled to once
    /// per [`PREVIEW_FRAME_TELEMETRY_INTERVAL`] rather than every frame. Called from
    /// [`eframe::App::ui`]'s `preview_playing` branch, the same place that already special-
    /// cases every-frame repaint for smooth video.
    pub(super) fn sample_preview_frame_telemetry(&mut self, ctx: &egui::Context) {
        let now = std::time::Instant::now();
        let due = self
            .telemetry_state
            .last_preview_frame_telemetry
            .is_none_or(|last| now.duration_since(last) >= PREVIEW_FRAME_TELEMETRY_INTERVAL);
        if !due {
            return;
        }
        self.telemetry_state.last_preview_frame_telemetry = Some(now);
        let frame_time_ms = ctx.input(|i| i.unstable_dt) * 1000.0;
        self.record_telemetry(avcore::TelemetryEvent::PreviewFrameTime { frame_time_ms });
    }
}
