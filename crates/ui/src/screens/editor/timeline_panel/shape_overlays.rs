use super::{shape_kind_glyph, TrimEdge};
use crate::i18n::{Locale, Text};
use crate::theme;
use eframe::egui;
fn canvas_sense(hand: bool, sense: egui::Sense) -> egui::Sense {
    if hand {
        egui::Sense::hover()
    } else {
        sense
    }
}
pub(super) struct ShapeOverlayRequests<'a> {
    pub(super) clicked: &'a mut Option<u64>,
    pub(super) deletes: &'a mut Vec<u64>,
    pub(super) drags: &'a mut Vec<(u64, f64)>,
    pub(super) trims: &'a mut Vec<(u64, TrimEdge)>,
    pub(super) drag_started: &'a mut bool,
}
pub(super) fn draw_shape_overlays(
    ui: &mut egui::Ui,
    painter: &egui::Painter,
    track: &avcore::timeline::Track,
    track_rect: egui::Rect,
    px_per_sec: f32,
    locale: Locale,
    hand_active: bool,
    selected_shape_clip_id: Option<u64>,
    requests: ShapeOverlayRequests<'_>,
) {
    if track.kind != avcore::timeline::TrackKind::Shape {
        return;
    }
    for sc in &track.shape_clips {
        let x = track_rect.left() + sc.start_secs as f32 * px_per_sec;
        let sc_widget_id = ui.id().with(("timeline_shape_clip", sc.id));
        let sc_trim_start_id = ui.id().with(("timeline_shape_clip_trim_start", sc.id));
        let sc_trim_end_id = ui.id().with(("timeline_shape_clip_trim_end", sc.id));
        let sc_being_dragged = ui
            .ctx()
            .dragged_id()
            .is_some_and(|id| id == sc_widget_id || id == sc_trim_start_id || id == sc_trim_end_id);
        // Same viewport-culling reasoning as the video/audio clip loop above.
        if x > track_rect.right() && !sc_being_dragged {
            continue;
        }
        let w = (sc.duration_secs as f32 * px_per_sec).max(3.0);
        let sc_rect = egui::Rect::from_min_size(
            egui::pos2(x, track_rect.top()),
            egui::vec2(w, track_rect.height()),
        );
        let edge_w = (w / 3.0).clamp(2.0, 6.0);
        let sc_left_edge_rect =
            egui::Rect::from_min_size(sc_rect.min, egui::vec2(edge_w, sc_rect.height()));
        let sc_right_edge_rect = egui::Rect::from_min_size(
            egui::pos2(sc_rect.right() - edge_w, sc_rect.top()),
            egui::vec2(edge_w, sc_rect.height()),
        );
        let sc_response = ui.interact(
            sc_rect,
            sc_widget_id,
            if track.locked {
                canvas_sense(hand_active, egui::Sense::click())
            } else {
                canvas_sense(hand_active, egui::Sense::click_and_drag())
            },
        );
        sc_response.context_menu(|ui| {
            if ui.button(Text::ContextMenuDelete.tr(locale)).clicked() {
                requests.deletes.push(sc.id);
                ui.close();
            }
        });
        if sc_response.clicked() {
            *requests.clicked = Some(sc.id);
        }
        if sc_response.drag_started() {
            *requests.drag_started = true;
        }
        if sc_response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            let delta_secs = (sc_response.drag_delta().x / px_per_sec) as f64;
            requests.drags.push((sc.id, sc.start_secs + delta_secs));
        }
        let sc_edge_sense = if track.locked {
            egui::Sense::hover()
        } else {
            canvas_sense(hand_active, egui::Sense::drag())
        };
        let sc_left_response = ui.interact(sc_left_edge_rect, sc_trim_start_id, sc_edge_sense);
        let sc_right_response = ui.interact(sc_right_edge_rect, sc_trim_end_id, sc_edge_sense);
        if sc_left_response.hovered()
            || sc_left_response.dragged()
            || sc_right_response.hovered()
            || sc_right_response.dragged()
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if sc_left_response.drag_started() || sc_right_response.drag_started() {
            *requests.drag_started = true;
        }
        if let Some(pos) = sc_left_response.interact_pointer_pos() {
            let secs = ((pos.x - track_rect.left()) / px_per_sec).max(0.0) as f64;
            requests.trims.push((sc.id, TrimEdge::Start(secs)));
        }
        if let Some(pos) = sc_right_response.interact_pointer_pos() {
            let secs = ((pos.x - track_rect.left()) / px_per_sec).max(0.0) as f64;
            requests.trims.push((sc.id, TrimEdge::End(secs)));
        }
        let block_color = egui::Color32::from_rgba_unmultiplied(
            sc.color_rgba[0],
            sc.color_rgba[1],
            sc.color_rgba[2],
            120,
        );
        painter.rect_filled(
            sc_rect,
            egui::CornerRadius::same(theme::RADIUS_SM),
            block_color,
        );
        let label_pos = sc_rect.left_center() + egui::vec2(4.0, 0.0);
        painter.text(
            label_pos,
            egui::Align2::LEFT_CENTER,
            shape_kind_glyph(&sc.shape_kind),
            egui::FontId::proportional(11.0),
            egui::Color32::WHITE,
        );
        if sc_left_response.hovered() || sc_left_response.dragged() {
            painter.rect_filled(sc_left_edge_rect, egui::CornerRadius::ZERO, theme::ACCENT);
        }
        if sc_right_response.hovered() || sc_right_response.dragged() {
            painter.rect_filled(sc_right_edge_rect, egui::CornerRadius::ZERO, theme::ACCENT);
        }
        // Selection ring
        if selected_shape_clip_id == Some(sc.id) {
            painter.rect_stroke(
                sc_rect,
                egui::CornerRadius::same(theme::RADIUS_SM),
                egui::Stroke::new(2.0, theme::ACCENT),
                egui::StrokeKind::Inside,
            );
        }
    }
}
