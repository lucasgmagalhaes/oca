use eframe::egui::{self, RichText};

use crate::app::{App, BindableAction, LUFS_PROFILES};
use crate::components;
use crate::i18n::{Locale, Text};
use crate::theme;

/// Renders the Ajustes screen: language switcher, audio/export/project settings, and the
/// keyboard shortcut reference table. Settings write straight into `app.prefs`/`app.locale`
/// and aren't persisted yet — see [`crate::app::PrefsState`].
pub fn show(app: &mut App, ui: &mut egui::Ui) {
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

        components::card_frame().show(ui, |ui| {
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

        components::card_frame().show(ui, |ui| {
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

        components::card_frame().show(ui, |ui| {
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
            ui.label(Text::PrefsGpuEncoder.tr(locale));
            ui.horizontal(|ui| {
                use avcore::GpuEncoderPreference as Gpu;
                for (choice, label) in [
                    (Gpu::Auto, Text::GpuEncoderAuto),
                    (Gpu::Cpu, Text::GpuEncoderCpu),
                    (Gpu::Nvenc, Text::GpuEncoderNvenc),
                    (Gpu::QuickSync, Text::GpuEncoderQuickSync),
                    (Gpu::Amf, Text::GpuEncoderAmf),
                ] {
                    if ui
                        .selectable_label(app.prefs.gpu_encoder == choice, label.tr(locale))
                        .clicked()
                    {
                        app.prefs.gpu_encoder = choice;
                    }
                }
            });
            ui.add_space(10.0);
            ui.label(Text::PrefsOutputFolder.tr(locale));
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.prefs.output_folder).desired_width(400.0),
                );
                if ui.button(Text::Browse.tr(locale)).clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        app.prefs.output_folder = folder.display().to_string();
                    }
                }
            });
            ui.add_space(10.0);
            ui.label(Text::PrefsWhisperModelPath.tr(locale));
            if let Some((downloaded, total)) = app.model_download_progress {
                ui.horizontal(|ui| {
                    if total > 0 {
                        ui.add(
                            egui::ProgressBar::new(downloaded as f32 / total as f32)
                                .desired_width(300.0)
                                .show_percentage(),
                        );
                    } else {
                        ui.add(
                            egui::ProgressBar::new(0.0)
                                .desired_width(300.0)
                                .text(format!("{:.1} MB", downloaded as f64 / 1_048_576.0)),
                        );
                    }
                    if ui.button(Text::CancelJob.tr(locale)).clicked() {
                        app.request_cancel_model_download();
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    for size in avcore::WhisperModelSize::ALL {
                        let label = format!("{} (~{} MB)", size_label(size), size.approx_size_mb());
                        if ui.button(label).clicked() {
                            app.spawn_download_whisper_model(size);
                        }
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.prefs.whisper_model_path)
                            .desired_width(400.0),
                    );
                    if ui.button(Text::Browse.tr(locale)).clicked() {
                        if let Some(file) = rfd::FileDialog::new()
                            .add_filter("GGML model", &["bin"])
                            .pick_file()
                        {
                            app.prefs.whisper_model_path = file.display().to_string();
                        }
                    }
                });
            }
            ui.add_space(10.0);
            ui.label(Text::PrefsSoundLibraryPath.tr(locale));
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.prefs.sound_library_path)
                        .desired_width(400.0),
                );
                if ui.button(Text::Browse.tr(locale)).clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        app.prefs.sound_library_path = folder.display().to_string();
                        app.rescan_sound_library();
                    }
                }
            });
        });
        ui.add_space(14.0);

        components::card_frame().show(ui, |ui| {
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

        components::card_frame().show(ui, |ui| {
            ui.label(
                RichText::new(Text::PrefsShortcuts.tr(locale))
                    .size(11.0)
                    .color(theme::TEXT_MUTED)
                    .strong(),
            );
            ui.add_space(6.0);
            shortcut_binding_editor(app, ui, locale);
        });
    });
}

/// Renders the configurable key binding table and handles key capture when the user clicks
/// "Change" on one of the four bindable actions.
fn shortcut_binding_editor(app: &mut App, ui: &mut egui::Ui, locale: Locale) {
    // When a binding is being captured, intercept the next non-modifier key press.
    // Escape cancels without changing the binding.
    if app.binding_capture.is_some() {
        let result = ui.input(|i| {
            for &key in egui::Key::ALL {
                if i.key_pressed(key) {
                    if key == egui::Key::Escape {
                        return Some(None);
                    }
                    return Some(Some(crate::app::KeyCombo {
                        ctrl: i.modifiers.ctrl,
                        shift: i.modifiers.shift,
                        key_name: key.name().to_string(),
                    }));
                }
            }
            None
        });
        match result {
            Some(Some(combo)) => {
                match app.binding_capture.unwrap() {
                    BindableAction::PlayPause => app.prefs.key_bindings.play_pause = combo,
                    BindableAction::SplitAtPlayhead => {
                        app.prefs.key_bindings.split_at_playhead = combo
                    }
                    BindableAction::CopyFormatting => {
                        app.prefs.key_bindings.copy_formatting = combo
                    }
                    BindableAction::PasteFormatting => {
                        app.prefs.key_bindings.paste_formatting = combo
                    }
                }
                app.binding_capture = None;
            }
            Some(None) => {
                app.binding_capture = None;
            }
            None => {}
        }
    }

    // Pre-compute display strings so we can borrow app freely inside the Grid closure.
    let rows: [(BindableAction, Text, String); 4] = [
        (
            BindableAction::PlayPause,
            Text::ShortcutPlayPause,
            app.prefs.key_bindings.play_pause.display(),
        ),
        (
            BindableAction::SplitAtPlayhead,
            Text::ShortcutSplit,
            app.prefs.key_bindings.split_at_playhead.display(),
        ),
        (
            BindableAction::CopyFormatting,
            Text::ShortcutCopyFormatting,
            app.prefs.key_bindings.copy_formatting.display(),
        ),
        (
            BindableAction::PasteFormatting,
            Text::ShortcutPasteFormatting,
            app.prefs.key_bindings.paste_formatting.display(),
        ),
    ];
    let binding_capture = app.binding_capture;

    let mut click_action: Option<BindableAction> = None;
    let mut cancel = false;

    egui::Grid::new("shortcuts_table")
        .num_columns(3)
        .spacing(egui::vec2(24.0, 6.0))
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
            ui.label("");
            ui.end_row();

            for (action, label_text, combo_str) in &rows {
                ui.label(label_text.tr(locale));
                if binding_capture == Some(*action) {
                    ui.label(
                        RichText::new(Text::BindingPressAnyKey.tr(locale))
                            .color(theme::ACCENT)
                            .monospace(),
                    );
                    if ui.small_button(Text::CancelJob.tr(locale)).clicked() {
                        cancel = true;
                    }
                } else {
                    ui.label(
                        RichText::new(combo_str.clone())
                            .color(theme::TEXT_MUTED)
                            .monospace(),
                    );
                    if ui.small_button(Text::BindingChange.tr(locale)).clicked() {
                        click_action = Some(*action);
                    }
                }
                ui.end_row();
            }
        });

    if cancel {
        app.binding_capture = None;
    } else if let Some(a) = click_action {
        app.binding_capture = Some(a);
    }
}

fn size_label(size: avcore::WhisperModelSize) -> &'static str {
    match size {
        avcore::WhisperModelSize::Tiny => "Tiny",
        avcore::WhisperModelSize::Base => "Base",
        avcore::WhisperModelSize::Small => "Small",
    }
}
