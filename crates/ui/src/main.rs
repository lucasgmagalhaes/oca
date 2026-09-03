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

//! `ui` — the native GUI shell for oca, built on `eframe`/`egui` with the
//! `glow` (OpenGL) backend. Owns everything UI-specific: screens ([`screens`]), the dark/teal
//! theme ([`theme`]), translated display text ([`i18n`]), and the top-level app state
//! ([`app::App`]). The actual project/media/timeline data model and `ffprobe`/`ffmpeg`
//! wrappers live in the UI-agnostic `core` crate this depends on.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod components;
mod i18n;
mod icons;
mod screens;
mod theme;

use app::App;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn main() -> eframe::Result<()> {
    // Must happen before GStreamer/ytbridge are initialized so packaged runtimes are selected
    // instead of similarly named system installations.
    avcore::configure_bundled_runtime();
    init_logging();
    install_panic_hook();

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("oca")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 600.0])
            // No OS window chrome — `screens::breadcrumb` draws oca's own themed title bar
            // (drag-to-move, double-click-to-maximize, minimize/maximize/close buttons) in its
            // place, and `screens::breadcrumb::handle_resize_borders` replaces the OS's own
            // edge/corner resize handles this removes along with the rest of the chrome.
            .with_decorations(false),
        ..Default::default()
    };

    eframe::run_native(
        "oca",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

/// Initialises the global tracing subscriber: a rolling daily file in the platform log dir
/// plus, in debug builds, a human-readable pretty-printed layer on stderr.
///
/// The `OCA_LOG` env var overrides the default filter (`info`).
/// Log files rotate daily; `tracing-appender` itself does not delete old files, so rotation
/// keeps accumulation bounded at one file per day.
fn init_logging() {
    let log_dir = platform_log_dir();
    let _ = std::fs::create_dir_all(&log_dir);

    let file_appender = tracing_appender::rolling::daily(&log_dir, "oca.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    // Keep the guard alive for the process lifetime — dropping it flushes and closes the
    // file, so we must not drop it before main() returns.
    std::mem::forget(guard);

    let filter = tracing_subscriber::EnvFilter::try_from_env("OCA_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking);

    #[cfg(debug_assertions)]
    {
        let stderr_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .pretty();
        tracing_subscriber::registry()
            .with(filter)
            .with(file_layer)
            .with(stderr_layer)
            .init();
    }
    #[cfg(not(debug_assertions))]
    {
        tracing_subscriber::registry()
            .with(filter)
            .with(file_layer)
            .init();
    }

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "oca starting");
}

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

/// Installs a custom panic hook that writes a crash report to the log directory before
/// calling the default hook (which prints to stderr). Each panic produces one
/// `crash_<unix_seconds>.txt` file containing the panic message, source location, app
/// version, and a full backtrace (always captured, regardless of `RUST_BACKTRACE`).
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = std::backtrace::Backtrace::force_capture();

        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("<unknown>");

        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".to_owned());

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let report = format!(
            "oca v{ver} crash report\ntimestamp (unix): {timestamp}\nlocation: {location}\nmessage: {msg}\n\nbacktrace:\n{backtrace}\n",
            ver = env!("CARGO_PKG_VERSION"),
        );

        let log_dir = platform_log_dir();
        let _ = std::fs::create_dir_all(&log_dir);
        let crash_path = log_dir.join(format!("crash_{timestamp}.txt"));
        let _ = std::fs::write(&crash_path, report.as_bytes());

        default_hook(info);
    }));
}
