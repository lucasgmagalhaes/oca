// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Layout and fixed presentation values for the editor timeline.

use eframe::egui;

use crate::app::EditorTool;

/// Bounds for the timeline zoom level.
pub(super) const MIN_PX_PER_SEC: f32 = 0.5;
pub(super) const MAX_PX_PER_SEC: f32 = 60.0;

/// Delay before a settled zoom may request new filmstrip thumbnails.
pub(super) const THUMBNAIL_ZOOM_DEBOUNCE: std::time::Duration =
    std::time::Duration::from_millis(150);

pub(super) const TRACK_LABEL_WIDTH: f32 = 86.0;
pub(super) const RULER_HEIGHT: f32 = 28.0;
pub(super) const TRACK_ROW_HEIGHT: f32 = 56.0;
pub(super) const COLLAPSED_TRACK_ROW_HEIGHT: f32 = 22.0;

/// Fixed color-label swatches offered by the clip and track context menus.
pub(crate) const CLIP_COLOR_LABEL_PALETTE: &[[u8; 3]] = &[
    [229, 83, 83],
    [230, 145, 56],
    [230, 200, 56],
    [96, 189, 104],
    [86, 156, 214],
    [178, 108, 219],
];

/// Frame-local geometry and interaction settings shared by the timeline canvases.
pub(super) struct TimelineCanvasLayout {
    pub(super) content_width: f32,
    pub(super) hscroll_source: egui::containers::scroll_area::ScrollSource,
    pub(super) hand_active: bool,
}

impl TimelineCanvasLayout {
    pub(super) fn new(
        available_width: f32,
        px_per_sec: f32,
        duration_secs: f64,
        tool: EditorTool,
    ) -> Self {
        let hand_active = tool == EditorTool::Hand;
        let visible_width = (available_width - TRACK_LABEL_WIDTH).max(0.0);
        // Keep a small region after the last clip available for panning.
        let content_width = (duration_secs as f32 * px_per_sec + 200.0).max(visible_width);
        let hscroll_source = egui::containers::scroll_area::ScrollSource {
            drag: if hand_active {
                egui::containers::scroll_area::DragScroll::Always
            } else {
                egui::containers::scroll_area::DragScroll::OnTouch
            },
            ..Default::default()
        };

        Self {
            content_width,
            hscroll_source,
            hand_active,
        }
    }

    /// Suppresses clip interactions while the Hand tool owns drag-to-pan.
    pub(super) fn canvas_sense(&self, normal: egui::Sense) -> egui::Sense {
        if self.hand_active {
            egui::Sense::hover()
        } else {
            normal
        }
    }
}
