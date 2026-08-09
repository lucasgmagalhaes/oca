//! `nivela-app` — the native GUI shell for NivelaEditor, built on `eframe`/`egui` with the
//! `glow` (OpenGL) backend. Owns everything UI-specific: screens ([`screens`]), the dark/teal
//! theme ([`theme`]), translated display text ([`i18n`]), and the top-level app state
//! ([`app::NivelaApp`]). The actual project/media/timeline data model and `ffprobe`/`ffmpeg`
//! wrappers live in the UI-agnostic `nivela-core` crate this depends on.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod i18n;
mod screens;
mod theme;

use app::NivelaApp;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("NivelaEditor")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "NivelaEditor",
        native_options,
        Box::new(|cc| Ok(Box::new(NivelaApp::new(cc)))),
    )
}
