use eframe::egui::{self, RichText};

use crate::app::NivelaApp;
use crate::i18n::Text;
use crate::screens::widgets;
use crate::theme;

/// Renders the Início screen: a grid of recent-project cards. Clicking a card opens that
/// project in the Editor via [`crate::app::NivelaApp::open_project`].
pub fn show(app: &mut NivelaApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(Text::HomeTitle.tr(app.locale)).size(22.0).strong());
                ui.label(
                    RichText::new(Text::HomeSubtitle.tr(app.locale))
                        .size(13.0)
                        .color(theme::TEXT_SECONDARY),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(Text::NewProject.tr(app.locale)).clicked() {
                    // Fase 1: cria um Project vazio e abre no Editor.
                }
            });
        });
        ui.add_space(20.0);

        let card_width = 280.0;
        egui::Grid::new("home_projects_grid")
            .spacing(egui::vec2(16.0, 16.0))
            .min_col_width(card_width)
            .show(ui, |ui| {
                let count = app.projects.len();
                let mut open_index: Option<usize> = None;
                let mut col = 0usize;
                let cols_per_row = 3usize;
                for i in 0..count {
                    let project = &app.projects[i];
                    let name = project.name.clone();
                    let summary = project.summary.clone();
                    let meta = crate::i18n::recency_label(app.locale, project.last_edited);
                    let track_names: Vec<String> =
                        project.timeline.tracks.iter().map(|t| t.name.clone()).collect();
                    let kicker = crate::i18n::track_summary(
                        app.locale,
                        project.media_library.len(),
                        &track_names,
                    );

                    let resp = widgets::card_frame().show(ui, |ui| {
                        ui.set_width(card_width - 28.0);
                        ui.vertical(|ui| {
                            egui::Frame::new()
                                .fill(theme::SURFACE_2)
                                .corner_radius(6)
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.set_min_height(90.0);
                                    ui.centered_and_justified(|ui| {
                                        ui.label(RichText::new("▶").size(26.0).color(theme::TEXT_MUTED));
                                    });
                                });
                            ui.add_space(10.0);
                            ui.label(RichText::new(kicker).size(11.0).color(theme::TEXT_MUTED));
                            ui.label(RichText::new(name).strong());
                            ui.label(RichText::new(summary).size(12.5).color(theme::TEXT_SECONDARY));
                            ui.add_space(6.0);
                            ui.label(RichText::new(format!("🕐 {meta}")).size(11.0).color(theme::TEXT_MUTED));
                        });
                    });

                    if resp
                        .response
                        .interact(egui::Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        open_index = Some(i);
                    }

                    col += 1;
                    if col >= cols_per_row {
                        col = 0;
                        ui.end_row();
                    }
                }

                if let Some(i) = open_index {
                    app.open_project(i);
                }
            });
    });
}
