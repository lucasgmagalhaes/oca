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

pub(crate) mod menu_bar;
mod properties_panel;
mod timeline_panel;

use avcore::media::format_timecode;
use eframe::egui::{self, RichText};

use crate::app::{
    App, EditorTool, MediaLibraryFilter, MediaViewMode, PreviewZoom, MOTION_TRACK_SIZE_RANGE,
};
use crate::components;
use crate::i18n::Text;
use crate::icons;
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

    // `bg_workspace` — the Editor is the one screen with real nested structure below the
    // app-wide `CentralPanel` (`app::mod`'s `show`, which fills with plain `BG`/`bg_canvas` for
    // every screen): a three-column body plus the timeline strip, with panel gaps/preview
    // backdrop/timeline canvas all showing through as background. Framing that whole area in
    // `BG_WORKSPACE` gives it the doc's own separate "main workspace" layer, one step lighter
    // than the outer canvas — every other screen (Home/Library/...) has no such nesting and
    // keeps sitting directly on plain `BG`.
    egui::Frame::new().fill(theme::BG_WORKSPACE).show(ui, |ui| {
        ui.vertical(|ui| {
            toolbar(app, ui);
            ui.add_space(4.0);

            let total_width = ui.available_width();
            let min_col = 160.0_f32;
            // Reported bug: the properties panel would visibly overflow past its own border, and
            // the audio-meter column at the body row's far right would vanish entirely. Root
            // cause — `max_col` (each resizable column's own independent width cap) was a flat
            // `total_width * 0.4`, with no check that *both* columns landing near that cap at
            // once still leaves room for `AUDIO_METER_COLUMN_WIDTH`, the dividers/gaps, and
            // `preview_w`'s own floor below. `lib_panel_width`/`props_panel_width` persist across
            // sessions (dragged wide once, saved), so this wasn't hypothetical — `preview_w`'s
            // `.max(MIN_PREVIEW_W)` floor could push the row's total reserved width past
            // `total_width`, and egui doesn't wrap/shrink an overflowing `ui.horizontal` to fit —
            // it just runs the last item (the audio meter) off the visible edge. Fixed the same
            // way the body/timeline height budget below already is: derive `max_col` from the
            // same fixed-cost budget (`AUDIO_METER_COLUMN_WIDTH` + `gaps` + `MIN_PREVIEW_W`)
            // `preview_w` itself is computed from, split evenly between the two resizable
            // columns, so their sum can never eat into what the meter/preview floor need.
            let gaps = ui.spacing().item_spacing.x * 3.0 + DIVIDER_HIT_WIDTH * 2.0;
            let max_col = ((total_width - AUDIO_METER_COLUMN_WIDTH - gaps - MIN_PREVIEW_W) / 2.0)
                .max(min_col);
            app.lib_panel_width = app.lib_panel_width.clamp(min_col, max_col);
            app.props_panel_width = app.props_panel_width.clamp(min_col, max_col);

            // Reported bug: the timeline strip would sometimes vanish outright. Root cause — the
            // body row's own minimum height (used to be a hardcoded 160.0 floor below) and the
            // timeline's minimum height were each enforced independently, with no check that the
            // two together actually fit `available_height`. Once the Editor's vertical space got
            // small enough (a short window, a small display, whatever), `body_height`'s floor plus
            // `timeline_height`'s floor plus the divider between them could add up to more than
            // `available_height` — egui doesn't shrink an overflowing `ui.vertical` to fit, so the
            // timeline (painted last) just got pushed past the visible/clipped area and disappeared,
            // with no error and no obvious trigger from the user's side beyond "the window got
            // short enough at some point." Fixed by deriving both heights from the same
            // `content_height` budget so `body_height + timeline_height` can never exceed what's
            // actually available, with each floor scaled down (not dropped — still visible, just
            // thinner) rather than held fixed when that budget itself is tight.
            let available_height = ui.available_height();
            let divider_overhead = 4.0 + DIVIDER_HIT_WIDTH + 4.0; // the add_space(4.0) calls flanking resizable_divider_horizontal below
            let content_height = (available_height - divider_overhead).max(0.0);
            let min_timeline = 120.0_f32.min(content_height * 0.5);
            let min_body = 160.0_f32.min(content_height - min_timeline).max(0.0);
            let max_timeline = (content_height - min_body).max(min_timeline);
            app.timeline_height = app.timeline_height.clamp(min_timeline, max_timeline);
            let body_height = (content_height - app.timeline_height).max(0.0);

            let preview_w = (total_width
                - app.lib_panel_width
                - app.props_panel_width
                - AUDIO_METER_COLUMN_WIDTH
                - gaps)
                .max(MIN_PREVIEW_W);

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
                ui.allocate_ui(egui::vec2(AUDIO_METER_COLUMN_WIDTH, body_height), |ui| {
                    audio_meter_column(app, ui, body_height);
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
    });
}

/// Fixed width of the persistent stereo-meter strip at the body row's far right edge, matching
/// the OCA mockup's own always-visible vertical L/R meter (`spec/architecture/
/// editor-ui-visual-redesign.md`'s Inspector section) — sized to fit
/// `properties_panel::stereo_db_meter`'s own label+two-bar width (26 + 16*2 + 4 = 62) plus a
/// little breathing room, not resizable like the other columns since there's nothing to resize
/// (fixed content, no scrollable/collapsible substance).
const AUDIO_METER_COLUMN_WIDTH: f32 = 70.0;

/// Floor for `preview_w` (the center preview column) — also the fixed-cost budget `max_col`
/// derives from, so the two resizable side columns can never claim so much width that this
/// floor (plus `AUDIO_METER_COLUMN_WIDTH` plus the divider/spacing gaps) no longer fits.
const MIN_PREVIEW_W: f32 = 200.0;

/// The persistent stereo dB meter at the editor body's far right edge — unlike the properties
/// panel next to it, this is *not* gated on a clip being selected or which Inspector/Effects/
/// Audio tab is active, matching the OCA mockup's own always-visible meter. Reads the same
/// [`App::current_audio_level`] the preview transport row's compact meter already does.
fn audio_meter_column(app: &App, ui: &mut egui::Ui, height: f32) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(4, 12))
        .show(ui, |ui| {
            ui.set_height(height);
            ui.vertical_centered(|ui| {
                properties_panel::stereo_db_meter(
                    ui,
                    app.current_audio_level(),
                    (height - 24.0).max(40.0),
                );
            });
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
        tool_button_icon_font(
            app,
            ui,
            EditorTool::Select,
            crate::icons::MOUSE_POINTER_2_STR,
            Text::ToolSelect.tr(locale),
        );
        let cut_job = components::icon_label_job(
            crate::icons::SCISSORS_STR,
            crate::icons::family(),
            14.0,
            theme::TEXT_PRIMARY,
            Text::ToolCut.tr(locale),
            14.0,
            theme::TEXT_PRIMARY,
        );
        if ui.button(cut_job).on_hover_text("Ctrl+B").clicked() {
            app.split_at_playhead();
        }
        // `fold-horizontal` is the mockup's resolved best-guess for the shared Trim/Ripple tool-
        // rail slot (see `spec/architecture/editor-ui-visual-redesign.md`'s Icon set table) —
        // applied to Trim only here since Ripple keeps its own distinct icon-less button below,
        // not a second, unconfirmed reuse of the same glyph.
        tool_button_icon_font(
            app,
            ui,
            EditorTool::Trim,
            crate::icons::FOLD_HORIZONTAL_STR,
            Text::ToolTrim.tr(locale),
        );
        // Confirmed via real screenshots: "⇥"/"⇄"/"⇉" AND, in a second round, even the very
        // basic "→" (used elsewhere in a LUFS tag) are all tofu against this app's bundled
        // default font. Only "↕" (Slip, below) is confirmed actually rendering — egui's default
        // font covers a curated symbol subset, not a whole Unicode block just because one glyph
        // from it happens to work, so ASCII is the only genuinely safe choice here. No vendored
        // Lucide icon exists yet for ripple/roll/slide (see nav_rail.rs's identical note).
        tool_button(
            app,
            ui,
            EditorTool::Ripple,
            ">",
            Text::ToolRipple.tr(locale),
        );
        tool_button(app, ui, EditorTool::Roll, "<>", Text::ToolRoll.tr(locale));
        tool_button(app, ui, EditorTool::Slip, "↕", Text::ToolSlip.tr(locale));
        tool_button(app, ui, EditorTool::Slide, ">>", Text::ToolSlide.tr(locale));
        tool_button_icon_font(
            app,
            ui,
            EditorTool::Hand,
            crate::icons::HAND_STR,
            Text::ToolHand.tr(locale),
        );
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
            // Section 41's Timeline Tab Bar: "Close X" per tab — previously only reachable via
            // the tab's right-click context menu (still there, unchanged). No per-sequence
            // "unsaved changes" tracking exists in this codebase to gate the spec's own "if
            // unsaved, show save/discard/cancel confirmation" — this closes directly, same as
            // the context menu's own "Excluir" already did with no such confirmation either;
            // documented here rather than silently guessed at.
            if count > 1
                && components::icon_button(
                    ui,
                    "X",
                    Text::SequenceTabCtxDelete.tr(locale),
                    components::IconButtonOpts::default(),
                )
                .clicked()
            {
                delete_request = Some((sequence_id, name.clone()));
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

/// Same active/inactive chip styling as [`tool_button`], for a tool that has a real vendored
/// Lucide icon (see `spec/architecture/editor-ui-visual-redesign.md`'s Icon set section) instead
/// of a plain-text/unicode glyph — `icon` is one of `crate::icons`' `_STR` constants, rendered
/// through the icon font via a [`components::icon_label_job`] `LayoutJob` since `RichText` can't
/// mix two fonts in one string.
fn tool_button_icon_font(
    app: &mut App,
    ui: &mut egui::Ui,
    tool: EditorTool,
    icon: &str,
    label: &str,
) {
    let active = app.tool == tool;
    let color = if active {
        theme::ACCENT
    } else {
        theme::TEXT_SECONDARY
    };
    let job = components::icon_label_job(
        icon,
        crate::icons::family(),
        14.0,
        color,
        label,
        14.0,
        color,
    );
    let button = egui::Button::new(job)
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

/// A real decoded poster frame when one's cached, falling back to a small colored placeholder
/// (matching the media-library asset row from `oca-editor-mock.html`'s `.asset-thumb`) —
/// either way, with a duration badge in the bottom-right corner. Video and audio assets get
/// distinct placeholder fills so the kind reads at a glance while no frame has decoded yet
/// (audio never gets a real frame at all — [`App::request_thumbnail`] is only ever asked for
/// video assets, matching every other caller of the same poster-frame pipeline: `screens::
/// library`, `screens::home`). Extraction/caching reuses `App::thumbnail_state`'s existing
/// timeline-filmstrip pipeline at frame 0 rather than inventing a second one — see
/// [`media_library_panel`]'s own request/touch bookkeeping.
/// Shared glyph size for every icon in the preview transport row (skip/step/loop/camera/marker)
/// — see the row's own comment for why this matters: without it, the two plain-Unicode glyphs
/// (no vendored Lucide icon exists for single-frame step or "add marker") fall back to their
/// font's own default metrics and read as a visibly different icon style from the Lucide glyphs
/// around them.
const TRANSPORT_ICON_SIZE: f32 = 14.0;
/// The play/pause button is the row's primary action (matches the OCA mockup's own slightly
/// larger, accent-colored play glyph) — deliberately bigger than [`TRANSPORT_ICON_SIZE`].
const TRANSPORT_PLAY_ICON_SIZE: f32 = 18.0;

const ASSET_THUMB_SIZE: egui::Vec2 = egui::vec2(48.0, 28.0);
/// Thumbnail size for [`MediaViewMode::Grid`]'s tiles — bigger than [`ASSET_THUMB_SIZE`]'s list
/// rows since a grid tile has no adjacent filename/metadata column competing for width.
const GRID_ASSET_THUMB_SIZE: egui::Vec2 = egui::vec2(120.0, 72.0);

fn asset_thumb(
    ui: &mut egui::Ui,
    asset: &avcore::media::MediaAsset,
    thumbnail: Option<&egui::TextureHandle>,
) {
    asset_thumb_sized(ui, asset, ASSET_THUMB_SIZE, thumbnail);
}

fn asset_thumb_sized(
    ui: &mut egui::Ui,
    asset: &avcore::media::MediaAsset,
    size: egui::Vec2,
    thumbnail: Option<&egui::TextureHandle>,
) {
    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter_at(rect);
    match thumbnail {
        Some(texture) => {
            // The image fully covers `rect` below, so the rounded fill underneath never
            // actually shows through -- kept anyway so a texture with any transparency (none
            // today, but poster frames are plain RGBA) doesn't reveal square corners.
            painter.rect_filled(
                rect,
                egui::CornerRadius::same(theme::RADIUS_SM),
                theme::SURFACE_2,
            );
            painter.image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        None => {
            let (fill, glyph) = match asset.kind {
                avcore::media::MediaKind::Video => (theme::SURFACE_2, "▶"),
                avcore::media::MediaKind::Audio => (theme::ACCENT_2.gamma_multiply(0.25), "♪"),
            };
            painter.rect_filled(rect, egui::CornerRadius::same(theme::RADIUS_SM), fill);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                glyph,
                egui::FontId::proportional(11.0),
                theme::TEXT_MUTED,
            );
        }
    }
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

/// One tab-style filter chip in the media library's header row — the OCA mockup's "MEDIA / Bins
/// / Favorites / Recent" tab strip, not the plain `selectable_label` pill this used to be.
/// Reuses `properties_panel::properties_tab_bar`'s own accent-tint-fill/accent-stroke-when-active
/// convention (this codebase's one established "tab" look) instead of inventing a second one.
/// Returns whether it was clicked.
fn media_filter_tab(ui: &mut egui::Ui, active: bool, label: &str) -> bool {
    let text = RichText::new(label).color(if active {
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
    ui.add(button).clicked()
}

fn media_library_panel(app: &mut App, ui: &mut egui::Ui, width: f32, height: f32) {
    let mut clicked_id = None;
    let mut add_to_timeline_id = None;
    let mut dropped_asset = None;
    let mut selected_filter = None;
    let mut edited_bin_id = None;
    let mut new_bin_clicked = false;
    let mut toggled_favorite_id = None;
    // Collected during the asset-list closure below, applied after it ends -- same
    // "collect-during-loop, apply-after" shape `screens::library`/`timeline_panel` use for
    // their own poster-frame requests, needed here because `App::request_thumbnail`/
    // `App::touch_thumbnails` take `&mut self` while the loop below iterates a shared borrow
    // of `app.active_project().media_library`.
    let mut thumbnail_requests: Vec<u64> = Vec::new();
    let mut thumbnail_touches: Vec<(u64, u64, i64)> = Vec::new();
    let project_id = app.active_project().id;

    components::panel_frame().show(ui, |ui| {
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
                    ui.add_space(theme::SPACE_SM);
                    if ui
                        .selectable_label(app.media_view_mode == MediaViewMode::Grid, "▦")
                        .on_hover_text(Text::MediaViewGrid.tr(app.locale))
                        .clicked()
                    {
                        app.media_view_mode = MediaViewMode::Grid;
                    }
                    if ui
                        .selectable_label(app.media_view_mode == MediaViewMode::List, "☰")
                        .on_hover_text(Text::MediaViewList.tr(app.locale))
                        .clicked()
                    {
                        app.media_view_mode = MediaViewMode::List;
                    }
                });
            });
            ui.add_space(theme::SPACE_SM);
            ui.add(
                egui::TextEdit::singleline(&mut app.media_search)
                    .hint_text(Text::SearchMediaPlaceholder.tr(app.locale))
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(4.0);
            // Smart bins (P4 item 22) plus the Favorites/Recent filters -- a row of filter
            // chips above the asset list, single-selection (see MediaLibraryFilter's own
            // doc comment). "All" clears the filter; each bin is click-to-select,
            // double-click-to-edit (the rules, not the assets themselves -- there's nothing
            // else to double-click a filter chip for).
            ui.horizontal_wrapped(|ui| {
                if media_filter_tab(
                    ui,
                    app.media_filter == MediaLibraryFilter::All,
                    Text::SmartBinAll.tr(app.locale),
                ) {
                    selected_filter = Some(MediaLibraryFilter::All);
                }
                if media_filter_tab(
                    ui,
                    app.media_filter == MediaLibraryFilter::Favorites,
                    Text::MediaFilterFavorites.tr(app.locale),
                ) {
                    selected_filter = Some(MediaLibraryFilter::Favorites);
                }
                if media_filter_tab(
                    ui,
                    app.media_filter == MediaLibraryFilter::Recent,
                    Text::MediaFilterRecent.tr(app.locale),
                ) {
                    selected_filter = Some(MediaLibraryFilter::Recent);
                }
                for bin in &app.active_project().smart_bins {
                    let response = ui.selectable_label(
                        app.media_filter == MediaLibraryFilter::SmartBin(bin.id),
                        &bin.name,
                    );
                    if response.clicked() {
                        selected_filter = Some(MediaLibraryFilter::SmartBin(bin.id));
                    }
                    if response.double_clicked() {
                        edited_bin_id = Some(bin.id);
                    }
                }
                if ui.button(Text::SmartBinNew.tr(app.locale)).clicked() {
                    new_bin_clicked = true;
                }
            });
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .id_salt("media_library_scroll")
                .show(ui, |ui| {
                    // Only the (small) bin rule is cloned here, not the assets it filters --
                    // `active_project()` is borrowed again right below for the actual iteration,
                    // which is fine since both borrows are immutable.
                    let bin = match app.media_filter {
                        MediaLibraryFilter::SmartBin(id) => app
                            .active_project()
                            .smart_bins
                            .iter()
                            .find(|b| b.id == id)
                            .cloned(),
                        _ => None,
                    };
                    let recent_asset_ids = app.active_project().recent_asset_ids.clone();
                    let media_filter = app.media_filter;
                    let search = app.media_search.to_lowercase();
                    let mut assets: Vec<_> = app
                        .active_project()
                        .media_library
                        .iter()
                        .filter(|a| {
                            let matches_filter = match media_filter {
                                MediaLibraryFilter::All => true,
                                MediaLibraryFilter::SmartBin(_) => {
                                    bin.as_ref().is_none_or(|b| b.matches(a))
                                }
                                MediaLibraryFilter::Favorites => a.favorited,
                                MediaLibraryFilter::Recent => recent_asset_ids.contains(&a.id),
                            };
                            matches_filter
                                && (search.is_empty()
                                    || a.file_name.to_lowercase().contains(&search))
                        })
                        .collect();
                    // Recent is most-recently-used-first, not the library's own insertion
                    // order -- everything else keeps that default order unchanged.
                    if media_filter == MediaLibraryFilter::Recent {
                        assets.sort_by_key(|a| {
                            recent_asset_ids
                                .iter()
                                .position(|&id| id == a.id)
                                .unwrap_or(usize::MAX)
                        });
                    }

                    // Shared across both layouts below: every asset's click/double-click/drag/
                    // drop behavior is identical, only the Frame's own content (list row vs.
                    // grid tile) differs.
                    let mut handle_interaction =
                        |ui: &egui::Ui,
                         asset: &avcore::media::MediaAsset,
                         response: egui::Response| {
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
                        };

                    // Same poster-frame lookup for both layouts below: a cached texture is
                    // touched (keeps it alive in the LRU), a missing one for a video asset is
                    // queued for extraction -- both applied after this closure returns, since
                    // `App::touch_thumbnails`/`App::request_thumbnail` need `&mut app`.
                    let mut resolve_thumbnail = |asset: &avcore::media::MediaAsset| {
                        if asset.kind != avcore::media::MediaKind::Video {
                            return None;
                        }
                        let key = (project_id, asset.id, 0);
                        match app.thumbnail_state.thumbnail_textures.get(&key) {
                            Some(texture) => {
                                thumbnail_touches.push(key);
                                Some(texture)
                            }
                            None => {
                                thumbnail_requests.push(asset.id);
                                None
                            }
                        }
                    };

                    match app.media_view_mode {
                        MediaViewMode::List => {
                            for asset in assets.iter().copied() {
                                let selected = app.selected_asset_id == Some(asset.id);
                                let bg = if selected {
                                    theme::ACCENT.gamma_multiply(0.18)
                                } else {
                                    theme::SURFACE
                                };
                                let thumbnail = resolve_thumbnail(asset);
                                let response = egui::Frame::new()
                                    .fill(bg)
                                    .corner_radius(theme::RADIUS_MD)
                                    .inner_margin(egui::Margin::same(4))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            asset_thumb(ui, asset, thumbnail);
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        RichText::new(&asset.file_name).size(12.0),
                                                    );
                                                    if components::icon_button(
                                                        ui,
                                                        icons::STAR_STR,
                                                        Text::ToggleFavorite.tr(app.locale),
                                                        components::IconButtonOpts {
                                                            family: Some(icons::family()),
                                                            color: Some(if asset.favorited {
                                                                theme::ACCENT
                                                            } else {
                                                                theme::TEXT_MUTED
                                                            }),
                                                            ..Default::default()
                                                        },
                                                    )
                                                    .clicked()
                                                    {
                                                        toggled_favorite_id = Some(asset.id);
                                                    }
                                                });
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
                                handle_interaction(ui, asset, response);
                                ui.add_space(4.0);
                            }
                        }
                        MediaViewMode::Grid => {
                            ui.horizontal_wrapped(|ui| {
                                for asset in assets.iter().copied() {
                                    let selected = app.selected_asset_id == Some(asset.id);
                                    let bg = if selected {
                                        theme::ACCENT.gamma_multiply(0.18)
                                    } else {
                                        theme::SURFACE
                                    };
                                    let thumbnail = resolve_thumbnail(asset);
                                    let response = egui::Frame::new()
                                        .fill(bg)
                                        .corner_radius(theme::RADIUS_MD)
                                        .inner_margin(egui::Margin::same(4))
                                        .show(ui, |ui| {
                                            ui.set_max_width(GRID_ASSET_THUMB_SIZE.x);
                                            ui.vertical(|ui| {
                                                asset_thumb_sized(
                                                    ui,
                                                    asset,
                                                    GRID_ASSET_THUMB_SIZE,
                                                    thumbnail,
                                                );
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        RichText::new(&asset.file_name)
                                                            .size(10.0)
                                                            .color(theme::TEXT_PRIMARY),
                                                    );
                                                    if components::icon_button(
                                                        ui,
                                                        icons::STAR_STR,
                                                        Text::ToggleFavorite.tr(app.locale),
                                                        components::IconButtonOpts {
                                                            family: Some(icons::family()),
                                                            color: Some(if asset.favorited {
                                                                theme::ACCENT
                                                            } else {
                                                                theme::TEXT_MUTED
                                                            }),
                                                            ..Default::default()
                                                        },
                                                    )
                                                    .clicked()
                                                    {
                                                        toggled_favorite_id = Some(asset.id);
                                                    }
                                                });
                                            });
                                        })
                                        .response
                                        .interact(egui::Sense::click_and_drag());
                                    handle_interaction(ui, asset, response);
                                }
                            });
                        }
                    }
                });
        });
    });

    app.touch_thumbnails(&thumbnail_touches);
    for asset_id in thumbnail_requests {
        app.request_thumbnail(project_id, asset_id, 0);
    }

    if let Some(id) = clicked_id {
        app.select_asset(Some(id));
    }
    if let Some(dropped) = dropped_asset {
        app.pending_asset_drop = Some(dropped);
    }
    if let Some(id) = add_to_timeline_id {
        app.add_asset_to_timeline(id);
    }
    if let Some(filter) = selected_filter {
        app.media_filter = filter;
    }
    if let Some(bin_id) = edited_bin_id {
        app.begin_edit_smart_bin(bin_id);
    }
    if new_bin_clicked {
        app.begin_new_smart_bin();
    }
    if let Some(asset_id) = toggled_favorite_id {
        app.toggle_asset_favorite(asset_id);
    }
}

/// A small dark rounded chip with monospace text, anchored by its top-left corner — the
/// preview panel's HUD overlay style (resolution/fps top-left, timecode/frame bottom-right).
fn draw_preview_hud_chip(painter: &egui::Painter, top_left: egui::Pos2, text: &str) {
    let text_pos = top_left + egui::vec2(4.0, 2.0);
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        egui::FontId::monospace(10.5),
        theme::TEXT_SECONDARY,
    );
    let bg_rect = egui::Rect::from_min_size(top_left, galley.size() + egui::vec2(8.0, 4.0));
    painter.rect_filled(
        bg_rect,
        egui::CornerRadius::same(theme::RADIUS_SM),
        egui::Color32::from_black_alpha(160),
    );
    painter.galley(text_pos, galley, theme::TEXT_SECONDARY);
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
        ui.add_space(theme::SPACE_SM);
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
                                    .color(theme::TEXT_DISABLED),
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
        // A small resolution(+fps) readout in the preview's bottom-left corner — matching the
        // Program Monitor mockup exactly (confirmed via a real screenshot of it: bottom-left =
        // resolution/fps, top-left = CAM, bottom-right = TC/frame) — from the actually decoded
        // texture's own size, not a fabricated/asset-declared value, so it never drifts from
        // what's on screen (proxy playback, letterboxing, etc.). fps comes from the previewed
        // clip's own asset (oca has no per-sequence fps) and is omitted when unknown.
        if let Some([w, h]) = preview_texture_size {
            let text = match app.current_preview_fps() {
                Some(fps) => format!("{w}×{h} · {fps:.2}fps"),
                None => format!("{w}×{h}"),
            };
            let size = ui
                .painter()
                .layout_no_wrap(
                    text.clone(),
                    egui::FontId::monospace(10.5),
                    theme::TEXT_SECONDARY,
                )
                .size()
                + egui::vec2(8.0, 4.0);
            let top_left = frame_response.rect.left_bottom() - egui::vec2(-8.0, 8.0 + size.y);
            draw_preview_hud_chip(ui.painter(), top_left, &text);
        }
        // "CAM 01" chip in the top-left corner, matching the mockup. Top-right is the mockup's
        // "REC ●" chip — a documented non-goal, since oca has no live-recording concept to
        // honestly wire it to (not a placeholder for a feature that doesn't exist). Wired to
        // real multicam-group data: only drawn when the previewed track is actually a multicam
        // group's program track, never faked when it isn't.
        if let Some(angle) = app.current_preview_multicam_angle() {
            let text = format!("CAM {angle:02}");
            draw_preview_hud_chip(
                ui.painter(),
                frame_response.rect.left_top() + egui::vec2(8.0, 8.0),
                &text,
            );
        }
        // Timecode+frame overlay in the preview's bottom-right corner, matching the mockup's
        // bottom-right overlay — a relocation of data already shown in the transport row's
        // timecode label below, plus a frame-within-second suffix when fps is known (omitted
        // otherwise rather than guessed).
        if preview_texture_size.is_some() {
            let playhead = app.active_project().timeline().playhead_secs;
            let mut text = format_timecode(playhead);
            if let Some(fps) = app.current_preview_fps() {
                let frame_count = fps.round().max(1.0) as i64;
                let frame =
                    ((playhead.fract() * fps as f64).round() as i64).clamp(0, frame_count - 1);
                text.push_str(&format!(":{frame:02}"));
            }
            // Measure first (this chip is bottom-right-anchored, unlike the top-left one above)
            // so its top-left corner can be derived from the frame's bottom-right corner.
            let size = ui
                .painter()
                .layout_no_wrap(
                    text.clone(),
                    egui::FontId::monospace(10.5),
                    theme::TEXT_SECONDARY,
                )
                .size()
                + egui::vec2(8.0, 4.0);
            let top_left = frame_response.rect.right_bottom() - egui::vec2(8.0, 8.0) - size;
            draw_preview_hud_chip(ui.painter(), top_left, &text);
        }
        let timeline_duration = app.active_project().timeline().duration_secs();
        // Restructured to match the Program Monitor mockup's own two-row shape (confirmed via
        // a real screenshot of it): a scrubber row with the current/total timecode flanking the
        // slider, THEN a separate, centered transport-controls row — previously one single
        // left-aligned row with the (small, inline) timecode text mixed in among the icons.
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
        ui.add_space(theme::SPACE_XS);
        ui.columns(3, |columns| {
            // Left column intentionally empty — its equal share of the row is what centers the
            // middle column's transport controls (`ui.columns` splits width evenly into thirds).
            columns[1].vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    transport_controls(app, ui, locale, timeline_duration);
                });
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
                // "🔹" (emoji-presentation, same broken class as nav_rail's "🧹") replaced with
                // plain ASCII — no vendored Lucide marker icon exists yet either.
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

/// The Program Monitor's centered transport-control cluster (skip-back, step-back, play/pause,
/// step-forward, skip-forward, loop) — split out of `preview_panel` so the 3-column centering
/// trick (`ui.columns(3, ...)`) there can call it just for the middle column.
fn transport_controls(
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
fn audio_level_meter(app: &App, ui: &mut egui::Ui) {
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
                        .corner_radius(theme::RADIUS_MD)
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .small_button(
                                        RichText::new(crate::icons::SKIP_BACK_STR)
                                            .family(crate::icons::family()),
                                    )
                                    .on_hover_text(Text::SeekToStart.tr(locale))
                                    .clicked()
                                {
                                    app.seek_preview(0.0);
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
                                            .color(theme::ACCENT.gamma_multiply(opacity)),
                                    )
                                    .on_hover_text(Text::ShortcutPlayPause.tr(locale))
                                    .clicked()
                                {
                                    app.toggle_preview_playback();
                                }
                                if ui
                                    .small_button(
                                        RichText::new(crate::icons::SKIP_FORWARD_STR)
                                            .family(crate::icons::family()),
                                    )
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

/// Pure size math for [`PreviewZoom`] — extracted out of `layer_transform_preview` so it's
/// testable without an `egui::Ui`. `Fit` always fills `avail`; `Percent50`/`Percent100` size
/// off the sequence's own real pixel dimensions instead, then every branch is clamped back down
/// to `avail` and aspect-corrected against `canvas_aspect` the same way — this preview has no
/// scrollable viewport (see `PreviewZoom`'s own doc comment), so an oversized zoom silently caps
/// back to whatever `Fit` would have produced rather than overflowing the panel.
fn preview_canvas_size(
    zoom: PreviewZoom,
    avail: egui::Vec2,
    canvas_w: u32,
    canvas_h: u32,
    canvas_aspect: f32,
) -> egui::Vec2 {
    let mut size = match zoom {
        PreviewZoom::Fit => avail,
        PreviewZoom::Percent50 => egui::vec2(canvas_w as f32 * 0.5, canvas_h as f32 * 0.5),
        PreviewZoom::Percent100 => egui::vec2(canvas_w as f32, canvas_h as f32),
    };
    size.x = size.x.min(avail.x);
    size.y = size.y.min(avail.y);
    if size.x / size.y > canvas_aspect {
        size.x = size.y * canvas_aspect;
    } else {
        size.y = size.x / canvas_aspect;
    }
    size
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
    // The layer-position drag below had no bound on how far the layer could be dragged, and
    // nothing clipped its paint to this panel — confirmed via a real report: drag far enough
    // and the layer image paints straight over the media library/properties panels next door.
    // A clipped painter (scoped to this panel's own rect) fixes the *visual* leak regardless of
    // how far the underlying position value goes; the drag clamp below additionally keeps the
    // value itself from wandering off to where the layer becomes unreachable to drag back.
    let clip_rect = ui.max_rect();
    let painter = ui.painter().with_clip_rect(clip_rect);
    let canvas_size = preview_canvas_size(
        app.preview_state.zoom,
        avail_rect.size(),
        canvas_w,
        canvas_h,
        canvas_aspect,
    );
    let canvas_rect = egui::Rect::from_center_size(avail_rect.center(), canvas_size);
    ui.allocate_rect(canvas_rect, egui::Sense::hover());
    painter.rect_stroke(
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

    painter.image(
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
    painter.rect_stroke(
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
            // Clamped to one full canvas width/height of overflow on either side (generous
            // enough for a slide-in/out animation's endpoints) — previously unbounded, so a
            // fast/long drag could push the layer's position value far enough that it became
            // impossible to find/drag back at all, on top of the paint-clipping fix above.
            let new_pos = avcore::Position {
                x: (current.x + delta.x / canvas_rect.width().max(1.0)).clamp(-1.0, 2.0),
                y: (current.y + delta.y / canvas_rect.height().max(1.0)).clamp(-1.0, 2.0),
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
        painter.text(
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
        painter.rect_filled(handle_rect, 2, theme::ACCENT_2);
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

#[cfg(test)]
mod mod_test;
