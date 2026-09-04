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

use eframe::egui::{self, RichText};

use crate::app::{App, WatchFolderFileStatus};
use crate::components;
use crate::i18n::Text;
use crate::theme;

/// Renders the Limpeza screen: watch-folder audio cleanup, the in-app replacement for
/// `scripts/Watch-Gameplay.ps1`. Not project-scoped — works directly against files on disk, no
/// active project needed.
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    let mut add_to_project: Option<std::path::PathBuf> = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(theme::SPACE_LG);
        components::page_title(ui, Text::WatchFolderTitle.tr(locale));
        ui.label(
            RichText::new(Text::WatchFolderSubtitle.tr(locale))
                .size(12.5)
                .color(theme::TEXT_SECONDARY),
        );
        ui.add_space(theme::SPACE_MD);

        components::card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(Text::WatchFolderPathLabel.tr(locale))
                    .size(11.0)
                    .color(theme::TEXT_MUTED)
                    .strong(),
            );
            ui.add_space(theme::SPACE_XS);
            ui.horizontal(|ui| {
                let path_label = app
                    .watch_folder_state
                    .watch_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| Text::WatchFolderNoFolder.tr(locale).to_string());
                ui.label(RichText::new(path_label).monospace().color(
                    if app.watch_folder_state.watch_path.is_some() {
                        theme::TEXT_PRIMARY
                    } else {
                        theme::TEXT_MUTED
                    },
                ));
                ui.add_enabled_ui(!app.watch_folder_state.running, |ui| {
                    if ui
                        .button(Text::WatchFolderChooseFolder.tr(locale))
                        .clicked()
                    {
                        app.choose_watch_folder();
                    }
                });
            });
            ui.add_space(theme::SPACE_SM);
            if app.watch_folder_state.running {
                if components::primary_button(ui, Text::WatchFolderStop.tr(locale)).clicked() {
                    app.stop_watching_folder();
                }
            } else {
                ui.add_enabled_ui(app.watch_folder_state.watch_path.is_some(), |ui| {
                    if components::primary_button(ui, Text::WatchFolderStart.tr(locale)).clicked() {
                        app.start_watching_folder();
                    }
                });
            }
            ui.add_space(theme::SPACE_XS);
            ui.label(
                RichText::new(Text::WatchFolderOutputHint.tr(locale))
                    .size(10.5)
                    .color(theme::TEXT_MUTED),
            );
        });
        ui.add_space(theme::SPACE_MD);

        if app.watch_folder_state.files.is_empty() {
            ui.add_space(theme::SPACE_MD);
            ui.label(RichText::new(Text::WatchFolderEmpty.tr(locale)).color(theme::TEXT_MUTED));
            return;
        }

        let has_open_project = !app.projects.is_empty();
        for row in &app.watch_folder_state.files {
            components::card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let status_label = match row.status {
                        WatchFolderFileStatus::Stabilizing => {
                            Text::WatchFolderStatusStabilizing.tr(locale)
                        }
                        WatchFolderFileStatus::Processing => {
                            Text::WatchFolderStatusProcessing.tr(locale)
                        }
                        WatchFolderFileStatus::Done => Text::WatchFolderStatusDone.tr(locale),
                        WatchFolderFileStatus::Error => Text::WatchFolderStatusError.tr(locale),
                    };
                    match row.status {
                        WatchFolderFileStatus::Stabilizing => {
                            components::tag_outline(ui, status_label)
                        }
                        WatchFolderFileStatus::Processing => {
                            components::tag_accent(ui, status_label)
                        }
                        WatchFolderFileStatus::Done => components::tag_success(ui, status_label),
                        WatchFolderFileStatus::Error => components::tag_error(ui, status_label),
                    }
                    ui.label(
                        RichText::new(
                            row.path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default(),
                        )
                        .size(13.0),
                    );
                });

                if row.status == WatchFolderFileStatus::Processing {
                    ui.add(egui::ProgressBar::new(row.percent as f32 / 100.0).desired_height(5.0));
                }

                if let (Some(before), Some(after)) = (&row.before, &row.after) {
                    ui.label(
                        RichText::new(
                            Text::WatchFolderLufsBeforeAfter
                                .tr(locale)
                                .replace("{before}", &format!("{:.1}", before.integrated_lufs))
                                .replace("{after}", &format!("{:.1}", after.integrated_lufs)),
                        )
                        .size(11.0)
                        .color(theme::TEXT_MUTED)
                        .monospace(),
                    );
                }

                if let Some(error) = &row.error {
                    ui.label(RichText::new(error).size(11.0).color(theme::ERROR));
                }

                if row.status == WatchFolderFileStatus::Done {
                    if row.added_to_project {
                        ui.label(
                            RichText::new(Text::WatchFolderAddedToProject.tr(locale))
                                .size(11.5)
                                .color(theme::TEXT_MUTED),
                        );
                    } else {
                        ui.add_enabled_ui(has_open_project, |ui| {
                            if ui
                                .button(Text::WatchFolderAddToProject.tr(locale))
                                .clicked()
                            {
                                add_to_project = Some(row.path.clone());
                            }
                        });
                    }
                }
            });
            ui.add_space(theme::SPACE_XS);
        }
    });

    if let Some(path) = add_to_project {
        app.add_watched_file_to_project(path);
    }
}
