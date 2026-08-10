use avcore::media::format_timecode;
use eframe::egui::{self, RichText};

use crate::app::{EditorTool, OcaApp};
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
pub fn show(app: &mut OcaApp, ui: &mut egui::Ui) {
    let ctrl_s_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S));
    if ctrl_s_pressed {
        save_active_project(app);
    }
    let ctrl_b_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::B));
    if ctrl_b_pressed {
        app.split_at_playhead();
    }
    let delete_pressed = ui.input(|i| i.key_pressed(egui::Key::Delete));
    if delete_pressed {
        app.delete_selected_clip();
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

fn toolbar(app: &mut OcaApp, ui: &mut egui::Ui) {
    let locale = app.locale;
    ui.horizontal(|ui| {
        tool_button(
            app,
            ui,
            EditorTool::Select,
            "↖",
            Text::ToolSelect.tr(locale),
        );
        if ui
            .button(format!("✂ {}", Text::ToolCut.tr(locale)))
            .on_hover_text("Ctrl+B")
            .clicked()
        {
            app.split_at_playhead();
        }
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

/// Saves the active project to its remembered [`avcore::Project::file_path`], or prompts
/// for a destination (and remembers it for next time) if it doesn't have one yet.
fn save_active_project(app: &mut OcaApp) {
    let path = match app.active_project().file_path.clone() {
        Some(path) => Some(path),
        None => rfd::FileDialog::new()
            .add_filter("JSON", &["json"])
            .set_file_name(format!("{}.json", app.active_project().name))
            .save_file(),
    };
    let Some(path) = path else { return };

    match avcore::save_project_to_file(app.active_project(), &path) {
        Ok(()) => app.active_project_mut().file_path = Some(path),
        Err(e) => eprintln!("failed to save project: {e}"),
    }
}

fn tool_button(app: &mut OcaApp, ui: &mut egui::Ui, tool: EditorTool, icon: &str, label: &str) {
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

fn media_library_panel(app: &mut OcaApp, ui: &mut egui::Ui, width: f32, height: f32) {
    let mut clicked_id = None;

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
                        let response = egui::Frame::new()
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
                            })
                            .response
                            .interact(egui::Sense::click());
                        if response.clicked() {
                            clicked_id = Some(asset.id);
                        }
                        ui.add_space(6.0);
                    }
                });
            });
        });

    if let Some(id) = clicked_id {
        app.select_asset(Some(id));
    }
}

fn preview_panel(app: &mut OcaApp, ui: &mut egui::Ui, height: f32) {
    let locale = app.locale;
    ui.vertical(|ui| {
        ui.set_height(height);
        egui::Frame::new()
            .fill(egui::Color32::BLACK)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.set_min_height(height - 40.0);
                ui.centered_and_justified(|ui| match &app.preview_texture {
                    Some(texture) => {
                        let tex_size = texture.size_vec2();
                        let aspect = tex_size.x / tex_size.y;
                        let mut size = ui.available_size();
                        if size.x / size.y > aspect {
                            size.x = size.y * aspect;
                        } else {
                            size.y = size.x / aspect;
                        }
                        ui.add(egui::Image::new(texture).fit_to_exact_size(size));
                    }
                    None if app.selected_asset_id.is_some() && !app.preview_available() => {
                        ui.label(
                            RichText::new(Text::PreviewUnavailable.tr(locale))
                                .size(13.0)
                                .color(theme::TEXT_MUTED),
                        );
                    }
                    None => {
                        ui.label(RichText::new("▶").size(48.0).color(theme::TEXT_MUTED));
                    }
                });
            });
        ui.horizontal(|ui| {
            if ui.small_button("⏮").clicked() {
                app.seek_preview(0.0);
            }
            let play_icon = if app.preview_playing { "⏸" } else { "▶" };
            if ui
                .button(RichText::new(play_icon).color(theme::ACCENT))
                .clicked()
            {
                app.toggle_preview_playback();
            }
            if ui.small_button("⏭").clicked() {
                if let Some(duration) = app.preview_duration_secs() {
                    app.seek_preview(duration);
                }
            }
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
        if let Some(duration) = app.preview_duration_secs().filter(|d| *d > 0.0) {
            let mut position = app.preview_position_secs().unwrap_or(0.0);
            let slider = ui.add(egui::Slider::new(&mut position, 0.0..=duration).show_value(false));
            if slider.changed() {
                app.seek_preview(position);
            }
        }
    });
}

fn properties_panel(app: &OcaApp, ui: &mut egui::Ui, width: f32, height: f32) {
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

/// Horizontal scale of the timeline strip and its ruler — seconds to pixels. No zoom control
/// yet (Fase 3), so this is a fixed constant rather than per-project state.
const PX_PER_SEC: f32 = 4.0;
const TRACK_LABEL_WIDTH: f32 = 50.0;

fn timeline_panel(app: &mut OcaApp, ui: &mut egui::Ui, height: f32) {
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

        // Ruler: click or drag to move the playhead. Kept as its own thin strip rather than
        // reusing a track row so scrubbing doesn't depend on there being any tracks yet.
        ui.horizontal(|ui| {
            ui.add_space(TRACK_LABEL_WIDTH);
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 14.0),
                egui::Sense::click_and_drag(),
            );
            ui.painter().rect_filled(rect, 0, theme::SURFACE_2);
            if let Some(pos) = response.interact_pointer_pos() {
                let secs = ((pos.x - rect.left()) / PX_PER_SEC).max(0.0) as f64;
                app.active_project_mut().timeline.playhead_secs = secs;
            }
            draw_playhead(ui, rect, app.active_project().timeline.playhead_secs, 2.0);
        });

        let mut clicked_clip_id = None;
        let mut trim_requests: Vec<(u64, TrimEdge)> = Vec::new();
        let mut move_requests: Vec<(u64, f64)> = Vec::new();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for track in &app.active_project().timeline.tracks {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [TRACK_LABEL_WIDTH, 28.0],
                        egui::Label::new(
                            RichText::new(&track.name)
                                .size(11.0)
                                .color(theme::TEXT_SECONDARY),
                        ),
                    );
                    let (track_rect, _resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 26.0),
                        egui::Sense::hover(),
                    );
                    let painter = ui.painter();
                    for clip in &track.clips {
                        let x = track_rect.left() + clip.start_secs as f32 * PX_PER_SEC;
                        let w = (clip.duration_secs() as f32 * PX_PER_SEC).max(3.0);
                        let clip_rect = egui::Rect::from_min_size(
                            egui::pos2(x, track_rect.top()),
                            egui::vec2(w, track_rect.height()),
                        );
                        let color = match (track.kind, track.name.as_str()) {
                            (avcore::timeline::TrackKind::Video, _) => theme::SURFACE_2,
                            (avcore::timeline::TrackKind::Audio, "A2") => {
                                theme::ACCENT_2.gamma_multiply(0.6)
                            }
                            (avcore::timeline::TrackKind::Audio, _) => {
                                theme::ACCENT.gamma_multiply(0.5)
                            }
                        };

                        // Narrow strips at each edge, on top of the body's click zone, so a
                        // drag started right at the edge trims instead of just selecting.
                        let edge_w = (w / 3.0).clamp(2.0, 6.0);
                        let left_edge_rect = egui::Rect::from_min_size(
                            clip_rect.min,
                            egui::vec2(edge_w, clip_rect.height()),
                        );
                        let right_edge_rect = egui::Rect::from_min_size(
                            egui::pos2(clip_rect.right() - edge_w, clip_rect.top()),
                            egui::vec2(edge_w, clip_rect.height()),
                        );

                        let body_response = ui.interact(
                            clip_rect,
                            ui.id().with(("timeline_clip", clip.id)),
                            egui::Sense::click_and_drag(),
                        );
                        let left_response = ui.interact(
                            left_edge_rect,
                            ui.id().with(("timeline_clip_trim_start", clip.id)),
                            egui::Sense::drag(),
                        );
                        let right_response = ui.interact(
                            right_edge_rect,
                            ui.id().with(("timeline_clip_trim_end", clip.id)),
                            egui::Sense::drag(),
                        );
                        if left_response.hovered()
                            || left_response.dragged()
                            || right_response.hovered()
                            || right_response.dragged()
                        {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }
                        if body_response.clicked() {
                            clicked_clip_id = Some(clip.id);
                        }
                        if body_response.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                            let delta_secs = (body_response.drag_delta().x / PX_PER_SEC) as f64;
                            if delta_secs != 0.0 {
                                move_requests.push((clip.id, clip.start_secs + delta_secs));
                            }
                        }
                        if let Some(pos) = left_response.interact_pointer_pos() {
                            let secs = ((pos.x - track_rect.left()) / PX_PER_SEC).max(0.0) as f64;
                            trim_requests.push((clip.id, TrimEdge::Start(secs)));
                        }
                        if let Some(pos) = right_response.interact_pointer_pos() {
                            let secs = ((pos.x - track_rect.left()) / PX_PER_SEC).max(0.0) as f64;
                            trim_requests.push((clip.id, TrimEdge::End(secs)));
                        }

                        painter.rect_filled(clip_rect, egui::CornerRadius::same(4), color);
                        if app.selected_clip_id == Some(clip.id) {
                            painter.rect_stroke(
                                clip_rect,
                                egui::CornerRadius::same(4),
                                egui::Stroke::new(2.0, theme::ACCENT),
                                egui::StrokeKind::Inside,
                            );
                        }
                    }
                    draw_playhead(
                        ui,
                        track_rect,
                        app.active_project().timeline.playhead_secs,
                        1.0,
                    );
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
        if let Some(id) = clicked_clip_id {
            app.selected_clip_id = Some(id);
        }
        for (clip_id, edge) in trim_requests {
            match edge {
                TrimEdge::Start(secs) => app.trim_clip_start(clip_id, secs),
                TrimEdge::End(secs) => app.trim_clip_end(clip_id, secs),
            }
        }
        for (clip_id, new_start_secs) in move_requests {
            app.move_clip(clip_id, new_start_secs);
        }
    });
}

/// Which edge of a timeline clip a drag targets — see the trim handling in `timeline_panel`.
enum TrimEdge {
    Start(f64),
    End(f64),
}

/// Draws the playhead as a vertical line inside `rect`, if it falls within `rect`'s horizontal
/// span — used both on the ruler strip and on every track row so it reads as one continuous
/// line down the timeline despite each row being drawn separately.
fn draw_playhead(ui: &egui::Ui, rect: egui::Rect, playhead_secs: f64, stroke_width: f32) {
    let x = rect.left() + playhead_secs as f32 * PX_PER_SEC;
    if x >= rect.left() && x <= rect.right() {
        ui.painter().vline(
            x,
            rect.y_range(),
            egui::Stroke::new(stroke_width, theme::ACCENT),
        );
    }
}
