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

mod draw;
mod snap;
mod track_header;

use eframe::egui::{self, RichText};

use crate::app::{App, EditorTool};
use crate::components;
use crate::i18n::Text;
use crate::icons;
use crate::theme;

/// Bounds for `App::timeline_px_per_sec` — tight enough to stay readable, loose enough to
/// go from several-projects-wide overview down to frame-accurate editing.
const MIN_PX_PER_SEC: f32 = 0.5;
const MAX_PX_PER_SEC: f32 = 60.0;
/// How long `timeline_px_per_sec` must sit still before new filmstrip thumbnail requests
/// (`draw::draw_filmstrip`'s cache misses) are allowed through again — see
/// `App::timeline_zoom_changed_at`'s own doc comment for why an active zoom drag needs this at
/// all. Short enough that zooming still feels responsive once it stops, long enough to skip
/// every discarded-a-frame-later request during a normal scroll-wheel/pinch zoom gesture.
const TIMELINE_THUMBNAIL_ZOOM_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(150);
const TRACK_LABEL_WIDTH: f32 = 86.0;
/// Height of the ruler strip. Was 20px — confirmed against a real report as visually broken at
/// that height: the timecode label (`draw::draw_ruler_ticks`) and the marker/playhead triangle
/// heads (`draw::draw_marker_ticks`/`draw::draw_playhead`) both anchor to the ruler's *top* 8px,
/// while the tick vline itself sits at the very bottom — 20px wasn't enough room for the top
/// triangle band and the label below it to avoid overlapping, so a timecode label routinely
/// collided with (or was fully covered by) a marker/playhead triangle sitting at the same x.
/// 28px gives the label its own dedicated band between the triangles and the tick line, with no
/// code change needed in `draw_marker_ticks`/`draw_playhead` themselves (both already anchor
/// purely off `rect.top()`, so they don't need to know the ruler grew taller).
const RULER_HEIGHT: f32 = 28.0;
/// Height of one track row, header and clip content alike. Was 26-28px — tall enough for a
/// label but too thin to make the filmstrip thumbnails (`draw::draw_filmstrip`, which sizes its
/// tiles to `track_rect.height()`) or a waveform actually useful at a glance. Doubled.
const TRACK_ROW_HEIGHT: f32 = 56.0;
/// Height of a track row toggled collapsed via its header's expand/collapse button
/// (`App::collapsed_track_ids`) — matches the mockup's `< >` track-header control. Thin enough
/// to lose the filmstrip/waveform detail `TRACK_ROW_HEIGHT` exists for, but still tall enough to
/// read the track name and stay clickable.
const COLLAPSED_TRACK_ROW_HEIGHT: f32 = 22.0;

/// Fixed color-label swatches offered in the clip/track "Rótulo de cor" context menu — per
/// `spec/ROADMAP.md` P4 item 27, matching Premiere/DaVinci/FCP's own fixed-palette convention
/// (a free color picker would let two clips end up with visually indistinguishable colors,
/// defeating the "recognize at a glance" point of a label).
pub(super) const CLIP_COLOR_LABEL_PALETTE: &[[u8; 3]] = &[
    [229, 83, 83],   // red
    [230, 145, 56],  // orange
    [230, 200, 56],  // yellow
    [96, 189, 104],  // green
    [86, 156, 214],  // blue
    [178, 108, 219], // purple
];

use draw::{
    color_filter_tint, draw_filmstrip, draw_frozen_poster, draw_keyframe_markers,
    draw_marker_ticks, draw_playhead, draw_ruler_ticks, draw_transition_wedge, draw_trim_info,
    draw_waveform, shape_kind_glyph, thumbnail_requests_settled, ThumbnailDrawWork,
};
use snap::{snap_move_start, snap_to_nearest, ClipDrag, SnapTargets};
use track_header::{audio_role_icon, audio_role_label};

/// Which edge of a timeline clip a drag targets — see the trim handling in `timeline_panel`.
enum TrimEdge {
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
        let thumbnail_requests_allowed = thumbnail_requests_settled(
            app.timeline_zoom_changed_at,
            now,
            TIMELINE_THUMBNAIL_ZOOM_DEBOUNCE,
        );
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
        let hand_active = app.tool == EditorTool::Hand;
        let timeline_duration_secs = app.active_project().timeline().duration_secs();
        let canvas_visible_width = (ui.available_width() - TRACK_LABEL_WIDTH).max(0.0);
        // +200pt of slack past the last clip, so there's always a little room to pan past the
        // end of the edit instead of hard-stopping exactly at it.
        let canvas_content_width =
            (timeline_duration_secs as f32 * px_per_sec + 200.0).max(canvas_visible_width);
        let hscroll_source = egui::containers::scroll_area::ScrollSource {
            drag: if hand_active {
                egui::containers::scroll_area::DragScroll::Always
            } else {
                egui::containers::scroll_area::DragScroll::OnTouch
            },
            ..Default::default()
        };
        let mut new_pan_px: Option<f32> = None;
        // Drags on the ruler/clips themselves need to stop reacting while Hand is active, or
        // they'd win the pointer over the ScrollArea's own background drag-to-pan sensing (egui
        // always gives a more specific child widget priority over its container).
        let canvas_sense = |normal: egui::Sense| {
            if hand_active {
                egui::Sense::hover()
            } else {
                normal
            }
        };

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

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(Text::Timeline.tr(app.locale))
                    .size(11.0)
                    .color(theme::TEXT_MUTED),
            );
            ui.add_space(theme::SPACE_MD);
            // Sequence tabs now live in the Timeline panel's own header row, matching the
            // mockup's "TIMELINE  Interview ✕ +" single strip (confirmed via a real screenshot)
            // — previously their own separate row above the whole three-column body.
            super::sequence_tab_bar(app, ui);
            // A visible zoom affordance for `timeline_px_per_sec` — until now only reachable via
            // Ctrl+scroll, with no on-screen indicator of the current zoom level at all. Matches
            // `oca-editor-mock.html`'s `.tl-zoom` slider in the timeline toolbar's right corner.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Discrete zoom in/out buttons, confirmed missing against a real report that
                // zooming felt "very limited" — Ctrl+scroll requires the pointer to sit exactly
                // over the timeline while holding a modifier, and the logarithmic slider's own
                // usable drag range is only a few pixels wide (most of its length maps to either
                // extreme), so neither was a practical way to reach a specific zoom level on
                // demand. A fixed multiplicative step (not additive — `MIN_PX_PER_SEC..=
                // MAX_PX_PER_SEC` spans two orders of magnitude, so a constant +/-N px/sec step
                // would feel instant near the top of the range and glacial near the bottom).
                const ZOOM_STEP_FACTOR: f32 = 1.25;
                if ui
                    .button(RichText::new("+").size(13.0))
                    .on_hover_text(Text::TimelineZoomIn.tr(app.locale))
                    .clicked()
                {
                    app.timeline_px_per_sec = (app.timeline_px_per_sec * ZOOM_STEP_FACTOR)
                        .clamp(MIN_PX_PER_SEC, MAX_PX_PER_SEC);
                }
                ui.add(
                    egui::Slider::new(
                        &mut app.timeline_px_per_sec,
                        MIN_PX_PER_SEC..=MAX_PX_PER_SEC,
                    )
                    .show_value(false)
                    .logarithmic(true),
                );
                if ui
                    .button(RichText::new("-").size(13.0))
                    .on_hover_text(Text::TimelineZoomOut.tr(app.locale))
                    .clicked()
                {
                    app.timeline_px_per_sec = (app.timeline_px_per_sec / ZOOM_STEP_FACTOR)
                        .clamp(MIN_PX_PER_SEC, MAX_PX_PER_SEC);
                }
                ui.label(
                    RichText::new(Text::TimelineZoom.tr(app.locale))
                        .size(11.0)
                        .color(theme::TEXT_MUTED),
                );
            });
        });
        ui.separator();

        // Ruler: click or drag to move the playhead. Kept as its own thin strip rather than
        // reusing a track row so scrubbing doesn't depend on there being any tracks yet.
        let mut ruler_top = 0.0_f32;
        ui.horizontal(|ui| {
            ui.add_space(TRACK_LABEL_WIDTH);
            // The visible pan scrollbar lives at the bottom of the whole timeline component
            // (below every track row, see the dedicated strip after the tracks ScrollArea below)
            // — a first attempt put it here on the ruler instead, which read wrong (a scrollbar
            // above the clips it scrolls, confirmed via a real report). Stays hidden here.
            let ruler_scroll = egui::ScrollArea::horizontal()
                .id_salt("timeline_ruler_hscroll")
                .scroll_source(hscroll_source)
                .scroll_bar_visibility(
                    egui::containers::scroll_area::ScrollBarVisibility::AlwaysHidden,
                )
                .horizontal_scroll_offset(app.timeline_pan_px)
                .show(ui, |ui| {
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(canvas_content_width, RULER_HEIGHT),
                        canvas_sense(egui::Sense::click_and_drag()),
                    );
                    ruler_top = rect.top();
                    ui.painter().rect_filled(rect, 0, theme::SURFACE_2);
                    draw_ruler_ticks(ui.painter(), rect, px_per_sec);
                    if let Some(pos) = response.interact_pointer_pos() {
                        let secs = ((pos.x - rect.left()) / px_per_sec).max(0.0) as f64;
                        let secs = if snap_enabled {
                            snap_to_nearest(secs, &snap_targets.all(), px_per_sec)
                        } else {
                            secs
                        };
                        app.active_project_mut().timeline_mut().playhead_secs = secs;
                    }
                    let markers = app.active_project().timeline().markers.clone();
                    if let Some(seek_secs) = draw_marker_ticks(ui, rect, &markers, px_per_sec) {
                        app.active_project_mut().timeline_mut().playhead_secs = seek_secs;
                    }
                    draw_playhead(
                        ui,
                        rect,
                        app.active_project().timeline().playhead_secs,
                        px_per_sec,
                        2.0,
                    );
                });
            if (ruler_scroll.state.offset.x - app.timeline_pan_px).abs() > 0.01 {
                new_pan_px = Some(ruler_scroll.state.offset.x);
            }
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
        let mut clip_color_label_requests: Vec<(u64, Option<[u8; 3]>)> = Vec::new();
        let mut detach_audio_requests: Vec<u64> = Vec::new();
        let mut speed_ramp_requests: Vec<(u64, f32, f32)> = Vec::new();
        let mut speed_ramp_custom_request: Option<u64> = None;
        let mut create_compound_clip_requested = false;
        let mut open_nested_sequence_request: Option<u64> = None;
        let mut paste_requested = false;
        let mut merge_into_composite_requested = false;
        let mut split_at_playhead_requested = false;
        let mut trim_requests: Vec<(u64, TrimEdge)> = Vec::new();
        let mut clip_drags: Vec<ClipDrag> = Vec::new();
        // Text/shape overlay clips get the same drag-to-move/drag-to-trim treatment as video/
        // audio clips (previously click-only — no way to reposition or resize a text/shape
        // block's duration by dragging its edges at all). Same-track only: unlike video/audio
        // clips, a text/shape overlay never moves across tracks by drag (no cross-track drop
        // target resolution exists for either kind).
        let mut text_clip_drags: Vec<(u64, f64)> = Vec::new();
        let mut shape_clip_drags: Vec<(u64, f64)> = Vec::new();
        let mut text_trim_requests: Vec<(u64, TrimEdge)> = Vec::new();
        let mut shape_trim_requests: Vec<(u64, TrimEdge)> = Vec::new();
        let mut track_rows: Vec<(u64, avcore::timeline::TrackKind, egui::Rect)> = Vec::new();
        let mut toggle_track_visibility_requests: Vec<u64> = Vec::new();
        let mut toggle_track_lock_requests: Vec<u64> = Vec::new();
        let mut toggle_track_collapsed_requests: Vec<u64> = Vec::new();
        let mut track_audio_role_requests: Vec<(u64, avcore::AudioRole)> = Vec::new();
        let mut track_color_label_requests: Vec<(u64, Option<[u8; 3]>)> = Vec::new();
        let mut track_rename_requests: Vec<(u64, String)> = Vec::new();
        let mut track_duplicate_requests: Vec<u64> = Vec::new();
        let mut track_move_up_requests: Vec<u64> = Vec::new();
        let mut track_move_down_requests: Vec<u64> = Vec::new();
        let mut track_delete_requests: Vec<(u64, String)> = Vec::new();
        let mut razor_split_requests: Vec<(u64, f64)> = Vec::new();
        // Set the first time a trim/move drag starts this frame — `app` is immutably borrowed
        // for the whole track/clip iteration below, so the undo snapshot itself is pushed once,
        // after that borrow ends, rather than inline at the drag_started() check.
        let mut drag_started_this_frame = false;
        // `(project_id, asset_id, frame_index)` — see `draw::ThumbnailKey`'s own doc comment for
        // why project_id is part of the key (asset ids are only unique within one project).
        let mut thumbnail_requests: Vec<(u64, u64, i64)> = Vec::new();
        let mut thumbnail_touches: Vec<(u64, u64, i64)> = Vec::new();
        let project_id = app.active_project().id;
        egui::ScrollArea::vertical()
            .id_salt("timeline_tracks_scroll")
            .show(ui, |ui| {
                for track in &app.active_project().timeline().tracks {
                    let row_height = if app.collapsed_track_ids.contains(&track.id) {
                        COLLAPSED_TRACK_ROW_HEIGHT
                    } else {
                        TRACK_ROW_HEIGHT
                    };
                    ui.horizontal(|ui| {
                        // Track header: visibility toggle + name.
                        ui.allocate_ui_with_layout(
                            egui::vec2(TRACK_LABEL_WIDTH, row_height),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                let track_id = track.id;
                                let visible = track.visible;
                                // Audio tracks read as "muted/unmuted" (a speaker glyph) rather than
                                // "hidden/shown" (an eye) — same `visible` flag underneath, since an
                                // invisible video track and a muted audio track are the same "doesn't
                                // contribute to preview/export" concept. No Lucide speaker icon is
                                // vendored (`spec/architecture/editor-ui-visual-redesign.md`'s Icon
                                // set section only covers `eye`/`eye-off`) — and the raw "🔊"/"🔇"
                                // emoji this used to fall back to is the same tofu class already
                                // fixed elsewhere this session, so audio tracks now reuse the same
                                // eye/eye-off icon as video tracks rather than a distinct glyph.
                                let is_audio = track.kind == avcore::timeline::TrackKind::Audio;
                                let (eye, eye_family) = if visible {
                                    (icons::EYE_STR, Some(icons::family()))
                                } else {
                                    (icons::EYE_OFF_STR, Some(icons::family()))
                                };
                                let tooltip = match (is_audio, visible) {
                                    (true, true) => Text::TrackMute.tr(locale),
                                    (true, false) => Text::TrackUnmute.tr(locale),
                                    (false, true) => Text::TrackHide.tr(locale),
                                    (false, false) => Text::TrackShow.tr(locale),
                                };
                                if components::icon_button(
                                    ui,
                                    eye,
                                    tooltip,
                                    components::IconButtonOpts {
                                        family: eye_family,
                                        ..Default::default()
                                    },
                                )
                                .clicked()
                                {
                                    toggle_track_visibility_requests.push(track_id);
                                }
                                let locked = track.locked;
                                let lock_glyph = if locked {
                                    icons::LOCK_STR
                                } else {
                                    icons::LOCK_OPEN_STR
                                };
                                let lock_tooltip = if locked {
                                    Text::TrackUnlock.tr(locale)
                                } else {
                                    Text::TrackLock.tr(locale)
                                };
                                // Section 45's Track Lock spec: "Visual: lock icon becomes
                                // active" — was glyph-only (open/closed padlock), same rest color
                                // regardless of state, before this fix.
                                if components::icon_button(
                                    ui,
                                    lock_glyph,
                                    lock_tooltip,
                                    components::IconButtonOpts {
                                        family: Some(icons::family()),
                                        color: locked.then_some(theme::ACCENT),
                                        ..Default::default()
                                    },
                                )
                                .clicked()
                                {
                                    toggle_track_lock_requests.push(track_id);
                                }
                                // Matches the mockup's `< >` track-header control (confirmed via
                                // a real screenshot) — collapses/expands just this track's row
                                // height. Plain ASCII ("v"/">"), not the Geometric-Shapes
                                // chevrons the mockup itself uses — same confirmed-tofu class
                                // (against this app's bundled default font) as every other icon
                                // fixed this session.
                                let collapsed = app.collapsed_track_ids.contains(&track_id);
                                let (collapse_glyph, collapse_tooltip) = if collapsed {
                                    (">", Text::TrackExpand.tr(locale))
                                } else {
                                    ("v", Text::TrackCollapse.tr(locale))
                                };
                                if components::icon_button(
                                    ui,
                                    collapse_glyph,
                                    collapse_tooltip,
                                    components::IconButtonOpts::default(),
                                )
                                .clicked()
                                {
                                    toggle_track_collapsed_requests.push(track_id);
                                }
                                let name_response = ui.add(
                                    egui::Label::new(RichText::new(&track.name).size(11.0).color(
                                        if let Some([r, g, b]) = track.color_label {
                                            egui::Color32::from_rgb(r, g, b)
                                        } else if visible {
                                            theme::TEXT_SECONDARY
                                        } else {
                                            theme::TEXT_MUTED
                                        },
                                    ))
                                    .truncate()
                                    .sense(egui::Sense::click()),
                                );
                                name_response.context_menu(|ui| {
                                    // Section 49's Track More Menu — Rename/Duplicate/Delete/
                                    // Move Up/Move Down, added to this existing track-color
                                    // context menu rather than a second right-click surface.
                                    if ui.button(Text::TrackCtxRename.tr(locale)).clicked() {
                                        track_rename_requests.push((track_id, track.name.clone()));
                                        ui.close();
                                    }
                                    if ui.button(Text::TrackCtxDuplicate.tr(locale)).clicked() {
                                        track_duplicate_requests.push(track_id);
                                        ui.close();
                                    }
                                    if ui.button(Text::TrackCtxMoveUp.tr(locale)).clicked() {
                                        track_move_up_requests.push(track_id);
                                        ui.close();
                                    }
                                    if ui.button(Text::TrackCtxMoveDown.tr(locale)).clicked() {
                                        track_move_down_requests.push(track_id);
                                        ui.close();
                                    }
                                    if ui.button(Text::TrackCtxDelete.tr(locale)).clicked() {
                                        track_delete_requests.push((track_id, track.name.clone()));
                                        ui.close();
                                    }
                                    ui.separator();
                                    for &[r, g, b] in CLIP_COLOR_LABEL_PALETTE {
                                        let swatch = egui::Color32::from_rgb(r, g, b);
                                        if ui.add(egui::Button::new("  ").fill(swatch)).clicked() {
                                            track_color_label_requests
                                                .push((track_id, Some([r, g, b])));
                                            ui.close();
                                        }
                                    }
                                    ui.separator();
                                    if ui
                                        .button(Text::ContextMenuColorLabelClear.tr(locale))
                                        .clicked()
                                    {
                                        track_color_label_requests.push((track_id, None));
                                        ui.close();
                                    }
                                });
                                // D2 (`spec/architecture/differentiators.md`): which audio source
                                // this track carries, if any — Text/Shape tracks never carry audio,
                                // so they don't get the picker at all.
                                if matches!(
                                    track.kind,
                                    avcore::timeline::TrackKind::Video
                                        | avcore::timeline::TrackKind::Audio
                                ) {
                                    let mut role = track.audio_role;
                                    egui::ComboBox::from_id_salt(("track_audio_role", track_id))
                                        .selected_text(audio_role_icon(role))
                                        .width(28.0)
                                        .show_ui(ui, |ui| {
                                            for candidate in [
                                                avcore::AudioRole::Unspecified,
                                                avcore::AudioRole::GameAudio,
                                                avcore::AudioRole::Mic,
                                                avcore::AudioRole::Music,
                                            ] {
                                                ui.selectable_value(
                                                    &mut role,
                                                    candidate,
                                                    audio_role_icon(candidate),
                                                );
                                            }
                                        })
                                        .response
                                        .on_hover_text(audio_role_label(role, locale));
                                    if role != track.audio_role {
                                        track_audio_role_requests.push((track_id, role));
                                    }
                                }
                            },
                        );
                        let track_scroll = egui::ScrollArea::horizontal()
                            .id_salt(("timeline_track_hscroll", track.id))
                            .scroll_source(hscroll_source)
                            .scroll_bar_visibility(
                                egui::containers::scroll_area::ScrollBarVisibility::AlwaysHidden,
                            )
                            .horizontal_scroll_offset(app.timeline_pan_px)
                            .show(ui, |ui| {
                                let (track_rect, _resp) = ui.allocate_exact_size(
                                    egui::vec2(canvas_content_width, row_height),
                                    egui::Sense::hover(),
                                );
                                track_rows.push((track.id, track.kind, track_rect));
                                let painter = ui.painter();
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
                                            canvas_sense(egui::Sense::click())
                                        } else {
                                            canvas_sense(egui::Sense::click_and_drag())
                                        },
                                    );
                                    let covers_playhead = clip.start_secs <= playhead_secs
                                        && playhead_secs < clip.start_secs + clip.duration_secs();
                                    body_response.context_menu(|ui| {
                                        clicked_clip_id = Some(clip.id);
                                        if ui
                                            .add_enabled(
                                                covers_playhead,
                                                egui::Button::new(
                                                    Text::ContextMenuSplit.tr(locale),
                                                ),
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
                                                egui::Button::new(
                                                    Text::ContextMenuPaste.tr(locale),
                                                ),
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
                                                egui::Button::new(
                                                    Text::MergeIntoComposite.tr(locale),
                                                ),
                                            )
                                            .clicked()
                                        {
                                            merge_into_composite_requested = true;
                                            ui.close();
                                        }
                                        if track.kind == avcore::timeline::TrackKind::Video
                                            && ui
                                                .button(Text::ContextMenuDetachAudio.tr(locale))
                                                .clicked()
                                        {
                                            detach_audio_requests.push(clip.id);
                                            ui.close();
                                        }
                                        if track.kind == avcore::timeline::TrackKind::Video
                                            && clip.nested_sequence_id.is_none()
                                            && ui
                                                .button(
                                                    Text::ContextMenuCreateCompoundClip.tr(locale),
                                                )
                                                .clicked()
                                        {
                                            clicked_clip_id = Some(clip.id);
                                            create_compound_clip_requested = true;
                                            ui.close();
                                        }
                                        if let Some(nested_id) = clip.nested_sequence_id {
                                            if ui
                                                .button(
                                                    Text::ContextMenuOpenCompoundClip.tr(locale),
                                                )
                                                .clicked()
                                            {
                                                open_nested_sequence_request = Some(nested_id);
                                                ui.close();
                                            }
                                        }
                                        ui.menu_button(
                                            Text::ContextMenuSpeedRamp.tr(locale),
                                            |ui| {
                                                if ui
                                                    .button(Text::SpeedRampSlowToFast.tr(locale))
                                                    .clicked()
                                                {
                                                    speed_ramp_requests.push((clip.id, 0.5, 2.0));
                                                    ui.close();
                                                }
                                                if ui
                                                    .button(Text::SpeedRampFastToSlow.tr(locale))
                                                    .clicked()
                                                {
                                                    speed_ramp_requests.push((clip.id, 2.0, 0.5));
                                                    ui.close();
                                                }
                                                ui.separator();
                                                if ui
                                                    .button(Text::SpeedRampCustom.tr(locale))
                                                    .clicked()
                                                {
                                                    speed_ramp_custom_request = Some(clip.id);
                                                    ui.close();
                                                }
                                            },
                                        );
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
                                                egui::Button::new(
                                                    Text::ContextMenuPasteFormatting.tr(locale),
                                                ),
                                            )
                                            .clicked()
                                        {
                                            paste_formatting_requests.push(clip.id);
                                            ui.close();
                                        }
                                        ui.separator();
                                        ui.menu_button(
                                            Text::ContextMenuColorLabel.tr(locale),
                                            |ui| {
                                                for &[r, g, b] in CLIP_COLOR_LABEL_PALETTE {
                                                    let swatch = egui::Color32::from_rgb(r, g, b);
                                                    if ui
                                                        .add(egui::Button::new("  ").fill(swatch))
                                                        .clicked()
                                                    {
                                                        clip_color_label_requests
                                                            .push((clip.id, Some([r, g, b])));
                                                        ui.close();
                                                    }
                                                }
                                                ui.separator();
                                                if ui
                                                    .button(
                                                        Text::ContextMenuColorLabelClear.tr(locale),
                                                    )
                                                    .clicked()
                                                {
                                                    clip_color_label_requests.push((clip.id, None));
                                                    ui.close();
                                                }
                                            },
                                        );
                                        ui.separator();
                                        if ui.button(Text::ContextMenuDelete.tr(locale)).clicked() {
                                            delete_requests.push(clip.id);
                                            ui.close();
                                        }
                                    });
                                    let edge_sense = if track.locked {
                                        egui::Sense::hover()
                                    } else {
                                        canvas_sense(egui::Sense::drag())
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
                                                razor_split_requests.push((track.id, at_secs));
                                            }
                                        } else if ui.input(|i| i.modifiers.ctrl) {
                                            multi_select_requests.push(clip.id);
                                        } else {
                                            clicked_clip_id = Some(clip.id);
                                        }
                                    }
                                    if body_response.double_clicked() {
                                        if let Some(nested_id) = clip.nested_sequence_id {
                                            open_nested_sequence_request = Some(nested_id);
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
                                            clip_drags.push(ClipDrag {
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
                                        trim_requests.push((clip.id, TrimEdge::Start(secs)));
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
                                        trim_requests.push((clip.id, TrimEdge::End(secs)));
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
                                                    painter,
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
                                                    painter,
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
                                                        painter,
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
                                    draw_keyframe_markers(painter, clip_rect, clip);
                                    draw_transition_wedge(painter, clip_rect, clip, px_per_sec);
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
                                                painter,
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
                                                painter,
                                                pos,
                                                locale,
                                                source_secs,
                                                sequence_secs,
                                                trim_duration,
                                            );
                                        }
                                    }
                                }
                                // Render text clips for text tracks as solid-color blocks with text label.
                                if track.kind == avcore::timeline::TrackKind::Text {
                                    for tc in &track.text_clips {
                                        let x =
                                            track_rect.left() + tc.start_secs as f32 * px_per_sec;
                                        let tc_widget_id =
                                            ui.id().with(("timeline_text_clip", tc.id));
                                        let tc_trim_start_id =
                                            ui.id().with(("timeline_text_clip_trim_start", tc.id));
                                        let tc_trim_end_id =
                                            ui.id().with(("timeline_text_clip_trim_end", tc.id));
                                        let tc_being_dragged =
                                            ui.ctx().dragged_id().is_some_and(|id| {
                                                id == tc_widget_id
                                                    || id == tc_trim_start_id
                                                    || id == tc_trim_end_id
                                            });
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
                                        let tc_left_edge_rect = egui::Rect::from_min_size(
                                            tc_rect.min,
                                            egui::vec2(edge_w, tc_rect.height()),
                                        );
                                        let tc_right_edge_rect = egui::Rect::from_min_size(
                                            egui::pos2(tc_rect.right() - edge_w, tc_rect.top()),
                                            egui::vec2(edge_w, tc_rect.height()),
                                        );
                                        let tc_response = ui.interact(
                                            tc_rect,
                                            tc_widget_id,
                                            if track.locked {
                                                canvas_sense(egui::Sense::click())
                                            } else {
                                                canvas_sense(egui::Sense::click_and_drag())
                                            },
                                        );
                                        tc_response.context_menu(|ui| {
                                            if ui
                                                .button(Text::ContextMenuDelete.tr(locale))
                                                .clicked()
                                            {
                                                delete_text_clip_requests.push(tc.id);
                                                ui.close();
                                            }
                                        });
                                        if tc_response.clicked() {
                                            clicked_text_clip_id = Some(tc.id);
                                        }
                                        if tc_response.drag_started() {
                                            drag_started_this_frame = true;
                                        }
                                        if tc_response.dragged() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                            let delta_secs =
                                                (tc_response.drag_delta().x / px_per_sec) as f64;
                                            text_clip_drags
                                                .push((tc.id, tc.start_secs + delta_secs));
                                        }
                                        let tc_edge_sense = if track.locked {
                                            egui::Sense::hover()
                                        } else {
                                            canvas_sense(egui::Sense::drag())
                                        };
                                        let tc_left_response = ui.interact(
                                            tc_left_edge_rect,
                                            tc_trim_start_id,
                                            tc_edge_sense,
                                        );
                                        let tc_right_response = ui.interact(
                                            tc_right_edge_rect,
                                            tc_trim_end_id,
                                            tc_edge_sense,
                                        );
                                        if tc_left_response.hovered()
                                            || tc_left_response.dragged()
                                            || tc_right_response.hovered()
                                            || tc_right_response.dragged()
                                        {
                                            ui.ctx().set_cursor_icon(
                                                egui::CursorIcon::ResizeHorizontal,
                                            );
                                        }
                                        if tc_left_response.drag_started()
                                            || tc_right_response.drag_started()
                                        {
                                            drag_started_this_frame = true;
                                        }
                                        if let Some(pos) = tc_left_response.interact_pointer_pos() {
                                            let secs = ((pos.x - track_rect.left()) / px_per_sec)
                                                .max(0.0)
                                                as f64;
                                            text_trim_requests.push((tc.id, TrimEdge::Start(secs)));
                                        }
                                        if let Some(pos) = tc_right_response.interact_pointer_pos()
                                        {
                                            let secs = ((pos.x - track_rect.left()) / px_per_sec)
                                                .max(0.0)
                                                as f64;
                                            text_trim_requests.push((tc.id, TrimEdge::End(secs)));
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
                                        let label_pos =
                                            tc_rect.left_center() + egui::vec2(4.0, 0.0);
                                        painter.text(
                                            label_pos,
                                            egui::Align2::LEFT_CENTER,
                                            &tc.text,
                                            egui::FontId::proportional(11.0),
                                            egui::Color32::WHITE,
                                        );
                                        if tc_left_response.hovered() || tc_left_response.dragged()
                                        {
                                            painter.rect_filled(
                                                tc_left_edge_rect,
                                                egui::CornerRadius::ZERO,
                                                theme::ACCENT,
                                            );
                                        }
                                        if tc_right_response.hovered()
                                            || tc_right_response.dragged()
                                        {
                                            painter.rect_filled(
                                                tc_right_edge_rect,
                                                egui::CornerRadius::ZERO,
                                                theme::ACCENT,
                                            );
                                        }
                                        // Selection ring
                                        if app.selected_text_clip_id == Some(tc.id) {
                                            painter.rect_stroke(
                                                tc_rect,
                                                egui::CornerRadius::same(theme::RADIUS_SM),
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
                                        let x =
                                            track_rect.left() + sc.start_secs as f32 * px_per_sec;
                                        let sc_widget_id =
                                            ui.id().with(("timeline_shape_clip", sc.id));
                                        let sc_trim_start_id =
                                            ui.id().with(("timeline_shape_clip_trim_start", sc.id));
                                        let sc_trim_end_id =
                                            ui.id().with(("timeline_shape_clip_trim_end", sc.id));
                                        let sc_being_dragged =
                                            ui.ctx().dragged_id().is_some_and(|id| {
                                                id == sc_widget_id
                                                    || id == sc_trim_start_id
                                                    || id == sc_trim_end_id
                                            });
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
                                        let sc_left_edge_rect = egui::Rect::from_min_size(
                                            sc_rect.min,
                                            egui::vec2(edge_w, sc_rect.height()),
                                        );
                                        let sc_right_edge_rect = egui::Rect::from_min_size(
                                            egui::pos2(sc_rect.right() - edge_w, sc_rect.top()),
                                            egui::vec2(edge_w, sc_rect.height()),
                                        );
                                        let sc_response = ui.interact(
                                            sc_rect,
                                            sc_widget_id,
                                            if track.locked {
                                                canvas_sense(egui::Sense::click())
                                            } else {
                                                canvas_sense(egui::Sense::click_and_drag())
                                            },
                                        );
                                        sc_response.context_menu(|ui| {
                                            if ui
                                                .button(Text::ContextMenuDelete.tr(locale))
                                                .clicked()
                                            {
                                                delete_shape_clip_requests.push(sc.id);
                                                ui.close();
                                            }
                                        });
                                        if sc_response.clicked() {
                                            clicked_shape_clip_id = Some(sc.id);
                                        }
                                        if sc_response.drag_started() {
                                            drag_started_this_frame = true;
                                        }
                                        if sc_response.dragged() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                            let delta_secs =
                                                (sc_response.drag_delta().x / px_per_sec) as f64;
                                            shape_clip_drags
                                                .push((sc.id, sc.start_secs + delta_secs));
                                        }
                                        let sc_edge_sense = if track.locked {
                                            egui::Sense::hover()
                                        } else {
                                            canvas_sense(egui::Sense::drag())
                                        };
                                        let sc_left_response = ui.interact(
                                            sc_left_edge_rect,
                                            sc_trim_start_id,
                                            sc_edge_sense,
                                        );
                                        let sc_right_response = ui.interact(
                                            sc_right_edge_rect,
                                            sc_trim_end_id,
                                            sc_edge_sense,
                                        );
                                        if sc_left_response.hovered()
                                            || sc_left_response.dragged()
                                            || sc_right_response.hovered()
                                            || sc_right_response.dragged()
                                        {
                                            ui.ctx().set_cursor_icon(
                                                egui::CursorIcon::ResizeHorizontal,
                                            );
                                        }
                                        if sc_left_response.drag_started()
                                            || sc_right_response.drag_started()
                                        {
                                            drag_started_this_frame = true;
                                        }
                                        if let Some(pos) = sc_left_response.interact_pointer_pos() {
                                            let secs = ((pos.x - track_rect.left()) / px_per_sec)
                                                .max(0.0)
                                                as f64;
                                            shape_trim_requests
                                                .push((sc.id, TrimEdge::Start(secs)));
                                        }
                                        if let Some(pos) = sc_right_response.interact_pointer_pos()
                                        {
                                            let secs = ((pos.x - track_rect.left()) / px_per_sec)
                                                .max(0.0)
                                                as f64;
                                            shape_trim_requests.push((sc.id, TrimEdge::End(secs)));
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
                                        let label_pos =
                                            sc_rect.left_center() + egui::vec2(4.0, 0.0);
                                        painter.text(
                                            label_pos,
                                            egui::Align2::LEFT_CENTER,
                                            shape_kind_glyph(&sc.shape_kind),
                                            egui::FontId::proportional(11.0),
                                            egui::Color32::WHITE,
                                        );
                                        if sc_left_response.hovered() || sc_left_response.dragged()
                                        {
                                            painter.rect_filled(
                                                sc_left_edge_rect,
                                                egui::CornerRadius::ZERO,
                                                theme::ACCENT,
                                            );
                                        }
                                        if sc_right_response.hovered()
                                            || sc_right_response.dragged()
                                        {
                                            painter.rect_filled(
                                                sc_right_edge_rect,
                                                egui::CornerRadius::ZERO,
                                                theme::ACCENT,
                                            );
                                        }
                                        // Selection ring
                                        if app.selected_shape_clip_id == Some(sc.id) {
                                            painter.rect_stroke(
                                                sc_rect,
                                                egui::CornerRadius::same(theme::RADIUS_SM),
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
        // The persistent horizontal pan scrollbar, pinned to the bottom of the whole timeline
        // component (below every track row) — matches where a scrollbar naturally reads,
        // confirmed via a real report that it belonged here, not up on the ruler.
        ui.horizontal(|ui| {
            ui.add_space(TRACK_LABEL_WIDTH);
            let bottom_scroll = egui::ScrollArea::horizontal()
                .id_salt("timeline_bottom_hscroll")
                .scroll_source(hscroll_source)
                .scroll_bar_visibility(
                    egui::containers::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                )
                .horizontal_scroll_offset(app.timeline_pan_px)
                .show(ui, |ui| {
                    ui.allocate_exact_size(
                        egui::vec2(canvas_content_width, 2.0),
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
        if create_compound_clip_requested {
            app.create_compound_clip_from_selected_clip();
        }
        if let Some(nested_id) = open_nested_sequence_request {
            app.open_nested_sequence(nested_id);
        }
        for track_id in toggle_track_visibility_requests {
            app.toggle_track_visibility(track_id);
        }
        for track_id in toggle_track_lock_requests {
            app.toggle_track_locked(track_id);
        }
        for track_id in toggle_track_collapsed_requests {
            if !app.collapsed_track_ids.remove(&track_id) {
                app.collapsed_track_ids.insert(track_id);
            }
        }
        for (track_id, role) in track_audio_role_requests {
            app.set_track_audio_role(track_id, role);
        }
        for (track_id, color_label) in track_color_label_requests {
            app.set_track_color_label(track_id, color_label);
        }
        for (track_id, name) in track_rename_requests {
            app.renaming_track = Some((track_id, name));
        }
        for track_id in track_duplicate_requests {
            app.duplicate_track(track_id);
        }
        for track_id in track_move_up_requests {
            app.move_track_up(track_id);
        }
        for track_id in track_move_down_requests {
            app.move_track_down(track_id);
        }
        for (track_id, at_secs) in razor_split_requests {
            app.split_track_clip_at(track_id, at_secs);
        }
        for (track_id, name) in track_delete_requests {
            // Section 49's own rule: only a track that actually carries content needs
            // confirmation before deleting — an empty track (no clips of any kind) just goes.
            let has_content = app.active_project().timeline().tracks.iter().any(|t| {
                t.id == track_id
                    && (!t.clips.is_empty()
                        || !t.text_clips.is_empty()
                        || !t.shape_clips.is_empty())
            });
            if has_content {
                app.deleting_track = Some((track_id, name));
            } else {
                app.delete_track(track_id);
            }
        }
        for clip_id in delete_requests {
            app.selected_clip_id = Some(clip_id);
            app.delete_selected_clip();
        }
        for (clip_id, color_label) in clip_color_label_requests {
            app.set_clip_color_label(clip_id, color_label);
        }
        for clip_id in detach_audio_requests {
            app.selected_clip_id = Some(clip_id);
            app.detach_audio_from_selected_clip();
        }
        for (clip_id, start_speed, end_speed) in speed_ramp_requests {
            app.selected_clip_id = Some(clip_id);
            app.apply_speed_ramp_to_selected_clip(start_speed, end_speed, 4);
        }
        if let Some(clip_id) = speed_ramp_custom_request {
            app.speed_ramp_dialog = Some((clip_id, 0.5, 2.0, "4".to_string(), false));
        }
        if split_at_playhead_requested {
            app.split_at_playhead();
        }
        if drag_started_this_frame {
            app.push_undo_snapshot();
        }
        // Ripple/Roll (ROADMAP.md P2 item 11) change what an edge drag commits as; every other
        // tool (including Slip/Slide, which act on the clip *body* instead — see the drag loop
        // below) falls back to the same plain trim edge-dragging has always done.
        for (clip_id, edge) in trim_requests {
            match (app.tool, edge) {
                (EditorTool::Ripple, TrimEdge::Start(secs)) => {
                    app.ripple_trim_clip_start(clip_id, secs)
                }
                (EditorTool::Ripple, TrimEdge::End(secs)) => {
                    app.ripple_trim_clip_end(clip_id, secs)
                }
                (EditorTool::Roll, TrimEdge::Start(secs)) => {
                    app.roll_edit_from_start_edge(clip_id, secs)
                }
                (EditorTool::Roll, TrimEdge::End(secs)) => app.roll_edit_clip(clip_id, secs),
                (_, TrimEdge::Start(secs)) => app.trim_clip_start(clip_id, secs),
                (_, TrimEdge::End(secs)) => app.trim_clip_end(clip_id, secs),
            }
        }
        // Text/shape overlay trims: no named-trim-mode variants (Ripple/Roll/Slip/Slide are
        // video/audio-only tools; an overlay clip is always plain-trimmed regardless of the
        // active `EditorTool`).
        for (tc_id, edge) in text_trim_requests {
            match edge {
                TrimEdge::Start(secs) => app.trim_text_clip_start(tc_id, secs),
                TrimEdge::End(secs) => app.trim_text_clip_end(tc_id, secs),
            }
        }
        for (sc_id, edge) in shape_trim_requests {
            match edge {
                TrimEdge::Start(secs) => app.trim_shape_clip_start(sc_id, secs),
                TrimEdge::End(secs) => app.trim_shape_clip_end(sc_id, secs),
            }
        }
        for (tc_id, new_start_secs) in text_clip_drags {
            app.move_text_clip(tc_id, new_start_secs);
        }
        for (sc_id, new_start_secs) in shape_clip_drags {
            app.move_shape_clip(sc_id, new_start_secs);
        }
        for drag in clip_drags {
            // Slip/Slide (ROADMAP.md P2 item 11) act on the clip in place rather than moving
            // it across tracks, so they skip the cross-track drop-target resolution below
            // entirely — dragging a clip's body while either is active always edits it on its
            // own track.
            if app.tool == EditorTool::Slip {
                let old_start_secs = app
                    .active_project()
                    .timeline()
                    .tracks
                    .iter()
                    .flat_map(|t| &t.clips)
                    .find(|c| c.id == drag.clip_id)
                    .map(|c| c.start_secs);
                if let Some(old_start_secs) = old_start_secs {
                    app.slip_clip(drag.clip_id, drag.new_start_secs - old_start_secs);
                }
                continue;
            }
            if app.tool == EditorTool::Slide {
                app.slide_clip(drag.clip_id, drag.new_start_secs);
                continue;
            }
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
        app.touch_thumbnails(&thumbnail_touches);
        // Skipped requests aren't lost — the same tile re-offers its (by-then possibly
        // different) key next frame once `draw_filmstrip` runs again, same as any other
        // cache-miss tile that hasn't been requested yet.
        if thumbnail_requests_allowed {
            for (project_id, asset_id, frame_index) in thumbnail_requests {
                app.request_thumbnail(project_id, asset_id, frame_index);
            }
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

#[cfg(test)]
mod timeline_panel_test;
