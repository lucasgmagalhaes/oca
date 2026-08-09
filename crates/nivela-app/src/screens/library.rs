use eframe::egui::{self, RichText};
use nivela_core::media::MediaKind;

use crate::app::NivelaApp;
use crate::i18n::Text;
use crate::screens::widgets;
use crate::theme;

/// Renders the Mídia screen: a grid of every asset in the active project's media library.
pub fn show(app: &mut NivelaApp, ui: &mut egui::Ui) {
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
                let _ = ui.button(Text::ImportFiles.tr(locale));
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
