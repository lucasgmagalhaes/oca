use eframe::egui::{self, RichText};

use crate::app::{OcaApp, LUFS_PROFILES};
use crate::i18n::{Locale, Text};
use crate::screens::widgets;
use crate::theme;

/// Renders the Ajustes screen: language switcher, audio/export/project settings, and the
/// keyboard shortcut reference table. Settings write straight into `app.prefs`/`app.locale`
/// and aren't persisted yet — see [`crate::app::PrefsState`].
pub fn show(app: &mut OcaApp, ui: &mut egui::Ui) {
    let locale = app.locale;
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(20.0);
        ui.label(
            RichText::new(Text::PrefsTitle.tr(locale))
                .size(20.0)
                .strong(),
        );
        ui.add_space(16.0);
        ui.set_max_width(640.0);

        widgets::card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(Text::PrefsLanguage.tr(locale))
                    .size(11.0)
                    .color(theme::TEXT_MUTED)
                    .strong(),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                for candidate in Locale::ALL {
                    if ui
                        .selectable_label(app.locale == candidate, candidate.native_name())
                        .clicked()
                    {
                        app.locale = candidate;
                    }
                }
            });
        });
        ui.add_space(14.0);

        widgets::card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(Text::PrefsAudio.tr(locale))
                    .size(11.0)
                    .color(theme::TEXT_MUTED)
                    .strong(),
            );
            ui.add_space(6.0);
            ui.label(Text::PrefsNormalizationProfile.tr(locale));
            ui.horizontal(|ui| {
                for (i, (label, _)) in LUFS_PROFILES.iter().enumerate() {
                    if ui
                        .selectable_label(app.prefs.lufs_profile == i, *label)
                        .clicked()
                    {
                        app.prefs.lufs_profile = i;
                    }
                }
            });
            ui.add_space(6.0);
            ui.checkbox(
                &mut app.prefs.true_peak_limiter,
                Text::PrefsTruePeakLimiter.tr(locale),
            );
        });
        ui.add_space(14.0);

        widgets::card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(Text::PrefsExport.tr(locale))
                    .size(11.0)
                    .color(theme::TEXT_MUTED)
                    .strong(),
            );
            ui.add_space(6.0);
            ui.label(Text::PrefsExportWorkers.tr(locale));
            ui.horizontal(|ui| {
                for w in [1u8, 2, 4, 8] {
                    if ui
                        .selectable_label(app.prefs.export_workers == w, format!("{w}"))
                        .clicked()
                    {
                        app.prefs.export_workers = w;
                    }
                }
            });
            ui.add_space(10.0);
            ui.label(Text::PrefsOutputFolder.tr(locale));
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.prefs.output_folder).desired_width(400.0),
                );
                let _ = ui.button(Text::Browse.tr(locale));
            });
        });
        ui.add_space(14.0);

        widgets::card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(Text::PrefsProject.tr(locale))
                    .size(11.0)
                    .color(theme::TEXT_MUTED)
                    .strong(),
            );
            ui.add_space(6.0);
            ui.label(Text::PrefsAutosaveInterval.tr(locale));
            ui.horizontal(|ui| {
                for m in [1u8, 5, 10] {
                    if ui
                        .selectable_label(app.prefs.autosave_minutes == m, format!("{m} min"))
                        .clicked()
                    {
                        app.prefs.autosave_minutes = m;
                    }
                }
            });
        });
        ui.add_space(14.0);

        widgets::card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(Text::PrefsShortcuts.tr(locale))
                    .size(11.0)
                    .color(theme::TEXT_MUTED)
                    .strong(),
            );
            ui.add_space(6.0);
            egui::Grid::new("shortcuts_table")
                .num_columns(2)
                .spacing(egui::vec2(24.0, 6.0))
                .striped(false)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(Text::TableAction.tr(locale))
                            .color(theme::TEXT_MUTED)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(Text::TableShortcut.tr(locale))
                            .color(theme::TEXT_MUTED)
                            .strong(),
                    );
                    ui.end_row();

                    for (action, key) in [
                        (Text::ShortcutSplit, "S"),
                        (Text::ShortcutCut, "X"),
                        (Text::ShortcutPlayPause, Text::KeySpace.tr(locale)),
                        (Text::ShortcutMarkInOut, "I / O"),
                        (Text::ShortcutSendToQueue, "Ctrl+E"),
                    ] {
                        ui.label(action.tr(locale));
                        ui.label(RichText::new(key).color(theme::TEXT_MUTED).monospace());
                        ui.end_row();
                    }
                });
        });
    });
}
