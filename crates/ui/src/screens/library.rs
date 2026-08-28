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

use avcore::media::MediaKind;
use eframe::egui::{self, RichText};

use crate::app::App;
use crate::components;
use crate::i18n::Text;
use crate::theme;

/// Renders the Mídia screen: a grid of every asset in the active project's media library.
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    app.ensure_active_project();
    let locale = app.locale;
    let mut transcribe_clicked: Option<u64> = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(theme::SPACE_LG);
        ui.horizontal(|ui| {
            components::page_title(ui, Text::LibraryTitle.tr(locale));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(Text::ImportFiles.tr(locale)).clicked() {
                    if let Some(paths) = rfd::FileDialog::new().pick_files() {
                        app.spawn_import(paths);
                    }
                }
                if ui.button(Text::TtsButton.tr(locale)).clicked() {
                    app.open_tts_modal();
                }
                if ui.button(Text::YoutubeDownloadButton.tr(locale)).clicked() {
                    app.open_youtube_modal();
                }
                if app.tts_state.tts_generating {
                    ui.label(
                        RichText::new(Text::TtsGenerating.tr(locale))
                            .size(12.0)
                            .color(theme::TEXT_MUTED),
                    );
                }
                if app.import_state.pending_imports > 0 {
                    ui.label(
                        RichText::new(Text::Importing.tr(locale))
                            .size(12.0)
                            .color(theme::TEXT_MUTED),
                    );
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
                    components::card_frame().show(ui, |ui| {
                        ui.set_width(card_width - 28.0);
                        ui.vertical(|ui| {
                            egui::Frame::new()
                                .fill(theme::SURFACE_2)
                                .corner_radius(theme::RADIUS_SM)
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.set_min_height(90.0);
                                    ui.centered_and_justified(|ui| {
                                        let icon = match asset.kind {
                                            MediaKind::Video => "▶",
                                            MediaKind::Audio => "♪",
                                        };
                                        ui.label(
                                            RichText::new(icon).size(22.0).color(theme::TEXT_MUTED),
                                        );
                                    });
                                });
                            ui.label(RichText::new(&asset.file_name).size(12.5));
                            ui.horizontal_wrapped(|ui| {
                                components::tag_outline(ui, &asset.codec);
                                if asset.source_bitrate_mbps > 0.0 {
                                    components::tag_outline(
                                        ui,
                                        &format!("{:.0} Mbps", asset.source_bitrate_mbps),
                                    );
                                }
                                if let Some(khz) = asset.sample_rate_khz {
                                    components::tag_outline(ui, &format!("{khz:.0}kHz"));
                                }
                                if let Some(l) = &asset.loudness {
                                    components::tag_outline(
                                        ui,
                                        &format!("{:.1} LUFS", l.integrated_lufs),
                                    );
                                }
                                if asset.proxy_path.is_some() {
                                    components::tag_accent(ui, Text::ProxyReady.tr(locale));
                                }
                            });
                            ui.label(
                                RichText::new(asset.duration_label())
                                    .size(10.5)
                                    .color(theme::TEXT_MUTED),
                            );
                            ui.add_space(4.0);
                            if app.transcribe_state.transcribing_asset_id == Some(asset.id) {
                                ui.label(
                                    RichText::new(Text::TranscribeInProgress.tr(locale))
                                        .size(10.5)
                                        .color(theme::TEXT_MUTED),
                                );
                            } else if ui.small_button(Text::TranscribeAction.tr(locale)).clicked() {
                                transcribe_clicked = Some(asset.id);
                            }
                        });
                    });
                    col += 1;
                    if col >= cols_per_row {
                        col = 0;
                        ui.end_row();
                    }
                }
            });

        if let Some(asset_id) = transcribe_clicked {
            app.spawn_transcribe(asset_id);
        }

        if app.active_project().media_library.is_empty() {
            ui.add_space(theme::SPACE_LG);
            ui.label(RichText::new(Text::LibraryEmpty.tr(locale)).color(theme::TEXT_MUTED));
        }
    });
}
