// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Saving the frame currently displayed by the Program Monitor.

use super::App;
use crate::i18n::Text;

impl App {
    /// Writes the latest displayed frame as PNG, reporting a toast for every outcome.
    pub fn save_preview_snapshot(&mut self, output_path: std::path::PathBuf) {
        let Some(frame) = &self.preview_state.last_frame else {
            self.push_toast(Text::SnapshotNoFrame.tr(self.locale).to_string());
            return;
        };
        match frame.save_png(&output_path) {
            Ok(()) => self.push_toast(Text::SnapshotSaved.tr(self.locale).to_string()),
            Err(error) => self.push_toast(format!("Failed to save snapshot: {error}")),
        }
    }
}
