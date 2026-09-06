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

use crate::app::App;
use crate::i18n::{Locale, Text};
use crate::theme;

/// Renders the disposable before/after voice-cleanup samples for one selected clip.
pub(super) fn voice_cleanup_preview(
    app: &mut App,
    ui: &mut egui::Ui,
    clip_id: u64,
    locale: Locale,
) {
    let rendering = app.voice_cleanup_preview_state.rendering_clip_id == Some(clip_id);
    let mut spawn_preview = false;
    let mut play_bypassed = false;
    let mut play_processed = false;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                !rendering,
                egui::Button::new(Text::VoiceCleanupPreviewButton.tr(locale)),
            )
            .clicked()
        {
            spawn_preview = true;
        }
        if rendering {
            ui.label(
                RichText::new(Text::VoiceCleanupPreviewRendering.tr(locale))
                    .size(10.5)
                    .color(theme::TEXT_MUTED),
            );
        }
    });

    // Keep a copy of the metrics so the immutable borrow ends before dispatching actions.
    let result_for_clip = app
        .voice_cleanup_preview_state
        .result
        .as_ref()
        .filter(|(result_clip_id, _)| *result_clip_id == clip_id)
        .map(|(_, result)| (result.bypassed.metrics, result.processed.metrics));
    if let Some((bypassed_metrics, processed_metrics)) = result_for_clip {
        let metrics_line = |metrics: avcore::media::LoudnessMetrics| {
            format!(
                "{:.1} LUFS · {:.1} dBTP · {:.1} LU",
                metrics.integrated_lufs, metrics.true_peak_dbtp, metrics.loudness_range_lu
            )
        };
        ui.horizontal(|ui| {
            if ui
                .button(Text::VoiceCleanupPreviewPlayOriginal.tr(locale))
                .clicked()
            {
                play_bypassed = true;
            }
            ui.label(
                RichText::new(metrics_line(bypassed_metrics))
                    .size(10.5)
                    .color(theme::TEXT_MUTED),
            );
        });
        ui.horizontal(|ui| {
            if ui
                .button(Text::VoiceCleanupPreviewPlayProcessed.tr(locale))
                .clicked()
            {
                play_processed = true;
            }
            ui.label(
                RichText::new(metrics_line(processed_metrics))
                    .size(10.5)
                    .color(theme::TEXT_MUTED),
            );
        });
    }
    if spawn_preview {
        app.spawn_voice_cleanup_preview();
    }
    if play_bypassed {
        app.play_voice_cleanup_preview_sample(false);
    }
    if play_processed {
        app.play_voice_cleanup_preview_sample(true);
    }
}
