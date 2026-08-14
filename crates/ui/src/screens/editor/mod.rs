mod properties_panel;
mod timeline_panel;

use avcore::media::format_timecode;
use eframe::egui::{self, RichText};

use crate::app::{App, EditorTool};
use crate::components;
use crate::i18n::Text;
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
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    app.ensure_active_project();

    // Clone the configurable combos before the first ui.input() call so we can pass them
    // into separate closures without holding a borrow on app across the closure boundary.
    let play_pause_combo = app.prefs.key_bindings.play_pause.clone();
    let split_combo = app.prefs.key_bindings.split_at_playhead.clone();
    let copy_fmt_combo = app.prefs.key_bindings.copy_formatting.clone();
    let paste_fmt_combo = app.prefs.key_bindings.paste_formatting.clone();

    let ctrl_s_pressed = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S));
    if ctrl_s_pressed {
        save_active_project(app);
    }
    let split_pressed = ui.input(|i| split_combo.matches(i));
    if split_pressed {
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
    // "Copiar/Colar formatação" — copies effect settings between clips without duplicating
    // the clip itself. Distinct from plain Ctrl+C/V (whole-clip copy/paste) above.
    let copy_fmt_pressed = ui.input(|i| copy_fmt_combo.matches(i));
    if copy_fmt_pressed {
        app.copy_selected_clip_formatting();
    }
    let paste_fmt_pressed = ui.input(|i| paste_fmt_combo.matches(i));
    if paste_fmt_pressed {
        app.paste_selected_clip_formatting();
    }
    let play_pause_pressed = ui.input(|i| play_pause_combo.matches(i));
    if play_pause_pressed {
        app.toggle_preview_playback();
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
                properties_panel::properties_panel(app, ui, app.props_panel_width, body_height);
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
        timeline_panel::timeline_panel(app, ui, app.timeline_height);
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

fn toolbar(app: &mut App, ui: &mut egui::Ui) {
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
        if ui.button(Text::AddVideoTrack.tr(locale)).clicked() {
            app.add_video_track();
        }
        ui.separator();
        if ui.button(Text::AddTextTrack.tr(locale)).clicked() {
            app.add_text_track();
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
fn sequence_tab_bar(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    let active_index = app.active_project().active_sequence;
    let mut select_index = None;
    let mut rename_index = None;
    let mut add_requested = false;

    ui.horizontal(|ui| {
        let count = app.active_project().sequences.len();
        for index in 0..count {
            let active = index == active_index;
            let name = app.active_project().sequences[index].name.clone();
            let text = RichText::new(&name).color(if active {
                theme::ACCENT
            } else {
                theme::TEXT_SECONDARY
            });
            let button = egui::Button::new(text).fill(if active {
                theme::SURFACE_2
            } else {
                theme::SURFACE
            });
            let resp = ui.add(button);
            if resp.clicked() && !active {
                select_index = Some(index);
            }
            resp.context_menu(|ui| {
                if ui
                    .button(crate::i18n::Text::SequenceTabCtxRename.tr(locale))
                    .clicked()
                {
                    rename_index = Some((index, name));
                }
            });
        }
        if ui.button(Text::AddSequenceTab.tr(locale)).clicked() {
            add_requested = true;
        }
    });

    if let Some(index) = select_index {
        app.select_sequence(index);
    }
    if let Some((index, current_name)) = rename_index {
        app.renaming_sequence = Some((index, current_name));
    }
    if add_requested {
        app.add_sequence();
    }
}

/// Saves the active project to its remembered [`avcore::Project::file_path`], or prompts
/// for a destination (and remembers it for next time) if it doesn't have one yet.
fn save_active_project(app: &mut App) {
    let path = match app.active_project().file_path.clone() {
        Some(path) => Some(path),
        None => rfd::FileDialog::new()
            .add_filter("oca project", &["ocproj"])
            .set_file_name(format!("{}.ocproj", app.active_project().name))
            .save_file(),
    };
    let Some(path) = path else { return };

    match avcore::save_project_to_file(app.active_project(), &path) {
        Ok(()) => {
            tracing::info!(path = %path.display(), "project saved");
            let path_str = path.display().to_string();
            app.active_project_mut().file_path = Some(path);
            // Track in recents (covers first Save As, where file_path was previously None).
            app.prefs.recent_project_paths.retain(|p| p != &path_str);
            app.prefs.recent_project_paths.insert(0, path_str);
            app.prefs.recent_project_paths.truncate(10);
            app.save_prefs();
        }
        Err(e) => app.push_toast(format!("Failed to save project: {e}")),
    }
}

fn tool_button(app: &mut App, ui: &mut egui::Ui, tool: EditorTool, icon: &str, label: &str) {
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

fn media_library_panel(app: &mut App, ui: &mut egui::Ui, width: f32, height: f32) {
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
                    components::section_label(ui, Text::MediaLibrary.tr(app.locale));
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

fn preview_panel(app: &mut App, ui: &mut egui::Ui, height: f32) {
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
                match &app.preview_texture {
                    Some(_) => layer_transform_preview(app, ui),
                    None if app.preview_clip_present() && !app.preview_available() => {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new(Text::PreviewUnavailable.tr(locale))
                                    .size(13.0)
                                    .color(theme::TEXT_MUTED),
                            );
                        });
                    }
                    None => {
                        ui.centered_and_justified(|ui| {
                            ui.label(RichText::new("▶").size(48.0).color(theme::TEXT_MUTED));
                        });
                    }
                };
            });
        let timeline_duration = app.active_project().timeline().duration_secs();
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
                app.seek_preview(timeline_duration);
            }
            let playhead = app.active_project().timeline().playhead_secs;
            ui.label(
                RichText::new(format!(
                    "{} / {}",
                    format_timecode(playhead),
                    format_timecode(timeline_duration.max(playhead))
                ))
                .size(12.0)
                .color(theme::TEXT_SECONDARY)
                .monospace(),
            );
        });
        if timeline_duration > 0.0 {
            let mut position = app.active_project().timeline().playhead_secs;
            let slider =
                ui.add(egui::Slider::new(&mut position, 0.0..=timeline_duration).show_value(false));
            if slider.changed() {
                app.seek_preview(position);
            }
        }
    });
}

/// Draws the selected clip's video inside a canvas-space box sized to `app.export_aspect_ratio`
/// (the same target dimensions export will actually use, via
/// [`avcore::ExportAspectRatio::dims_or`]) and, when its position isn't already animated
/// (`position_keyframes.len() <= 1`, the "static transform" case — see `request.md`'s Fase 4
/// "Transformação de camadas" spec), lets the user drag it to set a single position keyframe.
/// This maps directly onto `overlay=x:y`'s own coordinate space
/// (`crate::keyframe::position_overlay_xy_expr`, canvas-fraction offset from the top-left
/// default) — dragging writes exactly what export reads, no separate UI-only representation.
///
/// Layer *size* (`ClipInstance::layer_scale_x`/`_y`) is a multiplier on top of a fixed
/// stand-in baseline footprint (40% of the canvas's shorter side) rather than a real pixel
/// dimension — this panel doesn't know the clip's actual export-time decoded resolution (the
/// preview texture may be a lower-res editing proxy), so it can't draw the box at its true
/// composited size. Dragging the bottom-right handle still writes the real multiplier
/// `set_selected_clip_layer_scale` reads at export, same "editable but visually approximate"
/// shape as most of this panel. `Some(_)` in `preview_panel`'s match already guarantees
/// `app.preview_texture` is set, but this re-checks (and bails) rather than trust that
/// invariant across the borrow-splitting clone below.
fn layer_transform_preview(app: &mut App, ui: &mut egui::Ui) {
    let Some(texture) = app.preview_texture.clone() else {
        return;
    };
    let locale = app.locale;
    let tex_size = texture.size_vec2();
    let tex_aspect = tex_size.x / tex_size.y;

    let (canvas_w, canvas_h) = app
        .export_aspect_ratio
        .dims_or((tex_size.x as u32, tex_size.y as u32));
    let canvas_aspect = canvas_w as f32 / canvas_h.max(1) as f32;

    let avail_rect = ui.available_rect_before_wrap();
    let mut canvas_size = avail_rect.size();
    if canvas_size.x / canvas_size.y > canvas_aspect {
        canvas_size.x = canvas_size.y * canvas_aspect;
    } else {
        canvas_size.y = canvas_size.x / canvas_aspect;
    }
    let canvas_rect = egui::Rect::from_center_size(avail_rect.center(), canvas_size);
    ui.allocate_rect(canvas_rect, egui::Sense::hover());
    ui.painter().rect_stroke(
        canvas_rect,
        0,
        egui::Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Inside,
    );

    // Fixed stand-in baseline (40% of the canvas's shorter side, clipped to the canvas width) —
    // see this function's doc comment on why this is a multiplier applied to a stand-in size
    // rather than a real pixel dimension.
    let mut base_h = canvas_rect.height().min(canvas_rect.width()) * 0.4;
    let mut base_w = base_h * tex_aspect;
    if base_w > canvas_rect.width() {
        base_w = canvas_rect.width();
        base_h = base_w / tex_aspect;
    }

    let selected_clip = app.selected_clip();
    let position_keyframes = selected_clip
        .map(|c| c.position_keyframes.clone())
        .unwrap_or_default();
    let (layer_scale_x, layer_scale_y) = selected_clip
        .map(|c| (c.layer_scale_x, c.layer_scale_y))
        .unwrap_or((1.0, 1.0));
    let resizable = selected_clip.is_some();
    let draggable = position_keyframes.len() <= 1;
    let current = position_keyframes
        .first()
        .map(|k| k.value)
        .unwrap_or(avcore::Position { x: 0.0, y: 0.0 });

    let layer_w = base_w * layer_scale_x;
    let layer_h = base_h * layer_scale_y;

    let layer_min = canvas_rect.min
        + egui::vec2(
            current.x * canvas_rect.width(),
            current.y * canvas_rect.height(),
        );
    let layer_rect = egui::Rect::from_min_size(layer_min, egui::vec2(layer_w, layer_h));

    ui.painter().image(
        texture.id(),
        layer_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );

    let border_color = if draggable {
        theme::ACCENT_2
    } else {
        theme::TEXT_MUTED
    };
    ui.painter().rect_stroke(
        layer_rect,
        0,
        egui::Stroke::new(1.5, border_color),
        egui::StrokeKind::Outside,
    );

    if draggable {
        let resp = ui.interact(
            layer_rect,
            ui.id().with("preview_layer_transform"),
            egui::Sense::drag(),
        );
        if resp.dragged() {
            let delta = resp.drag_delta();
            let new_pos = avcore::Position {
                x: current.x + delta.x / canvas_rect.width().max(1.0),
                y: current.y + delta.y / canvas_rect.height().max(1.0),
            };
            app.set_selected_clip_position_keyframes(vec![avcore::Keyframe {
                time_fraction: 0.0,
                value: new_pos,
            }]);
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        resp.on_hover_text(Text::LayerTransformDragHint.tr(locale));
    } else {
        ui.painter().text(
            layer_rect.center_bottom() + egui::vec2(0.0, 4.0),
            egui::Align2::CENTER_TOP,
            Text::LayerTransformAnimatedHint.tr(locale),
            egui::FontId::proportional(10.0),
            theme::TEXT_MUTED,
        );
    }

    if resizable {
        const HANDLE_SIZE: f32 = 10.0;
        let handle_rect =
            egui::Rect::from_center_size(layer_rect.right_bottom(), egui::Vec2::splat(HANDLE_SIZE));
        let resize_resp = ui.interact(
            handle_rect,
            ui.id().with("preview_layer_resize"),
            egui::Sense::drag(),
        );
        ui.painter().rect_filled(handle_rect, 2, theme::ACCENT_2);
        if resize_resp.dragged() {
            let delta = resize_resp.drag_delta();
            let new_layer_w = (layer_w + delta.x).max(4.0);
            let new_layer_h = (layer_h + delta.y).max(4.0);
            app.set_selected_clip_layer_scale(
                layer_scale_x * (new_layer_w / layer_w.max(1.0)),
                layer_scale_y * (new_layer_h / layer_h.max(1.0)),
            );
        }
        if resize_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
        }
        resize_resp.on_hover_text(Text::LayerTransformResizeHint.tr(locale));
    }
}
