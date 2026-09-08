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

mod asset_drop;
mod clip_commands;
mod clip_context_menu;
mod clip_requests;
mod draw;
mod header;
mod interactions;
mod layout;
mod ruler;
mod selection_commands;
mod shape_overlays;
mod snap;
mod text_overlays;
mod thumbnails;
mod track_commands;
mod track_header;
mod track_requests;

use eframe::egui::{self, RichText};

use crate::app::{App, EditorTool};
use crate::i18n::Text;
use crate::theme;
pub(super) use layout::CLIP_COLOR_LABEL_PALETTE;
use layout::{
    track_area_height, TimelineCanvasLayout, COLLAPSED_TRACK_ROW_HEIGHT, MAX_PX_PER_SEC,
    MIN_PX_PER_SEC, RULER_HEIGHT, THUMBNAIL_ZOOM_DEBOUNCE, TRACK_LABEL_WIDTH, TRACK_ROW_HEIGHT,
};

use asset_drop::apply_pending_asset_drop;
use clip_commands::{apply_clip_commands, ClipCommands};
use clip_context_menu::{show_clip_context_menu, ClipContextMenuRequests};
use clip_requests::ClipRequests;
use draw::{
    color_filter_tint, draw_filmstrip, draw_frozen_poster, draw_keyframe_markers, draw_playhead,
    draw_transition_wedge, draw_trim_info, draw_waveform, shape_kind_glyph,
    thumbnail_requests_settled, ThumbnailDrawWork,
};
use header::timeline_header;
use interactions::apply_clip_interactions;
use ruler::{timeline_ruler, RulerLayout};
use selection_commands::{apply_selection_commands, SelectionCommands};
use shape_overlays::{draw_shape_overlays, ShapeOverlayRequests};
use snap::{snap_move_start, snap_to_nearest, ClipDrag, SnapTargets};
use text_overlays::{draw_text_overlays, TextOverlayRequests};
use thumbnails::apply_thumbnail_work;
use track_commands::{apply_track_commands, TrackCommands};
use track_header::{draw_track_header, TrackHeaderRequests};
use track_requests::TrackRequests;

/// Which edge of a timeline clip a drag targets — see the trim handling in `timeline_panel`.
pub(super) enum TrimEdge {
    Start(f64),
    End(f64),
}

pub(super) fn timeline_panel(app: &mut App, ui: &mut egui::Ui, height: f32) {
    crate::components::panel_frame().show(ui, |ui| {
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
        let now = std::time::Instant::now();
        if px_per_sec != app.timeline_thumbnail_zoom_settled_px_per_sec {
            app.timeline_thumbnail_zoom_settled_px_per_sec = px_per_sec;
            app.timeline_zoom_changed_at = Some(now);
        }
        let thumbnail_requests_allowed =
            thumbnail_requests_settled(app.timeline_zoom_changed_at, now, THUMBNAIL_ZOOM_DEBOUNCE);
        if thumbnail_requests_allowed {
            app.timeline_zoom_changed_at = None;
        }

        // Real horizontal pan (`EditorTool::Hand`): the ruler and every track row each get their
        // own small `ScrollArea::horizontal()` around just their canvas content (the track-label
        // gutter stays outside it, so labels/lock/eye never scroll away) — all forced to the same
        // `app.timeline_pan_px` at the start of this frame, so they stay in lockstep even though
        // egui gives each `ScrollArea` its own independent scroll state. Whichever one actually
        // received this frame's wheel/drag input ends up with a `state.offset.x` that differs
        // from the value we forced; that's read back into `new_pan_px` below and applied to
        // `app.timeline_pan_px` once the whole panel is done (can't mutate `app` mid-loop — the
        // per-track loop below holds an immutable borrow of it), so every other row picks up the
        // new offset next frame. One frame of lag between rows is imperceptible at normal rates.
        let canvas_layout = TimelineCanvasLayout::new(
            ui.available_width(),
            px_per_sec,
            app.active_project().timeline().duration_secs(),
            app.tool,
        );
        let mut new_pan_px: Option<f32> = None;
        // Drags on the ruler/clips themselves need to stop reacting while Hand is active, or
        // they'd win the pointer over the ScrollArea's own background drag-to-pan sensing (egui
        // always gives a more specific child widget priority over its container).

        // Magnetic snap targets (ROADMAP.md P0 item 2): every clip's start/end edge, across
        // every track — collected once per frame up front so both the ruler's playhead drag
        // and the per-clip trim/move drags below can use the same set without re-borrowing
        // `app` mid-loop. Section 56's own Snapping spec: "a timeline-level toggle" via a magnet
        // icon (`app.snap_enabled`, toolbar) — Alt still temporarily *inverts* whichever way
        // that toggle is set, the same quick-override convention most editors layer on top of a
        // persistent snap preference.
        let snap_enabled = app.snap_enabled ^ ui.input(|i| i.modifiers.alt);
        let snap_targets = SnapTargets::from_project(app.active_project());
        let snap_targets_excluding = |exclude_id: u64| snap_targets.excluding(exclude_id);

        let waveform_snap_targets = snap_targets.waveform_points();

        timeline_header(app, ui);

        // Ruler: click or drag to move the playhead. Kept as its own thin strip rather than
        // reusing a track row so scrubbing doesn't depend on there being any tracks yet.
        let (ruler_top, ruler_pan_px) = timeline_ruler(
            app,
            ui,
            RulerLayout {
                track_label_width: TRACK_LABEL_WIDTH,
                ruler_height: RULER_HEIGHT,
                canvas_content_width: canvas_layout.content_width,
                px_per_sec,
                hscroll_source: canvas_layout.hscroll_source,
                hand_active: canvas_layout.hand_active,
                snap_enabled,
                snap_targets: &snap_targets,
            },
        );
        new_pan_px = new_pan_px.or(ruler_pan_px);

        let locale = app.locale;
        let playhead_secs = app.active_project().timeline().playhead_secs;
        let has_clipboard_clip = app.has_clipboard_clip();
        let has_formatting_clipboard = app.has_formatting_clipboard();
        let multi_selected_count = app.multi_selected_clip_ids.len();
        // Clip and overlay widgets cannot mutate `app` while the timeline is borrowed for
        // painting, so collect their intent and apply it once this frame's iteration ends.
        let mut clip_requests = ClipRequests::default();
        let mut track_requests = TrackRequests::default();
        // Set the first time a trim/move drag starts this frame — `app` is immutably borrowed
        // for the whole track/clip iteration below, so the undo snapshot itself is pushed once,
        // after that borrow ends, rather than inline at the drag_started() check.
        let mut drag_started_this_frame = false;
        // `(project_id, asset_id, frame_index)` — see `draw::ThumbnailKey`'s own doc comment for
        // why project_id is part of the key (asset ids are only unique within one project).
        let mut thumbnail_requests: Vec<(u64, u64, i64)> = Vec::new();
        let mut thumbnail_touches: Vec<(u64, u64, i64)> = Vec::new();
        let project_id = app.active_project().id;
        // Keep the horizontal scrollbar at the bottom of the timeline panel even when there
        // are only one or two tracks. The vertical track area fills the remaining height; the
        // footer owns the scrollbar rather than following the last rendered row.
        const HORIZONTAL_SCROLL_CONTENT_HEIGHT: f32 = 2.0;
        let bottom_scroll_height =
            HORIZONTAL_SCROLL_CONTENT_HEIGHT + ui.spacing().scroll.allocated_width();
        let tracks_scroll_height = track_area_height(
            ui.available_height(),
            bottom_scroll_height,
            ui.spacing().item_spacing.y,
        );
        egui::ScrollArea::vertical()
            .id_salt("timeline_tracks_scroll")
            .max_height(tracks_scroll_height)
            .auto_shrink([false, false])
            // The ruler is outside this vertical scroll area. Its default content padding would
            // otherwise shift every track canvas independently of the ruler's canvas.
            .content_margin(egui::Margin::ZERO)
            .show(ui, |ui| {
                for track in &app.active_project().timeline().tracks {
                    let row_height = if app.collapsed_track_ids.contains(&track.id) {
                        COLLAPSED_TRACK_ROW_HEIGHT
                    } else {
                        TRACK_ROW_HEIGHT
                    };
                    ui.horizontal(|ui| {
                        draw_track_header(
                            app,
                            ui,
                            track,
                            row_height,
                            TRACK_LABEL_WIDTH,
                            locale,
                            TrackHeaderRequests {
                                toggle_visibility: &mut track_requests.toggle_visibility,
                                toggle_lock: &mut track_requests.toggle_lock,
                                toggle_collapsed: &mut track_requests.toggle_collapsed,
                                audio_role: &mut track_requests.audio_roles,
                                color_label: &mut track_requests.color_labels,
                                rename: &mut track_requests.renames,
                                duplicate: &mut track_requests.duplicates,
                                move_up: &mut track_requests.move_up,
                                move_down: &mut track_requests.move_down,
                                delete: &mut track_requests.deletes,
                            },
                        );
                        let track_scroll = egui::ScrollArea::horizontal()
                            .id_salt(("timeline_track_hscroll", track.id))
                            .scroll_source(canvas_layout.hscroll_source)
                            .scroll_bar_visibility(
                                egui::containers::scroll_area::ScrollBarVisibility::AlwaysHidden,
                            )
                            .horizontal_scroll_offset(app.timeline_pan_px)
                            .show(ui, |ui| {
                                let (track_rect, _resp) = ui.allocate_exact_size(
                                    egui::vec2(canvas_layout.content_width, row_height),
                                    egui::Sense::hover(),
                                );
                                track_requests.rows.push((track.id, track.kind, track_rect));
                                let painter = ui.painter().clone();
                                for clip in &track.clips {
                                    let x = track_rect.left() + clip.start_secs as f32 * px_per_sec;
                                    // Viewport culling (TIMELINE_PERFORMANCE.md's documented "no track/clip-
                                    // level culling" gap): a clip whose left edge already starts past the
                                    // visible right edge is entirely off-screen — nothing before x=0 is ever
                                    // hidden (the timeline has no horizontal scroll offset of its own, only
                                    // zoom, so the visible window always starts at 0), so this one-sided
                                    // check is enough to skip every off-screen clip's interaction/paint/
                                    // thumbnail cost without risking a false negative on a partially visible
                                    // one. Never skip a clip currently mid-drag/trim, even if the drag has
                                    // carried it past the visible edge — this loop is also what keeps
                                    // `ui.interact`'s click_and_drag/drag response alive for that id every
                                    // frame; skipping it mid-gesture would silently abandon the drag instead
                                    // of just not painting an off-screen clip. Checked against all three
                                    // widget ids this clip can register (body move, start trim, end trim).
                                    let clip_widget_id = ui.id().with(("timeline_clip", clip.id));
                                    let trim_start_id =
                                        ui.id().with(("timeline_clip_trim_start", clip.id));
                                    let trim_end_id =
                                        ui.id().with(("timeline_clip_trim_end", clip.id));
                                    let clip_being_dragged =
                                        ui.ctx().dragged_id().is_some_and(|id| {
                                            id == clip_widget_id
                                                || id == trim_start_id
                                                || id == trim_end_id
                                        });
                                    if x > track_rect.right() && !clip_being_dragged {
                                        continue;
                                    }
                                    let w = (clip.duration_secs() as f32 * px_per_sec).max(3.0);
                                    let clip_rect = egui::Rect::from_min_size(
                                        egui::pos2(x, track_rect.top()),
                                        egui::vec2(w, track_rect.height()),
                                    );
                                    let color = if let Some([r, g, b]) = clip.color_label {
                                        egui::Color32::from_rgb(r, g, b)
                                    } else {
                                        match (track.kind, track.name.as_str()) {
                                            (avcore::timeline::TrackKind::Video, _) => {
                                                theme::MEDIA_VIDEO
                                            }
                                            (avcore::timeline::TrackKind::Audio, "A2") => {
                                                theme::ACCENT_2.gamma_multiply(0.6)
                                            }
                                            (avcore::timeline::TrackKind::Audio, _) => {
                                                theme::AUDIO_TINT
                                            }
                                            // Text/Shape tracks carry text_clips/shape_clips, not clips —
                                            // these arms satisfy exhaustiveness but are never reached at
                                            // runtime.
                                            (avcore::timeline::TrackKind::Text, _) => {
                                                theme::SURFACE_2
                                            }
                                            (avcore::timeline::TrackKind::Shape, _) => {
                                                theme::SURFACE_2
                                            }
                                        }
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

                                    // A locked track (`oca-editor-mock.html`'s lock icon) still allows
                                    // clicking a clip to select/inspect it, just not dragging it — same
                                    // "metadata, not content" split as `visible` above.
                                    let body_response = ui.interact(
                                        clip_rect,
                                        clip_widget_id,
                                        if track.locked {
                                            canvas_layout.canvas_sense(egui::Sense::click())
                                        } else {
                                            canvas_layout
                                                .canvas_sense(egui::Sense::click_and_drag())
                                        },
                                    );
                                    let covers_playhead = clip.start_secs <= playhead_secs
                                        && playhead_secs < clip.start_secs + clip.duration_secs();
                                    show_clip_context_menu(
                                        &body_response,
                                        clip,
                                        track,
                                        locale,
                                        covers_playhead,
                                        has_clipboard_clip,
                                        has_formatting_clipboard,
                                        multi_selected_count,
                                        ClipContextMenuRequests {
                                            clicked: &mut clip_requests.clicked_clip_id,
                                            split_at_playhead: &mut clip_requests.split_at_playhead,
                                            copies: &mut clip_requests.copies,
                                            cuts: &mut clip_requests.cuts,
                                            paste: &mut clip_requests.paste,
                                            merge_into_composite: &mut clip_requests
                                                .merge_into_composite,
                                            detach_audio: &mut clip_requests.detach_audio,
                                            create_compound_clip: &mut clip_requests
                                                .create_compound,
                                            open_nested_sequence: &mut clip_requests
                                                .open_nested_sequence,
                                            speed_ramps: &mut clip_requests.speed_ramps,
                                            custom_speed_ramp: &mut clip_requests.custom_speed_ramp,
                                            copy_formatting: &mut clip_requests.copy_formatting,
                                            paste_formatting: &mut clip_requests.paste_formatting,
                                            color_labels: &mut clip_requests.color_labels,
                                            deletes: &mut clip_requests.deletes,
                                        },
                                    );
                                    let edge_sense = if track.locked {
                                        egui::Sense::hover()
                                    } else {
                                        canvas_layout.canvas_sense(egui::Sense::drag())
                                    };
                                    let left_response =
                                        ui.interact(left_edge_rect, trim_start_id, edge_sense);
                                    let right_response =
                                        ui.interact(right_edge_rect, trim_end_id, edge_sense);
                                    if left_response.hovered()
                                        || left_response.dragged()
                                        || right_response.hovered()
                                        || right_response.dragged()
                                    {
                                        ui.ctx()
                                            .set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                    }
                                    if left_response.drag_started() || right_response.drag_started()
                                    {
                                        drag_started_this_frame = true;
                                    }
                                    if body_response.clicked() {
                                        // Razor Tool (product-decisions addendum, Section 4):
                                        // click a clip to split it at that click's position,
                                        // instead of the normal select-on-click.
                                        if app.tool == EditorTool::Razor {
                                            if let Some(pointer) =
                                                body_response.interact_pointer_pos()
                                            {
                                                let at_secs = ((pointer.x - track_rect.left())
                                                    / px_per_sec)
                                                    .max(0.0)
                                                    as f64;
                                                track_requests
                                                    .razor_splits
                                                    .push((track.id, at_secs));
                                            }
                                        } else if ui.input(|i| i.modifiers.ctrl) {
                                            clip_requests.multi_select.push(clip.id);
                                        } else {
                                            clip_requests.clicked_clip_id = Some(clip.id);
                                        }
                                    }
                                    if body_response.double_clicked() {
                                        if let Some(nested_id) = clip.nested_sequence_id {
                                            clip_requests.open_nested_sequence = Some(nested_id);
                                        }
                                    }
                                    if body_response.drag_started() {
                                        drag_started_this_frame = true;
                                    }
                                    if body_response.dragged() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                        let delta_secs =
                                            (body_response.drag_delta().x / px_per_sec) as f64;
                                        if let Some(pointer) = body_response.interact_pointer_pos()
                                        {
                                            let candidate_start = clip.start_secs + delta_secs;
                                            let mut targets = snap_targets_excluding(clip.id);
                                            targets.push(playhead_secs);
                                            let new_start_secs = if snap_enabled {
                                                snap_move_start(
                                                    candidate_start,
                                                    clip.duration_secs(),
                                                    &targets,
                                                    px_per_sec,
                                                )
                                            } else {
                                                candidate_start
                                            };
                                            clip_requests.drags.push(ClipDrag {
                                                clip_id: clip.id,
                                                source_track_id: track.id,
                                                kind: track.kind,
                                                new_start_secs,
                                                pointer_y: pointer.y,
                                            });
                                        }
                                    }
                                    if let Some(pos) = left_response.interact_pointer_pos() {
                                        let secs = ((pos.x - track_rect.left()) / px_per_sec)
                                            .max(0.0)
                                            as f64;
                                        let mut targets = snap_targets_excluding(clip.id);
                                        targets.push(playhead_secs);
                                        targets.extend(waveform_snap_targets);
                                        let secs = if snap_enabled {
                                            snap_to_nearest(secs, &targets, px_per_sec)
                                        } else {
                                            secs
                                        };
                                        clip_requests.trims.push((clip.id, TrimEdge::Start(secs)));
                                    }
                                    if let Some(pos) = right_response.interact_pointer_pos() {
                                        let secs = ((pos.x - track_rect.left()) / px_per_sec)
                                            .max(0.0)
                                            as f64;
                                        let mut targets = snap_targets_excluding(clip.id);
                                        targets.push(playhead_secs);
                                        targets.extend(waveform_snap_targets);
                                        let secs = if snap_enabled {
                                            snap_to_nearest(secs, &targets, px_per_sec)
                                        } else {
                                            secs
                                        };
                                        clip_requests.trims.push((clip.id, TrimEdge::End(secs)));
                                    }

                                    painter.rect_filled(
                                        clip_rect,
                                        egui::CornerRadius::same(theme::RADIUS_SM),
                                        color,
                                    );
                                    let asset = app
                                        .active_project()
                                        .media_library
                                        .iter()
                                        .find(|a| a.id == clip.asset_id);
                                    if track.kind == avcore::timeline::TrackKind::Video {
                                        if let Some(asset) = asset {
                                            if clip.frozen {
                                                let mut thumbnail_work = ThumbnailDrawWork {
                                                    requests: &mut thumbnail_requests,
                                                    touches: &mut thumbnail_touches,
                                                };
                                                draw_frozen_poster(
                                                    &app.thumbnail_state.thumbnail_textures,
                                                    &painter,
                                                    clip_rect,
                                                    project_id,
                                                    asset,
                                                    clip.source_in_secs,
                                                    &mut thumbnail_work,
                                                );
                                            } else {
                                                let mut thumbnail_work = ThumbnailDrawWork {
                                                    requests: &mut thumbnail_requests,
                                                    touches: &mut thumbnail_touches,
                                                };
                                                draw_filmstrip(
                                                    &app.thumbnail_state.thumbnail_textures,
                                                    &painter,
                                                    clip_rect,
                                                    project_id,
                                                    asset,
                                                    clip.source_in_secs,
                                                    px_per_sec,
                                                    &mut thumbnail_work,
                                                );
                                            }
                                            // A video clip's own embedded audio waveform,
                                            // confirmed missing against a real report: previously
                                            // a Video-track clip drew a filmstrip *or* a waveform,
                                            // never both, so the audio carried by ordinary
                                            // gameplay footage never had any on-timeline visual
                                            // cue at all (only a clip detached onto its own Audio
                                            // track via "Destacar áudio" ever showed one). Drawn
                                            // as a thin band along the bottom edge — skipped
                                            // entirely on a collapsed row (too short to read) or
                                            // an asset with no audio/no cached peaks yet.
                                            let waveform_band_h =
                                                (clip_rect.height() * 0.32).min(18.0);
                                            if asset.has_audio
                                                && clip_rect.height() >= 32.0
                                                && !clip.frozen
                                            {
                                                if let Some(peaks) = &asset.waveform_peaks {
                                                    let waveform_rect = egui::Rect::from_min_size(
                                                        egui::pos2(
                                                            clip_rect.left(),
                                                            clip_rect.bottom() - waveform_band_h,
                                                        ),
                                                        egui::vec2(
                                                            clip_rect.width(),
                                                            waveform_band_h,
                                                        ),
                                                    );
                                                    painter.rect_filled(
                                                        waveform_rect,
                                                        egui::CornerRadius::ZERO,
                                                        theme::SURFACE_2.gamma_multiply(0.75),
                                                    );
                                                    draw_waveform(
                                                        &painter,
                                                        waveform_rect,
                                                        peaks,
                                                        asset.duration_secs,
                                                        clip.source_in_secs..clip.source_out_secs,
                                                        clip.gain_linear(),
                                                        theme::TEXT_PRIMARY.gamma_multiply(0.7),
                                                    );
                                                }
                                            }
                                            // File name + duration, confirmed missing against a
                                            // real report: a plain (non-nested, non-frozen)
                                            // video clip previously drew no identifying text at
                                            // all — every badge above is conditional on some
                                            // special state (crop/mask/flip/chroma-key/speed),
                                            // so an ordinary clip's block carried no name or
                                            // duration whatsoever. Skipped once a nested-sequence
                                            // name is drawn below instead (that's this block's
                                            // own identity for a compound clip) — same unclipped
                                            // `painter.text` convention every other clip badge in
                                            // this loop already uses (e.g. the nested-sequence
                                            // name itself), so a narrow zoomed-out block can
                                            // overflow its own text just like those already do.
                                            if clip.nested_sequence_id.is_none() {
                                                let label = format!(
                                                    "{}  {}",
                                                    asset.file_name,
                                                    avcore::media::format_timecode(
                                                        clip.duration_secs()
                                                    )
                                                );
                                                painter.text(
                                                    clip_rect.left_top() + egui::vec2(4.0, 2.0),
                                                    egui::Align2::LEFT_TOP,
                                                    label,
                                                    egui::FontId::proportional(11.0),
                                                    theme::TEXT_PRIMARY,
                                                );
                                            }
                                        }
                                    } else if let Some(asset) = asset {
                                        if let Some(peaks) = &asset.waveform_peaks {
                                            draw_waveform(
                                                &painter,
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
                                        painter.rect_filled(
                                            clip_rect,
                                            egui::CornerRadius::same(theme::RADIUS_SM),
                                            tint,
                                        );
                                    }
                                    if clip.has_vignette() {
                                        let alpha = (clip.vignette_intensity * 200.0) as u8;
                                        painter.rect_stroke(
                                            clip_rect,
                                            egui::CornerRadius::same(theme::RADIUS_SM),
                                            egui::Stroke::new(
                                                3.0,
                                                egui::Color32::from_black_alpha(alpha),
                                            ),
                                            egui::StrokeKind::Inside,
                                        );
                                    }
                                    if clip.composite_id.is_some() {
                                        painter.rect_stroke(
                                            clip_rect,
                                            egui::CornerRadius::same(theme::RADIUS_SM),
                                            egui::Stroke::new(1.5, theme::ACCENT_2),
                                            egui::StrokeKind::Inside,
                                        );
                                    }
                                    // Compound clip (nested sequence) — no `asset_id`/media-library entry to
                                    // draw a filmstrip/waveform from at all, so its own sequence name is the
                                    // only visual identity this block has.
                                    if let Some(nested_id) = clip.nested_sequence_id {
                                        let nested_name = app
                                            .active_project()
                                            .sequences
                                            .iter()
                                            .find(|s| s.id == nested_id)
                                            .map(|s| s.name.as_str())
                                            .unwrap_or("?");
                                        painter.text(
                                            clip_rect.left_top() + egui::vec2(4.0, 2.0),
                                            egui::Align2::LEFT_TOP,
                                            format!("N: {nested_name}"),
                                            egui::FontId::proportional(11.0),
                                            theme::TEXT_PRIMARY,
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
                                    // Plain ASCII badges rather than "⛶"/"●"/"▢"/"⇄" — same
                                    // confirmed-tofu class (against this app's bundled default
                                    // font) as everything else fixed this session.
                                    if clip.is_cropped() {
                                        painter.text(
                                            clip_rect.right_bottom() + egui::vec2(-3.0, -2.0),
                                            egui::Align2::RIGHT_BOTTOM,
                                            "C",
                                            egui::FontId::proportional(11.0),
                                            theme::TEXT_PRIMARY,
                                        );
                                    }
                                    if clip.is_masked() {
                                        let glyph = match clip.mask_shape {
                                            avcore::timeline::MaskShape::Circle => "O",
                                            avcore::timeline::MaskShape::RoundedRect => "#",
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
                                            "H",
                                            egui::FontId::proportional(11.0),
                                            theme::TEXT_PRIMARY,
                                        );
                                    }
                                    if clip.is_chroma_keyed() {
                                        // "K" (ASCII), not "🟩" -- same tofu class as this
                                        // module's other single-letter clip badges.
                                        painter.text(
                                            clip_rect.center_bottom() + egui::vec2(0.0, -2.0),
                                            egui::Align2::CENTER_BOTTOM,
                                            "K",
                                            egui::FontId::proportional(11.0),
                                            theme::TEXT_PRIMARY,
                                        );
                                    }
                                    draw_keyframe_markers(&painter, clip_rect, clip);
                                    draw_transition_wedge(&painter, clip_rect, clip, px_per_sec);
                                    // Section 51's Clip Selection: "1px petroleum-blue border...
                                    // do not add glow." Was 2px `theme::ACCENT` for a single
                                    // selection; multi-selection used a 2px `theme::ERROR` (red)
                                    // border — confusable with a genuine error/problem indicator,
                                    // not a real error state. Multi-selection now shares the same
                                    // accent color ("shared selection treatment") but keeps a
                                    // thicker 2px stroke as the one distinguishing cue, rather
                                    // than borrowing red for a purely selection-related state.
                                    if app.multi_selected_clip_ids.contains(&clip.id) {
                                        painter.rect_stroke(
                                            clip_rect,
                                            egui::CornerRadius::same(theme::RADIUS_SM),
                                            egui::Stroke::new(2.0, theme::ACCENT),
                                            egui::StrokeKind::Inside,
                                        );
                                    }
                                    if app.selected_clip_id == Some(clip.id) {
                                        painter.rect_stroke(
                                            clip_rect,
                                            egui::CornerRadius::same(theme::RADIUS_SM),
                                            egui::Stroke::new(1.0, theme::ACCENT),
                                            egui::StrokeKind::Inside,
                                        );
                                    }

                                    // Trim Tool (Section 5): highlight the editable edge on
                                    // hover/drag — a color cue on top of the cursor-icon change
                                    // set earlier, since the cursor itself isn't always visible
                                    // in a screenshot/recording.
                                    if left_response.hovered() || left_response.dragged() {
                                        painter.rect_filled(
                                            left_edge_rect,
                                            egui::CornerRadius::ZERO,
                                            theme::ACCENT,
                                        );
                                    }
                                    if right_response.hovered() || right_response.dragged() {
                                        painter.rect_filled(
                                            right_edge_rect,
                                            egui::CornerRadius::ZERO,
                                            theme::ACCENT,
                                        );
                                    }

                                    // Section 5: "while trimming, show ... source timecode;
                                    // sequence timecode; trim duration." Only while actively
                                    // dragging (not on a mere hover) — `interact_pointer_pos`
                                    // already returns `None` outside a drag/press, but the
                                    // explicit `dragged()` check keeps the two edges from ever
                                    // showing this simultaneously off a stale click.
                                    if left_response.dragged() {
                                        if let Some(pos) = left_response.interact_pointer_pos() {
                                            let sequence_secs =
                                                ((pos.x - track_rect.left()) / px_per_sec).max(0.0)
                                                    as f64;
                                            let source_secs = clip.source_in_secs
                                                + (sequence_secs - clip.start_secs);
                                            let trim_duration = (clip.start_secs
                                                + clip.duration_secs())
                                                - sequence_secs;
                                            draw_trim_info(
                                                &painter,
                                                pos,
                                                locale,
                                                source_secs,
                                                sequence_secs,
                                                trim_duration,
                                            );
                                        }
                                    } else if right_response.dragged() {
                                        if let Some(pos) = right_response.interact_pointer_pos() {
                                            let sequence_secs =
                                                ((pos.x - track_rect.left()) / px_per_sec).max(0.0)
                                                    as f64;
                                            let source_secs = clip.source_out_secs
                                                + (sequence_secs
                                                    - (clip.start_secs + clip.duration_secs()));
                                            let trim_duration = sequence_secs - clip.start_secs;
                                            draw_trim_info(
                                                &painter,
                                                pos,
                                                locale,
                                                source_secs,
                                                sequence_secs,
                                                trim_duration,
                                            );
                                        }
                                    }
                                }
                                draw_text_overlays(
                                    ui,
                                    &painter,
                                    track,
                                    track_rect,
                                    px_per_sec,
                                    locale,
                                    canvas_layout.hand_active,
                                    app.selected_text_clip_id,
                                    TextOverlayRequests {
                                        clicked: &mut clip_requests.clicked_text_clip_id,
                                        deletes: &mut clip_requests.delete_text_clips,
                                        drags: &mut clip_requests.text_drags,
                                        trims: &mut clip_requests.text_trims,
                                        drag_started: &mut drag_started_this_frame,
                                    },
                                );
                                draw_shape_overlays(
                                    ui,
                                    &painter,
                                    track,
                                    track_rect,
                                    px_per_sec,
                                    locale,
                                    canvas_layout.hand_active,
                                    app.selected_shape_clip_id,
                                    ShapeOverlayRequests {
                                        clicked: &mut clip_requests.clicked_shape_clip_id,
                                        deletes: &mut clip_requests.delete_shape_clips,
                                        drags: &mut clip_requests.shape_drags,
                                        trims: &mut clip_requests.shape_trims,
                                        drag_started: &mut drag_started_this_frame,
                                    },
                                );
                                draw_playhead(
                                    ui,
                                    track_rect,
                                    app.active_project().timeline().playhead_secs,
                                    px_per_sec,
                                    1.0,
                                );
                            });
                        if (track_scroll.state.offset.x - app.timeline_pan_px).abs() > 0.01 {
                            new_pan_px = Some(track_scroll.state.offset.x);
                        }
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
        // The persistent horizontal pan scrollbar is allocated as a fixed footer after the
        // full-height track area, so it remains at the bottom of the timeline instead of
        // immediately below a short list of clips.
        let (bottom_scroll_rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), bottom_scroll_height),
            egui::Sense::hover(),
        );
        let mut bottom_scroll_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bottom_scroll_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Min)),
        );
        bottom_scroll_ui.horizontal(|ui| {
            ui.allocate_exact_size(egui::vec2(TRACK_LABEL_WIDTH, 2.0), egui::Sense::hover());
            let bottom_scroll = egui::ScrollArea::horizontal()
                .id_salt("timeline_bottom_hscroll")
                .scroll_source(canvas_layout.hscroll_source)
                .scroll_bar_visibility(
                    egui::containers::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                )
                .horizontal_scroll_offset(app.timeline_pan_px)
                .show(ui, |ui| {
                    ui.allocate_exact_size(
                        egui::vec2(
                            canvas_layout.content_width,
                            HORIZONTAL_SCROLL_CONTENT_HEIGHT,
                        ),
                        egui::Sense::hover(),
                    );
                });
            if (bottom_scroll.state.offset.x - app.timeline_pan_px).abs() > 0.01 {
                new_pan_px = Some(bottom_scroll.state.offset.x);
            }
        });
        if let Some(px) = new_pan_px {
            app.timeline_pan_px = px;
        }
        apply_selection_commands(
            app,
            SelectionCommands {
                clicked_clip_id: clip_requests.clicked_clip_id,
                clicked_text_clip_id: clip_requests.clicked_text_clip_id,
                clicked_shape_clip_id: clip_requests.clicked_shape_clip_id,
                delete_text_clip_requests: clip_requests.delete_text_clips,
                delete_shape_clip_requests: clip_requests.delete_shape_clips,
                multi_select_requests: clip_requests.multi_select,
                copy_requests: clip_requests.copies,
                cut_requests: clip_requests.cuts,
                copy_formatting_requests: clip_requests.copy_formatting,
                paste_formatting_requests: clip_requests.paste_formatting,
                paste_requested: clip_requests.paste,
                merge_into_composite_requested: clip_requests.merge_into_composite,
                create_compound_clip_requested: clip_requests.create_compound,
                open_nested_sequence_request: clip_requests.open_nested_sequence,
            },
        );
        apply_track_commands(
            app,
            TrackCommands {
                toggle_visibility: track_requests.toggle_visibility,
                toggle_lock: track_requests.toggle_lock,
                toggle_collapsed: track_requests.toggle_collapsed,
                audio_roles: track_requests.audio_roles,
                color_labels: track_requests.color_labels,
                renames: track_requests.renames,
                duplicates: track_requests.duplicates,
                move_up: track_requests.move_up,
                move_down: track_requests.move_down,
                razor_splits: track_requests.razor_splits,
                deletes: track_requests.deletes,
            },
        );
        apply_clip_commands(
            app,
            ClipCommands {
                deletes: clip_requests.deletes,
                color_labels: clip_requests.color_labels,
                detach_audio: clip_requests.detach_audio,
                speed_ramps: clip_requests.speed_ramps,
                custom_speed_ramp: clip_requests.custom_speed_ramp,
                split_at_playhead: clip_requests.split_at_playhead,
            },
        );
        if drag_started_this_frame {
            app.push_undo_snapshot();
        }
        apply_clip_interactions(
            app,
            clip_requests.trims,
            clip_requests.text_trims,
            clip_requests.shape_trims,
            clip_requests.text_drags,
            clip_requests.shape_drags,
            clip_requests.drags,
            &track_requests.rows,
        );
        apply_thumbnail_work(
            app,
            &thumbnail_touches,
            thumbnail_requests_allowed,
            thumbnail_requests,
        );
        // An asset dragged out of the media library and released somewhere at or below the
        // ruler: whichever track row's Y-range the pointer landed on becomes the preferred
        // drop target (`App::add_asset_to_timeline_at` falls back to a matching-kind track
        // if that row's kind doesn't match the asset, same as a cross-track clip move). A
        // release above the ruler means the drag never reached the timeline at all, so it's
        // ignored rather than silently appending.
        apply_pending_asset_drop(app, ruler_top, px_per_sec, &track_requests.rows);
    });
}

#[cfg(test)]
mod timeline_panel_test;
