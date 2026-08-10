use std::path::PathBuf;

use avcore::media::MediaKind;
use avcore::project::Project;
use eframe::egui::{self, RichText};

use crate::app::OcaApp;
use crate::i18n::Text;
use crate::screens::widgets;
use crate::theme;

/// Renders the Mídia screen: a grid of every asset in the active project's media library.
pub fn show(app: &mut OcaApp, ui: &mut egui::Ui) {
    let locale = app.locale;
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(Text::LibraryTitle.tr(locale))
                    .size(20.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(Text::ImportFiles.tr(locale)).clicked() {
                    if let Some(paths) = rfd::FileDialog::new().pick_files() {
                        import_files(app, &paths);
                    }
                }
            });
        });
        ui.add_space(16.0);

        let card_width = 210.0;
        egui::Grid::new("library_grid")
            .spacing(egui::vec2(14.0, 14.0))
            .min_col_width(card_width)
            .show(ui, |ui| {
                let cols_per_row = 4usize;
                let mut col = 0usize;
                for asset in &app.active_project().media_library {
                    egui::Frame::new()
                        .fill(theme::SURFACE)
                        .stroke(egui::Stroke::new(1.0, theme::BORDER))
                        .corner_radius(8)
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.set_width(card_width - 16.0);
                            ui.vertical(|ui| {
                                egui::Frame::new()
                                    .fill(theme::SURFACE_2)
                                    .corner_radius(6)
                                    .show(ui, |ui| {
                                        ui.set_min_width(ui.available_width());
                                        ui.set_min_height(90.0);
                                        ui.centered_and_justified(|ui| {
                                            let icon = match asset.kind {
                                                MediaKind::Video => "▶",
                                                MediaKind::Audio => "♪",
                                            };
                                            ui.label(
                                                RichText::new(icon)
                                                    .size(22.0)
                                                    .color(theme::TEXT_MUTED),
                                            );
                                        });
                                    });
                                ui.label(RichText::new(&asset.file_name).size(12.5));
                                ui.horizontal_wrapped(|ui| {
                                    widgets::tag_outline(ui, &asset.codec);
                                    if asset.source_bitrate_mbps > 0.0 {
                                        widgets::tag_outline(
                                            ui,
                                            &format!("{:.0} Mbps", asset.source_bitrate_mbps),
                                        );
                                    }
                                    if let Some(khz) = asset.sample_rate_khz {
                                        widgets::tag_outline(ui, &format!("{khz:.0}kHz"));
                                    }
                                    if let Some(l) = &asset.loudness {
                                        widgets::tag_outline(
                                            ui,
                                            &format!("{:.1} LUFS", l.integrated_lufs),
                                        );
                                    }
                                    if asset.proxy_path.is_some() {
                                        widgets::tag_accent(ui, Text::ProxyReady.tr(locale));
                                    }
                                });
                                ui.label(
                                    RichText::new(asset.duration_label())
                                        .size(10.5)
                                        .color(theme::TEXT_MUTED),
                                );
                            });
                        });
                    col += 1;
                    if col >= cols_per_row {
                        col = 0;
                        ui.end_row();
                    }
                }
            });

        if app.active_project().media_library.is_empty() {
            ui.add_space(20.0);
            ui.label(RichText::new(Text::LibraryEmpty.tr(locale)).color(theme::TEXT_MUTED));
        }
    });
}

/// Probes and measures each path with `avcore::probe`/`avcore::loudness`, generates
/// a scrubbing-friendly proxy for video clips (see `avcore::proxy` — the same trick
/// CapCut/Premiere use so the timeline isn't decoding full 4K/60fps source every frame), and
/// adds the resulting assets to the active project's media library.
///
/// Runs synchronously on the UI thread (each file blocks on `ffprobe`/`ffmpeg` subprocesses,
/// and the proxy transcode is the slowest of the three) — fine for a handful of files from a
/// picker dialog; a real "import a folder of hour-long recordings" flow would want this on a
/// background thread instead. A file that fails to probe is skipped (logged to stderr) rather
/// than aborting the whole import; a proxy that fails to generate just leaves that asset
/// without one — the original file is still fully editable, just not as light to scrub.
fn import_files(app: &mut OcaApp, paths: &[PathBuf]) {
    let mut next_id = app
        .active_project()
        .media_library
        .iter()
        .map(|a| a.id)
        .max()
        .unwrap_or(0);
    let proxy_dir = proxy_cache_dir(app.active_project());

    for path in paths {
        let probed = match avcore::probe_media(path) {
            Ok(probed) => probed,
            Err(e) => {
                eprintln!("failed to probe {}: {e}", path.display());
                continue;
            }
        };

        next_id += 1;
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut asset = probed.into_media_asset(next_id, file_name, path.clone());

        match avcore::measure_loudness(path) {
            Ok(metrics) => asset.loudness = Some(metrics),
            Err(e) => eprintln!("failed to measure loudness for {}: {e}", path.display()),
        }

        if asset.kind == MediaKind::Video {
            match avcore::ensure_proxy(path, &proxy_dir) {
                Ok(proxy_path) => asset.proxy_path = Some(proxy_path),
                Err(e) => eprintln!("failed to generate proxy for {}: {e}", path.display()),
            }
        }

        app.active_project_mut().media_library.push(asset);
    }
}

/// Where imported clips' editing proxies are cached: a hidden sibling folder next to the
/// project file (`myproject.json` -> `.myproject_proxies/`), or a temp folder for a project
/// that hasn't been saved yet (proxies there won't survive a reboot, but neither would
/// anything else about an unsaved project).
fn proxy_cache_dir(project: &Project) -> PathBuf {
    match &project.file_path {
        Some(path) => {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("project");
            path.with_file_name(format!(".{stem}_proxies"))
        }
        None => std::env::temp_dir().join("oca_unsaved_proxies"),
    }
}
