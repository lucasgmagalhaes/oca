use std::collections::HashMap;

use avcore::MediaAsset;
use eframe::egui::{self, RichText};

use crate::app::{App, THUMBNAIL_BUCKET_SECS};
use crate::i18n::Text;
use crate::theme;

/// Bounds for `App::timeline_px_per_sec` — tight enough to stay readable, loose enough to
/// go from several-projects-wide overview down to frame-accurate editing.
const MIN_PX_PER_SEC: f32 = 0.5;
const MAX_PX_PER_SEC: f32 = 60.0;
const TRACK_LABEL_WIDTH: f32 = 50.0;

/// A clip body drag in progress: which clip, where it started from, and where the pointer
/// currently is — resolved into a same-track reposition or a cross-track move once every
/// track's row rect has been computed (see the loop in `timeline_panel`).
struct ClipDrag {
    clip_id: u64,
    source_track_id: u64,
    kind: avcore::timeline::TrackKind,
    new_start_secs: f64,
    pointer_y: f32,
}

/// Which edge of a timeline clip a drag targets — see the trim handling in `timeline_panel`.
enum TrimEdge {
    Start(f64),
    End(f64),
}

pub(super) fn timeline_panel(app: &mut App, ui: &mut egui::Ui, height: f32) {
    crate::components::card_frame().show(ui, |ui| {
        ui.set_height(height - 20.0);

        // Ctrl+scroll zooms the timeline in/out (request.md's Fase 3 spec). egui's
        // `zoom_delta()` already separates ctrl-held scroll from plain scroll at the input
        // level (`Modifiers::COMMAND`, which is Ctrl outside macOS) — plain scroll still
        // reaches the ScrollArea below as normal, no manual event-consuming needed.
        let zoom_delta = ui.input(|i| i.zoom_delta());
        if zoom_delta != 1.0 && ui.rect_contains_pointer(ui.max_rect()) {
            app.timeline_px_per_sec =
                (app.timeline_px_per_sec * zoom_delta).clamp(MIN_PX_PER_SEC, MAX_PX_PER_SEC);
        }
        let px_per_sec = app.timeline_px_per_sec;

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(Text::Timeline.tr(app.locale))
                    .size(11.0)
                    .color(theme::TEXT_MUTED),
            );
        });
        ui.separator();

        // Ruler: click or drag to move the playhead. Kept as its own thin strip rather than
        // reusing a track row so scrubbing doesn't depend on there being any tracks yet.
        let mut ruler_top = 0.0_f32;
        ui.horizontal(|ui| {
            ui.add_space(TRACK_LABEL_WIDTH);
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 14.0),
                egui::Sense::click_and_drag(),
            );
            ruler_top = rect.top();
            ui.painter().rect_filled(rect, 0, theme::SURFACE_2);
            if let Some(pos) = response.interact_pointer_pos() {
                let secs = ((pos.x - rect.left()) / px_per_sec).max(0.0) as f64;
                app.active_project_mut().timeline_mut().playhead_secs = secs;
            }
            draw_playhead(
                ui,
                rect,
                app.active_project().timeline().playhead_secs,
                px_per_sec,
                2.0,
            );
        });

        let locale = app.locale;
        let playhead_secs = app.active_project().timeline().playhead_secs;
        let has_clipboard_clip = app.has_clipboard_clip();
        let has_formatting_clipboard = app.has_formatting_clipboard();
        let multi_selected_count = app.multi_selected_clip_ids.len();
        let mut clicked_clip_id = None;
        let mut clicked_text_clip_id: Option<u64> = None;
        let mut clicked_shape_clip_id: Option<u64> = None;
        let mut delete_text_clip_requests: Vec<u64> = Vec::new();
        let mut delete_shape_clip_requests: Vec<u64> = Vec::new();
        let mut delete_requests: Vec<u64> = Vec::new();
        let mut copy_requests: Vec<u64> = Vec::new();
        let mut cut_requests: Vec<u64> = Vec::new();
        let mut copy_formatting_requests: Vec<u64> = Vec::new();
        let mut paste_formatting_requests: Vec<u64> = Vec::new();
        let mut multi_select_requests: Vec<u64> = Vec::new();
        let mut paste_requested = false;
        let mut merge_into_composite_requested = false;
        let mut split_at_playhead_requested = false;
        let mut trim_requests: Vec<(u64, TrimEdge)> = Vec::new();
        let mut clip_drags: Vec<ClipDrag> = Vec::new();
        let mut track_rows: Vec<(u64, avcore::timeline::TrackKind, egui::Rect)> = Vec::new();
        let mut toggle_track_visibility_requests: Vec<u64> = Vec::new();
        let mut thumbnail_requests: Vec<(u64, i64)> = Vec::new();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for track in &app.active_project().timeline().tracks {
                ui.horizontal(|ui| {
                    // Track header: visibility toggle + name.
                    ui.allocate_ui_with_layout(
                        egui::vec2(TRACK_LABEL_WIDTH, 28.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let track_id = track.id;
                            let visible = track.visible;
                            let eye = if visible { "👁" } else { "—" };
                            if ui
                                .small_button(eye)
                                .on_hover_text(if visible { "Hide track" } else { "Show track" })
                                .clicked()
                            {
                                toggle_track_visibility_requests.push(track_id);
                            }
                            ui.add(
                                egui::Label::new(RichText::new(&track.name).size(11.0).color(
                                    if visible {
                                        theme::TEXT_SECONDARY
                                    } else {
                                        theme::TEXT_MUTED
                                    },
                                ))
                                .truncate(),
                            );
                        },
                    );
                    let (track_rect, _resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 26.0),
                        egui::Sense::hover(),
                    );
                    track_rows.push((track.id, track.kind, track_rect));
                    let painter = ui.painter();
                    for clip in &track.clips {
                        let x = track_rect.left() + clip.start_secs as f32 * px_per_sec;
                        let w = (clip.duration_secs() as f32 * px_per_sec).max(3.0);
                        let clip_rect = egui::Rect::from_min_size(
                            egui::pos2(x, track_rect.top()),
                            egui::vec2(w, track_rect.height()),
                        );
                        let color = match (track.kind, track.name.as_str()) {
                            (avcore::timeline::TrackKind::Video, _) => theme::SURFACE_2,
                            (avcore::timeline::TrackKind::Audio, "A2") => {
                                theme::ACCENT_2.gamma_multiply(0.6)
                            }
                            (avcore::timeline::TrackKind::Audio, _) => {
                                theme::ACCENT.gamma_multiply(0.5)
                            }
                            // Text/Shape tracks carry text_clips/shape_clips, not clips — these
                            // arms satisfy exhaustiveness but are never reached at runtime.
                            (avcore::timeline::TrackKind::Text, _) => theme::SURFACE_2,
                            (avcore::timeline::TrackKind::Shape, _) => theme::SURFACE_2,
                        };

                        // Narrow strips at each edge, on top of the body's click zone, so a
                        // drag started right at the edge trims instead of just selecting.
                        let edge_w = (w / 3.0).clamp(2.0, 6.0);
                        let left_edge_rect = egui::Rect::from_min_size(
                            clip_rect.min,
                            egui::vec2(edge_w, clip_rect.height()),
                        );
                        let right_edge_rect = egui::Rect::from_min_size(
                            egui::pos2(clip_rect.right() - edge_w, clip_rect.top()),
                            egui::vec2(edge_w, clip_rect.height()),
                        );

                        let body_response = ui.interact(
                            clip_rect,
                            ui.id().with(("timeline_clip", clip.id)),
                            egui::Sense::click_and_drag(),
                        );
                        let covers_playhead = clip.start_secs <= playhead_secs
                            && playhead_secs < clip.start_secs + clip.duration_secs();
                        body_response.context_menu(|ui| {
                            clicked_clip_id = Some(clip.id);
                            if ui
                                .add_enabled(
                                    covers_playhead,
                                    egui::Button::new(Text::ContextMenuSplit.tr(locale)),
                                )
                                .clicked()
                            {
                                split_at_playhead_requested = true;
                                ui.close();
                            }
                            if ui.button(Text::ContextMenuCopy.tr(locale)).clicked() {
                                copy_requests.push(clip.id);
                                ui.close();
                            }
                            if ui.button(Text::ContextMenuCut.tr(locale)).clicked() {
                                cut_requests.push(clip.id);
                                ui.close();
                            }
                            if ui
                                .add_enabled(
                                    has_clipboard_clip,
                                    egui::Button::new(Text::ContextMenuPaste.tr(locale)),
                                )
                                .clicked()
                            {
                                paste_requested = true;
                                ui.close();
                            }
                            ui.separator();
                            // Same enablement as the toolbar's "Mesclar em bloco composto"
                            // button — needs at least two clips ctrl-clicked into a
                            // multi-selection first; this just gives the context menu (per
                            // request.md's Fase 3 spec) the same action, not a new one.
                            if ui
                                .add_enabled(
                                    multi_selected_count >= 2,
                                    egui::Button::new(Text::MergeIntoComposite.tr(locale)),
                                )
                                .clicked()
                            {
                                merge_into_composite_requested = true;
                                ui.close();
                            }
                            ui.separator();
                            if ui
                                .button(Text::ContextMenuCopyFormatting.tr(locale))
                                .clicked()
                            {
                                copy_formatting_requests.push(clip.id);
                                ui.close();
                            }
                            if ui
                                .add_enabled(
                                    has_formatting_clipboard,
                                    egui::Button::new(Text::ContextMenuPasteFormatting.tr(locale)),
                                )
                                .clicked()
                            {
                                paste_formatting_requests.push(clip.id);
                                ui.close();
                            }
                            ui.separator();
                            if ui.button(Text::ContextMenuDelete.tr(locale)).clicked() {
                                delete_requests.push(clip.id);
                                ui.close();
                            }
                        });
                        let left_response = ui.interact(
                            left_edge_rect,
                            ui.id().with(("timeline_clip_trim_start", clip.id)),
                            egui::Sense::drag(),
                        );
                        let right_response = ui.interact(
                            right_edge_rect,
                            ui.id().with(("timeline_clip_trim_end", clip.id)),
                            egui::Sense::drag(),
                        );
                        if left_response.hovered()
                            || left_response.dragged()
                            || right_response.hovered()
                            || right_response.dragged()
                        {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }
                        if body_response.clicked() {
                            if ui.input(|i| i.modifiers.ctrl) {
                                multi_select_requests.push(clip.id);
                            } else {
                                clicked_clip_id = Some(clip.id);
                            }
                        }
                        if body_response.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                            let delta_secs = (body_response.drag_delta().x / px_per_sec) as f64;
                            if let Some(pointer) = body_response.interact_pointer_pos() {
                                clip_drags.push(ClipDrag {
                                    clip_id: clip.id,
                                    source_track_id: track.id,
                                    kind: track.kind,
                                    new_start_secs: clip.start_secs + delta_secs,
                                    pointer_y: pointer.y,
                                });
                            }
                        }
                        if let Some(pos) = left_response.interact_pointer_pos() {
                            let secs = ((pos.x - track_rect.left()) / px_per_sec).max(0.0) as f64;
                            trim_requests.push((clip.id, TrimEdge::Start(secs)));
                        }
                        if let Some(pos) = right_response.interact_pointer_pos() {
                            let secs = ((pos.x - track_rect.left()) / px_per_sec).max(0.0) as f64;
                            trim_requests.push((clip.id, TrimEdge::End(secs)));
                        }

                        painter.rect_filled(clip_rect, egui::CornerRadius::same(4), color);
                        let asset = app
                            .active_project()
                            .media_library
                            .iter()
                            .find(|a| a.id == clip.asset_id);
                        if track.kind == avcore::timeline::TrackKind::Video {
                            if let Some(asset) = asset {
                                if clip.frozen {
                                    draw_frozen_poster(
                                        &app.thumbnail_textures,
                                        painter,
                                        clip_rect,
                                        asset,
                                        clip.source_in_secs,
                                        &mut thumbnail_requests,
                                    );
                                } else {
                                    draw_filmstrip(
                                        &app.thumbnail_textures,
                                        painter,
                                        clip_rect,
                                        asset,
                                        clip.source_in_secs,
                                        px_per_sec,
                                        &mut thumbnail_requests,
                                    );
                                }
                            }
                        } else if let Some(asset) = asset {
                            if let Some(peaks) = &asset.waveform_peaks {
                                draw_waveform(
                                    painter,
                                    clip_rect,
                                    peaks,
                                    asset.duration_secs,
                                    clip.source_in_secs..clip.source_out_secs,
                                    clip.gain_linear(),
                                    theme::TEXT_PRIMARY.gamma_multiply(0.7),
                                );
                            }
                        }
                        if let Some(tint) = color_filter_tint(clip.color_filter) {
                            painter.rect_filled(clip_rect, egui::CornerRadius::same(4), tint);
                        }
                        if clip.has_vignette() {
                            let alpha = (clip.vignette_intensity * 200.0) as u8;
                            painter.rect_stroke(
                                clip_rect,
                                egui::CornerRadius::same(4),
                                egui::Stroke::new(3.0, egui::Color32::from_black_alpha(alpha)),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if clip.composite_id.is_some() {
                            painter.rect_stroke(
                                clip_rect,
                                egui::CornerRadius::same(4),
                                egui::Stroke::new(1.5, theme::ACCENT_2),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if clip.speed_factor != 1.0 {
                            painter.text(
                                clip_rect.right_top() + egui::vec2(-3.0, 2.0),
                                egui::Align2::RIGHT_TOP,
                                format!("{:.2}x", clip.speed_factor),
                                egui::FontId::proportional(11.0),
                                theme::TEXT_PRIMARY,
                            );
                        }
                        if clip.is_cropped() {
                            painter.text(
                                clip_rect.right_bottom() + egui::vec2(-3.0, -2.0),
                                egui::Align2::RIGHT_BOTTOM,
                                "⛶",
                                egui::FontId::proportional(11.0),
                                theme::TEXT_PRIMARY,
                            );
                        }
                        if clip.is_masked() {
                            let glyph = match clip.mask_shape {
                                avcore::timeline::MaskShape::Circle => "●",
                                avcore::timeline::MaskShape::RoundedRect => "▢",
                                avcore::timeline::MaskShape::None => "",
                            };
                            painter.text(
                                clip_rect.left_bottom() + egui::vec2(3.0, -2.0),
                                egui::Align2::LEFT_BOTTOM,
                                glyph,
                                egui::FontId::proportional(11.0),
                                theme::TEXT_PRIMARY,
                            );
                        }
                        if clip.flipped_h {
                            painter.text(
                                clip_rect.center_top() + egui::vec2(0.0, 2.0),
                                egui::Align2::CENTER_TOP,
                                "⇄",
                                egui::FontId::proportional(11.0),
                                theme::TEXT_PRIMARY,
                            );
                        }
                        if clip.is_chroma_keyed() {
                            painter.text(
                                clip_rect.center_bottom() + egui::vec2(0.0, -2.0),
                                egui::Align2::CENTER_BOTTOM,
                                "🟩",
                                egui::FontId::proportional(11.0),
                                theme::TEXT_PRIMARY,
                            );
                        }
                        draw_keyframe_markers(painter, clip_rect, clip);
                        if app.multi_selected_clip_ids.contains(&clip.id) {
                            painter.rect_stroke(
                                clip_rect,
                                egui::CornerRadius::same(4),
                                egui::Stroke::new(2.0, theme::ERROR),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if app.selected_clip_id == Some(clip.id) {
                            painter.rect_stroke(
                                clip_rect,
                                egui::CornerRadius::same(4),
                                egui::Stroke::new(2.0, theme::ACCENT),
                                egui::StrokeKind::Inside,
                            );
                        }
                    }
                    // Render text clips for text tracks as solid-color blocks with text label.
                    if track.kind == avcore::timeline::TrackKind::Text {
                        for tc in &track.text_clips {
                            let x = track_rect.left() + tc.start_secs as f32 * px_per_sec;
                            let w = (tc.duration_secs as f32 * px_per_sec).max(3.0);
                            let tc_rect = egui::Rect::from_min_size(
                                egui::pos2(x, track_rect.top()),
                                egui::vec2(w, track_rect.height()),
                            );
                            let tc_response = ui.interact(
                                tc_rect,
                                ui.id().with(("timeline_text_clip", tc.id)),
                                egui::Sense::click(),
                            );
                            tc_response.context_menu(|ui| {
                                if ui.button(Text::ContextMenuDelete.tr(locale)).clicked() {
                                    delete_text_clip_requests.push(tc.id);
                                    ui.close();
                                }
                            });
                            if tc_response.clicked() {
                                clicked_text_clip_id = Some(tc.id);
                            }
                            let block_color = egui::Color32::from_rgba_unmultiplied(
                                tc.color_rgba[0],
                                tc.color_rgba[1],
                                tc.color_rgba[2],
                                120,
                            );
                            painter.rect_filled(tc_rect, egui::CornerRadius::same(4), block_color);
                            // Clip the text label to the block width.
                            let label_pos = tc_rect.left_center() + egui::vec2(4.0, 0.0);
                            painter.text(
                                label_pos,
                                egui::Align2::LEFT_CENTER,
                                &tc.text,
                                egui::FontId::proportional(11.0),
                                egui::Color32::WHITE,
                            );
                            // Selection ring
                            if app.selected_text_clip_id == Some(tc.id) {
                                painter.rect_stroke(
                                    tc_rect,
                                    egui::CornerRadius::same(4),
                                    egui::Stroke::new(2.0, theme::ACCENT),
                                    egui::StrokeKind::Inside,
                                );
                            }
                        }
                    }
                    // Render shape clips for shape tracks as solid-color blocks — mirrors the
                    // text-clip block above, swapping the text label for the shape's own color.
                    if track.kind == avcore::timeline::TrackKind::Shape {
                        for sc in &track.shape_clips {
                            let x = track_rect.left() + sc.start_secs as f32 * px_per_sec;
                            let w = (sc.duration_secs as f32 * px_per_sec).max(3.0);
                            let sc_rect = egui::Rect::from_min_size(
                                egui::pos2(x, track_rect.top()),
                                egui::vec2(w, track_rect.height()),
                            );
                            let sc_response = ui.interact(
                                sc_rect,
                                ui.id().with(("timeline_shape_clip", sc.id)),
                                egui::Sense::click(),
                            );
                            sc_response.context_menu(|ui| {
                                if ui.button(Text::ContextMenuDelete.tr(locale)).clicked() {
                                    delete_shape_clip_requests.push(sc.id);
                                    ui.close();
                                }
                            });
                            if sc_response.clicked() {
                                clicked_shape_clip_id = Some(sc.id);
                            }
                            let block_color = egui::Color32::from_rgba_unmultiplied(
                                sc.color_rgba[0],
                                sc.color_rgba[1],
                                sc.color_rgba[2],
                                120,
                            );
                            painter.rect_filled(sc_rect, egui::CornerRadius::same(4), block_color);
                            let label_pos = sc_rect.left_center() + egui::vec2(4.0, 0.0);
                            painter.text(
                                label_pos,
                                egui::Align2::LEFT_CENTER,
                                shape_kind_glyph(&sc.shape_kind),
                                egui::FontId::proportional(11.0),
                                egui::Color32::WHITE,
                            );
                            // Selection ring
                            if app.selected_shape_clip_id == Some(sc.id) {
                                painter.rect_stroke(
                                    sc_rect,
                                    egui::CornerRadius::same(4),
                                    egui::Stroke::new(2.0, theme::ACCENT),
                                    egui::StrokeKind::Inside,
                                );
                            }
                        }
                    }
                    draw_playhead(
                        ui,
                        track_rect,
                        app.active_project().timeline().playhead_secs,
                        px_per_sec,
                        1.0,
                    );
                });
                ui.add_space(4.0);
            }
            if app.active_project().timeline().tracks.is_empty() {
                ui.label(
                    RichText::new(Text::TimelineEmpty.tr(app.locale))
                        .size(12.0)
                        .color(theme::TEXT_MUTED),
                );
            }
        });
        if let Some(id) = clicked_clip_id {
            app.selected_text_clip_id = None;
            app.selected_shape_clip_id = None;
            app.select_timeline_clip(id);
        }
        if let Some(id) = clicked_text_clip_id {
            app.selected_clip_id = None;
            app.selected_shape_clip_id = None;
            app.selected_text_clip_id = Some(id);
        }
        if let Some(id) = clicked_shape_clip_id {
            app.selected_clip_id = None;
            app.selected_text_clip_id = None;
            app.selected_shape_clip_id = Some(id);
        }
        for tc_id in delete_text_clip_requests {
            let timeline = app.active_project_mut().timeline_mut();
            for track in &mut timeline.tracks {
                if track.kind == avcore::timeline::TrackKind::Text {
                    track.text_clips.retain(|tc| tc.id != tc_id);
                }
            }
            if app.selected_text_clip_id == Some(tc_id) {
                app.selected_text_clip_id = None;
            }
        }
        for sc_id in delete_shape_clip_requests {
            let timeline = app.active_project_mut().timeline_mut();
            for track in &mut timeline.tracks {
                if track.kind == avcore::timeline::TrackKind::Shape {
                    track.shape_clips.retain(|sc| sc.id != sc_id);
                }
            }
            if app.selected_shape_clip_id == Some(sc_id) {
                app.selected_shape_clip_id = None;
            }
        }
        for clip_id in multi_select_requests {
            app.toggle_multi_select(clip_id);
        }
        for clip_id in copy_requests {
            app.selected_clip_id = Some(clip_id);
            app.copy_selected_clip();
        }
        for clip_id in cut_requests {
            app.selected_clip_id = Some(clip_id);
            app.cut_selected_clip();
        }
        for clip_id in copy_formatting_requests {
            app.selected_clip_id = Some(clip_id);
            app.copy_selected_clip_formatting();
        }
        for clip_id in paste_formatting_requests {
            app.selected_clip_id = Some(clip_id);
            app.paste_selected_clip_formatting();
        }
        if paste_requested {
            app.paste_clip_at_playhead();
        }
        if merge_into_composite_requested {
            app.merge_into_composite();
        }
        for track_id in toggle_track_visibility_requests {
            app.toggle_track_visibility(track_id);
        }
        for clip_id in delete_requests {
            app.selected_clip_id = Some(clip_id);
            app.delete_selected_clip();
        }
        if split_at_playhead_requested {
            app.split_at_playhead();
        }
        for (clip_id, edge) in trim_requests {
            match edge {
                TrimEdge::Start(secs) => app.trim_clip_start(clip_id, secs),
                TrimEdge::End(secs) => app.trim_clip_end(clip_id, secs),
            }
        }
        for drag in clip_drags {
            // Whichever track row's Y-range the pointer is currently over, if its kind
            // matches the dragged clip's own track — a video clip can't be dropped onto an
            // audio row or vice versa. Falls back to a same-track reposition if the pointer
            // isn't over any matching row (including its own, the common case).
            let target_track_id = track_rows
                .iter()
                .find(|(_, kind, rect)| {
                    *kind == drag.kind && rect.y_range().contains(drag.pointer_y)
                })
                .map(|(id, _, _)| *id);
            // A composite block's members must all stay on the same track (see
            // ClipInstance::composite_id's doc comment), so a cross-track drop is refused for
            // one — it falls back to the same-track group move below instead.
            let is_composite = app
                .active_project()
                .timeline()
                .tracks
                .iter()
                .flat_map(|t| &t.clips)
                .find(|c| c.id == drag.clip_id)
                .is_some_and(|c| c.composite_id.is_some());
            match target_track_id {
                Some(track_id) if track_id != drag.source_track_id && !is_composite => {
                    app.move_clip_to_track(drag.clip_id, track_id, drag.new_start_secs);
                }
                _ => app.move_clip_with_group(drag.clip_id, drag.new_start_secs),
            }
        }
        for (asset_id, bucket) in thumbnail_requests {
            app.request_thumbnail(asset_id, bucket);
        }
        // An asset dragged out of the media library and released somewhere at or below the
        // ruler: whichever track row's Y-range the pointer landed on becomes the preferred
        // drop target (`App::add_asset_to_timeline_at` falls back to a matching-kind track
        // if that row's kind doesn't match the asset, same as a cross-track clip move). A
        // release above the ruler means the drag never reached the timeline at all, so it's
        // ignored rather than silently appending.
        if let Some((asset_id, pos)) = app.pending_asset_drop.take() {
            if pos.y >= ruler_top {
                let target = track_rows
                    .iter()
                    .find(|(_, _, rect)| rect.y_range().contains(pos.y));
                match target {
                    Some((track_id, _, rect)) => {
                        let secs = ((pos.x - rect.left()) / px_per_sec).max(0.0) as f64;
                        app.add_asset_to_timeline_at(asset_id, Some(*track_id), secs);
                    }
                    None => app.add_asset_to_timeline(asset_id),
                }
            }
        }
    });
}

/// Draws one filmstrip tile per column of `clip_rect` (each column's width set by `asset`'s
/// aspect ratio scaled to the clip's height, same as a single poster frame tiled), each showing
/// the cached thumbnail texture for whichever [`THUMBNAIL_BUCKET_SECS`]-quantized source-time
/// bucket that column falls in (`App::thumbnail_textures`) — distinct buckets read like a
/// true filmstrip, unlike one poster frame repeated. A column whose bucket has no texture yet
/// is queued into `thumbnail_requests` (deduplicated against already-pending buckets by
/// [`App::request_thumbnail`] once the caller applies it) and left showing the clip's plain
/// background color, already painted underneath by the caller, until it arrives. A no-op if
/// `asset` has no resolution (audio-only, shouldn't happen for a clip on a video track).
fn draw_filmstrip(
    thumbnail_textures: &HashMap<(u64, i64), egui::TextureHandle>,
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    asset: &MediaAsset,
    source_in_secs: f64,
    px_per_sec: f32,
    thumbnail_requests: &mut Vec<(u64, i64)>,
) {
    let Some((res_w, res_h)) = asset.resolution else {
        return;
    };
    if res_h == 0 {
        return;
    }
    let tile_height = clip_rect.height();
    let tile_width = (tile_height * res_w as f32 / res_h as f32).max(1.0);

    let mut x = clip_rect.left();
    while x < clip_rect.right() {
        let w = tile_width.min(clip_rect.right() - x);
        let tile_rect =
            egui::Rect::from_min_size(egui::pos2(x, clip_rect.top()), egui::vec2(w, tile_height));
        let time_in_source = source_in_secs + ((x - clip_rect.left()) / px_per_sec) as f64;
        let bucket = (time_in_source / THUMBNAIL_BUCKET_SECS).floor() as i64;
        match thumbnail_textures.get(&(asset.id, bucket)) {
            Some(texture) => {
                painter.image(
                    texture.id(),
                    tile_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            None => thumbnail_requests.push((asset.id, bucket)),
        }
        x += tile_width;
    }
}

/// Draws a single poster frame — the one at `source_in_secs`, the frame a frozen block
/// ([`avcore::timeline::ClipInstance::frozen`]) holds per `request.md`'s Fase 4 "Congelar"
/// spec — tiled across all of `clip_rect`, plus a small "❄" badge marking the block as frozen
/// even at a zoom level too tight to tell a still poster from a real filmstrip. Reuses
/// `draw_filmstrip`'s texture cache/request plumbing, just pinned to one bucket instead of one
/// per column. A no-op if `asset` has no resolution (audio-only, shouldn't happen for a clip on
/// a video track).
fn draw_frozen_poster(
    thumbnail_textures: &HashMap<(u64, i64), egui::TextureHandle>,
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    asset: &MediaAsset,
    source_in_secs: f64,
    thumbnail_requests: &mut Vec<(u64, i64)>,
) {
    let Some((res_w, res_h)) = asset.resolution else {
        return;
    };
    if res_h == 0 {
        return;
    }
    let tile_height = clip_rect.height();
    let tile_width = (tile_height * res_w as f32 / res_h as f32).max(1.0);
    let bucket = (source_in_secs / THUMBNAIL_BUCKET_SECS).floor() as i64;

    match thumbnail_textures.get(&(asset.id, bucket)) {
        Some(texture) => {
            let mut x = clip_rect.left();
            while x < clip_rect.right() {
                let w = tile_width.min(clip_rect.right() - x);
                let tile_rect = egui::Rect::from_min_size(
                    egui::pos2(x, clip_rect.top()),
                    egui::vec2(w, tile_height),
                );
                painter.image(
                    texture.id(),
                    tile_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                x += tile_width;
            }
        }
        None => thumbnail_requests.push((asset.id, bucket)),
    }

    painter.text(
        clip_rect.left_top() + egui::vec2(3.0, 2.0),
        egui::Align2::LEFT_TOP,
        "❄",
        egui::FontId::proportional(12.0),
        egui::Color32::WHITE,
    );
}

/// Draws one vertical min/max bar per horizontal pixel of `clip_rect`, resampling `peaks`
/// (the asset's full-duration waveform, see [`avcore::waveform`]) down to whatever's visible
/// between `source_in_secs` and `source_out_secs` — the same "fixed-resolution source data
/// resampled to the current on-screen width" idea as `draw_filmstrip`'s zoom handling, just
/// per-column instead of per-tile. A no-op if `peaks` is empty or `asset_duration_secs`
/// is non-positive (shouldn't happen for a real decoded asset, but guards div-by-zero).
fn draw_waveform(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    peaks: &[(f32, f32)],
    asset_duration_secs: f64,
    source_range_secs: std::ops::Range<f64>,
    gain_linear: f32,
    color: egui::Color32,
) {
    if peaks.is_empty() || asset_duration_secs <= 0.0 {
        return;
    }
    let bucket_count = peaks.len() as f64;
    let start_bucket = (source_range_secs.start / asset_duration_secs * bucket_count)
        .clamp(0.0, bucket_count - 1.0);
    let end_bucket =
        (source_range_secs.end / asset_duration_secs * bucket_count).clamp(0.0, bucket_count);
    let bucket_span = (end_bucket - start_bucket).max(1.0);

    let mid_y = clip_rect.center().y;
    let half_h = clip_rect.height() / 2.0 - 1.0;
    let width_px = clip_rect.width().max(1.0) as usize;

    for x in 0..width_px {
        let t = x as f64 / width_px as f64;
        let bucket = ((start_bucket + t * bucket_span) as usize).min(peaks.len() - 1);
        let (min, max) = peaks[bucket];
        // Live gain preview (Fase 4 "ganho de volume por bloco") — bars scale with the block's
        // gain, clamped so extreme gain doesn't paint outside the clip's own row.
        let min = (min * gain_linear).clamp(-1.0, 1.0);
        let max = (max * gain_linear).clamp(-1.0, 1.0);
        let px = clip_rect.left() + x as f32;
        painter.line_segment(
            [
                egui::pos2(px, mid_y - max * half_h),
                egui::pos2(px, mid_y - min * half_h),
            ],
            egui::Stroke::new(1.0, color),
        );
    }
}

/// Small diamond marker for every distinct `time_fraction` used across `clip`'s four keyframe
/// lists (position/scale/rotation/opacity — see [`avcore::keyframe`]), along the clip block's
/// bottom edge. Read-only: this pass covers visibility only, not on-timeline add/drag/delete —
/// editing keyframes still goes through the properties panel's list editor. A no-op if the clip
/// has no keyframes on any property (the common case).
fn draw_keyframe_markers(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    clip: &avcore::timeline::ClipInstance,
) {
    if !clip.has_position_keyframes()
        && !clip.has_scale_keyframes()
        && !clip.has_rotation_keyframes()
        && !clip.has_opacity_keyframes()
    {
        return;
    }
    let mut fractions: Vec<f32> = clip
        .position_keyframes
        .iter()
        .map(|k| k.time_fraction)
        .chain(clip.scale_keyframes.iter().map(|k| k.time_fraction))
        .chain(clip.rotation_keyframes.iter().map(|k| k.time_fraction))
        .chain(clip.opacity_keyframes.iter().map(|k| k.time_fraction))
        .collect();
    fractions.sort_by(|a, b| a.total_cmp(b));
    fractions.dedup_by(|a, b| (*a - *b).abs() < 1e-3);

    let y = clip_rect.bottom() - 5.0;
    let r = 3.0;
    for frac in fractions {
        let x = clip_rect.left() + frac.clamp(0.0, 1.0) * clip_rect.width();
        painter.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(x, y - r),
                egui::pos2(x + r, y),
                egui::pos2(x, y + r),
                egui::pos2(x - r, y),
            ],
            theme::ACCENT_2,
            egui::Stroke::new(1.0, theme::TEXT_PRIMARY.gamma_multiply(0.6)),
        ));
    }
}

/// Draws the playhead as a vertical line inside `rect`, if it falls within `rect`'s horizontal
/// span — used both on the ruler strip and on every track row so it reads as one continuous
/// line down the timeline despite each row being drawn separately.
fn draw_playhead(
    ui: &egui::Ui,
    rect: egui::Rect,
    playhead_secs: f64,
    px_per_sec: f32,
    stroke_width: f32,
) {
    let x = rect.left() + playhead_secs as f32 * px_per_sec;
    if x >= rect.left() && x <= rect.right() {
        ui.painter().vline(
            x,
            rect.y_range(),
            egui::Stroke::new(stroke_width, theme::ACCENT),
        );
    }
}

/// A short glyph labeling a shape clip's block on the timeline strip, standing in for the
/// full-fidelity render (which only happens on export, same preview gap as [`avcore::timeline::
/// TextClip`]'s).
fn shape_kind_glyph(kind: &avcore::timeline::ShapeKind) -> &'static str {
    match kind {
        avcore::timeline::ShapeKind::Ellipse => "●",
        avcore::timeline::ShapeKind::Polygon(vertices) => match vertices.len() {
            3 => "▲",
            4 => "■",
            _ => "⬠",
        },
    }
}

/// A translucent overlay color hinting at [`avcore::timeline::ClipInstance::color_filter`] on
/// the timeline block — `None` for [`avcore::timeline::ColorFilter::None`] (nothing drawn).
/// Just a preview-panel cue, not the real filtered pixels (see the field's doc comment for the
/// preview/export gap).
pub(super) fn color_filter_tint(filter: avcore::timeline::ColorFilter) -> Option<egui::Color32> {
    match filter {
        avcore::timeline::ColorFilter::None => None,
        avcore::timeline::ColorFilter::BlackAndWhite => {
            Some(egui::Color32::from_rgba_unmultiplied(128, 128, 128, 90))
        }
        avcore::timeline::ColorFilter::Sepia => {
            Some(egui::Color32::from_rgba_unmultiplied(180, 130, 60, 70))
        }
    }
}
