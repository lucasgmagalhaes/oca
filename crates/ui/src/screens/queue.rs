use avcore::export::ExportJobStatus;
use eframe::egui::{self, RichText};

use crate::app::{OcaApp, LUFS_PROFILES};
use crate::i18n::{self, Text};
use crate::screens::widgets;
use crate::theme;

/// Renders the Fila screen: the export queue's job list (reorder for queued jobs, cancel for
/// anything in flight, retry for failures) and the concurrent-worker count. Jobs are rendered
/// for real on background threads dispatched by [`crate::app::OcaApp`] each frame — see
/// its `pump_export_queue`.
pub fn show(app: &mut OcaApp, ui: &mut egui::Ui) {
    let locale = app.locale;
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(Text::QueueTitle.tr(locale))
                    .size(20.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let selected_asset = app.selected_asset().cloned();
                let add_button = egui::Button::new(Text::AddExport.tr(locale));
                let response = ui
                    .add_enabled(selected_asset.is_some(), add_button)
                    .on_disabled_hover_text(Text::AddExportNeedsClip.tr(locale));
                if response.clicked() {
                    if let Some(asset) = selected_asset {
                        let (_, target_lufs) = LUFS_PROFILES[app.prefs.lufs_profile];
                        let default_name =
                            format!("{}_export.mp4", asset.file_name.trim_end_matches(".mp4"));
                        if let Some(output) = rfd::FileDialog::new()
                            .add_filter("MP4", &["mp4"])
                            .set_file_name(default_name)
                            .save_file()
                        {
                            app.queue_export(
                                asset.file_name.clone(),
                                asset.source_path.clone(),
                                target_lufs,
                                asset.source_bitrate_mbps,
                                output.display().to_string(),
                            );
                        }
                    }
                }
                ui.add_space(10.0);
                ui.label(
                    RichText::new(Text::ConcurrentWorkers.tr(locale))
                        .size(12.0)
                        .color(theme::TEXT_MUTED),
                );
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
        let mut cancel: Option<u64> = None;
        let mut retry: Option<u64> = None;
        let mut resume: Option<u64> = None;

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
                                ExportJobStatus::Rendering { .. } => {
                                    widgets::tag_accent(ui, status_label)
                                }
                                ExportJobStatus::Queued => widgets::tag_outline(ui, status_label),
                                ExportJobStatus::Paused { .. } => {
                                    widgets::tag_outline(ui, status_label)
                                }
                                ExportJobStatus::Done => widgets::tag_accent(ui, status_label),
                                ExportJobStatus::Failed { .. } => {
                                    widgets::tag_error(ui, status_label)
                                }
                            }
                            ui.label(RichText::new(&job.title).size(13.0));
                        });

                        if let ExportJobStatus::Rendering { percent }
                        | ExportJobStatus::Paused { percent } = &job.status
                        {
                            ui.add(
                                egui::ProgressBar::new(*percent as f32 / 100.0).desired_height(5.0),
                            );
                        }
                        ui.label(
                            RichText::new(i18n::job_detail_line(locale, job))
                                .size(11.0)
                                .color(theme::TEXT_MUTED)
                                .monospace(),
                        );
                    });

                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| match &job.status {
                            // ffmpeg has no notion of pausing an in-flight render (see
                            // avcore::render's module docs), so a rendering job only
                            // offers Cancel — there's no Pause button to promise something
                            // this queue can't actually do.
                            ExportJobStatus::Rendering { .. } => {
                                if ui
                                    .button("✕")
                                    .on_hover_text(Text::CancelJob.tr(locale))
                                    .clicked()
                                {
                                    cancel = Some(job.id);
                                }
                            }
                            ExportJobStatus::Queued => {
                                if ui
                                    .button("✕")
                                    .on_hover_text(Text::RemoveJob.tr(locale))
                                    .clicked()
                                {
                                    cancel = Some(job.id);
                                }
                                if ui.button("▼").clicked() {
                                    move_down = Some(i);
                                }
                                if ui.button("▲").clicked() {
                                    move_up = Some(i);
                                }
                            }
                            ExportJobStatus::Paused { .. } => {
                                if ui
                                    .button("✕")
                                    .on_hover_text(Text::RemoveJob.tr(locale))
                                    .clicked()
                                {
                                    cancel = Some(job.id);
                                }
                                if ui
                                    .button("▶")
                                    .on_hover_text(Text::Resume.tr(locale))
                                    .clicked()
                                {
                                    resume = Some(job.id);
                                }
                            }
                            ExportJobStatus::Done => {
                                if ui
                                    .button("📂")
                                    .on_hover_text(Text::OpenFolder.tr(locale))
                                    .clicked()
                                {
                                    open_containing_folder(&job.output_path);
                                }
                            }
                            ExportJobStatus::Failed { .. } => {
                                if ui.button(Text::RetryExport.tr(locale)).clicked() {
                                    retry = Some(job.id);
                                }
                            }
                        },
                    );
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
        if let Some(id) = cancel {
            app.cancel_export_job(id);
        }
        if let Some(id) = resume {
            if let Some(job) = app.export_jobs.iter_mut().find(|j| j.id == id) {
                job.status = ExportJobStatus::Queued;
            }
        }
        if let Some(id) = retry {
            if let Some(job) = app.export_jobs.iter_mut().find(|j| j.id == id) {
                job.status = ExportJobStatus::Queued;
            }
        }
    });
}

#[cfg(target_os = "windows")]
fn open_containing_folder(output_path: &str) {
    let path = std::path::Path::new(output_path);
    let folder = path.parent().unwrap_or(path);
    let _ = std::process::Command::new("explorer").arg(folder).spawn();
}

#[cfg(not(target_os = "windows"))]
fn open_containing_folder(_output_path: &str) {}
