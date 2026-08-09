use eframe::egui::{self, RichText};
use nivela_core::export::ExportJobStatus;

use crate::app::NivelaApp;
use crate::i18n::{self, Text};
use crate::screens::widgets;
use crate::theme;

/// Renders the Fila screen: the export queue's job list (with reorder/pause/cancel/retry
/// controls) and the concurrent-worker count. The list is all local UI state for now — Fase
/// 4 wires it up to a real background render worker over a `tokio::mpsc` channel.
pub fn show(app: &mut NivelaApp, ui: &mut egui::Ui) {
    let locale = app.locale;
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(Text::QueueTitle.tr(locale)).size(20.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(Text::AddExport.tr(locale)).clicked() {}
                ui.add_space(10.0);
                ui.label(RichText::new(Text::ConcurrentWorkers.tr(locale)).size(12.0).color(theme::TEXT_MUTED));
                for w in [4u8, 2, 1] {
                    let selected = app.queue_workers == w;
                    if ui.selectable_label(selected, format!("{w}")).clicked() {
                        app.queue_workers = w;
                    }
                }
            });
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new(Text::QueueSubtitle.tr(locale))
                .size(12.5)
                .color(theme::TEXT_SECONDARY),
        );
        ui.add_space(12.0);

        egui::Frame::new()
            .fill(theme::ACCENT.gamma_multiply(0.10))
            .corner_radius(8)
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(Text::QueueTechNote.tr(locale))
                        .size(11.0)
                        .color(theme::TEXT_SECONDARY),
                );
            });
        ui.add_space(16.0);

        let mut move_up: Option<usize> = None;
        let mut move_down: Option<usize> = None;
        let mut remove: Option<usize> = None;

        let len = app.export_jobs.len();
        for i in 0..len {
            let job = &app.export_jobs[i];
            widgets::card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⠿").color(theme::TEXT_MUTED));
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width() - 90.0);
                        ui.horizontal(|ui| {
                            let status_label = i18n::job_status_label(locale, &job.status);
                            match &job.status {
                                ExportJobStatus::Rendering { .. } => widgets::tag_accent(ui, status_label),
                                ExportJobStatus::Queued => widgets::tag_outline(ui, status_label),
                                ExportJobStatus::Paused { .. } => widgets::tag_outline(ui, status_label),
                                ExportJobStatus::Done => widgets::tag_accent(ui, status_label),
                                ExportJobStatus::Failed { .. } => widgets::tag_error(ui, status_label),
                            }
                            ui.label(RichText::new(&job.title).size(13.0));
                        });

                        if let ExportJobStatus::Rendering { percent } | ExportJobStatus::Paused { percent } = &job.status {
                            ui.add(egui::ProgressBar::new(*percent as f32 / 100.0).desired_height(5.0));
                        }
                        ui.label(
                            RichText::new(i18n::job_detail_line(locale, job))
                                .size(11.0)
                                .color(theme::TEXT_MUTED)
                                .monospace(),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        match &job.status {
                            ExportJobStatus::Rendering { .. } => {
                                if ui.button("⏸").on_hover_text(Text::Pause.tr(locale)).clicked() {}
                                if ui.button("✕").on_hover_text(Text::CancelJob.tr(locale)).clicked() {
                                    remove = Some(i);
                                }
                            }
                            ExportJobStatus::Queued => {
                                if ui.button("✕").on_hover_text(Text::RemoveJob.tr(locale)).clicked() {
                                    remove = Some(i);
                                }
                                if ui.button("▼").clicked() {
                                    move_down = Some(i);
                                }
                                if ui.button("▲").clicked() {
                                    move_up = Some(i);
                                }
                            }
                            ExportJobStatus::Paused { .. } => {
                                if ui.button("✕").on_hover_text(Text::RemoveJob.tr(locale)).clicked() {
                                    remove = Some(i);
                                }
                                if ui.button("▶").on_hover_text(Text::Resume.tr(locale)).clicked() {}
                            }
                            ExportJobStatus::Done => {
                                if ui.button("📂").on_hover_text(Text::OpenFolder.tr(locale)).clicked() {}
                            }
                            ExportJobStatus::Failed { .. } => {
                                if ui.button(Text::RetryExport.tr(locale)).clicked() {}
                            }
                        }
                    });
                });
            });
            ui.add_space(8.0);
        }

        if let Some(i) = move_up {
            if i > 0 {
                app.export_jobs.swap(i, i - 1);
            }
        }
        if let Some(i) = move_down {
            if i + 1 < app.export_jobs.len() {
                app.export_jobs.swap(i, i + 1);
            }
        }
        if let Some(i) = remove {
            app.export_jobs.remove(i);
        }
    });
}
