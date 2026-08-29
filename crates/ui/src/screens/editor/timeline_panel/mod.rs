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

use eframe::egui::{self, RichText};

use crate::app::{App, EditorTool};
use crate::components;
use crate::i18n::Text;
use crate::theme;

/// Bounds for `App::timeline_px_per_sec` — tight enough to stay readable, loose enough to
/// go from several-projects-wide overview down to frame-accurate editing.
const MIN_PX_PER_SEC: f32 = 0.5;
const MAX_PX_PER_SEC: f32 = 60.0;
const TRACK_LABEL_WIDTH: f32 = 86.0;

/// Fixed color-label swatches offered in the clip/track "Rótulo de cor" context menu — per
/// `spec/ROADMAP.md` P4 item 27, matching Premiere/DaVinci/FCP's own fixed-palette convention
/// (a free color picker would let two clips end up with visually indistinguishable colors,
/// defeating the "recognize at a glance" point of a label).
const CLIP_COLOR_LABEL_PALETTE: &[[u8; 3]] = &[
    [229, 83, 83],   // red
    [230, 145, 56],  // orange
    [230, 200, 56],  // yellow
    [96, 189, 104],  // green
    [86, 156, 214],  // blue
    [178, 108, 219], // purple
];

use draw::{
    color_filter_tint, draw_filmstrip, draw_frozen_poster, draw_keyframe_markers,
    draw_marker_ticks, draw_playhead, draw_waveform, shape_kind_glyph, ThumbnailDrawWork,
};
use snap::{snap_move_start, snap_to_nearest, waveform_snap_points_for_clip, ClipDrag};

/// Icon for a track's [`avcore::AudioRole`] (D2, `spec/architecture/differentiators.md`) — the
/// track header's role picker, and its own collapsed `ComboBox` display.
fn audio_role_icon(role: avcore::AudioRole) -> &'static str {
    match role {
        avcore::AudioRole::Unspecified => "–",
        avcore::AudioRole::GameAudio => "🎮",
        avcore::AudioRole::Mic => "🎤",
        avcore::AudioRole::Music => "🎵",
    }
}

/// Hover text for the role picker's collapsed state — the icon alone is too terse to stand
/// alone.
fn audio_role_label(role: avcore::AudioRole, locale: crate::i18n::Locale) -> &'static str {
    match role {
        avcore::AudioRole::Unspecified => Text::AudioRoleUnspecified.tr(locale),
        avcore::AudioRole::GameAudio => Text::AudioRoleGameAudio.tr(locale),
        avcore::AudioRole::Mic => Text::AudioRoleMic.tr(locale),
        avcore::AudioRole::Music => Text::AudioRoleMusic.tr(locale),
    }
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

        // Magnetic snap targets (ROADMAP.md P0 item 2): every clip's start/end edge, across
        // every track — collected once per frame up front so both the ruler's playhead drag
        // and the per-clip trim/move drags below can use the same set without re-borrowing
        // `app` mid-loop. Held with a modifier (Alt) to temporarily disable snapping, the same
        // convention most editors use.
        let snap_enabled = !ui.input(|i| i.modifiers.alt);
        let clip_edges: Vec<(u64, f64, f64)> = app
            .active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .map(|c| (c.id, c.start_secs, c.start_secs + c.duration_secs()))
            .collect();
        // Markers are magnetic-snap targets too (`spec/architecture/competitive-feature-plan.md`
        // quick wins — P0 item 2's own doc comment flagged this as "revisit when [markers]
        // land," which they since have, via P2 item 9).
        let marker_secs: Vec<f64> = app
            .active_project()
            .timeline()
            .markers
            .iter()
            .map(|m| m.position_secs)
            .collect();
        let snap_targets_excluding = |exclude_id: u64| -> Vec<f64> {
            clip_edges
                .iter()
                .filter(|(id, _, _)| *id != exclude_id)
                .flat_map(|(_, start, end)| [*start, *end])
                .chain(marker_secs.iter().copied())
                .collect()
        };

        // D5 (`spec/architecture/differentiators.md`): waveform low-energy points as an extra
        // snap target for trim-edge (cut-point) drags specifically, not whole-clip moves — a
        // dragged cut should be able to magnetically land mid-pause instead of mid-word/mid-
        // sound-effect. See `waveform_snap_points_for_clip`'s own doc comment for the mechanism.
        let waveform_snap_targets: Vec<f64> = {
            let project = app.active_project();
            project
                .timeline()
                .tracks
                .iter()
                .flat_map(|t| &t.clips)
                .flat_map(|clip| {
                    let asset = project.media_library.iter().find(|a| a.id == clip.asset_id);
                    asset
                        .map(|asset| waveform_snap_points_for_clip(asset, clip))
                        .unwrap_or_default()
                })
                .collect()
        };

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(Text::Timeline.tr(app.locale))
                    .size(11.0)
                    .color(theme::TEXT_MUTED),
            );
            // A visible zoom affordance for `timeline_px_per_sec` — until now only reachable via
            // Ctrl+scroll, with no on-screen indicator of the current zoom level at all. Matches
            // `oca-editor-mock.html`'s `.tl-zoom` slider in the timeline toolbar's right corner.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(
                    egui::Slider::new(
                        &mut app.timeline_px_per_sec,
                        MIN_PX_PER_SEC..=MAX_PX_PER_SEC,
                    )
                    .show_value(false)
                    .logarithmic(true),
                );
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
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), 14.0),
                egui::Sense::click_and_drag(),
            );
            ruler_top = rect.top();
            ui.painter().rect_filled(rect, 0, theme::SURFACE_2);
            if let Some(pos) = response.interact_pointer_pos() {
                let secs = ((pos.x - rect.left()) / px_per_sec).max(0.0) as f64;
                let all_edges: Vec<f64> = clip_edges
                    .iter()
                    .flat_map(|(_, start, end)| [*start, *end])
                    .chain(marker_secs.iter().copied())
                    .collect();
                let secs = if snap_enabled {
                    snap_to_nearest(secs, &all_edges, px_per_sec)
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
        let mut track_rows: Vec<(u64, avcore::timeline::TrackKind, egui::Rect)> = Vec::new();
        let mut toggle_track_visibility_requests: Vec<u64> = Vec::new();
        let mut toggle_track_lock_requests: Vec<u64> = Vec::new();
        let mut track_audio_role_requests: Vec<(u64, avcore::AudioRole)> = Vec::new();
        let mut track_color_label_requests: Vec<(u64, Option<[u8; 3]>)> = Vec::new();
        // Set the first time a trim/move drag starts this frame — `app` is immutably borrowed
        // for the whole track/clip iteration below, so the undo snapshot itself is pushed once,
        // after that borrow ends, rather than inline at the drag_started() check.
        let mut drag_started_this_frame = false;
        let mut thumbnail_requests: Vec<(u64, i64)> = Vec::new();
        let mut thumbnail_touches: Vec<(u64, i64)> = Vec::new();
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
                            let eye = if visible { "👁" } else { "⊘" };
                            let tooltip = if visible {
                                Text::TrackHide.tr(locale)
                            } else {
                                Text::TrackShow.tr(locale)
                            };
                            if components::icon_button(
                                ui,
                                eye,
                                tooltip,
                                components::IconButtonOpts::default(),
                            )
                            .clicked()
                            {
                                toggle_track_visibility_requests.push(track_id);
                            }
                            let locked = track.locked;
                            let lock_glyph = if locked { "🔒" } else { "🔓" };
                            let lock_tooltip = if locked {
                                Text::TrackUnlock.tr(locale)
                            } else {
                                Text::TrackLock.tr(locale)
                            };
                            if components::icon_button(
                                ui,
                                lock_glyph,
                                lock_tooltip,
                                components::IconButtonOpts::default(),
                            )
                            .clicked()
                            {
                                toggle_track_lock_requests.push(track_id);
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
                        let color = if let Some([r, g, b]) = clip.color_label {
                            egui::Color32::from_rgb(r, g, b)
                        } else {
                            match (track.kind, track.name.as_str()) {
                                (avcore::timeline::TrackKind::Video, _) => theme::SURFACE_2,
                                (avcore::timeline::TrackKind::Audio, "A2") => {
                                    theme::ACCENT_2.gamma_multiply(0.6)
                                }
                                (avcore::timeline::TrackKind::Audio, _) => {
                                    theme::ACCENT.gamma_multiply(0.5)
                                }
                                // Text/Shape tracks carry text_clips/shape_clips, not clips —
                                // these arms satisfy exhaustiveness but are never reached at
                                // runtime.
                                (avcore::timeline::TrackKind::Text, _) => theme::SURFACE_2,
                                (avcore::timeline::TrackKind::Shape, _) => theme::SURFACE_2,
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
                            ui.id().with(("timeline_clip", clip.id)),
                            if track.locked {
                                egui::Sense::click()
                            } else {
                                egui::Sense::click_and_drag()
                            },
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
                            if track.kind == avcore::timeline::TrackKind::Video
                                && ui.button(Text::ContextMenuDetachAudio.tr(locale)).clicked()
                            {
                                detach_audio_requests.push(clip.id);
                                ui.close();
                            }
                            if track.kind == avcore::timeline::TrackKind::Video
                                && clip.nested_sequence_id.is_none()
                                && ui
                                    .button(Text::ContextMenuCreateCompoundClip.tr(locale))
                                    .clicked()
                            {
                                clicked_clip_id = Some(clip.id);
                                create_compound_clip_requested = true;
                                ui.close();
                            }
                            if let Some(nested_id) = clip.nested_sequence_id {
                                if ui
                                    .button(Text::ContextMenuOpenCompoundClip.tr(locale))
                                    .clicked()
                                {
                                    open_nested_sequence_request = Some(nested_id);
                                    ui.close();
                                }
                            }
                            ui.menu_button(Text::ContextMenuSpeedRamp.tr(locale), |ui| {
                                if ui.button(Text::SpeedRampSlowToFast.tr(locale)).clicked() {
                                    speed_ramp_requests.push((clip.id, 0.5, 2.0));
                                    ui.close();
                                }
                                if ui.button(Text::SpeedRampFastToSlow.tr(locale)).clicked() {
                                    speed_ramp_requests.push((clip.id, 2.0, 0.5));
                                    ui.close();
                                }
                                ui.separator();
                                if ui.button(Text::SpeedRampCustom.tr(locale)).clicked() {
                                    speed_ramp_custom_request = Some(clip.id);
                                    ui.close();
                                }
                            });
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
                            ui.menu_button(Text::ContextMenuColorLabel.tr(locale), |ui| {
                                for &[r, g, b] in CLIP_COLOR_LABEL_PALETTE {
                                    let swatch = egui::Color32::from_rgb(r, g, b);
                                    if ui.add(egui::Button::new("  ").fill(swatch)).clicked() {
                                        clip_color_label_requests.push((clip.id, Some([r, g, b])));
                                        ui.close();
                                    }
                                }
                                ui.separator();
                                if ui
                                    .button(Text::ContextMenuColorLabelClear.tr(locale))
                                    .clicked()
                                {
                                    clip_color_label_requests.push((clip.id, None));
                                    ui.close();
                                }
                            });
                            ui.separator();
                            if ui.button(Text::ContextMenuDelete.tr(locale)).clicked() {
                                delete_requests.push(clip.id);
                                ui.close();
                            }
                        });
                        let edge_sense = if track.locked {
                            egui::Sense::hover()
                        } else {
                            egui::Sense::drag()
                        };
                        let left_response = ui.interact(
                            left_edge_rect,
                            ui.id().with(("timeline_clip_trim_start", clip.id)),
                            edge_sense,
                        );
                        let right_response = ui.interact(
                            right_edge_rect,
                            ui.id().with(("timeline_clip_trim_end", clip.id)),
                            edge_sense,
                        );
                        if left_response.hovered()
                            || left_response.dragged()
                            || right_response.hovered()
                            || right_response.dragged()
                        {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }
                        if left_response.drag_started() || right_response.drag_started() {
                            drag_started_this_frame = true;
                        }
                        if body_response.clicked() {
                            if ui.input(|i| i.modifiers.ctrl) {
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
                            let delta_secs = (body_response.drag_delta().x / px_per_sec) as f64;
                            if let Some(pointer) = body_response.interact_pointer_pos() {
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
                            let secs = ((pos.x - track_rect.left()) / px_per_sec).max(0.0) as f64;
                            let mut targets = snap_targets_excluding(clip.id);
                            targets.push(playhead_secs);
                            targets.extend(&waveform_snap_targets);
                            let secs = if snap_enabled {
                                snap_to_nearest(secs, &targets, px_per_sec)
                            } else {
                                secs
                            };
                            trim_requests.push((clip.id, TrimEdge::Start(secs)));
                        }
                        if let Some(pos) = right_response.interact_pointer_pos() {
                            let secs = ((pos.x - track_rect.left()) / px_per_sec).max(0.0) as f64;
                            let mut targets = snap_targets_excluding(clip.id);
                            targets.push(playhead_secs);
                            targets.extend(&waveform_snap_targets);
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
                                        asset,
                                        clip.source_in_secs,
                                        px_per_sec,
                                        &mut thumbnail_work,
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
                                egui::Stroke::new(3.0, egui::Color32::from_black_alpha(alpha)),
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
                                format!("📦 {nested_name}"),
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
                                egui::CornerRadius::same(theme::RADIUS_SM),
                                egui::Stroke::new(2.0, theme::ERROR),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if app.selected_clip_id == Some(clip.id) {
                            painter.rect_stroke(
                                clip_rect,
                                egui::CornerRadius::same(theme::RADIUS_SM),
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
        for (track_id, role) in track_audio_role_requests {
            app.set_track_audio_role(track_id, role);
        }
        for (track_id, color_label) in track_color_label_requests {
            app.set_track_color_label(track_id, color_label);
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
        for (asset_id, frame_index) in thumbnail_requests {
            app.request_thumbnail(asset_id, frame_index);
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
