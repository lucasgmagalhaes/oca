use eframe::egui::{self, RichText};
use nivela_core::export::ExportJobStatus;

use crate::app::NivelaApp;
use crate::screens::widgets;
use crate::theme;

pub fn show(app: &mut NivelaApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Fila de exportação").size(20.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("＋ Adicionar exportação").clicked() {}
                ui.add_space(10.0);
                ui.label(RichText::new("Workers simultâneos").size(12.0).color(theme::TEXT_MUTED));
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
            RichText::new("A edição continua responsiva enquanto os jobs renderizam em segundo plano. A fila persiste entre sessões.")
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
                    RichText::new(
                        "Nota técnica: cada job é um snapshot (bitrate/perfil/destino) tirado no momento em que entra na fila — \
                         mudanças no projeto ativo depois disso não afetam o job. Render roda em worker separado da UI (tokio::mpsc); \
                         1 worker por padrão, configurável em Preferências.",
                    )
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
                            match &job.status {
                                ExportJobStatus::Rendering { .. } => widgets::tag_accent(ui, "Renderizando"),
                                ExportJobStatus::Queued => widgets::tag_outline(ui, "Na fila"),
                                ExportJobStatus::Paused { .. } => widgets::tag_outline(ui, "Pausado"),
                                ExportJobStatus::Done => widgets::tag_accent(ui, "Concluído"),
                                ExportJobStatus::Failed { .. } => widgets::tag_error(ui, "Falhou"),
                            }
                            ui.label(RichText::new(&job.title).size(13.0));
                        });

                        if let ExportJobStatus::Rendering { percent } | ExportJobStatus::Paused { percent } = &job.status {
                            ui.add(egui::ProgressBar::new(*percent as f32 / 100.0).desired_height(5.0));
                        }
                        ui.label(
                            RichText::new(job.detail_line())
                                .size(11.0)
                                .color(theme::TEXT_MUTED)
                                .monospace(),
                        );
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        match &job.status {
                            ExportJobStatus::Rendering { .. } => {
                                if ui.button("⏸").on_hover_text("Pausar").clicked() {}
                                if ui.button("✕").on_hover_text("Cancelar").clicked() {
                                    remove = Some(i);
                                }
                            }
                            ExportJobStatus::Queued => {
                                if ui.button("✕").on_hover_text("Remover").clicked() {
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
                                if ui.button("✕").on_hover_text("Remover").clicked() {
                                    remove = Some(i);
                                }
                                if ui.button("▶").on_hover_text("Retomar").clicked() {}
                            }
                            ExportJobStatus::Done => {
                                if ui.button("📂").on_hover_text("Abrir pasta").clicked() {}
                            }
                            ExportJobStatus::Failed { .. } => {
                                if ui.button("↻ Tentar novamente").clicked() {}
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
