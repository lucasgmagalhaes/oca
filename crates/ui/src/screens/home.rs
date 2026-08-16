use eframe::egui::{self, RichText};

use crate::app::App;
use crate::components;
use crate::i18n::Text;
use crate::theme;

/// Renders the Início screen: a grid of recent-project cards. Clicking a card opens that
/// project in the Editor via [`crate::app::App::open_project`].
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(Text::HomeTitle.tr(app.locale))
                        .size(22.0)
                        .strong(),
                );
                ui.label(
                    RichText::new(Text::HomeSubtitle.tr(app.locale))
                        .size(13.0)
                        .color(theme::TEXT_SECONDARY),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(Text::NewProject.tr(app.locale)).clicked() {
                    app.create_new_project(Text::UntitledProject.tr(app.locale).to_string());
                }
                if ui.button(Text::OpenProject.tr(app.locale)).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("oca project", &["ocproj"])
                        .pick_file()
                    {
                        match avcore::load_project_from_file(&path) {
                            Ok(mut project) => {
                                project.file_path = Some(path.clone());
                                app.add_and_open_project(project);
                                app.check_autosave_on_open(&path);
                            }
                            Err(e) => app.push_toast(format!("Failed to open project: {e}")),
                        }
                    }
                }
            });
        });

        if let Some(update) = app.available_update.clone() {
            ui.add_space(10.0);
            components::card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!(
                        "{} {}",
                        Text::UpdateAvailable.tr(app.locale),
                        update.version
                    )));
                    ui.hyperlink_to(Text::UpdateAvailableLink.tr(app.locale), &update.html_url);
                });
            });
        }
        ui.add_space(20.0);

        let card_width = 280.0;
        egui::Grid::new("home_projects_grid")
            .spacing(egui::vec2(16.0, 16.0))
            .min_col_width(card_width)
            .show(ui, |ui| {
                let count = app.projects.len();
                let mut open_index: Option<usize> = None;
                let mut remove_index: Option<usize> = None;
                let mut rename_index: Option<usize> = None;
                let mut col = 0usize;
                let cols_per_row = 3usize;
                for i in 0..count {
                    let project = &app.projects[i];
                    let name = project.name.clone();
                    let summary = project.summary.clone();
                    let meta = crate::i18n::recency_label(app.locale, project.last_edited);
                    let track_names: Vec<String> = project
                        .timeline()
                        .tracks
                        .iter()
                        .map(|t| t.name.clone())
                        .collect();
                    let kicker = crate::i18n::track_summary(
                        app.locale,
                        project.media_library.len(),
                        &track_names,
                    );

                    let resp = components::card_frame().show(ui, |ui| {
                        ui.set_width(card_width - 28.0);
                        ui.vertical(|ui| {
                            egui::Frame::new()
                                .fill(theme::SURFACE_2)
                                .corner_radius(6)
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.set_min_height(90.0);
                                    ui.centered_and_justified(|ui| {
                                        ui.label(
                                            RichText::new("▶").size(26.0).color(theme::TEXT_MUTED),
                                        );
                                    });
                                });
                            ui.add_space(10.0);
                            ui.label(RichText::new(kicker).size(11.0).color(theme::TEXT_MUTED));
                            ui.label(RichText::new(name).strong());
                            ui.label(
                                RichText::new(summary)
                                    .size(12.5)
                                    .color(theme::TEXT_SECONDARY),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(format!("🕐 {meta}"))
                                    .size(11.0)
                                    .color(theme::TEXT_MUTED),
                            );
                        });
                    });

                    let card_resp = resp
                        .response
                        .interact(egui::Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand);

                    if card_resp.clicked() {
                        open_index = Some(i);
                    }

                    let file_path = app.projects[i].file_path.clone();
                    card_resp.context_menu(|ui| {
                        if ui
                            .button(crate::i18n::Text::HomeCtxRename.tr(app.locale))
                            .clicked()
                        {
                            rename_index = Some(i);
                        }
                        if ui
                            .button(crate::i18n::Text::HomeCtxRemove.tr(app.locale))
                            .clicked()
                        {
                            remove_index = Some(i);
                        }
                        if let Some(path) = &file_path {
                            if ui
                                .button(crate::i18n::Text::HomeCtxShowInFinder.tr(app.locale))
                                .clicked()
                            {
                                open_in_finder(path);
                            }
                        }
                    });

                    col += 1;
                    if col >= cols_per_row {
                        col = 0;
                        ui.end_row();
                    }
                }

                if let Some(i) = rename_index {
                    let current_name = app.projects[i].name.clone();
                    let current_summary = app.projects[i].summary.clone();
                    app.renaming_project = Some((i, current_name, current_summary));
                } else if let Some(i) = remove_index {
                    app.remove_project(i);
                } else if let Some(i) = open_index {
                    app.open_project(i);
                }
            });
    });
}

#[cfg(target_os = "macos")]
fn open_in_finder(path: &std::path::PathBuf) {
    let _ = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn();
}

#[cfg(target_os = "windows")]
fn open_in_finder(path: &std::path::PathBuf) {
    let _ = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn open_in_finder(path: &std::path::PathBuf) {
    if let Some(dir) = path.parent() {
        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
    }
}
