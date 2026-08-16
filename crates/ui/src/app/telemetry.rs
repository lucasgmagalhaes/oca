use std::path::PathBuf;

use eframe::egui;
use tokio::sync::mpsc::UnboundedReceiver;

use super::{App, PREVIEW_FRAME_TELEMETRY_INTERVAL};

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
        let _ = self.telemetry_tx.send(event);
    }

    /// Records a `PreviewFrameTime` sample while the preview is playing — throttled to once
    /// per [`PREVIEW_FRAME_TELEMETRY_INTERVAL`] rather than every frame. Called from
    /// [`eframe::App::ui`]'s `preview_playing` branch, the same place that already special-
    /// cases every-frame repaint for smooth video.
    pub(super) fn sample_preview_frame_telemetry(&mut self, ctx: &egui::Context) {
        let now = std::time::Instant::now();
        let due = self
            .last_preview_frame_telemetry
            .is_none_or(|last| now.duration_since(last) >= PREVIEW_FRAME_TELEMETRY_INTERVAL);
        if !due {
            return;
        }
        self.last_preview_frame_telemetry = Some(now);
        let frame_time_ms = ctx.input(|i| i.unstable_dt) * 1000.0;
        self.record_telemetry(avcore::TelemetryEvent::PreviewFrameTime { frame_time_ms });
    }
}
