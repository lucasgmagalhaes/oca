//! `ui` — the native GUI shell for oca, built on `eframe`/`egui` with the
//! `glow` (OpenGL) backend. Owns everything UI-specific: screens ([`screens`]), the dark/teal
//! theme ([`theme`]), translated display text ([`i18n`]), and the top-level app state
//! ([`app::OcaApp`]). The actual project/media/timeline data model and `ffprobe`/`ffmpeg`
//! wrappers live in the UI-agnostic `core` crate this depends on.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod components;
mod i18n;
mod screens;
mod theme;

use app::OcaApp;

fn main() -> eframe::Result<()> {
    // TODO: call init_logging() here once tracing-subscriber + tracing-appender are in the
    // local cargo cache (run `cargo fetch` with network access, then uncomment the call and
    // the two dep lines in ui/Cargo.toml).

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("oca")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "oca",
        native_options,
        Box::new(|cc| Ok(Box::new(OcaApp::new(cc)))),
    )
}

/// Initialises the global tracing subscriber: a rolling daily file in the platform log dir
/// plus, in debug builds, a human-readable layer on stderr.
///
/// **Currently stubbed** — the subscriber crates (`tracing-subscriber`, `tracing-appender`)
/// are not yet in the local cargo cache. Restore the body below once they are available:
///
/// ```ignore
/// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
///
/// let log_dir = platform_log_dir();
/// let _ = std::fs::create_dir_all(&log_dir);
/// let file_appender = tracing_appender::rolling::daily(&log_dir, "oca.log");
/// let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
/// std::mem::forget(guard); // keep alive for process lifetime
/// let filter = EnvFilter::try_from_env("OCA_LOG")
///     .unwrap_or_else(|_| EnvFilter::new("info"));
/// tracing_subscriber::registry()
///     .with(filter)
///     .with(tracing_subscriber::fmt::layer().with_ansi(false).with_writer(non_blocking))
///     .init();
/// tracing::info!(version = env!("CARGO_PKG_VERSION"), "oca starting");
/// ```
#[allow(dead_code)]
fn init_logging() {}

/// Returns the platform-appropriate directory for log files.
///
/// - macOS:   `~/Library/Logs/oca`
/// - Windows: `%APPDATA%\oca\logs`
/// - Linux:   `~/.local/share/oca/logs`
///
/// Falls back to `logs/` next to the executable if the home/appdata dirs are unavailable.
fn platform_log_dir() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(home).join("Library/Logs/oca");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return std::path::PathBuf::from(appdata).join("oca\\logs");
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(home).join(".local/share/oca/logs");
        }
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("logs")))
        .unwrap_or_else(|| std::path::PathBuf::from("logs"))
}
