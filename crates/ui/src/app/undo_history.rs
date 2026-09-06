// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Timeline undo and redo history management.

use super::App;

impl App {
    pub(crate) fn push_undo_snapshot(&mut self) {
        let sequence = self.active_project().active_sequence().clone();
        self.undo_stack.push(sequence);
    }

    pub(crate) fn push_undo_snapshot_for_drag(&mut self) {
        if !self.undo_drag_active {
            self.push_undo_snapshot();
            self.undo_drag_active = true;
        }
    }

    pub(crate) fn end_undo_drag_tracking_if_pointer_released(&mut self, pointer_down: bool) {
        if !pointer_down {
            self.undo_drag_active = false;
        }
    }

    pub fn can_undo(&self) -> bool {
        self.undo_stack.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.undo_stack.can_redo()
    }

    fn restore_history_sequence(&mut self, sequence: avcore::Sequence) {
        *self.active_project_mut().active_sequence_mut() = sequence;
        self.selected_clip_id = None;
        self.selected_text_clip_id = None;
        self.selected_shape_clip_id = None;
        self.multi_selected_clip_ids.clear();
        self.preview_state.preview_playing = false;
        self.preview_state.preview_frozen_since = None;
        self.invalidate_preview_rendering();
    }

    pub fn undo(&mut self) {
        let current = self.active_project().active_sequence().clone();
        let Some(previous) = self.undo_stack.undo(current) else {
            return;
        };
        self.restore_history_sequence(previous);
    }

    pub fn redo(&mut self) {
        let current = self.active_project().active_sequence().clone();
        let Some(next) = self.undo_stack.redo(current) else {
            return;
        };
        self.restore_history_sequence(next);
    }
}
