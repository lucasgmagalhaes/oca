use eframe::egui::{self, RichText};
use nivela_core::media::format_timecode;

use crate::app::{EditorTool, NivelaApp};
use crate::i18n::Text;
use crate::screens::widgets;
use crate::theme;

/// Renders the Editor screen: toolbar, then a three-column row (media library / preview /
/// clip properties), then the timeline strip.
///
/// Every column and the timeline are wrapped in `ui.vertical(...)` (and the columns also in
/// `ui.allocate_ui(...)`) rather than just calling `ui.set_width()`/`ui.set_height()` inside
/// their `Frame` — this egui version doesn't default a `Frame`'s or `ScrollArea`'s child `Ui`
/// to a vertical top-down layout, it inherits whatever direction the enclosing `Ui` is in
/// (horizontal, here), and `set_width` alone only affects how much space is reported back to
/// the parent afterwards, not what the child actually paints. Skipping either wrapper
/// reintroduces the overlapping-text/full-width-panel bugs fixed in this screen — see the
/// "Add i18n" commit for the concrete symptoms.
pub fn show(app: &mut NivelaApp, ui: &mut egui::Ui) {
    let ctrl_s_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S));
    if ctrl_s_pressed {
        save_active_project(app);
    }

    ui.vertical(|ui| {
        toolbar(app, ui);
        ui.add_space(4.0);

        let timeline_height = 190.0;
        let body_height = (ui.available_height() - timeline_height - 16.0).max(160.0);
        let total_width = ui.available_width();
        let lib_w = 220.0_f32.min(total_width * 0.28);
        let right_w = 240.0_f32.min(total_width * 0.3);
        let gaps = ui.spacing().item_spacing.x * 2.0 + 2.0;
        let preview_w = (total_width - lib_w - right_w - gaps).max(200.0);

        ui.horizontal(|ui| {
            ui.set_height(body_height);

            // `allocate_ui` reserves the exact rect up front, so children (ScrollArea, Frame)
            // see a properly bounded `max_rect` instead of the horizontal layout's full
            // remaining width — a bare `ui.set_width()` inside the panel only affects how much
            // space is reported *back* to this layout afterwards, not what the panel can paint.
            ui.allocate_ui(egui::vec2(lib_w, body_height), |ui| {
                media_library_panel(app, ui, lib_w, body_height);
            });
            ui.separator();
            ui.allocate_ui(egui::vec2(preview_w, body_height), |ui| {
                preview_panel(app, ui, body_height);
            });
            ui.separator();
            ui.allocate_ui(egui::vec2(right_w, body_height), |ui| {
                properties_panel(app, ui, right_w, body_height);
            });
        });

        ui.add_space(8.0);
        timeline_panel(app, ui, timeline_height);
    });
}

fn toolbar(app: &mut NivelaApp, ui: &mut egui::Ui) {
    let locale = app.locale;
    ui.horizontal(|ui| {
        tool_button(
            app,
            ui,
            EditorTool::Select,
            "↖",
            Text::ToolSelect.tr(locale),
        );
        tool_button(app, ui, EditorTool::Cut, "✂", Text::ToolCut.tr(locale));
        tool_button(app, ui, EditorTool::Trim, "⇔", Text::ToolTrim.tr(locale));
        ui.separator();
        let _ = ui.button("↺");
        let _ = ui.button("↻");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(Text::Export.tr(locale)).clicked() {
                app.screen = crate::app::Screen::Queue;
            }
            if ui
                .button(Text::Save.tr(locale))
                .on_hover_text("Ctrl+S")
                .clicked()
            {
                save_active_project(app);
            }
        });
    });
}

/// Saves the active project to its remembered [`nivela_core::Project::file_path`], or prompts
/// for a destination (and remembers it for next time) if it doesn't have one yet.
fn save_active_project(app: &mut NivelaApp) {
    let path = match app.active_project().file_path.clone() {
        Some(path) => Some(path),
        None => rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name(format!("{}.json", app.active_project().name))
            .save_file(),
    };
    let Some(path) = path else { return };

    match nivela_core::save_project_to_file(app.active_project(), &path) {
        Ok(()) => app.active_project_mut().file_path = Some(path),
        Err(e) => eprintln!("failed to save project: {e}"),
    }
}

fn tool_button(app: &mut NivelaApp, ui: &mut egui::Ui, tool: EditorTool, icon: &str, label: &str) {
    let active = app.tool == tool;
    let text = RichText::new(format!("{icon} {label}")).color(if active {
        theme::ACCENT
    } else {
        theme::TEXT_SECONDARY
    });
    if ui.button(text).clicked() {
        app.tool = tool;
    }
}

fn media_library_panel(app: &NivelaApp, ui: &mut egui::Ui, width: f32, height: f32) {
    egui::Frame::new()
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(height);
            ui.vertical(|ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    widgets::section_label(ui, Text::MediaLibrary.tr(app.locale));
                    for asset in &app.active_project().media_library {
                        let selected = app.selected_asset_id == Some(asset.id);
                        let bg = if selected {
                            theme::ACCENT.gamma_multiply(0.18)
                        } else {
                            theme::SURFACE
                        };
                        egui::Frame::new()
                            .fill(bg)
                            .corner_radius(6)
                            .inner_margin(egui::Margin::same(6))
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(&asset.file_name).size(12.0));
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {}",
                                            asset.duration_label(),
                                            asset
                                                .resolution
                                                .map(|(w, h)| format!("{w}×{h}"))
                                                .unwrap_or_else(|| asset
                                                    .sample_rate_khz
                                                    .map(|k| format!("{k:.0}kHz"))
                                                    .unwrap_or_default())
                                        ))
                                        .size(10.0)
                                        .color(theme::TEXT_MUTED),
                                    );
                                });
                            });
                        ui.add_space(6.0);
                    }
                });
            });
        });
}

fn preview_panel(app: &NivelaApp, ui: &mut egui::Ui, height: f32) {
    ui.vertical(|ui| {
        ui.set_height(height);
        egui::Frame::new()
            .fill(egui::Color32::BLACK)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.set_min_height(height - 40.0);
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("▶").size(48.0).color(theme::TEXT_MUTED));
                });
            });
        ui.horizontal(|ui| {
            let _ = ui.small_button("⏮");
            ui.label(RichText::new("▶").color(theme::ACCENT));
            let _ = ui.small_button("⏭");
            let timeline = &app.active_project().timeline;
            ui.label(
                RichText::new(format!(
                    "{} / {}",
                    format_timecode(timeline.playhead_secs),
                    format_timecode(timeline.duration_secs().max(timeline.playhead_secs))
                ))
                .size(12.0)
                .color(theme::TEXT_SECONDARY)
                .monospace(),
            );
        });
    });
}

fn properties_panel(app: &NivelaApp, ui: &mut egui::Ui, width: f32, height: f32) {
    let locale = app.locale;
    egui::Frame::new()
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(height);
            ui.vertical(|ui| {
                widgets::section_label(ui, Text::SelectedClip.tr(locale));
                let Some(asset) = app.selected_asset() else {
                    ui.label(
                        RichText::new(Text::NoClipSelected.tr(locale)).color(theme::TEXT_MUTED),
                    );
                    return;
                };
                ui.label(RichText::new(&asset.file_name).size(13.0));
                ui.add_space(8.0);

                prop_row(ui, Text::PropCodec.tr(locale), &asset.codec);
                prop_row(
                    ui,
                    Text::PropSourceBitrate.tr(locale),
                    &format!("{:.0} Mbps", asset.source_bitrate_mbps),
                );
                if let Some((w, h)) = asset.resolution {
                    prop_row(ui, Text::PropResolution.tr(locale), &format!("{w}×{h}"));
                }
                if let Some(fps) = asset.fps {
                    prop_row(ui, Text::PropFps.tr(locale), &format!("{fps:.0}"));
                }
                if let Some(l) = &asset.loudness {
                    prop_row(
                        ui,
                        Text::PropLoudness.tr(locale),
                        &format!("{:.1} LUFS", l.integrated_lufs),
                    );
                }
                prop_row(
                    ui,
                    Text::PropProxy.tr(locale),
                    if asset.proxy_path.is_some() {
                        Text::ProxyPresent.tr(locale)
                    } else {
                        Text::ProxyAbsent.tr(locale)
                    },
                );

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);
                widgets::section_label(ui, Text::OnExport.tr(locale));
                ui.horizontal_wrapped(|ui| {
                    let (_, target) = crate::app::LUFS_PROFILES[app.prefs.lufs_profile];
                    widgets::tag_accent(
                        ui,
                        &format!("{} → {target:.0} LUFS", Text::NormalizeTo.tr(locale)),
                    );
                    widgets::tag_outline(
                        ui,
                        &format!(
                            "{} ({:.0} Mbps)",
                            Text::BitrateFromSource.tr(locale),
                            asset.source_bitrate_mbps
                        ),
                    );
                });
                ui.add_space(6.0);
                ui.label(
                    RichText::new(Text::ExportAutoNote.tr(locale))
                        .size(10.5)
                        .color(theme::TEXT_MUTED),
                );
            });
        });
}

fn prop_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.0).color(theme::TEXT_MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(12.0));
        });
    });
}

fn timeline_panel(app: &NivelaApp, ui: &mut egui::Ui, height: f32) {
    widgets::card_frame().show(ui, |ui| {
        ui.set_height(height - 20.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(Text::Timeline.tr(app.locale))
                    .size(11.0)
                    .color(theme::TEXT_MUTED),
            );
        });
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for track in &app.active_project().timeline.tracks {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [50.0, 28.0],
                        egui::Label::new(
                            RichText::new(&track.name)
                                .size(11.0)
                                .color(theme::TEXT_SECONDARY),
                        ),
                    );
                    for clip in &track.clips {
                        let w = (clip.duration_secs() as f32 * 4.0).max(24.0);
                        let color = match (track.kind, track.name.as_str()) {
                            (nivela_core::timeline::TrackKind::Video, _) => theme::SURFACE_2,
                            (nivela_core::timeline::TrackKind::Audio, "A2") => {
                                theme::ACCENT_2.gamma_multiply(0.6)
                            }
                            (nivela_core::timeline::TrackKind::Audio, _) => {
                                theme::ACCENT.gamma_multiply(0.5)
                            }
                        };
                        let (rect, _resp) =
                            ui.allocate_exact_size(egui::vec2(w, 26.0), egui::Sense::click());
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(4), color);
                    }
                });
                ui.add_space(4.0);
            }
            if app.active_project().timeline.tracks.is_empty() {
                ui.label(
                    RichText::new(Text::TimelineEmpty.tr(app.locale))
                        .size(12.0)
                        .color(theme::TEXT_MUTED),
                );
            }
        });
    });
}
