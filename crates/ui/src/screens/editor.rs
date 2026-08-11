use std::collections::HashMap;

use avcore::media::format_timecode;
use avcore::MediaAsset;
use eframe::egui::{self, RichText};

use crate::app::{
    EditorTool, OcaApp, BRIGHTNESS_RANGE, CONTRAST_RANGE, CROP_MIN_SIZE, GAIN_DB_RANGE,
    MASK_CORNER_RADIUS_RANGE, SATURATION_RANGE, SPEED_FACTOR_RANGE, THUMBNAIL_BUCKET_SECS,
    VIGNETTE_INTENSITY_RANGE,
};
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
    let ctrl_c_pressed =
        ui.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::C));
    if ctrl_c_pressed {
        app.copy_selected_clip();
    }
    let ctrl_x_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::X));
    if ctrl_x_pressed {
        app.cut_selected_clip();
    }
    let ctrl_v_pressed =
        ui.input(|i| i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::V));
    if ctrl_v_pressed {
        app.paste_clip_at_playhead();
    }
    // Ctrl+Shift+C/V — "copiar formatação" (request.md's Fase 4 spec): copies just the
    // gain/freeze settings from the selected clip onto another, without duplicating the clip
    // itself. Distinct from plain Ctrl+C/V (whole-clip copy/paste) above.
    let ctrl_shift_c_pressed =
        ui.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::C));
    if ctrl_shift_c_pressed {
        app.copy_selected_clip_formatting();
    }
    let ctrl_shift_v_pressed =
        ui.input(|i| i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::V));
    if ctrl_shift_v_pressed {
        app.paste_selected_clip_formatting();
    }

    ui.vertical(|ui| {
        toolbar(app, ui);
        ui.add_space(4.0);
        sequence_tab_bar(app, ui);
        ui.add_space(4.0);

        let total_width = ui.available_width();
        let min_col = 160.0_f32;
        let max_col = (total_width * 0.4).max(min_col);
        app.lib_panel_width = app.lib_panel_width.clamp(min_col, max_col);
        app.props_panel_width = app.props_panel_width.clamp(min_col, max_col);

        let available_height = ui.available_height();
        let min_timeline = 120.0_f32;
        let max_timeline = (available_height - 200.0).max(min_timeline);
        app.timeline_height = app.timeline_height.clamp(min_timeline, max_timeline);
        let body_height = (available_height - app.timeline_height - 24.0).max(160.0);

        let gaps = ui.spacing().item_spacing.x * 2.0 + DIVIDER_HIT_WIDTH * 2.0;
        let preview_w =
            (total_width - app.lib_panel_width - app.props_panel_width - gaps).max(200.0);

        ui.horizontal(|ui| {
            ui.set_height(body_height);

            // `allocate_ui` reserves the exact rect up front, so children (ScrollArea, Frame)
            // see a properly bounded `max_rect` instead of the horizontal layout's full
            // remaining width — a bare `ui.set_width()` inside the panel only affects how much
            // space is reported *back* to this layout afterwards, not what the panel can paint.
            ui.allocate_ui(egui::vec2(app.lib_panel_width, body_height), |ui| {
                media_library_panel(app, ui, app.lib_panel_width, body_height);
            });
            resizable_divider(
                ui,
                body_height,
                &mut app.lib_panel_width,
                min_col,
                max_col,
                1.0,
            );
            ui.allocate_ui(egui::vec2(preview_w, body_height), |ui| {
                preview_panel(app, ui, body_height);
            });
            resizable_divider(
                ui,
                body_height,
                &mut app.props_panel_width,
                min_col,
                max_col,
                -1.0,
            );
            ui.allocate_ui(egui::vec2(app.props_panel_width, body_height), |ui| {
                properties_panel(app, ui, app.props_panel_width, body_height);
            });
        });

        ui.add_space(4.0);
        resizable_divider_horizontal(
            ui,
            total_width,
            &mut app.timeline_height,
            min_timeline,
            max_timeline,
        );
        ui.add_space(4.0);
        timeline_panel(app, ui, app.timeline_height);
    });
}

/// Hit-testable width of a [`resizable_divider`]/[`resizable_divider_horizontal`] handle — wider
/// than the 1px line it draws, since a bare 1px strip is unreliable to grab with a mouse.
const DIVIDER_HIT_WIDTH: f32 = 6.0;

/// A draggable divider between two side-by-side panels (per `request.md`'s Fase 3 "painéis de
/// UI redimensionáveis" spec). Dragging it left/right adjusts `*width` by the pointer's
/// horizontal movement — `sign` is `1.0` when `*width` belongs to the panel on the divider's
/// left (dragging right grows it) or `-1.0` when it belongs to the panel on the right (dragging
/// right shrinks it) — clamped to `[min_width, max_width]`.
fn resizable_divider(
    ui: &mut egui::Ui,
    height: f32,
    width: &mut f32,
    min_width: f32,
    max_width: f32,
    sign: f32,
) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(DIVIDER_HIT_WIDTH, height), egui::Sense::drag());
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }
    if response.dragged() {
        *width = (*width + sign * response.drag_delta().x).clamp(min_width, max_width);
    }
    let line_x = rect.center().x;
    ui.painter().line_segment(
        [
            egui::pos2(line_x, rect.top()),
            egui::pos2(line_x, rect.bottom()),
        ],
        egui::Stroke::new(1.0, theme::BORDER),
    );
}

/// Same idea as [`resizable_divider`] but for the horizontal boundary above the timeline strip:
/// dragging it up grows `*height` (the timeline), dragging it down shrinks it, clamped to
/// `[min_height, max_height]`.
fn resizable_divider_horizontal(
    ui: &mut egui::Ui,
    width: f32,
    height: &mut f32,
    min_height: f32,
    max_height: f32,
) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, DIVIDER_HIT_WIDTH), egui::Sense::drag());
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    if response.dragged() {
        *height = (*height - response.drag_delta().y).clamp(min_height, max_height);
    }
    let line_y = rect.center().y;
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), line_y),
            egui::pos2(rect.right(), line_y),
        ],
        egui::Stroke::new(1.0, theme::BORDER),
    );
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
        if ui
            .add_enabled(
                app.multi_selected_clip_ids.len() >= 2,
                egui::Button::new(Text::MergeIntoComposite.tr(locale)),
            )
            .on_hover_text(Text::MergeIntoCompositeHint.tr(locale))
            .clicked()
        {
            app.merge_into_composite();
        }
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

/// Row of tabs, one per sequence in the active project (per `request.md`'s Fase 3 "abas de
/// projeto" spec) — click a tab to switch which sequence's timeline the rest of the Editor
/// screen shows, or the trailing "+" to append a new empty one and switch to it.
fn sequence_tab_bar(app: &mut OcaApp, ui: &mut egui::Ui) {
    let locale = app.locale;
    let active_index = app.active_project().active_sequence;
    let mut select_index = None;
    let mut add_requested = false;

    ui.horizontal(|ui| {
        for (index, sequence) in app.active_project().sequences.iter().enumerate() {
            let active = index == active_index;
            let text = RichText::new(&sequence.name).color(if active {
                theme::ACCENT
            } else {
                theme::TEXT_SECONDARY
            });
            let button = egui::Button::new(text).fill(if active {
                theme::SURFACE_2
            } else {
                theme::SURFACE
            });
            if ui.add(button).clicked() && !active {
                select_index = Some(index);
            }
        }
        if ui.button(Text::AddSequenceTab.tr(locale)).clicked() {
            add_requested = true;
        }
    });

    if let Some(index) = select_index {
        app.select_sequence(index);
    }
    if add_requested {
        app.add_sequence();
    }
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
    let mut add_to_timeline_id = None;
    let mut dropped_asset = None;

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
                            .interact(egui::Sense::click_and_drag());
                        if response.clicked() {
                            clicked_id = Some(asset.id);
                        }
                        if response.double_clicked() {
                            add_to_timeline_id = Some(asset.id);
                        }
                        if response.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                            if let Some(pos) = response.interact_pointer_pos() {
                                egui::Area::new(ui.id().with(("asset_drag_ghost", asset.id)))
                                    .fixed_pos(pos + egui::vec2(12.0, 12.0))
                                    .order(egui::Order::Tooltip)
                                    .interactable(false)
                                    .show(ui.ctx(), |ui| {
                                        egui::Frame::new()
                                            .fill(theme::SURFACE_2)
                                            .corner_radius(4)
                                            .inner_margin(egui::Margin::symmetric(8, 4))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(&asset.file_name).size(11.0),
                                                );
                                            });
                                    });
                            }
                        }
                        if response.drag_stopped() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                dropped_asset = Some((asset.id, pos));
                            }
                        }
                        ui.add_space(6.0);
                    }
                });
            });
        });

    if let Some(id) = clicked_id {
        app.select_asset(Some(id));
    }
    if let Some(dropped) = dropped_asset {
        app.pending_asset_drop = Some(dropped);
    }
    if let Some(id) = add_to_timeline_id {
        app.add_asset_to_timeline(id);
    }
}

fn preview_panel(app: &mut OcaApp, ui: &mut egui::Ui, height: f32) {
    // Lazy: the pipeline for the current selection is opened here, on the first paint of this
    // panel after a selection change — not by `select_asset` itself — so opening a project or
    // launching the app never pays GStreamer's open cost for an asset the Editor screen hasn't
    // actually been shown for yet.
    app.ensure_preview_loaded();
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
            let timeline = app.active_project().timeline();
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

fn properties_panel(app: &mut OcaApp, ui: &mut egui::Ui, width: f32, height: f32) {
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

                if let Some(clip) = app.selected_clip() {
                    let mut gain_db = clip.gain_db;
                    let mut frozen = clip.frozen;
                    let mut speed_factor = clip.speed_factor;
                    let (mut crop_x, mut crop_y, mut crop_w, mut crop_h) =
                        (clip.crop_x, clip.crop_y, clip.crop_w, clip.crop_h);
                    let mut mask_shape = clip.mask_shape;
                    let mut mask_corner_radius = clip.mask_corner_radius;
                    let mut flipped_h = clip.flipped_h;
                    let mut color_filter = clip.color_filter;
                    let mut vignette_intensity = clip.vignette_intensity;
                    let (mut brightness, mut contrast, mut saturation) =
                        (clip.brightness, clip.contrast, clip.saturation);
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);
                    widgets::section_label(ui, Text::PropGain.tr(locale));
                    let slider = ui.add(
                        egui::Slider::new(&mut gain_db, GAIN_DB_RANGE)
                            .suffix(" dB")
                            .fixed_decimals(1),
                    );
                    if slider.changed() {
                        app.set_selected_clip_gain(gain_db);
                    }
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(Text::GainExportNote.tr(locale))
                            .size(10.5)
                            .color(theme::TEXT_MUTED),
                    );

                    // "Congelar" only makes sense for a video block — audio clips have no
                    // frame to hold.
                    if app.selected_clip_track_kind() == Some(avcore::timeline::TrackKind::Video) {
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);
                        let checkbox = ui.checkbox(&mut frozen, Text::PropFreeze.tr(locale));
                        if checkbox.changed() {
                            app.set_selected_clip_frozen(frozen);
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(Text::FreezeExportNote.tr(locale))
                                .size(10.5)
                                .color(theme::TEXT_MUTED),
                        );
                    }

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);
                    widgets::section_label(ui, Text::PropSpeed.tr(locale));
                    let speed_slider = ui.add(
                        egui::Slider::new(&mut speed_factor, SPEED_FACTOR_RANGE)
                            .suffix("x")
                            .fixed_decimals(2),
                    );
                    if speed_slider.changed() {
                        app.set_selected_clip_speed(speed_factor);
                    }
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(Text::SpeedExportNote.tr(locale))
                            .size(10.5)
                            .color(theme::TEXT_MUTED),
                    );

                    // Crop reframes the video frame itself — no meaning for an audio block.
                    if app.selected_clip_track_kind() == Some(avcore::timeline::TrackKind::Video) {
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);
                        widgets::section_label(ui, Text::PropCrop.tr(locale));
                        let mut crop_changed = false;
                        ui.horizontal(|ui| {
                            crop_changed |= ui
                                .add(
                                    egui::DragValue::new(&mut crop_x)
                                        .speed(0.01)
                                        .range(0.0..=1.0)
                                        .prefix("x "),
                                )
                                .changed();
                            crop_changed |= ui
                                .add(
                                    egui::DragValue::new(&mut crop_y)
                                        .speed(0.01)
                                        .range(0.0..=1.0)
                                        .prefix("y "),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            crop_changed |= ui
                                .add(
                                    egui::DragValue::new(&mut crop_w)
                                        .speed(0.01)
                                        .range(CROP_MIN_SIZE..=1.0)
                                        .prefix("w "),
                                )
                                .changed();
                            crop_changed |= ui
                                .add(
                                    egui::DragValue::new(&mut crop_h)
                                        .speed(0.01)
                                        .range(CROP_MIN_SIZE..=1.0)
                                        .prefix("h "),
                                )
                                .changed();
                        });
                        if crop_changed {
                            app.set_selected_clip_crop(crop_x, crop_y, crop_w, crop_h);
                        }
                        if ui.button(Text::CropReset.tr(locale)).clicked() {
                            app.set_selected_clip_crop(0.0, 0.0, 1.0, 1.0);
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(Text::CropExportNote.tr(locale))
                                .size(10.5)
                                .color(theme::TEXT_MUTED),
                        );

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);
                        widgets::section_label(ui, Text::PropMask.tr(locale));
                        let mut mask_changed = false;
                        egui::ComboBox::from_id_salt("mask_shape")
                            .selected_text(mask_shape_label(mask_shape, locale))
                            .show_ui(ui, |ui| {
                                for shape in [
                                    avcore::timeline::MaskShape::None,
                                    avcore::timeline::MaskShape::Circle,
                                    avcore::timeline::MaskShape::RoundedRect,
                                ] {
                                    mask_changed |= ui
                                        .selectable_value(
                                            &mut mask_shape,
                                            shape,
                                            mask_shape_label(shape, locale),
                                        )
                                        .changed();
                                }
                            });
                        if mask_shape == avcore::timeline::MaskShape::RoundedRect {
                            let radius_slider = ui.add(
                                egui::Slider::new(
                                    &mut mask_corner_radius,
                                    MASK_CORNER_RADIUS_RANGE,
                                )
                                .text(Text::MaskCornerRadius.tr(locale)),
                            );
                            mask_changed |= radius_slider.changed();
                        }
                        if mask_changed {
                            app.set_selected_clip_mask(mask_shape, mask_corner_radius);
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(Text::MaskExportNote.tr(locale))
                                .size(10.5)
                                .color(theme::TEXT_MUTED),
                        );

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);
                        let flip_checkbox = ui.checkbox(&mut flipped_h, Text::PropFlip.tr(locale));
                        if flip_checkbox.changed() {
                            app.set_selected_clip_flip_h(flipped_h);
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(Text::FlipExportNote.tr(locale))
                                .size(10.5)
                                .color(theme::TEXT_MUTED),
                        );

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);
                        widgets::section_label(ui, Text::PropColorFilter.tr(locale));
                        let mut color_filter_changed = false;
                        egui::ComboBox::from_id_salt("color_filter")
                            .selected_text(color_filter_label(color_filter, locale))
                            .show_ui(ui, |ui| {
                                for filter in [
                                    avcore::timeline::ColorFilter::None,
                                    avcore::timeline::ColorFilter::BlackAndWhite,
                                    avcore::timeline::ColorFilter::Sepia,
                                ] {
                                    color_filter_changed |= ui
                                        .selectable_value(
                                            &mut color_filter,
                                            filter,
                                            color_filter_label(filter, locale),
                                        )
                                        .changed();
                                }
                            });
                        if color_filter_changed {
                            app.set_selected_clip_color_filter(color_filter);
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(Text::ColorFilterExportNote.tr(locale))
                                .size(10.5)
                                .color(theme::TEXT_MUTED),
                        );

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);
                        widgets::section_label(ui, Text::PropVignette.tr(locale));
                        let vignette_slider = ui.add(
                            egui::Slider::new(&mut vignette_intensity, VIGNETTE_INTENSITY_RANGE)
                                .fixed_decimals(2),
                        );
                        if vignette_slider.changed() {
                            app.set_selected_clip_vignette(vignette_intensity);
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(Text::VignetteExportNote.tr(locale))
                                .size(10.5)
                                .color(theme::TEXT_MUTED),
                        );

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);
                        widgets::section_label(ui, Text::PropColorAdjust.tr(locale));
                        let mut color_adjust_changed = false;
                        color_adjust_changed |= ui
                            .add(
                                egui::Slider::new(&mut brightness, BRIGHTNESS_RANGE)
                                    .text(Text::PropBrightness.tr(locale)),
                            )
                            .changed();
                        color_adjust_changed |= ui
                            .add(
                                egui::Slider::new(&mut contrast, CONTRAST_RANGE)
                                    .text(Text::PropContrast.tr(locale)),
                            )
                            .changed();
                        color_adjust_changed |= ui
                            .add(
                                egui::Slider::new(&mut saturation, SATURATION_RANGE)
                                    .text(Text::PropSaturation.tr(locale)),
                            )
                            .changed();
                        if color_adjust_changed {
                            app.set_selected_clip_color_adjust(brightness, contrast, saturation);
                        }
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(Text::ColorAdjustExportNote.tr(locale))
                                .size(10.5)
                                .color(theme::TEXT_MUTED),
                        );
                    }
                }
            });
        });
}

fn mask_shape_label(shape: avcore::timeline::MaskShape, locale: crate::i18n::Locale) -> String {
    match shape {
        avcore::timeline::MaskShape::None => Text::MaskNone.tr(locale).to_string(),
        avcore::timeline::MaskShape::Circle => Text::MaskCircle.tr(locale).to_string(),
        avcore::timeline::MaskShape::RoundedRect => Text::MaskRoundedRect.tr(locale).to_string(),
    }
}

/// A translucent overlay color hinting at [`avcore::timeline::ClipInstance::color_filter`] on
/// the timeline block — `None` for [`avcore::timeline::ColorFilter::None`] (nothing drawn).
/// Just a preview-panel cue, not the real filtered pixels (see the field's doc comment for the
/// preview/export gap).
fn color_filter_tint(filter: avcore::timeline::ColorFilter) -> Option<egui::Color32> {
    match filter {
        avcore::timeline::ColorFilter::None => None,
        avcore::timeline::ColorFilter::BlackAndWhite => {
            Some(egui::Color32::from_rgba_unmultiplied(128, 128, 128, 90))
        }
        avcore::timeline::ColorFilter::Sepia => {
            Some(egui::Color32::from_rgba_unmultiplied(180, 130, 60, 70))
        }
    }
}

fn color_filter_label(
    filter: avcore::timeline::ColorFilter,
    locale: crate::i18n::Locale,
) -> String {
    match filter {
        avcore::timeline::ColorFilter::None => Text::ColorFilterNone.tr(locale).to_string(),
        avcore::timeline::ColorFilter::BlackAndWhite => {
            Text::ColorFilterBlackAndWhite.tr(locale).to_string()
        }
        avcore::timeline::ColorFilter::Sepia => Text::ColorFilterSepia.tr(locale).to_string(),
    }
}

fn prop_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.0).color(theme::TEXT_MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(12.0));
        });
    });
}

/// Bounds for `OcaApp::timeline_px_per_sec` — tight enough to stay readable, loose enough to
/// go from several-projects-wide overview down to frame-accurate editing.
const MIN_PX_PER_SEC: f32 = 0.5;
const MAX_PX_PER_SEC: f32 = 60.0;
const TRACK_LABEL_WIDTH: f32 = 50.0;

fn timeline_panel(app: &mut OcaApp, ui: &mut egui::Ui, height: f32) {
    widgets::card_frame().show(ui, |ui| {
        ui.set_height(height - 20.0);

        // Ctrl+scroll zooms the timeline in/out (request.md's Fase 3 spec). egui's
        // `zoom_delta()` already separates ctrl-held scroll from plain scroll at the input
        // level (`Modifiers::COMMAND`, which is Ctrl outside macOS) — plain scroll still
        // reaches the ScrollArea below as normal, no manual event-consuming needed.
        let zoom_delta = ui.input(|i| i.zoom_delta());
        if zoom_delta != 1.0 && ui.rect_contains_pointer(ui.max_rect()) {
            app.timeline_px_per_sec =
                (app.timeline_px_per_sec * zoom_delta).clamp(MIN_PX_PER_SEC, MAX_PX_PER_SEC);
        }
        let px_per_sec = app.timeline_px_per_sec;

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
        let mut ruler_top = 0.0_f32;
        ui.horizontal(|ui| {
            ui.add_space(TRACK_LABEL_WIDTH);
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 14.0),
                egui::Sense::click_and_drag(),
            );
            ruler_top = rect.top();
            ui.painter().rect_filled(rect, 0, theme::SURFACE_2);
            if let Some(pos) = response.interact_pointer_pos() {
                let secs = ((pos.x - rect.left()) / px_per_sec).max(0.0) as f64;
                app.active_project_mut().timeline_mut().playhead_secs = secs;
            }
            draw_playhead(
                ui,
                rect,
                app.active_project().timeline().playhead_secs,
                px_per_sec,
                2.0,
            );
        });

        let locale = app.locale;
        let playhead_secs = app.active_project().timeline().playhead_secs;
        let has_clipboard_clip = app.has_clipboard_clip();
        let has_formatting_clipboard = app.has_formatting_clipboard();
        let mut clicked_clip_id = None;
        let mut delete_requests: Vec<u64> = Vec::new();
        let mut copy_requests: Vec<u64> = Vec::new();
        let mut cut_requests: Vec<u64> = Vec::new();
        let mut copy_formatting_requests: Vec<u64> = Vec::new();
        let mut paste_formatting_requests: Vec<u64> = Vec::new();
        let mut multi_select_requests: Vec<u64> = Vec::new();
        let mut paste_requested = false;
        let mut split_at_playhead_requested = false;
        let mut trim_requests: Vec<(u64, TrimEdge)> = Vec::new();
        let mut clip_drags: Vec<ClipDrag> = Vec::new();
        let mut track_rows: Vec<(u64, avcore::timeline::TrackKind, egui::Rect)> = Vec::new();
        let mut thumbnail_requests: Vec<(u64, i64)> = Vec::new();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for track in &app.active_project().timeline().tracks {
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
                    track_rows.push((track.id, track.kind, track_rect));
                    let painter = ui.painter();
                    for clip in &track.clips {
                        let x = track_rect.left() + clip.start_secs as f32 * px_per_sec;
                        let w = (clip.duration_secs() as f32 * px_per_sec).max(3.0);
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
                        let covers_playhead = clip.start_secs <= playhead_secs
                            && playhead_secs < clip.start_secs + clip.duration_secs();
                        body_response.context_menu(|ui| {
                            clicked_clip_id = Some(clip.id);
                            if ui
                                .add_enabled(
                                    covers_playhead,
                                    egui::Button::new(Text::ContextMenuSplit.tr(locale)),
                                )
                                .clicked()
                            {
                                split_at_playhead_requested = true;
                                ui.close();
                            }
                            if ui.button(Text::ContextMenuCopy.tr(locale)).clicked() {
                                copy_requests.push(clip.id);
                                ui.close();
                            }
                            if ui.button(Text::ContextMenuCut.tr(locale)).clicked() {
                                cut_requests.push(clip.id);
                                ui.close();
                            }
                            if ui
                                .add_enabled(
                                    has_clipboard_clip,
                                    egui::Button::new(Text::ContextMenuPaste.tr(locale)),
                                )
                                .clicked()
                            {
                                paste_requested = true;
                                ui.close();
                            }
                            ui.separator();
                            if ui
                                .button(Text::ContextMenuCopyFormatting.tr(locale))
                                .clicked()
                            {
                                copy_formatting_requests.push(clip.id);
                                ui.close();
                            }
                            if ui
                                .add_enabled(
                                    has_formatting_clipboard,
                                    egui::Button::new(Text::ContextMenuPasteFormatting.tr(locale)),
                                )
                                .clicked()
                            {
                                paste_formatting_requests.push(clip.id);
                                ui.close();
                            }
                            ui.separator();
                            if ui.button(Text::ContextMenuDelete.tr(locale)).clicked() {
                                delete_requests.push(clip.id);
                                ui.close();
                            }
                        });
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
                            if ui.input(|i| i.modifiers.ctrl) {
                                multi_select_requests.push(clip.id);
                            } else {
                                clicked_clip_id = Some(clip.id);
                            }
                        }
                        if body_response.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                            let delta_secs = (body_response.drag_delta().x / px_per_sec) as f64;
                            if let Some(pointer) = body_response.interact_pointer_pos() {
                                clip_drags.push(ClipDrag {
                                    clip_id: clip.id,
                                    source_track_id: track.id,
                                    kind: track.kind,
                                    new_start_secs: clip.start_secs + delta_secs,
                                    pointer_y: pointer.y,
                                });
                            }
                        }
                        if let Some(pos) = left_response.interact_pointer_pos() {
                            let secs = ((pos.x - track_rect.left()) / px_per_sec).max(0.0) as f64;
                            trim_requests.push((clip.id, TrimEdge::Start(secs)));
                        }
                        if let Some(pos) = right_response.interact_pointer_pos() {
                            let secs = ((pos.x - track_rect.left()) / px_per_sec).max(0.0) as f64;
                            trim_requests.push((clip.id, TrimEdge::End(secs)));
                        }

                        painter.rect_filled(clip_rect, egui::CornerRadius::same(4), color);
                        let asset = app
                            .active_project()
                            .media_library
                            .iter()
                            .find(|a| a.id == clip.asset_id);
                        if track.kind == avcore::timeline::TrackKind::Video {
                            if let Some(asset) = asset {
                                if clip.frozen {
                                    draw_frozen_poster(
                                        &app.thumbnail_textures,
                                        painter,
                                        clip_rect,
                                        asset,
                                        clip.source_in_secs,
                                        &mut thumbnail_requests,
                                    );
                                } else {
                                    draw_filmstrip(
                                        &app.thumbnail_textures,
                                        painter,
                                        clip_rect,
                                        asset,
                                        clip.source_in_secs,
                                        px_per_sec,
                                        &mut thumbnail_requests,
                                    );
                                }
                            }
                        } else if let Some(asset) = asset {
                            if let Some(peaks) = &asset.waveform_peaks {
                                draw_waveform(
                                    painter,
                                    clip_rect,
                                    peaks,
                                    asset.duration_secs,
                                    clip.source_in_secs..clip.source_out_secs,
                                    clip.gain_linear(),
                                    theme::TEXT_PRIMARY.gamma_multiply(0.7),
                                );
                            }
                        }
                        if let Some(tint) = color_filter_tint(clip.color_filter) {
                            painter.rect_filled(clip_rect, egui::CornerRadius::same(4), tint);
                        }
                        if clip.has_vignette() {
                            let alpha = (clip.vignette_intensity * 200.0) as u8;
                            painter.rect_stroke(
                                clip_rect,
                                egui::CornerRadius::same(4),
                                egui::Stroke::new(3.0, egui::Color32::from_black_alpha(alpha)),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if clip.composite_id.is_some() {
                            painter.rect_stroke(
                                clip_rect,
                                egui::CornerRadius::same(4),
                                egui::Stroke::new(1.5, theme::ACCENT_2),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if clip.speed_factor != 1.0 {
                            painter.text(
                                clip_rect.right_top() + egui::vec2(-3.0, 2.0),
                                egui::Align2::RIGHT_TOP,
                                format!("{:.2}x", clip.speed_factor),
                                egui::FontId::proportional(11.0),
                                theme::TEXT_PRIMARY,
                            );
                        }
                        if clip.is_cropped() {
                            painter.text(
                                clip_rect.right_bottom() + egui::vec2(-3.0, -2.0),
                                egui::Align2::RIGHT_BOTTOM,
                                "⛶",
                                egui::FontId::proportional(11.0),
                                theme::TEXT_PRIMARY,
                            );
                        }
                        if clip.is_masked() {
                            let glyph = match clip.mask_shape {
                                avcore::timeline::MaskShape::Circle => "●",
                                avcore::timeline::MaskShape::RoundedRect => "▢",
                                avcore::timeline::MaskShape::None => "",
                            };
                            painter.text(
                                clip_rect.left_bottom() + egui::vec2(3.0, -2.0),
                                egui::Align2::LEFT_BOTTOM,
                                glyph,
                                egui::FontId::proportional(11.0),
                                theme::TEXT_PRIMARY,
                            );
                        }
                        if clip.flipped_h {
                            painter.text(
                                clip_rect.center_top() + egui::vec2(0.0, 2.0),
                                egui::Align2::CENTER_TOP,
                                "⇄",
                                egui::FontId::proportional(11.0),
                                theme::TEXT_PRIMARY,
                            );
                        }
                        if app.multi_selected_clip_ids.contains(&clip.id) {
                            painter.rect_stroke(
                                clip_rect,
                                egui::CornerRadius::same(4),
                                egui::Stroke::new(2.0, theme::ERROR),
                                egui::StrokeKind::Inside,
                            );
                        }
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
                        app.active_project().timeline().playhead_secs,
                        px_per_sec,
                        1.0,
                    );
                });
                ui.add_space(4.0);
            }
            if app.active_project().timeline().tracks.is_empty() {
                ui.label(
                    RichText::new(Text::TimelineEmpty.tr(app.locale))
                        .size(12.0)
                        .color(theme::TEXT_MUTED),
                );
            }
        });
        if let Some(id) = clicked_clip_id {
            app.select_timeline_clip(id);
        }
        for clip_id in multi_select_requests {
            app.toggle_multi_select(clip_id);
        }
        for clip_id in copy_requests {
            app.selected_clip_id = Some(clip_id);
            app.copy_selected_clip();
        }
        for clip_id in cut_requests {
            app.selected_clip_id = Some(clip_id);
            app.cut_selected_clip();
        }
        for clip_id in copy_formatting_requests {
            app.selected_clip_id = Some(clip_id);
            app.copy_selected_clip_formatting();
        }
        for clip_id in paste_formatting_requests {
            app.selected_clip_id = Some(clip_id);
            app.paste_selected_clip_formatting();
        }
        if paste_requested {
            app.paste_clip_at_playhead();
        }
        for clip_id in delete_requests {
            app.selected_clip_id = Some(clip_id);
            app.delete_selected_clip();
        }
        if split_at_playhead_requested {
            app.split_at_playhead();
        }
        for (clip_id, edge) in trim_requests {
            match edge {
                TrimEdge::Start(secs) => app.trim_clip_start(clip_id, secs),
                TrimEdge::End(secs) => app.trim_clip_end(clip_id, secs),
            }
        }
        for drag in clip_drags {
            // Whichever track row's Y-range the pointer is currently over, if its kind
            // matches the dragged clip's own track — a video clip can't be dropped onto an
            // audio row or vice versa. Falls back to a same-track reposition if the pointer
            // isn't over any matching row (including its own, the common case).
            let target_track_id = track_rows
                .iter()
                .find(|(_, kind, rect)| {
                    *kind == drag.kind && rect.y_range().contains(drag.pointer_y)
                })
                .map(|(id, _, _)| *id);
            // A composite block's members must all stay on the same track (see
            // ClipInstance::composite_id's doc comment), so a cross-track drop is refused for
            // one — it falls back to the same-track group move below instead.
            let is_composite = app
                .active_project()
                .timeline()
                .tracks
                .iter()
                .flat_map(|t| &t.clips)
                .find(|c| c.id == drag.clip_id)
                .is_some_and(|c| c.composite_id.is_some());
            match target_track_id {
                Some(track_id) if track_id != drag.source_track_id && !is_composite => {
                    app.move_clip_to_track(drag.clip_id, track_id, drag.new_start_secs);
                }
                _ => app.move_clip_with_group(drag.clip_id, drag.new_start_secs),
            }
        }
        for (asset_id, bucket) in thumbnail_requests {
            app.request_thumbnail(asset_id, bucket);
        }
        // An asset dragged out of the media library and released somewhere at or below the
        // ruler: whichever track row's Y-range the pointer landed on becomes the preferred
        // drop target (`OcaApp::add_asset_to_timeline_at` falls back to a matching-kind track
        // if that row's kind doesn't match the asset, same as a cross-track clip move). A
        // release above the ruler means the drag never reached the timeline at all, so it's
        // ignored rather than silently appending.
        if let Some((asset_id, pos)) = app.pending_asset_drop.take() {
            if pos.y >= ruler_top {
                let target = track_rows
                    .iter()
                    .find(|(_, _, rect)| rect.y_range().contains(pos.y));
                match target {
                    Some((track_id, _, rect)) => {
                        let secs = ((pos.x - rect.left()) / px_per_sec).max(0.0) as f64;
                        app.add_asset_to_timeline_at(asset_id, Some(*track_id), secs);
                    }
                    None => app.add_asset_to_timeline(asset_id),
                }
            }
        }
    });
}

/// Draws one filmstrip tile per column of `clip_rect` (each column's width set by `asset`'s
/// aspect ratio scaled to the clip's height, same as a single poster frame tiled), each showing
/// the cached thumbnail texture for whichever [`THUMBNAIL_BUCKET_SECS`]-quantized source-time
/// bucket that column falls in (`OcaApp::thumbnail_textures`) — distinct buckets read like a
/// true filmstrip, unlike one poster frame repeated. A column whose bucket has no texture yet
/// is queued into `thumbnail_requests` (deduplicated against already-pending buckets by
/// [`OcaApp::request_thumbnail`] once the caller applies it) and left showing the clip's plain
/// background color, already painted underneath by the caller, until it arrives. A no-op if
/// `asset` has no resolution (audio-only, shouldn't happen for a clip on a video track).
fn draw_filmstrip(
    thumbnail_textures: &HashMap<(u64, i64), egui::TextureHandle>,
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    asset: &MediaAsset,
    source_in_secs: f64,
    px_per_sec: f32,
    thumbnail_requests: &mut Vec<(u64, i64)>,
) {
    let Some((res_w, res_h)) = asset.resolution else {
        return;
    };
    if res_h == 0 {
        return;
    }
    let tile_height = clip_rect.height();
    let tile_width = (tile_height * res_w as f32 / res_h as f32).max(1.0);

    let mut x = clip_rect.left();
    while x < clip_rect.right() {
        let w = tile_width.min(clip_rect.right() - x);
        let tile_rect =
            egui::Rect::from_min_size(egui::pos2(x, clip_rect.top()), egui::vec2(w, tile_height));
        let time_in_source = source_in_secs + ((x - clip_rect.left()) / px_per_sec) as f64;
        let bucket = (time_in_source / THUMBNAIL_BUCKET_SECS).floor() as i64;
        match thumbnail_textures.get(&(asset.id, bucket)) {
            Some(texture) => {
                painter.image(
                    texture.id(),
                    tile_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            None => thumbnail_requests.push((asset.id, bucket)),
        }
        x += tile_width;
    }
}

/// Draws a single poster frame — the one at `source_in_secs`, the frame a frozen block
/// ([`avcore::timeline::ClipInstance::frozen`]) holds per `request.md`'s Fase 4 "Congelar"
/// spec — tiled across all of `clip_rect`, plus a small "❄" badge marking the block as frozen
/// even at a zoom level too tight to tell a still poster from a real filmstrip. Reuses
/// `draw_filmstrip`'s texture cache/request plumbing, just pinned to one bucket instead of one
/// per column. A no-op if `asset` has no resolution (audio-only, shouldn't happen for a clip on
/// a video track).
fn draw_frozen_poster(
    thumbnail_textures: &HashMap<(u64, i64), egui::TextureHandle>,
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    asset: &MediaAsset,
    source_in_secs: f64,
    thumbnail_requests: &mut Vec<(u64, i64)>,
) {
    let Some((res_w, res_h)) = asset.resolution else {
        return;
    };
    if res_h == 0 {
        return;
    }
    let tile_height = clip_rect.height();
    let tile_width = (tile_height * res_w as f32 / res_h as f32).max(1.0);
    let bucket = (source_in_secs / THUMBNAIL_BUCKET_SECS).floor() as i64;

    match thumbnail_textures.get(&(asset.id, bucket)) {
        Some(texture) => {
            let mut x = clip_rect.left();
            while x < clip_rect.right() {
                let w = tile_width.min(clip_rect.right() - x);
                let tile_rect = egui::Rect::from_min_size(
                    egui::pos2(x, clip_rect.top()),
                    egui::vec2(w, tile_height),
                );
                painter.image(
                    texture.id(),
                    tile_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                x += tile_width;
            }
        }
        None => thumbnail_requests.push((asset.id, bucket)),
    }

    painter.text(
        clip_rect.left_top() + egui::vec2(3.0, 2.0),
        egui::Align2::LEFT_TOP,
        "❄",
        egui::FontId::proportional(12.0),
        egui::Color32::WHITE,
    );
}

/// Draws one vertical min/max bar per horizontal pixel of `clip_rect`, resampling `peaks`
/// (the asset's full-duration waveform, see [`avcore::waveform`]) down to whatever's visible
/// between `source_in_secs` and `source_out_secs` — the same "fixed-resolution source data
/// resampled to the current on-screen width" idea as `draw_filmstrip`'s zoom handling, just
/// per-column instead of per-tile. A no-op if `peaks` is empty or `asset_duration_secs`
/// is non-positive (shouldn't happen for a real decoded asset, but guards div-by-zero).
fn draw_waveform(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    peaks: &[(f32, f32)],
    asset_duration_secs: f64,
    source_range_secs: std::ops::Range<f64>,
    gain_linear: f32,
    color: egui::Color32,
) {
    if peaks.is_empty() || asset_duration_secs <= 0.0 {
        return;
    }
    let bucket_count = peaks.len() as f64;
    let start_bucket = (source_range_secs.start / asset_duration_secs * bucket_count)
        .clamp(0.0, bucket_count - 1.0);
    let end_bucket =
        (source_range_secs.end / asset_duration_secs * bucket_count).clamp(0.0, bucket_count);
    let bucket_span = (end_bucket - start_bucket).max(1.0);

    let mid_y = clip_rect.center().y;
    let half_h = clip_rect.height() / 2.0 - 1.0;
    let width_px = clip_rect.width().max(1.0) as usize;

    for x in 0..width_px {
        let t = x as f64 / width_px as f64;
        let bucket = ((start_bucket + t * bucket_span) as usize).min(peaks.len() - 1);
        let (min, max) = peaks[bucket];
        // Live gain preview (Fase 4 "ganho de volume por bloco") — bars scale with the block's
        // gain, clamped so extreme gain doesn't paint outside the clip's own row.
        let min = (min * gain_linear).clamp(-1.0, 1.0);
        let max = (max * gain_linear).clamp(-1.0, 1.0);
        let px = clip_rect.left() + x as f32;
        painter.line_segment(
            [
                egui::pos2(px, mid_y - max * half_h),
                egui::pos2(px, mid_y - min * half_h),
            ],
            egui::Stroke::new(1.0, color),
        );
    }
}

/// A clip body drag in progress: which clip, where it started from, and where the pointer
/// currently is — resolved into a same-track reposition or a cross-track move once every
/// track's row rect has been computed (see the loop in `timeline_panel`).
struct ClipDrag {
    clip_id: u64,
    source_track_id: u64,
    kind: avcore::timeline::TrackKind,
    new_start_secs: f64,
    pointer_y: f32,
}

/// Which edge of a timeline clip a drag targets — see the trim handling in `timeline_panel`.
enum TrimEdge {
    Start(f64),
    End(f64),
}

/// Draws the playhead as a vertical line inside `rect`, if it falls within `rect`'s horizontal
/// span — used both on the ruler strip and on every track row so it reads as one continuous
/// line down the timeline despite each row being drawn separately.
fn draw_playhead(
    ui: &egui::Ui,
    rect: egui::Rect,
    playhead_secs: f64,
    px_per_sec: f32,
    stroke_width: f32,
) {
    let x = rect.left() + playhead_secs as f32 * px_per_sec;
    if x >= rect.left() && x <= rect.right() {
        ui.painter().vline(
            x,
            rect.y_range(),
            egui::Stroke::new(stroke_width, theme::ACCENT),
        );
    }
}
