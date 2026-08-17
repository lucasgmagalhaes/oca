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

use avcore::{self, export::ExportJobStatus};
use eframe::egui::{self, RichText};

use crate::app::{App, LUFS_PROFILES};
use crate::components;
use crate::i18n::{self, Text};
use crate::theme;

/// Renders the Fila screen: the export queue's job list (reorder, pause/resume, cancel, retry)
/// and the concurrent-worker count. Jobs are rendered for real on background threads dispatched
/// by [`crate::app::App`] each frame — see its `pump_export_queue`.
pub fn show(app: &mut App, ui: &mut egui::Ui) {
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
                let project = app.active_project();
                let sequence_name = project.sequences[project.active_sequence].name.clone();
                let sequence = &project.sequences[project.active_sequence];
                let resolved =
                    avcore::resolve_timeline_segments_multi(sequence, &project.media_library)
                        .and_then(|(track_segments, canvas)| {
                            avcore::resolve_audio_segments(sequence, &project.media_library)
                                .map(|audio_segments| (track_segments, audio_segments, canvas))
                        });
                let size_estimate_label = if let Ok((ref track_segments, _, ref canvas)) = resolved
                {
                    let duration_secs: f64 = track_segments
                        .first()
                        .map(|segs| {
                            segs.iter()
                                .map(|s| {
                                    (s.source_out_secs - s.source_in_secs) / s.speed_factor as f64
                                })
                                .sum::<f64>()
                        })
                        .unwrap_or(0.0);
                    // 192 kbps AAC (matches bridge.c aenc_ctx->bit_rate for timeline export)
                    let bytes = (canvas.bit_rate_bps + 192_000) as f64 * duration_secs / 8.0;
                    Some(format_file_size(bytes))
                } else {
                    None
                };
                let add_button = egui::Button::new(Text::AddExport.tr(locale));
                let response = ui
                    .add_enabled(resolved.is_ok(), add_button)
                    .on_disabled_hover_text(Text::AddExportNeedsClip.tr(locale));
                if response.clicked() {
                    if let Ok((track_segments, audio_segments, canvas)) = resolved {
                        let (_, target_lufs) = LUFS_PROFILES[app.prefs.lufs_profile];
                        let canvas =
                            avcore::apply_export_aspect_ratio(canvas, app.export_aspect_ratio);
                        let active_sequence =
                            &app.active_project().sequences[app.active_project().active_sequence];
                        let text_segments =
                            avcore::resolve_text_segments(active_sequence, canvas.width);
                        let shape_segments = avcore::resolve_shape_segments(
                            active_sequence,
                            canvas.width,
                            canvas.height,
                        );
                        let default_name = format!("{sequence_name}_export.mp4");
                        let mut dialog = rfd::FileDialog::new()
                            .add_filter("MP4", &["mp4"])
                            .set_file_name(default_name);
                        if !app.prefs.output_folder.is_empty() {
                            dialog = dialog.set_directory(&app.prefs.output_folder);
                        }
                        if let Some(output) = dialog.save_file() {
                            if output.exists() {
                                app.pending_export_conflict =
                                    Some(crate::app::export::PendingExportConflict {
                                        title: sequence_name,
                                        track_segments,
                                        audio_segments,
                                        text_segments,
                                        shape_segments,
                                        canvas,
                                        target_lufs,
                                        output_path: output,
                                    });
                            } else {
                                app.queue_export(
                                    sequence_name,
                                    track_segments,
                                    audio_segments,
                                    text_segments,
                                    shape_segments,
                                    canvas,
                                    target_lufs,
                                    output.display().to_string(),
                                );
                            }
                        }
                    }
                }
                ui.add_space(8.0);
                ui.label(
                    eframe::egui::RichText::new(Text::ExportAspectRatioLabel.tr(locale))
                        .size(12.0)
                        .color(crate::theme::TEXT_MUTED),
                );
                for ratio in avcore::ExportAspectRatio::ALL {
                    let selected = app.export_aspect_ratio == *ratio;
                    if ui.selectable_label(selected, ratio.label()).clicked() {
                        app.export_aspect_ratio = *ratio;
                    }
                }
                if let Some(size_label) = size_estimate_label {
                    ui.add_space(6.0);
                    ui.label(
                        eframe::egui::RichText::new(
                            Text::ExportSizeEstimate
                                .tr(locale)
                                .replace("{size}", &size_label),
                        )
                        .size(11.0)
                        .color(crate::theme::TEXT_MUTED),
                    );
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
        let mut pause: Option<u64> = None;
        let mut retry: Option<u64> = None;
        let mut resume: Option<u64> = None;

        let len = app.export_jobs.len();
        for i in 0..len {
            let job = &app.export_jobs[i];
            components::card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⠿").color(theme::TEXT_MUTED));
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width() - 90.0);
                        ui.horizontal(|ui| {
                            let status_label = i18n::job_status_label(locale, &job.status);
                            match &job.status {
                                ExportJobStatus::Rendering { .. } => {
                                    components::tag_accent(ui, status_label)
                                }
                                ExportJobStatus::Queued => {
                                    components::tag_outline(ui, status_label)
                                }
                                ExportJobStatus::Paused { .. } => {
                                    components::tag_outline(ui, status_label)
                                }
                                ExportJobStatus::Done => components::tag_accent(ui, status_label),
                                ExportJobStatus::Failed { .. } => {
                                    components::tag_error(ui, status_label)
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
                            ExportJobStatus::Rendering { .. } => {
                                if ui
                                    .button("✕")
                                    .on_hover_text(Text::CancelJob.tr(locale))
                                    .clicked()
                                {
                                    cancel = Some(job.id);
                                }
                                if ui
                                    .button("⏸")
                                    .on_hover_text(Text::PauseJob.tr(locale))
                                    .clicked()
                                {
                                    pause = Some(job.id);
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
                                if ui
                                    .button("⏸")
                                    .on_hover_text(Text::PauseJob.tr(locale))
                                    .clicked()
                                {
                                    pause = Some(job.id);
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
        if let Some(id) = pause {
            app.pause_export_job(id);
        }
        if let Some(id) = resume {
            app.resume_export_job(id);
        }
        if let Some(id) = retry {
            if let Some(job) = app.export_jobs.iter_mut().find(|j| j.id == id) {
                job.status = ExportJobStatus::Queued;
            }
        }
    });
}

fn format_file_size(bytes: f64) -> String {
    if bytes >= 1_073_741_824.0 {
        format!("{:.1} GB", bytes / 1_073_741_824.0)
    } else {
        format!("{:.0} MB", bytes / 1_048_576.0)
    }
}

#[cfg(target_os = "windows")]
fn open_containing_folder(output_path: &str) {
    let path = std::path::Path::new(output_path);
    let folder = path.parent().unwrap_or(path);
    let _ = std::process::Command::new("explorer").arg(folder).spawn();
}

#[cfg(target_os = "macos")]
fn open_containing_folder(output_path: &str) {
    let path = std::path::Path::new(output_path);
    let folder = path.parent().unwrap_or(path);
    let _ = std::process::Command::new("open").arg(folder).spawn();
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn open_containing_folder(output_path: &str) {
    let path = std::path::Path::new(output_path);
    let folder = path.parent().unwrap_or(path);
    let _ = std::process::Command::new("xdg-open").arg(folder).spawn();
}
