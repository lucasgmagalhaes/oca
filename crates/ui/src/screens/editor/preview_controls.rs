// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use eframe::egui::{self, RichText};

use crate::app::{App, PreviewZoom};
use crate::components;
use crate::i18n::Text;
use crate::theme;

use super::{format_timecode, TRANSPORT_ICON_SIZE, TRANSPORT_PLAY_ICON_SIZE};

/// Program Monitor label and viewport zoom controls.
pub(super) fn preview_header(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.horizontal(|ui| {
        components::section_label(ui, Text::ProgramMonitor.tr(locale));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .selectable_label(
                    app.preview_state.zoom == PreviewZoom::Percent100,
                    Text::PreviewZoom100.tr(locale),
                )
                .clicked()
            {
                app.preview_state.zoom = PreviewZoom::Percent100;
            }
            if ui
                .selectable_label(
                    app.preview_state.zoom == PreviewZoom::Fit,
                    Text::PreviewZoomFit.tr(locale),
                )
                .clicked()
            {
                app.preview_state.zoom = PreviewZoom::Fit;
            }
            if ui
                .selectable_label(
                    app.preview_state.zoom == PreviewZoom::Percent50,
                    Text::PreviewZoom50.tr(locale),
                )
                .clicked()
            {
                app.preview_state.zoom = PreviewZoom::Percent50;
            }
        });
    });
}

/// Timeline position readout and scrubber displayed above the transport controls.
pub(super) fn preview_scrubber(app: &mut App, ui: &mut egui::Ui, timeline_duration: f64) {
    let playhead = app.active_project().timeline().playhead_secs;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format_timecode(playhead))
                .size(12.0)
                .color(theme::ACCENT)
                .monospace(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format_timecode(timeline_duration.max(playhead)))
                    .size(12.0)
                    .color(theme::TEXT_SECONDARY)
                    .monospace(),
            );
            if timeline_duration > 0.0 {
                let mut position = playhead;
                let slider = ui.add(
                    egui::Slider::new(&mut position, 0.0..=timeline_duration).show_value(false),
                );
                if slider.changed() {
                    app.seek_preview(position);
                }
            }
        });
    });
}

/// Centered transport buttons plus the Program Monitor's right-aligned auxiliary actions.
pub(super) fn preview_transport_row(
    app: &mut App,
    ui: &mut egui::Ui,
    locale: crate::i18n::Locale,
    timeline_duration: f64,
) {
    ui.columns(3, |columns| {
        // The empty left column balances the centered transport cluster.
        columns[1].vertical_centered(|ui| {
            ui.horizontal(|ui| transport_controls(app, ui, locale, timeline_duration));
        });
        columns[2].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            audio_level_meter(app, ui);
            if ui
                .selectable_label(
                    app.preview_state.scopes_enabled,
                    RichText::new("S").color(theme::TEXT_SECONDARY),
                )
                .on_hover_text(Text::PreviewScopesToggle.tr(locale))
                .clicked()
            {
                app.preview_state.scopes_enabled = !app.preview_state.scopes_enabled;
            }
            if ui
                .small_button(RichText::new("[+]").color(theme::TEXT_SECONDARY))
                .on_hover_text(Text::EnterFullscreenPreview.tr(locale))
                .clicked()
            {
                app.toggle_fullscreen_preview();
            }
            if ui
                .small_button(
                    RichText::new("*")
                        .size(TRANSPORT_ICON_SIZE)
                        .color(theme::TEXT_SECONDARY),
                )
                .on_hover_text(Text::AddMarkerButton.tr(locale))
                .clicked()
            {
                app.add_marker_at_playhead(avcore::MarkerKind::Standard);
                app.push_toast(Text::MarkerAdded.tr(locale).to_string());
            }
            if components::icon_button(
                ui,
                crate::icons::CAMERA_STR,
                Text::SnapshotButton.tr(locale),
                components::IconButtonOpts {
                    family: Some(crate::icons::family()),
                    size: Some(TRANSPORT_ICON_SIZE),
                    ..Default::default()
                },
            )
            .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("png", &["png"])
                    .set_file_name("snapshot.png")
                    .save_file()
                {
                    app.save_preview_snapshot(path);
                }
            }
        });
    });
}

/// Optional decoded waveform and vectorscope images beneath the transport row.
pub(super) fn preview_scopes(app: &App, ui: &mut egui::Ui) {
    if !app.preview_state.scopes_enabled {
        return;
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if let Some(texture) = &app.preview_state.waveform_texture {
            ui.image((texture.id(), egui::vec2(200.0, 100.0)));
        }
        if let Some(texture) = &app.preview_state.vectorscope_texture {
            ui.image((texture.id(), egui::vec2(100.0, 100.0)));
        }
    });
}

/// The Program Monitor's centered transport-control cluster (skip-back, step-back, play/pause,
/// step-forward, skip-forward, loop) — split out of `preview_panel` so the 3-column centering
/// trick (`ui.columns(3, ...)`) there can call it just for the middle column.
pub(super) fn transport_controls(
    app: &mut App,
    ui: &mut egui::Ui,
    locale: crate::i18n::Locale,
    timeline_duration: f64,
) {
    {
        // Every glyph in this row shares one explicit size (`TRANSPORT_ICON_SIZE`) so the
        // two plain-Unicode step-frame/marker glyphs (no vendored Lucide icon exists for
        // either — see their own comments below) read at the same visual weight as the
        // Lucide icon-font glyphs around them, instead of each falling back to its own
        // font's default metrics and reading as an inconsistent mix of icon styles.
        if components::icon_button(
            ui,
            crate::icons::SKIP_BACK_STR,
            Text::SeekToStart.tr(locale),
            components::IconButtonOpts {
                family: Some(crate::icons::family()),
                size: Some(TRANSPORT_ICON_SIZE),
                ..Default::default()
            },
        )
        .clicked()
        {
            app.seek_preview(0.0);
        }
        // No vendored Lucide icon for single-frame step (`skip-back`/`skip-forward` are
        // start/end, already used above/below). Plain ASCII "<"/">" rather than the
        // Geometric-Shapes triangles ("◁"/"▷") originally here — confirmed tofu against
        // this app's bundled default font via a real screenshot (same class of bug as
        // nav_rail's monogram fallback and the toolbar's tool icons).
        if ui
            .small_button(
                RichText::new("<")
                    .size(TRANSPORT_ICON_SIZE)
                    .color(theme::TEXT_SECONDARY),
            )
            .on_hover_text(Text::StepFrameBack.tr(locale))
            .clicked()
        {
            app.step_preview_frame(-1);
        }
        let play_icon = if app.preview_state.preview_playing {
            crate::icons::PAUSE_STR
        } else {
            crate::icons::PLAY_STR
        };
        if ui
            .button(
                RichText::new(play_icon)
                    .family(crate::icons::family())
                    .size(TRANSPORT_PLAY_ICON_SIZE)
                    .color(theme::ACCENT),
            )
            .on_hover_text(Text::ShortcutPlayPause.tr(locale))
            .clicked()
        {
            app.toggle_preview_playback();
        }
        if ui
            .small_button(
                RichText::new(">")
                    .size(TRANSPORT_ICON_SIZE)
                    .color(theme::TEXT_SECONDARY),
            )
            .on_hover_text(Text::StepFrameForward.tr(locale))
            .clicked()
        {
            app.step_preview_frame(1);
        }
        if components::icon_button(
            ui,
            crate::icons::SKIP_FORWARD_STR,
            Text::SeekToEnd.tr(locale),
            components::IconButtonOpts {
                family: Some(crate::icons::family()),
                size: Some(TRANSPORT_ICON_SIZE),
                ..Default::default()
            },
        )
        .clicked()
        {
            app.seek_preview(timeline_duration);
        }
        if ui
            .selectable_label(
                app.preview_state.loop_enabled,
                RichText::new(crate::icons::REPEAT_STR)
                    .family(crate::icons::family())
                    .size(TRANSPORT_ICON_SIZE)
                    .color(theme::TEXT_SECONDARY),
            )
            .on_hover_text(Text::PreviewLoopToggle.tr(locale))
            .clicked()
        {
            app.preview_state.loop_enabled = !app.preview_state.loop_enabled;
        }
    }
}

/// Small live peak/RMS bar for the Editor preview panel's transport row — `spec/ROADMAP.md`
/// P4 item 30. Reads [`App::current_audio_level`] every frame the panel draws; stays visually
/// flat at zero when no pipeline is open, playback is paused, or the current clip has no
/// audio, same as any other VU meter idling on silence.
/// One bar of [`audio_level_meter`] — filled to `rms`, plus a peak-hold tick at `peak` (red
/// past 0.98, the same clip-warning threshold the single-channel meter used before the L/R
/// split below existed).
fn draw_meter_bar(painter: &egui::Painter, rect: egui::Rect, peak: f32, rms: f32) {
    painter.rect_filled(rect, theme::RADIUS_XS, theme::SURFACE_2);
    let peak = peak.clamp(0.0, 1.0);
    let rms = rms.clamp(0.0, 1.0);
    if rms > 0.0 {
        let mut rms_rect = rect;
        rms_rect.set_width(rect.width() * rms);
        // Standard green/yellow/red level convention, matching stereo_db_meter's own fix —
        // Section 39's own rule: "Meters are informational and must not use Petroleum Blue as
        // their signal color."
        let fill_color = if rms > 0.9 {
            theme::ERROR
        } else if rms > 0.7 {
            theme::WARNING
        } else {
            theme::SUCCESS
        };
        painter.rect_filled(rms_rect, theme::RADIUS_XS, fill_color);
    }
    if peak > 0.0 {
        let peak_x = rect.left() + rect.width() * peak;
        let peak_color = if peak > 0.98 {
            theme::ERROR
        } else {
            theme::ACCENT_2
        };
        painter.vline(peak_x, rect.y_range(), egui::Stroke::new(2.0, peak_color));
    }
}

/// Two thin stacked horizontal bars (L on top, R on bottom) — the preview transport row's
/// compact stereo meter, per `avcore::AudioLevel`'s real per-channel peak/rms
/// (`spec/architecture/editor-ui-visual-redesign.md`'s Inspector section: "build the real
/// per-channel metering path first" — done in `avcore::preview`'s buffer probe — "or ship the
/// honest single-channel meter restyled taller"; this does the former, so both bars are
/// genuinely independent, not the same mono value drawn twice).
pub(super) fn audio_level_meter(app: &App, ui: &mut egui::Ui) {
    let level = app.current_audio_level();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(60.0, 14.0), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let bar_h = (rect.height() - 2.0) / 2.0;
    let l_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), bar_h));
    let r_rect = egui::Rect::from_min_size(
        rect.min + egui::vec2(0.0, bar_h + 2.0),
        egui::vec2(rect.width(), bar_h),
    );
    let painter = ui.painter();
    draw_meter_bar(painter, l_rect, level.peak_l, level.rms_l);
    draw_meter_bar(painter, r_rect, level.peak_r, level.rms_r);
    response.on_hover_text(Text::PreviewAudioLevelMeter.tr(app.locale));
}
