// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Fullscreen Program Monitor state transitions and control fading.

use super::App;

impl App {
    /// Enters or exits the fullscreen preview and resets the control fade timer.
    pub fn toggle_fullscreen_preview(&mut self) {
        self.preview_state.fullscreen_preview = !self.preview_state.fullscreen_preview;
        self.preview_state.fullscreen_controls_last_moved = None;
    }

    pub fn exit_fullscreen_preview(&mut self) {
        self.preview_state.fullscreen_preview = false;
    }

    /// Resets the fullscreen control fade timer after pointer activity.
    pub fn note_fullscreen_controls_activity(&mut self) {
        self.preview_state.fullscreen_controls_last_moved = Some(std::time::Instant::now());
    }

    /// Opacity multiplier for fullscreen controls after the configured idle interval.
    pub fn fullscreen_controls_opacity(&self) -> f32 {
        const FADE_SECS: f32 = 0.5;
        let idle_secs = self
            .preview_state
            .fullscreen_controls_last_moved
            .map(|time| time.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        let over = idle_secs - crate::screens::editor::FULLSCREEN_CONTROLS_IDLE_SECS;
        if over <= 0.0 {
            1.0
        } else {
            (1.0 - over / FADE_SECS).clamp(0.0, 1.0)
        }
    }
}
