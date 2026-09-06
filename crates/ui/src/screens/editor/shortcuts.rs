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

use eframe::egui;

use crate::app::{App, EditorTool};

use super::save_active_project;

/// Applies editor keyboard shortcuts before rendering interactive controls.
pub(super) fn handle_editor_shortcuts(app: &mut App, ui: &mut egui::Ui) {
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
    // Section 3/4/6's own tool shortcuts (V/C/T) — bare letter keys, so gated on
    // `!wants_keyboard_input()` and "no modifier held" or they'd misfire while typing in any
    // text field (renaming a clip, a text graphic's own content, media search, ...) and would
    // double-fire alongside modifier combos that happen to share the same letter (Ctrl+C copy).
    if !ui.ctx().egui_wants_keyboard_input() && !ui.input(|i| i.modifiers.ctrl || i.modifiers.alt) {
        if ui.input(|i| i.key_pressed(egui::Key::V)) {
            app.tool = EditorTool::Select;
        }
        if ui.input(|i| i.key_pressed(egui::Key::C)) {
            app.tool = EditorTool::Razor;
        }
        if ui.input(|i| i.key_pressed(egui::Key::T)) {
            app.tool = EditorTool::Text;
        }
    }
    // Text Tool, Section 6's "empty text" rule: Escape before typing anything discards the
    // graphic the tool just created; Escape after real content is typed leaves it (checked
    // against the clip's *current* text, not a stale flag, so this stays correct regardless of
    // what else happened in between — no separate invalidation bookkeeping needed).
    if let Some(pending_id) = app.text_tool_pending_empty_clip_id.take() {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            let is_empty = app
                .active_project()
                .timeline()
                .tracks
                .iter()
                .flat_map(|t| &t.text_clips)
                .find(|c| c.id == pending_id)
                .is_some_and(|c| c.text.trim().is_empty());
            if is_empty {
                let timeline = app.active_project_mut().timeline_mut();
                for track in &mut timeline.tracks {
                    track.text_clips.retain(|c| c.id != pending_id);
                }
                if app.selected_text_clip_id == Some(pending_id) {
                    app.selected_text_clip_id = None;
                }
            }
        } else {
            // Not consumed this frame — put it back so a later frame's Escape still sees it.
            app.text_tool_pending_empty_clip_id = Some(pending_id);
        }
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
}
