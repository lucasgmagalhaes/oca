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

mod properties_panel;
mod timeline_panel;

use avcore::media::format_timecode;
use eframe::egui::{self, RichText};

use crate::app::{App, EditorTool, MOTION_TRACK_SIZE_RANGE};
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

    // Coalesces a held-down properties-panel slider/DragValue drag into one undo step — see
    // App::push_undo_snapshot_for_drag's doc comment for why this can't just be a
    // drag_started() check at each of the ~20 individual slider call sites.
    let pointer_down = ui.input(|i| i.pointer.any_down());
    app.end_undo_drag_tracking_if_pointer_released(pointer_down);

    // Clone the configurable combos before the first ui.input() call so we can pass them
    // into separate closures without holding a borrow on app across the closure boundary.
    let play_pause_combo = app.prefs.key_bindings.play_pause.clone();
    let split_combo = app.prefs.key_bindings.split_at_playhead.clone();
    let copy_fmt_combo = app.prefs.key_bindings.copy_formatting.clone();
    let paste_fmt_combo = app.prefs.key_bindings.paste_formatting.clone();
    let add_opacity_marker_combo = app.prefs.key_bindings.add_opacity_marker.clone();
    let undo_combo = app.prefs.key_bindings.undo.clone();
    let redo_combo = app.prefs.key_bindings.redo.clone();

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
    let add_opacity_marker_pressed = ui.input(|i| add_opacity_marker_combo.matches(i));
    if add_opacity_marker_pressed {
        app.add_opacity_marker_at_playhead();
    }
    let undo_pressed = ui.input(|i| undo_combo.matches(i));
    if undo_pressed {
        app.undo();
    }
    let redo_pressed = ui.input(|i| redo_combo.matches(i));
    if redo_pressed {
        app.redo();
    }
    // Multicam angle switching (P2 item 10) -- 1..9 at the playhead, no modifier, matching
    // every other bare-key editing shortcut above (Delete, split) rather than needing a
    // configurable KeyCombo of its own.
    const NUMBER_KEYS: [egui::Key; 9] = [
        egui::Key::Num1,
        egui::Key::Num2,
        egui::Key::Num3,
        egui::Key::Num4,
        egui::Key::Num5,
        egui::Key::Num6,
        egui::Key::Num7,
        egui::Key::Num8,
        egui::Key::Num9,
    ];
    for (angle_index, key) in NUMBER_KEYS.into_iter().enumerate() {
        if ui.input(|i| !i.modifiers.any() && i.key_pressed(key)) {
            app.switch_multicam_angle_at_playhead(angle_index);
        }
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
        tool_button(
            app,
            ui,
            EditorTool::Ripple,
            "⇥",
            Text::ToolRipple.tr(locale),
        );
        tool_button(app, ui, EditorTool::Roll, "⇄", Text::ToolRoll.tr(locale));
        tool_button(app, ui, EditorTool::Slip, "↕", Text::ToolSlip.tr(locale));
        tool_button(app, ui, EditorTool::Slide, "⇉", Text::ToolSlide.tr(locale));
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
        if ui
            .add_enabled(
                !app.multi_selected_clip_ids.is_empty(),
                egui::Button::new(Text::SaveAsTemplate.tr(locale)),
            )
            .on_hover_text(Text::SaveAsTemplateHint.tr(locale))
            .clicked()
        {
            app.begin_save_layer_template();
        }
        if ui.button(Text::Templates.tr(locale)).clicked() {
            app.layer_templates_menu_open = true;
        }
        ui.separator();
        if ui.button(Text::AddVideoTrack.tr(locale)).clicked() {
            app.add_video_track();
        }
        ui.separator();
        if ui.button(Text::AddTextTrack.tr(locale)).clicked() {
            app.add_text_track();
        }
        if ui.button(Text::AddTextClip.tr(locale)).clicked() {
            app.add_text_clip();
        }
        ui.separator();
        if ui.button(Text::AddShapeTrack.tr(locale)).clicked() {
            app.add_shape_track();
        }
        if ui.button(Text::AddShapeClip.tr(locale)).clicked() {
            app.add_shape_clip();
        }
        if ui.button(Text::DrawCustomShape.tr(locale)).clicked() {
            if app.preview_state.preview_texture.is_some() {
                app.start_drawing_custom_shape();
            } else {
                app.push_toast(Text::ShapeDrawNeedsPreview.tr(locale).to_string());
            }
        }
        ui.separator();
        if ui
            .add_enabled(app.can_undo(), egui::Button::new("↺"))
            .on_hover_text(Text::ShortcutUndo.tr(locale))
            .clicked()
        {
            app.undo();
        }
        if ui
            .add_enabled(app.can_redo(), egui::Button::new("↻"))
            .on_hover_text(Text::ShortcutRedo.tr(locale))
            .clicked()
        {
            app.redo();
        }
        ui.separator();
        if ui
            .selectable_label(
                app.timeline_index_open,
                format!("🏷 {}", Text::TimelineIndexToggle.tr(locale)),
            )
            .clicked()
        {
            app.toggle_timeline_index();
        }
        if ui
            .selectable_label(
                app.transcript_panel_open,
                format!("📝 {}", Text::TranscriptPanelToggle.tr(locale)),
            )
            .clicked()
        {
            app.toggle_transcript_panel();
        }
        ui.separator();
        // Detection/analysis actions (silence, speech edits, chapters, highlights, gameplay
        // events, shorts pack) are reach-for-occasionally passes over the whole sequence, not
        // moment-to-moment editing tools — grouped behind one menu instead of seven permanent
        // toolbar buttons sitting alongside Select/Trim/Cut, per UX_PRINCIPLES.md's progressive
        // disclosure ("advanced operations should not permanently clutter the workspace").
        ui.menu_button(Text::AnalyzeMenu.tr(locale), |ui| {
            if ui.button(Text::DetectSilence.tr(locale)).clicked() {
                app.begin_silence_review();
                ui.close();
            }
            if ui.button(Text::DetectSpeechEdits.tr(locale)).clicked() {
                app.begin_transcript_proposals();
                ui.close();
            }
            if ui.button(Text::DetectChapters.tr(locale)).clicked() {
                app.spawn_detect_scene_cuts_for_selected_clip();
                ui.close();
            }
            if ui.button(Text::ExportChapters.tr(locale)).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("text", &["txt"])
                    .set_file_name("chapters.txt")
                    .save_file()
                {
                    app.export_chapters_txt(path);
                }
                ui.close();
            }
            if ui.button(Text::DetectHighlights.tr(locale)).clicked() {
                app.detect_highlights();
                ui.close();
            }
            if ui.button(Text::ImportGameplayEvents.tr(locale)).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("json", &["json"])
                    .pick_file()
                {
                    app.import_gameplay_events(path);
                }
                ui.close();
            }
            ui.separator();
            if ui.button(Text::ShortsPack.tr(locale)).clicked() {
                let mut dialog = rfd::FileDialog::new();
                if !app.prefs.output_folder.is_empty() {
                    dialog = dialog.set_directory(&app.prefs.output_folder);
                }
                if let Some(output_dir) = dialog.pick_folder() {
                    app.spawn_shorts_pack(output_dir);
                }
                ui.close();
            }
        });
        ui.separator();
        if ui
            .button(Text::CreateMulticamGroup.tr(locale))
            .on_hover_text("1-9")
            .clicked()
        {
            app.create_multicam_group_from_video_tracks();
        }
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
            if ui
                .button(Text::ExportSrt.tr(locale))
                .on_hover_text(Text::ExportSrtHint.tr(locale))
                .clicked()
            {
                export_srt_for_active_sequence(app);
            }
            if ui.button(Text::ExportCollabBundle.tr(locale)).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("oca collaboration bundle", &["zip"])
                    .set_file_name(format!("{}.zip", app.active_project().name))
                    .save_file()
                {
                    app.export_collab_bundle(path);
                }
            }
        });
    });
}

/// Row of tabs, one per sequence in the active project (per `request.md`'s Fase 3 "abas de
/// projeto" spec) — click a tab to switch which sequence's timeline the rest of the Editor
/// screen shows, or the trailing "+" to append a new empty one and switch to it.
#[derive(Clone, Copy)]
struct SequenceTabDrag {
    sequence_id: u64,
}

fn sequence_tab_bar(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    let active_index = app.active_project().active_sequence;
    let mut select_index = None;
    let mut rename_index = None;
    let mut duplicate_index = None;
    let mut move_request = None;
    let mut drag_move_request = None;
    let mut delete_request = None;
    let mut add_requested = false;

    ui.horizontal(|ui| {
        let count = app.active_project().sequences.len();
        for index in 0..count {
            let active = index == active_index;
            let sequence = &app.active_project().sequences[index];
            let sequence_id = sequence.id;
            let name = sequence.name.clone();
            let text = RichText::new(&name).color(if active {
                theme::ACCENT
            } else {
                theme::TEXT_SECONDARY
            });
            let button = egui::Button::new(text)
                .fill(if active {
                    theme::SURFACE_2
                } else {
                    theme::SURFACE
                })
                .sense(egui::Sense::click_and_drag());
            let resp = ui
                .add(button)
                .on_hover_text(Text::SequenceTabDragHint.tr(locale));
            resp.dnd_set_drag_payload(SequenceTabDrag { sequence_id });
            if resp
                .dnd_hover_payload::<SequenceTabDrag>()
                .is_some_and(|payload| payload.sequence_id != sequence_id)
            {
                ui.painter().rect_stroke(
                    resp.rect,
                    4,
                    egui::Stroke::new(2.0, theme::ACCENT),
                    egui::StrokeKind::Inside,
                );
            }
            if let Some(payload) = resp.dnd_release_payload::<SequenceTabDrag>() {
                if payload.sequence_id != sequence_id {
                    drag_move_request = Some((payload.sequence_id, sequence_id));
                }
            }
            if resp.clicked() && !active {
                select_index = Some(index);
            }
            resp.context_menu(|ui| {
                if ui
                    .button(crate::i18n::Text::SequenceTabCtxRename.tr(locale))
                    .clicked()
                {
                    rename_index = Some((index, name.clone()));
                }
                if ui
                    .button(Text::SequenceTabCtxDuplicate.tr(locale))
                    .clicked()
                {
                    duplicate_index = Some(index);
                }
                ui.separator();
                if ui
                    .add_enabled(
                        index > 0,
                        egui::Button::new(Text::SequenceTabCtxMoveLeft.tr(locale)),
                    )
                    .clicked()
                {
                    move_request = Some((index, index - 1));
                }
                if ui
                    .add_enabled(
                        index + 1 < count,
                        egui::Button::new(Text::SequenceTabCtxMoveRight.tr(locale)),
                    )
                    .clicked()
                {
                    move_request = Some((index, index + 1));
                }
                ui.separator();
                if ui
                    .add_enabled(
                        count > 1,
                        egui::Button::new(Text::SequenceTabCtxDelete.tr(locale)),
                    )
                    .clicked()
                {
                    delete_request = Some((sequence_id, name.clone()));
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
    if let Some(index) = duplicate_index {
        app.duplicate_sequence(index);
    }
    if let Some((from_index, target_index)) = move_request {
        app.move_sequence(from_index, target_index);
    }
    if let Some((source_id, target_id)) = drag_move_request {
        let source_index = app
            .active_project()
            .sequences
            .iter()
            .position(|sequence| sequence.id == source_id);
        let target_index = app
            .active_project()
            .sequences
            .iter()
            .position(|sequence| sequence.id == target_id);
        if let (Some(source_index), Some(target_index)) = (source_index, target_index) {
            app.move_sequence(source_index, target_index);
        }
    }
    if let Some(sequence) = delete_request {
        app.deleting_sequence = Some(sequence);
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

    app.sync_panel_layout_into_active_project();
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

/// Prompts for a destination and writes the active sequence's text-track captions out as a
/// standalone `.srt` file (`request.md`'s "arquivo `.srt` separado" half of the subtitle
/// export ask — the embedded raster-overlay half already happens on every normal export). Doesn't
/// remember the chosen path the way project saves do — each export is a one-off action, not an
/// ongoing document with its own save location.
fn export_srt_for_active_sequence(app: &mut App) {
    let locale = app.locale;
    let sequence = &app.active_project().sequences[app.active_project().active_sequence];
    let srt = avcore::export_srt(&sequence.timeline);
    let default_name = format!("{}.srt", sequence.name);
    if srt.is_empty() {
        app.push_toast(Text::ExportSrtEmpty.tr(locale).to_string());
        return;
    }

    let mut dialog = rfd::FileDialog::new()
        .add_filter("SubRip", &["srt"])
        .set_file_name(default_name);
    if !app.prefs.output_folder.is_empty() {
        dialog = dialog.set_directory(&app.prefs.output_folder);
    }
    let Some(path) = dialog.save_file() else {
        return;
    };

    match std::fs::write(&path, srt) {
        Ok(()) => tracing::info!(path = %path.display(), "subtitles exported"),
        Err(e) => app.push_toast(format!("Failed to export subtitles: {e}")),
    }
}

/// Toolbar tool-select chip: a filled, bordered pill that reads active/inactive at a glance,
/// matching `oca-editor-mock.html`'s `.tb-btn`/`.tb-btn.active` (accent-tinted fill + accent
/// border when selected) instead of the plain color-only text button this used to be.
fn tool_button(app: &mut App, ui: &mut egui::Ui, tool: EditorTool, icon: &str, label: &str) {
    let active = app.tool == tool;
    let text = RichText::new(format!("{icon} {label}")).color(if active {
        theme::ACCENT
    } else {
        theme::TEXT_SECONDARY
    });
    let button = egui::Button::new(text)
        .fill(if active {
            theme::ACCENT_TINT
        } else {
            egui::Color32::TRANSPARENT
        })
        .stroke(egui::Stroke::new(
            1.0,
            if active {
                theme::ACCENT
            } else {
                egui::Color32::TRANSPARENT
            },
        ));
    if ui.add(button).clicked() {
        app.tool = tool;
    }
}

/// A small colored placeholder thumbnail with a duration badge in the bottom-right corner,
/// matching the media-library asset row from `oca-editor-mock.html`'s `.asset-thumb` — video
/// and audio assets get distinct fills so the kind reads at a glance without a real decoded
/// frame (fetching/caching one here would duplicate `thumbnail_state`'s timeline-clip pipeline
/// for a list row that's rarely more than a name lookup).
const ASSET_THUMB_SIZE: egui::Vec2 = egui::vec2(48.0, 28.0);

fn asset_thumb(ui: &mut egui::Ui, asset: &avcore::media::MediaAsset) {
    let (rect, _response) = ui.allocate_exact_size(ASSET_THUMB_SIZE, egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let (fill, glyph) = match asset.kind {
        avcore::media::MediaKind::Video => (theme::SURFACE_2, "▶"),
        avcore::media::MediaKind::Audio => (theme::ACCENT_2.gamma_multiply(0.25), "♪"),
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, egui::CornerRadius::same(theme::RADIUS_SM), fill);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::proportional(11.0),
        theme::TEXT_MUTED,
    );
    let badge_text = asset.duration_label();
    let badge_pos = rect.right_bottom() - egui::vec2(2.0, 2.0);
    painter.text(
        badge_pos,
        egui::Align2::RIGHT_BOTTOM,
        &badge_text,
        egui::FontId::proportional(8.0),
        theme::TEXT_PRIMARY,
    );
}

fn media_library_panel(app: &mut App, ui: &mut egui::Ui, width: f32, height: f32) {
    let mut clicked_id = None;
    let mut add_to_timeline_id = None;
    let mut dropped_asset = None;
    let mut selected_bin_id = None;
    let mut edited_bin_id = None;
    let mut new_bin_clicked = false;

    egui::Frame::new()
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(height);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    components::section_label(ui, Text::MediaLibrary.tr(app.locale));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(crate::i18n::media_item_count_label(
                                app.locale,
                                app.active_project().media_library.len(),
                            ))
                            .size(11.0)
                            .color(theme::TEXT_MUTED),
                        );
                    });
                });
                ui.add_space(theme::SPACE_SM);
                ui.add(
                    egui::TextEdit::singleline(&mut app.media_search)
                        .hint_text(Text::SearchMediaPlaceholder.tr(app.locale))
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(6.0);
                // Smart bins (P4 item 22) -- a row of filter chips above the asset list. "All"
                // clears the filter; each bin is click-to-select, double-click-to-edit (the
                // rules, not the assets themselves -- there's nothing else to double-click a
                // filter chip for).
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .selectable_label(
                            app.active_smart_bin_id.is_none(),
                            Text::SmartBinAll.tr(app.locale),
                        )
                        .clicked()
                    {
                        selected_bin_id = Some(None);
                    }
                    for bin in &app.active_project().smart_bins {
                        let response =
                            ui.selectable_label(app.active_smart_bin_id == Some(bin.id), &bin.name);
                        if response.clicked() {
                            selected_bin_id = Some(Some(bin.id));
                        }
                        if response.double_clicked() {
                            edited_bin_id = Some(bin.id);
                        }
                    }
                    if ui.button(Text::SmartBinNew.tr(app.locale)).clicked() {
                        new_bin_clicked = true;
                    }
                });
                ui.add_space(6.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    // Only the (small) bin rule is cloned here, not the assets it filters --
                    // `active_project()` is borrowed again right below for the actual iteration,
                    // which is fine since both borrows are immutable.
                    let bin = app.active_smart_bin_id.and_then(|id| {
                        app.active_project()
                            .smart_bins
                            .iter()
                            .find(|b| b.id == id)
                            .cloned()
                    });
                    let search = app.media_search.to_lowercase();
                    let assets = app.active_project().media_library.iter();
                    for asset in assets.filter(|a| {
                        bin.as_ref().is_none_or(|b| b.matches(a))
                            && (search.is_empty() || a.file_name.to_lowercase().contains(&search))
                    }) {
                        let selected = app.selected_asset_id == Some(asset.id);
                        let bg = if selected {
                            theme::ACCENT.gamma_multiply(0.18)
                        } else {
                            theme::SURFACE
                        };
                        let response = egui::Frame::new()
                            .fill(bg)
                            .corner_radius(theme::RADIUS_MD)
                            .inner_margin(egui::Margin::same(6))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    asset_thumb(ui, asset);
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
                                            .corner_radius(theme::RADIUS_SM)
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
    if let Some(bin_id) = selected_bin_id {
        app.active_smart_bin_id = bin_id;
    }
    if let Some(bin_id) = edited_bin_id {
        app.begin_edit_smart_bin(bin_id);
    }
    if new_bin_clicked {
        app.begin_new_smart_bin();
    }
}

fn preview_panel(app: &mut App, ui: &mut egui::Ui, height: f32) {
    // Lazy: the pipeline for the current selection is opened here, on the first paint of this
    // panel after a selection change — not by `select_asset` itself — so opening a project or
    // launching the app never pays GStreamer's open cost for an asset the Editor screen hasn't
    // actually been shown for yet.
    app.ensure_preview_loaded();
    app.ensure_transcript_loaded_for_preview();
    let locale = app.locale;
    ui.vertical(|ui| {
        ui.set_height(height);
        let preview_texture_size = app.preview_state.preview_texture.as_ref().map(|t| t.size());
        let frame_response = egui::Frame::new()
            .fill(egui::Color32::BLACK)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.set_min_height(height - 40.0);
                match &app.preview_state.preview_texture {
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
            })
            .response;
        // A small resolution readout in the preview's top-left corner — matching
        // oca-editor-mock.html's `.preview-hud-top` chip — from the actually decoded texture's
        // own size, not a fabricated/asset-declared value, so it never drifts from what's on
        // screen (proxy playback, letterboxing, etc.).
        if let Some([w, h]) = preview_texture_size {
            let chip_pos = frame_response.rect.left_top() + egui::vec2(8.0, 8.0);
            let painter = ui.painter();
            let text_pos = chip_pos + egui::vec2(4.0, 2.0);
            let galley = painter.layout_no_wrap(
                format!("{w}×{h}"),
                egui::FontId::monospace(10.5),
                theme::TEXT_SECONDARY,
            );
            let bg_rect = egui::Rect::from_min_size(chip_pos, galley.size() + egui::vec2(8.0, 4.0));
            painter.rect_filled(
                bg_rect,
                egui::CornerRadius::same(theme::RADIUS_SM),
                egui::Color32::from_black_alpha(160),
            );
            painter.galley(text_pos, galley, theme::TEXT_SECONDARY);
        }
        let timeline_duration = app.active_project().timeline().duration_secs();
        ui.horizontal(|ui| {
            if components::icon_button(
                ui,
                "⏮",
                Text::SeekToStart.tr(locale),
                components::IconButtonOpts::default(),
            )
            .clicked()
            {
                app.seek_preview(0.0);
            }
            let play_icon = if app.preview_state.preview_playing {
                "⏸"
            } else {
                "▶"
            };
            if ui
                .button(RichText::new(play_icon).color(theme::ACCENT))
                .on_hover_text(Text::ShortcutPlayPause.tr(locale))
                .clicked()
            {
                app.toggle_preview_playback();
            }
            if components::icon_button(
                ui,
                "⏭",
                Text::SeekToEnd.tr(locale),
                components::IconButtonOpts::default(),
            )
            .clicked()
            {
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
            if ui
                .small_button(RichText::new("⛶").color(theme::TEXT_SECONDARY))
                .on_hover_text(Text::EnterFullscreenPreview.tr(locale))
                .clicked()
            {
                app.toggle_fullscreen_preview();
            }
            if ui
                .selectable_label(
                    app.preview_state.scopes_enabled,
                    RichText::new("📊").color(theme::TEXT_SECONDARY),
                )
                .on_hover_text(Text::PreviewScopesToggle.tr(locale))
                .clicked()
            {
                app.preview_state.scopes_enabled = !app.preview_state.scopes_enabled;
            }
            audio_level_meter(app, ui);
        });
        if timeline_duration > 0.0 {
            let mut position = app.active_project().timeline().playhead_secs;
            let slider =
                ui.add(egui::Slider::new(&mut position, 0.0..=timeline_duration).show_value(false));
            if slider.changed() {
                app.seek_preview(position);
            }
        }
        if app.preview_state.scopes_enabled {
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
    });
}

/// Small live peak/RMS bar for the Editor preview panel's transport row — `spec/ROADMAP.md`
/// P4 item 30. Reads [`App::current_audio_level`] every frame the panel draws; stays visually
/// flat at zero when no pipeline is open, playback is paused, or the current clip has no
/// audio, same as any other VU meter idling on silence.
fn audio_level_meter(app: &App, ui: &mut egui::Ui) {
    let level = app.current_audio_level();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(60.0, 14.0), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, theme::SURFACE_2);
    let peak = level.peak.clamp(0.0, 1.0);
    let rms = level.rms.clamp(0.0, 1.0);
    if rms > 0.0 {
        let mut rms_rect = rect;
        rms_rect.set_width(rect.width() * rms);
        painter.rect_filled(rms_rect, 2.0, theme::ACCENT);
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
    response.on_hover_text(Text::PreviewAudioLevelMeter.tr(app.locale));
}

/// Idle time (no pointer movement/click) before the fullscreen preview overlay's controls
/// start fading out — see [`App::fullscreen_controls_opacity`].
pub const FULLSCREEN_CONTROLS_IDLE_SECS: f32 = 2.5;

/// Renders the Editor preview panel as a fullscreen overlay covering the whole window —
/// called instead of the normal nav-rail/breadcrumb/screen chain from
/// `impl eframe::App for App::ui` whenever `app.preview_state.fullscreen_preview` is set, so the overlay
/// truly covers everything rather than sitting inside the Editor's three-column layout.
/// Controls (play/pause/seek/exit) fade out after [`FULLSCREEN_CONTROLS_IDLE_SECS`] of no
/// pointer activity and fade back in on the next movement or click; Esc always exits
/// regardless of control visibility.
pub fn fullscreen_preview_overlay(app: &mut App, ui: &mut egui::Ui) {
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.exit_fullscreen_preview();
        return;
    }
    let pointer_active =
        ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO || i.pointer.any_pressed());
    if pointer_active {
        app.note_fullscreen_controls_activity();
    }
    // Playback and the fade both need a steady repaint cadence, not just the throttled
    // 200ms one the rest of the app falls back to when idle.
    ui.ctx().request_repaint();

    let locale = app.locale;
    let opacity = app.fullscreen_controls_opacity();
    let texture = app.preview_state.preview_texture.clone();
    let timeline_duration = app.active_project().timeline().duration_secs();
    let playhead = app.active_project().timeline().playhead_secs;

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::BLACK))
        .show(ui, |ui| {
            let avail = ui.available_rect_before_wrap();
            if let Some(texture) = &texture {
                let tex_size = texture.size_vec2();
                let tex_aspect = tex_size.x / tex_size.y.max(1.0);
                let mut size = avail.size();
                if size.x / size.y > tex_aspect {
                    size.x = size.y * tex_aspect;
                } else {
                    size.y = size.x / tex_aspect;
                }
                let rect = egui::Rect::from_center_size(avail.center(), size);
                ui.painter().image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(RichText::new("▶").size(64.0).color(theme::TEXT_MUTED));
                });
            }

            if opacity <= 0.0 {
                return;
            }

            egui::Area::new(ui.id().with("fullscreen_exit"))
                .fixed_pos(egui::pos2(avail.right() - 56.0, avail.top() + 16.0))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    let clicked = ui
                        .add(
                            egui::Button::new(
                                RichText::new("✕")
                                    .size(16.0)
                                    .color(theme::TEXT_SECONDARY.gamma_multiply(opacity)),
                            )
                            .fill(theme::SURFACE_2.gamma_multiply(opacity)),
                        )
                        .on_hover_text(Text::ExitFullscreenPreview.tr(locale))
                        .clicked();
                    if clicked {
                        app.exit_fullscreen_preview();
                    }
                });

            egui::Area::new(ui.id().with("fullscreen_controls"))
                .fixed_pos(egui::pos2(avail.left() + 24.0, avail.bottom() - 76.0))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    ui.set_width(avail.width() - 48.0);
                    egui::Frame::new()
                        .fill(theme::SURFACE.gamma_multiply(0.85 * opacity))
                        .corner_radius(8)
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .small_button("⏮")
                                    .on_hover_text(Text::SeekToStart.tr(locale))
                                    .clicked()
                                {
                                    app.seek_preview(0.0);
                                }
                                let play_icon = if app.preview_state.preview_playing {
                                    "⏸"
                                } else {
                                    "▶"
                                };
                                if ui
                                    .button(
                                        RichText::new(play_icon)
                                            .color(theme::ACCENT.gamma_multiply(opacity)),
                                    )
                                    .on_hover_text(Text::ShortcutPlayPause.tr(locale))
                                    .clicked()
                                {
                                    app.toggle_preview_playback();
                                }
                                if ui
                                    .small_button("⏭")
                                    .on_hover_text(Text::SeekToEnd.tr(locale))
                                    .clicked()
                                {
                                    app.seek_preview(timeline_duration);
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "{} / {}",
                                        format_timecode(playhead),
                                        format_timecode(timeline_duration.max(playhead))
                                    ))
                                    .size(12.0)
                                    .color(theme::TEXT_SECONDARY.gamma_multiply(opacity))
                                    .monospace(),
                                );
                            });
                            if timeline_duration > 0.0 {
                                let mut position = playhead;
                                let slider = ui.add(
                                    egui::Slider::new(&mut position, 0.0..=timeline_duration)
                                        .show_value(false),
                                );
                                if slider.changed() {
                                    app.seek_preview(position);
                                }
                            }
                        });
                });
        });
}

/// Draws the selected clip's video inside a canvas-space box sized to the active sequence's
/// export aspect ratio (the same target dimensions export will actually use, via
/// [`avcore::ExportAspectRatio::dims_or`]) and, when its position isn't already animated
/// (`position_keyframes.len() <= 1`, the "static transform" case — see `request.md`'s Fase 4
/// "Transformação de camadas" spec), lets the user drag it to set a single position keyframe.
/// This maps directly onto `overlay=x:y`'s own coordinate space
/// (`crate::keyframe::position_overlay_xy_expr`, canvas-fraction offset from the top-left
/// default) — dragging writes exactly what export reads, no separate UI-only representation.
///
/// Layer *size* (`ClipInstance::layer_scale_x`/`_y`) is a multiplier on top of a fixed
/// stand-in baseline footprint — the video fit to the *full* canvas, preserving its own aspect
/// ratio (letterboxed/pillarboxed against `canvas_aspect` if it doesn't match) — rather than a
/// real pixel dimension: this panel doesn't know the clip's actual export-time decoded
/// resolution (the preview texture may be a lower-res editing proxy), so it can't draw the box
/// at its true composited size. `layer_scale_x`/`_y` default to `1.0`
/// ([`crate::app::LAYER_SCALE_RANGE`] is `0.1..=3.0`), so the baseline has to *be* "fills the
/// canvas" for an untouched clip to preview at its expected full size instead of shrunk before
/// the multiplier is even applied — this was previously a fixed 40%-of-canvas stand-in
/// regardless of `layer_scale`, which made every clip preview as a small box even with no
/// transform ever applied. Dragging the bottom-right handle still writes the real multiplier
/// `set_selected_clip_layer_scale` reads at export, same "editable but visually approximate"
/// shape as most of this panel. `Some(_)` in `preview_panel`'s match already guarantees
/// `app.preview_state.preview_texture` is set, but this re-checks (and bails) rather than trust that
/// invariant across the borrow-splitting clone below.
fn layer_transform_preview(app: &mut App, ui: &mut egui::Ui) {
    let Some(texture) = app.preview_state.preview_texture.clone() else {
        return;
    };
    let locale = app.locale;
    let tex_size = texture.size_vec2();
    let tex_aspect = tex_size.x / tex_size.y;

    let (canvas_w, canvas_h) = app
        .active_sequence_export_settings()
        .aspect_ratio
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

    if app.drawing_shape_points.is_some() {
        draw_custom_shape_surface(app, ui, canvas_rect, &texture);
        return;
    }

    // Fixed stand-in baseline — the video fit to fill the canvas — see this function's doc
    // comment on why `layer_scale` needs "fills the canvas" as its own baseline (1.0 = full
    // size) rather than some smaller stand-in fraction of it.
    let mut base_h = canvas_rect.height().min(canvas_rect.width());
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

    if app.motion_track_region.picking_motion_track_region {
        draw_motion_track_region_picker(app, ui, layer_rect, tex_size);
        return;
    }

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

/// Click-to-place-vertex surface for `request.md`'s Fase 4 "forma personalizada" — active
/// whenever `app.drawing_shape_points` is `Some` (see [`App::start_drawing_custom_shape`]).
/// Takes over `canvas_rect` entirely in place of [`layer_transform_preview`]'s usual layer
/// drag/resize handling for the duration of the drawing; the two modes are mutually exclusive.
///
/// Draws `texture` stretched to fill `canvas_rect` first (same "stretch to the target rect,
/// don't preserve its own aspect separately" convention [`layer_transform_preview`]'s own image
/// paint and `fullscreen_preview_overlay` already use) so there's an actual frame to trace a
/// shape over, rather than the bare canvas outline this surface used to be limited to.
///
/// Each click on `canvas_rect` appends one point in canvas-fraction coordinates (the same
/// space [`avcore::timeline::ShapeClip::center_x`]/`_y` use) via
/// [`App::push_drawing_shape_point`]. Placed points are drawn as small filled dots connected by
/// straight lines, plus a lighter closing segment back to the first point once there are
/// enough to see the shape taking form. Enter finishes (a no-op below 3 points — the drawing
/// stays active); Escape cancels outright.
fn draw_custom_shape_surface(
    app: &mut App,
    ui: &mut egui::Ui,
    canvas_rect: egui::Rect,
    texture: &egui::TextureHandle,
) {
    let locale = app.locale;
    ui.painter().image(
        texture.id(),
        canvas_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    let to_screen = |p: (f32, f32)| {
        canvas_rect.min + egui::vec2(p.0 * canvas_rect.width(), p.1 * canvas_rect.height())
    };

    let click_resp = ui.interact(
        canvas_rect,
        ui.id().with("shape_draw_surface"),
        egui::Sense::click(),
    );
    if click_resp.clicked() {
        if let Some(pos) = click_resp.interact_pointer_pos() {
            let frac_x = (pos.x - canvas_rect.min.x) / canvas_rect.width().max(1.0);
            let frac_y = (pos.y - canvas_rect.min.y) / canvas_rect.height().max(1.0);
            app.push_drawing_shape_point(frac_x, frac_y);
        }
    }
    if click_resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }

    let points = app.drawing_shape_points.clone().unwrap_or_default();
    let screen_points: Vec<egui::Pos2> = points.iter().copied().map(to_screen).collect();
    for &p in &screen_points {
        ui.painter().circle_filled(p, 4.0, theme::ACCENT);
    }
    if screen_points.len() >= 2 {
        ui.painter().add(egui::Shape::line(
            screen_points.clone(),
            egui::Stroke::new(1.5, theme::ACCENT),
        ));
    }
    if screen_points.len() >= 3 {
        ui.painter().line_segment(
            [screen_points[screen_points.len() - 1], screen_points[0]],
            egui::Stroke::new(1.0, theme::TEXT_MUTED),
        );
    }

    ui.painter().text(
        canvas_rect.center_bottom() + egui::vec2(0.0, -6.0),
        egui::Align2::CENTER_BOTTOM,
        Text::ShapeDrawHint.tr(locale),
        egui::FontId::proportional(11.0),
        theme::TEXT_SECONDARY,
    );

    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.cancel_drawing_custom_shape();
    } else if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        app.finish_drawing_custom_shape();
    }
}

/// Drag-to-select surface for the motion-tracking region — active whenever
/// `app.motion_track_region.picking_motion_track_region` is set (see [`App::start_picking_motion_track_region`]).
/// Takes over `layer_rect` (the preview's already-computed texture rect — see
/// [`layer_transform_preview`]) in place of its usual layer drag/resize handling, mutually
/// exclusive with it the same way [`draw_custom_shape_surface`] is.
///
/// `layer_rect` spans the full source frame at UV `0.0..=1.0` on both axes (however the layer
/// itself is currently positioned/scaled on an overlay track), so a fraction of `layer_rect`
/// maps directly to `motion_track_center_x`/`_y`'s own "fraction of source frame" convention.
/// `motion_track_width`/`_height` are each a fraction of the source frame's *shorter*
/// dimension (`avcore::track_region`'s convention) rather than of `layer_rect` itself, so the
/// on-screen region rect additionally scales by `layer_rect`'s per-axis stretch relative to
/// `tex_size` — `layer_rect`'s aspect only matches `tex_size`'s when `layer_scale_x` equals
/// `layer_scale_y`, otherwise the two diverge and this conversion keeps the drawn region
/// faithful to what `avcore::track_region` will actually sample.
fn draw_motion_track_region_picker(
    app: &mut App,
    ui: &mut egui::Ui,
    layer_rect: egui::Rect,
    tex_size: egui::Vec2,
) {
    let locale = app.locale;
    let short_side = tex_size.x.min(tex_size.y).max(1.0);
    let scale = egui::vec2(
        layer_rect.width() / tex_size.x.max(1.0),
        layer_rect.height() / tex_size.y.max(1.0),
    );

    let center_px = layer_rect.min
        + egui::vec2(
            app.motion_track_region.motion_track_center_x * layer_rect.width(),
            app.motion_track_region.motion_track_center_y * layer_rect.height(),
        );
    let size_px = egui::vec2(
        app.motion_track_region.motion_track_width * short_side * scale.x,
        app.motion_track_region.motion_track_height * short_side * scale.y,
    );
    let region_rect = egui::Rect::from_center_size(center_px, size_px);

    ui.painter().rect_stroke(
        region_rect,
        0,
        egui::Stroke::new(1.5, theme::ACCENT_2),
        egui::StrokeKind::Outside,
    );

    let body_resp = ui.interact(
        region_rect,
        ui.id().with("motion_track_region_body"),
        egui::Sense::drag(),
    );
    if body_resp.dragged() {
        let delta = body_resp.drag_delta();
        app.motion_track_region.motion_track_center_x =
            (app.motion_track_region.motion_track_center_x + delta.x / layer_rect.width().max(1.0))
                .clamp(0.0, 1.0);
        app.motion_track_region.motion_track_center_y =
            (app.motion_track_region.motion_track_center_y
                + delta.y / layer_rect.height().max(1.0))
            .clamp(0.0, 1.0);
    }
    if body_resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }

    const HANDLE_SIZE: f32 = 10.0;
    let handle_rect =
        egui::Rect::from_center_size(region_rect.right_bottom(), egui::Vec2::splat(HANDLE_SIZE));
    ui.painter().rect_filled(handle_rect, 2, theme::ACCENT_2);
    let resize_resp = ui.interact(
        handle_rect,
        ui.id().with("motion_track_region_resize"),
        egui::Sense::drag(),
    );
    if resize_resp.dragged() {
        let delta = resize_resp.drag_delta();
        let new_width_px = (size_px.x + delta.x).max(4.0);
        let new_height_px = (size_px.y + delta.y).max(4.0);
        app.motion_track_region.motion_track_width =
            (new_width_px / (short_side * scale.x).max(0.001)).clamp(
                *MOTION_TRACK_SIZE_RANGE.start(),
                *MOTION_TRACK_SIZE_RANGE.end(),
            );
        app.motion_track_region.motion_track_height =
            (new_height_px / (short_side * scale.y).max(0.001)).clamp(
                *MOTION_TRACK_SIZE_RANGE.start(),
                *MOTION_TRACK_SIZE_RANGE.end(),
            );
    }

    ui.painter().text(
        layer_rect.center_bottom() + egui::vec2(0.0, -6.0),
        egui::Align2::CENTER_BOTTOM,
        Text::MotionTrackRegionPickHint.tr(locale),
        egui::FontId::proportional(11.0),
        theme::TEXT_SECONDARY,
    );

    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.stop_picking_motion_track_region();
    }
}
