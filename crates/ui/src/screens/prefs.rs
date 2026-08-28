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

use crate::app::{App, BindableAction, LUFS_PROFILES};
use crate::components;
use crate::i18n::{Locale, Text};
use crate::theme;

/// Renders the Ajustes screen: language switcher, audio/export/project settings, and the
/// keyboard shortcut reference table. Settings write straight into `app.prefs`/`app.locale`,
/// persisted to disk via `App::save_prefs` — see [`crate::app::PrefsState`].
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(theme::SPACE_LG);
        components::page_title(ui, Text::PrefsTitle.tr(locale));
        ui.add_space(theme::SPACE_MD);
        ui.set_max_width(640.0);

        components::card_frame().show(ui, |ui| {
            prefs_section(ui, Text::PrefsLanguage.tr(locale), true, |ui| {
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
        });
        ui.add_space(14.0);

        components::card_frame().show(ui, |ui| {
            prefs_section(ui, Text::PrefsAudio.tr(locale), true, |ui| {
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
        });
        ui.add_space(14.0);

        components::card_frame().show(ui, |ui| {
            prefs_section(ui, Text::PrefsExport.tr(locale), false, |ui| {
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
                ui.horizontal_wrapped(|ui| {
                    use avcore::GpuEncoderPreference as Gpu;
                    for (choice, label) in [
                        (Gpu::Auto, Text::GpuEncoderAuto),
                        (Gpu::Cpu, Text::GpuEncoderCpu),
                        (Gpu::Nvenc, Text::GpuEncoderNvenc),
                        (Gpu::QuickSync, Text::GpuEncoderQuickSync),
                        (Gpu::Amf, Text::GpuEncoderAmf),
                        (Gpu::Vaapi, Text::GpuEncoderVaapi),
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
                ui.label(Text::PrefsPreviewQuality.tr(locale));
                ui.horizontal(|ui| {
                    use avcore::PreviewQuality as Quality;
                    for (choice, label) in [
                        (Quality::Low, Text::PreviewQualityLow),
                        (Quality::Medium, Text::PreviewQualityMedium),
                        (Quality::High, Text::PreviewQualityHigh),
                    ] {
                        if ui
                            .selectable_label(app.prefs.preview_quality == choice, label.tr(locale))
                            .clicked()
                        {
                            app.prefs.preview_quality = choice;
                        }
                    }
                });
                ui.add_space(10.0);
                let mut hardware_decode = app.prefs.preview_hardware_decode;
                if ui
                    .checkbox(&mut hardware_decode, Text::PrefsHardwareDecode.tr(locale))
                    .changed()
                {
                    app.set_preview_hardware_decode(hardware_decode);
                }
                ui.label(
                    RichText::new(Text::PrefsHardwareDecodeHint.tr(locale))
                        .size(11.0)
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(10.0);
                ui.label(Text::PrefsOutputFolder.tr(locale));
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.prefs.output_folder)
                            .desired_width(400.0),
                    );
                    if ui.button(Text::Browse.tr(locale)).clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            app.prefs.output_folder = folder.display().to_string();
                        }
                    }
                });
                ui.add_space(10.0);
                ui.label(Text::PrefsWhisperModelPath.tr(locale));
                model_path_row(ui, &mut app.prefs.whisper_model_path, &["bin"], locale);
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
                ui.add_space(10.0);
                ui.label(Text::PrefsReframeModelPath.tr(locale));
                model_path_row(ui, &mut app.prefs.reframe_model_path, &["onnx"], locale);
                ui.add_space(10.0);
                ui.label(Text::PrefsBackgroundRemovalModelPath.tr(locale));
                model_path_row(
                    ui,
                    &mut app.prefs.background_removal_model_path,
                    &["onnx"],
                    locale,
                );
                ui.add_space(10.0);
                ui.label(Text::PrefsTtsModelPath.tr(locale));
                model_path_row(ui, &mut app.prefs.tts_model_path, &["onnx"], locale);
            });
        });
        ui.add_space(14.0);

        components::card_frame().show(ui, |ui| {
            prefs_section(ui, Text::PrefsProject.tr(locale), true, |ui| {
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
                ui.add_space(10.0);
                if ui
                    .checkbox(
                        &mut app.prefs.telemetry_enabled,
                        Text::PrefsTelemetryEnabled.tr(locale),
                    )
                    .changed()
                {
                    app.telemetry_state.telemetry_enabled_flag.store(
                        app.prefs.telemetry_enabled,
                        std::sync::atomic::Ordering::Relaxed,
                    );
                }
                ui.add_space(10.0);
                ui.label(Text::PrefsLayoutScope.tr(locale));
                ui.horizontal(|ui| {
                    use crate::app::LayoutScope;
                    for (choice, label) in [
                        (LayoutScope::PerUser, Text::LayoutScopePerUser),
                        (LayoutScope::PerProject, Text::LayoutScopePerProject),
                    ] {
                        if ui
                            .selectable_label(app.prefs.layout_scope == choice, label.tr(locale))
                            .clicked()
                        {
                            app.prefs.layout_scope = choice;
                            if choice == LayoutScope::PerProject {
                                app.load_panel_layout_for_active_project();
                            }
                        }
                    }
                });
            });
        });
        ui.add_space(14.0);

        components::card_frame().show(ui, |ui| {
            prefs_section(ui, Text::PrefsShortcuts.tr(locale), false, |ui| {
                shortcut_binding_editor(app, ui, locale);
            });
        });
        ui.add_space(14.0);

        components::card_frame().show(ui, |ui| {
            prefs_section(ui, Text::AppName.tr(locale), false, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "{} {}",
                        Text::AppName.tr(locale),
                        env!("CARGO_PKG_VERSION")
                    ));
                    if ui.button(Text::AboutOpen.tr(locale)).clicked() {
                        app.open_about();
                    }
                });
            });
        });
    });
}

/// Renders the configurable key binding table and handles key capture when the user clicks
/// "Change" on one of the five bindable actions.
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
                    BindableAction::AddOpacityMarker => {
                        app.prefs.key_bindings.add_opacity_marker = combo
                    }
                    BindableAction::Undo => app.prefs.key_bindings.undo = combo,
                    BindableAction::Redo => app.prefs.key_bindings.redo = combo,
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
    let rows: [(BindableAction, Text, String); 7] = [
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
        (
            BindableAction::AddOpacityMarker,
            Text::ShortcutAddOpacityMarker,
            app.prefs.key_bindings.add_opacity_marker.display(),
        ),
        (
            BindableAction::Undo,
            Text::ShortcutUndo,
            app.prefs.key_bindings.undo.display(),
        ),
        (
            BindableAction::Redo,
            Text::ShortcutRedo,
            app.prefs.key_bindings.redo.display(),
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

/// A `CollapsingHeader` styled to match the section headers used everywhere else
/// (`components::section_label`'s uppercase/muted/strong look), so Prefs stops always showing
/// every one of its six sections fully expanded at once — the confirmed progressive-disclosure
/// gap from `UI_DESIGN_AUDIT.md`. `default_open` is chosen per section by how frequently it's
/// actually touched after initial setup, not uniformly.
fn prefs_section(
    ui: &mut egui::Ui,
    title: &str,
    default_open: bool,
    content: impl FnOnce(&mut egui::Ui),
) {
    egui::CollapsingHeader::new(
        RichText::new(title.to_uppercase())
            .size(11.0)
            .color(theme::TEXT_MUTED)
            .strong(),
    )
    .default_open(default_open)
    .show(ui, content);
}

fn model_path_row(ui: &mut egui::Ui, path: &mut String, extensions: &[&str], locale: Locale) {
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(path).desired_width(400.0));
        if ui.button(Text::Browse.tr(locale)).clicked() {
            if let Some(file) = rfd::FileDialog::new()
                .add_filter(Text::ModelFile.tr(locale), extensions)
                .pick_file()
            {
                *path = file.display().to_string();
            }
        }
    });
    let available = std::path::Path::new(path).is_file();
    ui.label(
        RichText::new(if available {
            Text::BundledResourceAvailable.tr(locale)
        } else {
            Text::BundledResourceMissing.tr(locale)
        })
        .size(11.0)
        .color(if available {
            theme::ACCENT
        } else {
            theme::ERROR
        }),
    );
}
