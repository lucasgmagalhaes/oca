// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Read-only status overlays for the editor program monitor.

use eframe::egui;

use crate::app::App;
use crate::theme;

use super::format_timecode;

fn draw_chip(painter: &egui::Painter, top_left: egui::Pos2, text: &str) {
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

/// Draws monitor metadata derived from the decoded texture and active timeline state.
pub(super) fn draw_preview_hud(
    app: &App,
    painter: &egui::Painter,
    frame_rect: egui::Rect,
    texture_size: Option<[usize; 2]>,
) {
    if let Some([w, h]) = texture_size {
        let text = match app.current_preview_fps() {
            Some(fps) => format!("{w}x{h} | {fps:.2}fps"),
            None => format!("{w}x{h}"),
        };
        let size = painter
            .layout_no_wrap(
                text.clone(),
                egui::FontId::monospace(10.5),
                theme::TEXT_SECONDARY,
            )
            .size()
            + egui::vec2(8.0, 4.0);
        let top_left = frame_rect.left_bottom() - egui::vec2(-8.0, 8.0 + size.y);
        draw_chip(painter, top_left, &text);
    }

    if let Some(angle) = app.current_preview_multicam_angle() {
        draw_chip(
            painter,
            frame_rect.left_top() + egui::vec2(8.0, 8.0),
            &format!("CAM {angle:02}"),
        );
    }

    if texture_size.is_some() {
        let playhead = app.active_project().timeline().playhead_secs;
        let mut text = format_timecode(playhead);
        if let Some(fps) = app.current_preview_fps() {
            let frame_count = fps.round().max(1.0) as i64;
            let frame = ((playhead.fract() * fps as f64).round() as i64).clamp(0, frame_count - 1);
            text.push_str(&format!(":{frame:02}"));
        }
        let size = painter
            .layout_no_wrap(
                text.clone(),
                egui::FontId::monospace(10.5),
                theme::TEXT_SECONDARY,
            )
            .size()
            + egui::vec2(8.0, 4.0);
        let top_left = frame_rect.right_bottom() - egui::vec2(8.0, 8.0) - size;
        draw_chip(painter, top_left, &text);
    }
}
