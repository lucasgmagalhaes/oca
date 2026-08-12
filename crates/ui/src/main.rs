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
