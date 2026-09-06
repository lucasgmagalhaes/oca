use eframe::egui;

use crate::i18n::{Locale, Text};
use crate::theme;

use super::TrimEdge;

fn canvas_sense(hand_active: bool, normal: egui::Sense) -> egui::Sense {
    if hand_active {
        egui::Sense::hover()
    } else {
        normal
    }
}

pub(super) struct TextOverlayRequests<'a> {
    pub(super) clicked: &'a mut Option<u64>,
    pub(super) deletes: &'a mut Vec<u64>,
    pub(super) drags: &'a mut Vec<(u64, f64)>,
    pub(super) trims: &'a mut Vec<(u64, TrimEdge)>,
    pub(super) drag_started: &'a mut bool,
}

pub(super) fn draw_text_overlays(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    track: &avcore::timeline::Track,
    track_rect: egui::Rect,
    px_per_sec: f32,
    locale: Locale,
    hand_active: bool,
    selected_text_clip_id: Option<u64>,
    requests: TextOverlayRequests<'_>,
) {
    if track.kind != avcore::timeline::TrackKind::Text {
        return;
    }

    for tc in &track.text_clips {
        let x = track_rect.left() + tc.start_secs as f32 * px_per_sec;
        let tc_widget_id = ui.id().with(("timeline_text_clip", tc.id));
        let tc_trim_start_id = ui.id().with(("timeline_text_clip_trim_start", tc.id));
        let tc_trim_end_id = ui.id().with(("timeline_text_clip_trim_end", tc.id));
        let tc_being_dragged = ui
            .ctx()
            .dragged_id()
            .is_some_and(|id| id == tc_widget_id || id == tc_trim_start_id || id == tc_trim_end_id);
        // Same viewport-culling reasoning as the video/audio clip loop above.
        if x > track_rect.right() && !tc_being_dragged {
            continue;
        }
        let w = (tc.duration_secs as f32 * px_per_sec).max(3.0);
        let tc_rect = egui::Rect::from_min_size(
            egui::pos2(x, track_rect.top()),
            egui::vec2(w, track_rect.height()),
        );
        // Narrow edge strips for drag-to-trim, same layout as the
        // video/audio clip loop above.
        let edge_w = (w / 3.0).clamp(2.0, 6.0);
        let tc_left_edge_rect =
            egui::Rect::from_min_size(tc_rect.min, egui::vec2(edge_w, tc_rect.height()));
        let tc_right_edge_rect = egui::Rect::from_min_size(
            egui::pos2(tc_rect.right() - edge_w, tc_rect.top()),
            egui::vec2(edge_w, tc_rect.height()),
        );
        let tc_response = ui.interact(
            tc_rect,
            tc_widget_id,
            if track.locked {
                canvas_sense(hand_active, egui::Sense::click())
            } else {
                canvas_sense(hand_active, egui::Sense::click_and_drag())
            },
        );
        tc_response.context_menu(|ui| {
            if ui.button(Text::ContextMenuDelete.tr(locale)).clicked() {
                requests.deletes.push(tc.id);
                ui.close();
            }
        });
        if tc_response.clicked() {
            *requests.clicked = Some(tc.id);
        }
        if tc_response.drag_started() {
            *requests.drag_started = true;
        }
        if tc_response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            let delta_secs = (tc_response.drag_delta().x / px_per_sec) as f64;
            requests.drags.push((tc.id, tc.start_secs + delta_secs));
        }
        let tc_edge_sense = if track.locked {
            egui::Sense::hover()
        } else {
            canvas_sense(hand_active, egui::Sense::drag())
        };
        let tc_left_response = ui.interact(tc_left_edge_rect, tc_trim_start_id, tc_edge_sense);
        let tc_right_response = ui.interact(tc_right_edge_rect, tc_trim_end_id, tc_edge_sense);
        if tc_left_response.hovered()
            || tc_left_response.dragged()
            || tc_right_response.hovered()
            || tc_right_response.dragged()
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if tc_left_response.drag_started() || tc_right_response.drag_started() {
            *requests.drag_started = true;
        }
        if let Some(pos) = tc_left_response.interact_pointer_pos() {
            let secs = ((pos.x - track_rect.left()) / px_per_sec).max(0.0) as f64;
            requests.trims.push((tc.id, TrimEdge::Start(secs)));
        }
        if let Some(pos) = tc_right_response.interact_pointer_pos() {
            let secs = ((pos.x - track_rect.left()) / px_per_sec).max(0.0) as f64;
            requests.trims.push((tc.id, TrimEdge::End(secs)));
        }
        let block_color = egui::Color32::from_rgba_unmultiplied(
            tc.color_rgba[0],
            tc.color_rgba[1],
            tc.color_rgba[2],
            120,
        );
        painter.rect_filled(
            tc_rect,
            egui::CornerRadius::same(theme::RADIUS_SM),
            block_color,
        );
        // Clip the text label to the block width.
        let label_pos = tc_rect.left_center() + egui::vec2(4.0, 0.0);
        painter.text(
            label_pos,
            egui::Align2::LEFT_CENTER,
            &tc.text,
            egui::FontId::proportional(11.0),
            egui::Color32::WHITE,
        );
        if tc_left_response.hovered() || tc_left_response.dragged() {
            painter.rect_filled(tc_left_edge_rect, egui::CornerRadius::ZERO, theme::ACCENT);
        }
        if tc_right_response.hovered() || tc_right_response.dragged() {
            painter.rect_filled(tc_right_edge_rect, egui::CornerRadius::ZERO, theme::ACCENT);
        }
        // Selection ring
        if selected_text_clip_id == Some(tc.id) {
            painter.rect_stroke(
                tc_rect,
                egui::CornerRadius::same(theme::RADIUS_SM),
                egui::Stroke::new(2.0, theme::ACCENT),
                egui::StrokeKind::Inside,
            );
        }
    }
}
