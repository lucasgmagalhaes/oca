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

mod fullscreen_preview;
mod layout;
mod media_library_panel;
pub(crate) mod menu_bar;
mod preview_controls;
mod properties_panel;
mod shortcuts;
mod timeline_panel;
mod toolbar;

pub use fullscreen_preview::{fullscreen_preview_overlay, FULLSCREEN_CONTROLS_IDLE_SECS};
use layout::{
    audio_meter_column, resizable_divider, resizable_divider_horizontal, DIVIDER_HIT_WIDTH,
};
use media_library_panel::media_library_panel;
use preview_controls::{audio_level_meter, transport_controls};
use shortcuts::handle_editor_shortcuts;
pub(super) use toolbar::{
    export_srt_for_active_sequence, save_active_project, sequence_tab_bar, toolbar,
};

use eframe::egui::{self, RichText};

use avcore::media::format_timecode;

use crate::app::{App, EditorTool, PreviewZoom, MOTION_TRACK_SIZE_RANGE};
use crate::components;
use crate::i18n::Text;
use crate::icons;
use crate::theme;

/// Renders the Editor screen: toolbar, then a three-column row (media library / preview /
/// clip properties), then the timeline strip.
///
/// Every column and the timeline are wrapped in `ui.vertical(...)` (and the columns also in
/// `ui.allocate_ui(...)`) rather than just calling `ui.set_width()`/`ui.set_height()` inside
/// their `Frame` — this egui version doesn't default a `Frame`'s or `ScrollArea`'s child `Ui`
/// to a vertical top-down layout, it inherits whatever direction the enclosing `Ui` is in
/// (horizontal, here), and `set_width` alone only affects how much space is reported back to
/// the parent afterwards, not what the child actually paints. Skipping either wrapper
/// reintroduces the overlapping-text/full-width-panel bugs fixed in this screen — see the
/// "Add i18n" commit for the concrete symptoms.
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    app.ensure_active_project();

    handle_editor_shortcuts(app, ui);

    // `bg_workspace` — the Editor is the one screen with real nested structure below the
    // app-wide `CentralPanel` (`app::mod`'s `show`, which fills with plain `BG`/`bg_canvas` for
    // every screen): a three-column body plus the timeline strip, with panel gaps/preview
    // backdrop/timeline canvas all showing through as background. Framing that whole area in
    // `BG_WORKSPACE` gives it the doc's own separate "main workspace" layer, one step lighter
    // than the outer canvas — every other screen (Home/Library/...) has no such nesting and
    // keeps sitting directly on plain `BG`.
    egui::Frame::new().fill(theme::BG_WORKSPACE).show(ui, |ui| {
        ui.vertical(|ui| {
            toolbar(app, ui);
            ui.add_space(4.0);

            let total_width = ui.available_width();
            let min_col = 160.0_f32;
            // Reported bug: the properties panel would visibly overflow past its own border, and
            // the audio-meter column at the body row's far right would vanish entirely. Root
            // cause — `max_col` (each resizable column's own independent width cap) was a flat
            // `total_width * 0.4`, with no check that *both* columns landing near that cap at
            // once still leaves room for `AUDIO_METER_COLUMN_WIDTH`, the dividers/gaps, and
            // `preview_w`'s own floor below. `lib_panel_width`/`props_panel_width` persist across
            // sessions (dragged wide once, saved), so this wasn't hypothetical — `preview_w`'s
            // `.max(MIN_PREVIEW_W)` floor could push the row's total reserved width past
            // `total_width`, and egui doesn't wrap/shrink an overflowing `ui.horizontal` to fit —
            // it just runs the last item (the audio meter) off the visible edge. Fixed the same
            // way the body/timeline height budget below already is: derive `max_col` from the
            // same fixed-cost budget (`AUDIO_METER_COLUMN_WIDTH` + `gaps` + `MIN_PREVIEW_W`)
            // `preview_w` itself is computed from, split evenly between the two resizable
            // columns, so their sum can never eat into what the meter/preview floor need.
            let gaps = ui.spacing().item_spacing.x * 3.0 + DIVIDER_HIT_WIDTH * 2.0;
            let max_col = ((total_width - AUDIO_METER_COLUMN_WIDTH - gaps - MIN_PREVIEW_W) / 2.0)
                .max(min_col);
            app.lib_panel_width = app.lib_panel_width.clamp(min_col, max_col);
            app.props_panel_width = app.props_panel_width.clamp(min_col, max_col);

            // Reported bug: the timeline strip would sometimes vanish outright. Root cause — the
            // body row's own minimum height (used to be a hardcoded 160.0 floor below) and the
            // timeline's minimum height were each enforced independently, with no check that the
            // two together actually fit `available_height`. Once the Editor's vertical space got
            // small enough (a short window, a small display, whatever), `body_height`'s floor plus
            // `timeline_height`'s floor plus the divider between them could add up to more than
            // `available_height` — egui doesn't shrink an overflowing `ui.vertical` to fit, so the
            // timeline (painted last) just got pushed past the visible/clipped area and disappeared,
            // with no error and no obvious trigger from the user's side beyond "the window got
            // short enough at some point." Fixed by deriving both heights from the same
            // `content_height` budget so `body_height + timeline_height` can never exceed what's
            // actually available, with each floor scaled down (not dropped — still visible, just
            // thinner) rather than held fixed when that budget itself is tight.
            let available_height = ui.available_height();
            let divider_overhead = 4.0 + DIVIDER_HIT_WIDTH + 4.0; // the add_space(4.0) calls flanking resizable_divider_horizontal below
            let content_height = (available_height - divider_overhead).max(0.0);
            let min_timeline = 120.0_f32.min(content_height * 0.5);
            let min_body = 160.0_f32.min(content_height - min_timeline).max(0.0);
            let max_timeline = (content_height - min_body).max(min_timeline);
            app.timeline_height = app.timeline_height.clamp(min_timeline, max_timeline);
            let body_height = (content_height - app.timeline_height).max(0.0);

            let preview_w = (total_width
                - app.lib_panel_width
                - app.props_panel_width
                - AUDIO_METER_COLUMN_WIDTH
                - gaps)
                .max(MIN_PREVIEW_W);

            ui.horizontal(|ui| {
                ui.set_height(body_height);

                // `allocate_ui` reserves the exact rect up front, so children (ScrollArea, Frame)
                // see a properly bounded `max_rect` instead of the horizontal layout's full
                // remaining width — a bare `ui.set_width()` inside the panel only affects how much
                // space is reported *back* to this layout afterwards, not what the panel can paint.
                ui.allocate_ui(egui::vec2(app.lib_panel_width, body_height), |ui| {
                    media_library_panel(app, ui, app.lib_panel_width, body_height);
                });
                resizable_divider(
                    ui,
                    body_height,
                    &mut app.lib_panel_width,
                    min_col,
                    max_col,
                    1.0,
                );
                ui.allocate_ui(egui::vec2(preview_w, body_height), |ui| {
                    preview_panel(app, ui, body_height);
                });
                resizable_divider(
                    ui,
                    body_height,
                    &mut app.props_panel_width,
                    min_col,
                    max_col,
                    -1.0,
                );
                ui.allocate_ui(egui::vec2(app.props_panel_width, body_height), |ui| {
                    properties_panel::properties_panel(app, ui, app.props_panel_width, body_height);
                });
                ui.allocate_ui(egui::vec2(AUDIO_METER_COLUMN_WIDTH, body_height), |ui| {
                    audio_meter_column(app, ui, body_height);
                });
            });

            ui.add_space(4.0);
            resizable_divider_horizontal(
                ui,
                total_width,
                &mut app.timeline_height,
                min_timeline,
                max_timeline,
            );
            ui.add_space(4.0);
            timeline_panel::timeline_panel(app, ui, app.timeline_height);
        });
    });
}

/// Fixed width of the persistent stereo-meter strip at the body row's far right edge, matching
/// the OCA mockup's own always-visible vertical L/R meter (`spec/architecture/
/// editor-ui-visual-redesign.md`'s Inspector section) — sized to fit
/// `properties_panel::stereo_db_meter`'s own label+two-bar width (26 + 16*2 + 4 = 62) plus a
/// little breathing room, not resizable like the other columns since there's nothing to resize
/// (fixed content, no scrollable/collapsible substance).
const AUDIO_METER_COLUMN_WIDTH: f32 = 70.0;

/// Floor for `preview_w` (the center preview column) — also the fixed-cost budget `max_col`
/// derives from, so the two resizable side columns can never claim so much width that this
/// floor (plus `AUDIO_METER_COLUMN_WIDTH` plus the divider/spacing gaps) no longer fits.
const MIN_PREVIEW_W: f32 = 200.0;

/// The persistent stereo dB meter at the editor body's far right edge — unlike the properties
/// panel next to it, this is *not* gated on a clip being selected or which Inspector/Effects/
/// Audio tab is active, matching the OCA mockup's own always-visible meter. Reads the same
/// [`App::current_audio_level`] the preview transport row's compact meter already does.
/// Shared glyph size for every icon in the preview transport row (skip/step/loop/camera/marker).
pub(super) const TRANSPORT_ICON_SIZE: f32 = 14.0;
/// The play/pause button is the row's primary action, deliberately larger than transport icons.
pub(super) const TRANSPORT_PLAY_ICON_SIZE: f32 = 18.0;

fn draw_preview_hud_chip(painter: &egui::Painter, top_left: egui::Pos2, text: &str) {
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

fn preview_panel(app: &mut App, ui: &mut egui::Ui, height: f32) {
    // Lazy: the pipeline for the current selection is opened here, on the first paint of this
    // panel after a selection change — not by `select_asset` itself — so opening a project or
    // launching the app never pays GStreamer's open cost for an asset the Editor screen hasn't
    // actually been shown for yet.
    app.ensure_preview_loaded();
    app.ensure_transcript_loaded_for_preview();
    let locale = app.locale;
    ui.vertical(|ui| {
        ui.set_height(height);
        ui.horizontal(|ui| {
            components::section_label(ui, Text::ProgramMonitor.tr(locale));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .selectable_label(
                        app.preview_state.zoom == PreviewZoom::Percent100,
                        Text::PreviewZoom100.tr(locale),
                    )
                    .clicked()
                {
                    app.preview_state.zoom = PreviewZoom::Percent100;
                }
                if ui
                    .selectable_label(
                        app.preview_state.zoom == PreviewZoom::Fit,
                        Text::PreviewZoomFit.tr(locale),
                    )
                    .clicked()
                {
                    app.preview_state.zoom = PreviewZoom::Fit;
                }
                if ui
                    .selectable_label(
                        app.preview_state.zoom == PreviewZoom::Percent50,
                        Text::PreviewZoom50.tr(locale),
                    )
                    .clicked()
                {
                    app.preview_state.zoom = PreviewZoom::Percent50;
                }
            });
        });
        ui.add_space(theme::SPACE_SM);
        let preview_texture_size = app.preview_state.preview_texture.as_ref().map(|t| t.size());
        let frame_response = egui::Frame::new()
            .fill(egui::Color32::BLACK)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.set_min_height(height - 40.0);
                match &app.preview_state.preview_texture {
                    Some(_) => layer_transform_preview(app, ui),
                    None if app.preview_clip_present() && !app.preview_available() => {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new(Text::PreviewUnavailable.tr(locale))
                                    .size(13.0)
                                    .color(theme::TEXT_DISABLED),
                            );
                        });
                    }
                    None => {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                RichText::new(icons::PLAY_STR)
                                    .family(icons::family())
                                    .size(48.0)
                                    .color(theme::TEXT_MUTED),
                            );
                        });
                    }
                };
            })
            .response;
        // A small resolution(+fps) readout in the preview's bottom-left corner — matching the
        // Program Monitor mockup exactly (confirmed via a real screenshot of it: bottom-left =
        // resolution/fps, top-left = CAM, bottom-right = TC/frame) — from the actually decoded
        // texture's own size, not a fabricated/asset-declared value, so it never drifts from
        // what's on screen (proxy playback, letterboxing, etc.). fps comes from the previewed
        // clip's own asset (oca has no per-sequence fps) and is omitted when unknown.
        if let Some([w, h]) = preview_texture_size {
            let text = match app.current_preview_fps() {
                Some(fps) => format!("{w}x{h} | {fps:.2}fps"),
                None => format!("{w}x{h}"),
            };
            let size = ui
                .painter()
                .layout_no_wrap(
                    text.clone(),
                    egui::FontId::monospace(10.5),
                    theme::TEXT_SECONDARY,
                )
                .size()
                + egui::vec2(8.0, 4.0);
            let top_left = frame_response.rect.left_bottom() - egui::vec2(-8.0, 8.0 + size.y);
            draw_preview_hud_chip(ui.painter(), top_left, &text);
        }
        // "CAM 01" chip in the top-left corner, matching the mockup. Top-right is the mockup's
        // "REC ●" chip — a documented non-goal, since oca has no live-recording concept to
        // honestly wire it to (not a placeholder for a feature that doesn't exist). Wired to
        // real multicam-group data: only drawn when the previewed track is actually a multicam
        // group's program track, never faked when it isn't.
        if let Some(angle) = app.current_preview_multicam_angle() {
            let text = format!("CAM {angle:02}");
            draw_preview_hud_chip(
                ui.painter(),
                frame_response.rect.left_top() + egui::vec2(8.0, 8.0),
                &text,
            );
        }
        // Timecode+frame overlay in the preview's bottom-right corner, matching the mockup's
        // bottom-right overlay — a relocation of data already shown in the transport row's
        // timecode label below, plus a frame-within-second suffix when fps is known (omitted
        // otherwise rather than guessed).
        if preview_texture_size.is_some() {
            let playhead = app.active_project().timeline().playhead_secs;
            let mut text = format_timecode(playhead);
            if let Some(fps) = app.current_preview_fps() {
                let frame_count = fps.round().max(1.0) as i64;
                let frame =
                    ((playhead.fract() * fps as f64).round() as i64).clamp(0, frame_count - 1);
                text.push_str(&format!(":{frame:02}"));
            }
            // Measure first (this chip is bottom-right-anchored, unlike the top-left one above)
            // so its top-left corner can be derived from the frame's bottom-right corner.
            let size = ui
                .painter()
                .layout_no_wrap(
                    text.clone(),
                    egui::FontId::monospace(10.5),
                    theme::TEXT_SECONDARY,
                )
                .size()
                + egui::vec2(8.0, 4.0);
            let top_left = frame_response.rect.right_bottom() - egui::vec2(8.0, 8.0) - size;
            draw_preview_hud_chip(ui.painter(), top_left, &text);
        }
        let timeline_duration = app.active_project().timeline().duration_secs();
        // Restructured to match the Program Monitor mockup's own two-row shape (confirmed via
        // a real screenshot of it): a scrubber row with the current/total timecode flanking the
        // slider, THEN a separate, centered transport-controls row — previously one single
        // left-aligned row with the (small, inline) timecode text mixed in among the icons.
        let playhead = app.active_project().timeline().playhead_secs;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format_timecode(playhead))
                    .size(12.0)
                    .color(theme::ACCENT)
                    .monospace(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format_timecode(timeline_duration.max(playhead)))
                        .size(12.0)
                        .color(theme::TEXT_SECONDARY)
                        .monospace(),
                );
                if timeline_duration > 0.0 {
                    let mut position = playhead;
                    let slider = ui.add(
                        egui::Slider::new(&mut position, 0.0..=timeline_duration).show_value(false),
                    );
                    if slider.changed() {
                        app.seek_preview(position);
                    }
                }
            });
        });
        ui.add_space(theme::SPACE_XS);
        ui.columns(3, |columns| {
            // Left column intentionally empty — its equal share of the row is what centers the
            // middle column's transport controls (`ui.columns` splits width evenly into thirds).
            columns[1].vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    transport_controls(app, ui, locale, timeline_duration);
                });
            });
            columns[2].with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                audio_level_meter(app, ui);
                if ui
                    .selectable_label(
                        app.preview_state.scopes_enabled,
                        RichText::new("S").color(theme::TEXT_SECONDARY),
                    )
                    .on_hover_text(Text::PreviewScopesToggle.tr(locale))
                    .clicked()
                {
                    app.preview_state.scopes_enabled = !app.preview_state.scopes_enabled;
                }
                if ui
                    .small_button(RichText::new("[+]").color(theme::TEXT_SECONDARY))
                    .on_hover_text(Text::EnterFullscreenPreview.tr(locale))
                    .clicked()
                {
                    app.toggle_fullscreen_preview();
                }
                // "🔹" (emoji-presentation, same broken class as nav_rail's "🧹") replaced with
                // plain ASCII — no vendored Lucide marker icon exists yet either.
                if ui
                    .small_button(
                        RichText::new("*")
                            .size(TRANSPORT_ICON_SIZE)
                            .color(theme::TEXT_SECONDARY),
                    )
                    .on_hover_text(Text::AddMarkerButton.tr(locale))
                    .clicked()
                {
                    app.add_marker_at_playhead(avcore::MarkerKind::Standard);
                    app.push_toast(Text::MarkerAdded.tr(locale).to_string());
                }
                if components::icon_button(
                    ui,
                    crate::icons::CAMERA_STR,
                    Text::SnapshotButton.tr(locale),
                    components::IconButtonOpts {
                        family: Some(crate::icons::family()),
                        size: Some(TRANSPORT_ICON_SIZE),
                        ..Default::default()
                    },
                )
                .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("png", &["png"])
                        .set_file_name("snapshot.png")
                        .save_file()
                    {
                        app.save_preview_snapshot(path);
                    }
                }
            });
        });
        if app.preview_state.scopes_enabled {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if let Some(texture) = &app.preview_state.waveform_texture {
                    ui.image((texture.id(), egui::vec2(200.0, 100.0)));
                }
                if let Some(texture) = &app.preview_state.vectorscope_texture {
                    ui.image((texture.id(), egui::vec2(100.0, 100.0)));
                }
            });
        }
    });
}

/// Pure size math for [`PreviewZoom`] — extracted out of `layer_transform_preview` so it's
/// testable without an `egui::Ui`. `Fit` always fills `avail`; `Percent50`/`Percent100` size
/// off the sequence's own real pixel dimensions instead, then every branch is clamped back down
/// to `avail` and aspect-corrected against `canvas_aspect` the same way — this preview has no
/// scrollable viewport (see `PreviewZoom`'s own doc comment), so an oversized zoom silently caps
/// back to whatever `Fit` would have produced rather than overflowing the panel.
fn preview_canvas_size(
    zoom: PreviewZoom,
    avail: egui::Vec2,
    canvas_w: u32,
    canvas_h: u32,
    canvas_aspect: f32,
) -> egui::Vec2 {
    let mut size = match zoom {
        PreviewZoom::Fit => avail,
        PreviewZoom::Percent50 => egui::vec2(canvas_w as f32 * 0.5, canvas_h as f32 * 0.5),
        PreviewZoom::Percent100 => egui::vec2(canvas_w as f32, canvas_h as f32),
        PreviewZoom::Custom(factor) => {
            egui::vec2(canvas_w as f32 * factor, canvas_h as f32 * factor)
        }
    };
    size.x = size.x.min(avail.x);
    size.y = size.y.min(avail.y);
    if size.x / size.y > canvas_aspect {
        size.x = size.y * canvas_aspect;
    } else {
        size.y = size.x / canvas_aspect;
    }
    size
}

/// Draws the selected clip's video inside a canvas-space box sized to the active sequence's
/// export aspect ratio (the same target dimensions export will actually use, via
/// [`avcore::ExportAspectRatio::dims_or`]) and, when its position isn't already animated
/// (`position_keyframes.len() <= 1`, the "static transform" case — see `request.md`'s Fase 4
/// "Transformação de camadas" spec), lets the user drag it to set a single position keyframe.
/// This maps directly onto `overlay=x:y`'s own coordinate space
/// (`crate::keyframe::position_overlay_xy_expr`, canvas-fraction offset from the top-left
/// default) — dragging writes exactly what export reads, no separate UI-only representation.
///
/// Layer *size* (`ClipInstance::layer_scale_x`/`_y`) is a multiplier on top of a fixed
/// stand-in baseline footprint — the video fit to the *full* canvas, preserving its own aspect
/// ratio (letterboxed/pillarboxed against `canvas_aspect` if it doesn't match) — rather than a
/// real pixel dimension: this panel doesn't know the clip's actual export-time decoded
/// resolution (the preview texture may be a lower-res editing proxy), so it can't draw the box
/// at its true composited size. `layer_scale_x`/`_y` default to `1.0`
/// ([`crate::app::LAYER_SCALE_RANGE`] is `0.1..=3.0`), so the baseline has to *be* "fills the
/// canvas" for an untouched clip to preview at its expected full size instead of shrunk before
/// the multiplier is even applied — this was previously a fixed 40%-of-canvas stand-in
/// regardless of `layer_scale`, which made every clip preview as a small box even with no
/// transform ever applied. Dragging the bottom-right handle still writes the real multiplier
/// `set_selected_clip_layer_scale` reads at export, same "editable but visually approximate"
/// shape as most of this panel. `Some(_)` in `preview_panel`'s match already guarantees
/// `app.preview_state.preview_texture` is set, but this re-checks (and bails) rather than trust that
/// invariant across the borrow-splitting clone below.
fn layer_transform_preview(app: &mut App, ui: &mut egui::Ui) {
    let Some(texture) = app.preview_state.preview_texture.clone() else {
        return;
    };
    let locale = app.locale;
    let tex_size = texture.size_vec2();
    let tex_aspect = tex_size.x / tex_size.y;

    let (canvas_w, canvas_h) = app
        .active_sequence_export_settings()
        .aspect_ratio
        .dims_or((tex_size.x as u32, tex_size.y as u32));
    let canvas_aspect = canvas_w as f32 / canvas_h.max(1) as f32;

    let avail_rect = ui.available_rect_before_wrap();
    // Zoom Tool (product-decisions addendum, Section 13/14): controls the Program Monitor
    // *viewport* zoom only — never clip Transform/Scale (that's `PropertiesTab::Inspector`'s
    // own Scale field, untouched here). Click zooms in one step, Alt+click zooms out, wheel
    // zooms continuously, double-click resets to Fit. No cursor-anchored pan exists (this
    // preview's canvas always centers — `PreviewZoom`'s own doc comment already documents "no
    // scrollable viewport" as a known gap), so this zooms centered rather than literally
    // keeping the point under the cursor fixed; a real follow-up if 1:1 pixel panning is wanted.
    if app.tool == EditorTool::Zoom {
        let zoom_resp = ui.interact(
            avail_rect,
            ui.id().with("preview_zoom_tool"),
            egui::Sense::click(),
        );
        let current = match app.preview_state.zoom {
            PreviewZoom::Fit | PreviewZoom::Percent100 => 1.0,
            PreviewZoom::Percent50 => 0.5,
            PreviewZoom::Custom(f) => f,
        };
        if zoom_resp.double_clicked() {
            app.preview_state.zoom = PreviewZoom::Fit;
        } else if zoom_resp.clicked() {
            let factor = if ui.input(|i| i.modifiers.alt) {
                0.8
            } else {
                1.25
            };
            app.preview_state.zoom = PreviewZoom::Custom((current * factor).clamp(0.1, 8.0));
        }
        if zoom_resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let factor = (1.0 + scroll * 0.001).clamp(0.5, 2.0);
                app.preview_state.zoom = PreviewZoom::Custom((current * factor).clamp(0.1, 8.0));
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::ZoomIn);
        }
    }
    // The layer-position drag below had no bound on how far the layer could be dragged, and
    // nothing clipped its paint to this panel — confirmed via a real report: drag far enough
    // and the layer image paints straight over the media library/properties panels next door.
    // A clipped painter (scoped to this panel's own rect) fixes the *visual* leak regardless of
    // how far the underlying position value goes; the drag clamp below additionally keeps the
    // value itself from wandering off to where the layer becomes unreachable to drag back.
    let clip_rect = ui.max_rect();
    let painter = ui.painter().with_clip_rect(clip_rect);
    let canvas_size = preview_canvas_size(
        app.preview_state.zoom,
        avail_rect.size(),
        canvas_w,
        canvas_h,
        canvas_aspect,
    );
    let canvas_rect = egui::Rect::from_center_size(avail_rect.center(), canvas_size);
    ui.allocate_rect(canvas_rect, egui::Sense::hover());
    painter.rect_stroke(
        canvas_rect,
        0,
        egui::Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Inside,
    );

    // Text Tool (product-decisions addendum, Section 6): a single click anywhere in the
    // Program Monitor creates a new Text Graphic at that position, immediately selected — no
    // click-and-drag required (explicitly out of scope for this pass). Clicking an *existing*
    // text graphic to select/reposition it isn't wired here: text/shape overlays aren't
    // rendered as separate interactive objects in this canvas today (they're already baked into
    // the decoded `texture` by the preview pipeline), so that half of Section 6 needs real
    // per-overlay hit-testing infrastructure this pass doesn't add — a real, separate follow-up,
    // not silently skipped.
    if app.tool == EditorTool::Text {
        let text_resp = ui.interact(
            canvas_rect,
            ui.id().with("preview_text_tool"),
            egui::Sense::click(),
        );
        if text_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
        }
        if let Some(pointer) = text_resp.interact_pointer_pos() {
            if text_resp.clicked() {
                let pos_x =
                    ((pointer.x - canvas_rect.left()) / canvas_rect.width()).clamp(0.0, 1.0);
                let pos_y =
                    ((pointer.y - canvas_rect.top()) / canvas_rect.height()).clamp(0.0, 1.0);
                let new_id = app.add_text_clip_at(pos_x, pos_y);
                app.text_tool_pending_empty_clip_id = Some(new_id);
            }
        }
        return;
    }

    if app.drawing_shape_points.is_some() {
        draw_custom_shape_surface(app, ui, canvas_rect, &texture);
        return;
    }

    // Fixed stand-in baseline — the video fit to fill the canvas — see this function's doc
    // comment on why `layer_scale` needs "fills the canvas" as its own baseline (1.0 = full
    // size) rather than some smaller stand-in fraction of it.
    let mut base_h = canvas_rect.height().min(canvas_rect.width());
    let mut base_w = base_h * tex_aspect;
    if base_w > canvas_rect.width() {
        base_w = canvas_rect.width();
        base_h = base_w / tex_aspect;
    }

    let selected_clip = app.selected_clip();
    let position_keyframes = selected_clip
        .map(|c| c.position_keyframes.clone())
        .unwrap_or_default();
    let (layer_scale_x, layer_scale_y) = selected_clip
        .map(|c| (c.layer_scale_x, c.layer_scale_y))
        .unwrap_or((1.0, 1.0));
    let resizable = selected_clip.is_some();
    let draggable = position_keyframes.len() <= 1;
    let current = position_keyframes
        .first()
        .map(|k| k.value)
        .unwrap_or(avcore::Position { x: 0.0, y: 0.0 });

    let layer_w = base_w * layer_scale_x;
    let layer_h = base_h * layer_scale_y;

    let layer_min = canvas_rect.min
        + egui::vec2(
            current.x * canvas_rect.width(),
            current.y * canvas_rect.height(),
        );
    let layer_rect = egui::Rect::from_min_size(layer_min, egui::vec2(layer_w, layer_h));

    painter.image(
        texture.id(),
        layer_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );

    let border_color = if draggable {
        theme::ACCENT_2
    } else {
        theme::TEXT_MUTED
    };
    painter.rect_stroke(
        layer_rect,
        0,
        egui::Stroke::new(1.5, border_color),
        egui::StrokeKind::Outside,
    );

    if app.motion_track_region.picking_motion_track_region {
        draw_motion_track_region_picker(app, ui, layer_rect, tex_size);
        return;
    }

    if draggable {
        let resp = ui.interact(
            layer_rect,
            ui.id().with("preview_layer_transform"),
            egui::Sense::drag(),
        );
        if resp.dragged() {
            let delta = resp.drag_delta();
            // Clamped to one full canvas width/height of overflow on either side (generous
            // enough for a slide-in/out animation's endpoints) — previously unbounded, so a
            // fast/long drag could push the layer's position value far enough that it became
            // impossible to find/drag back at all, on top of the paint-clipping fix above.
            let new_pos = avcore::Position {
                x: (current.x + delta.x / canvas_rect.width().max(1.0)).clamp(-1.0, 2.0),
                y: (current.y + delta.y / canvas_rect.height().max(1.0)).clamp(-1.0, 2.0),
            };
            app.set_selected_clip_position_keyframes(vec![avcore::Keyframe {
                time_fraction: 0.0,
                value: new_pos,
            }]);
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        resp.on_hover_text(Text::LayerTransformDragHint.tr(locale));
    } else {
        painter.text(
            layer_rect.center_bottom() + egui::vec2(0.0, 4.0),
            egui::Align2::CENTER_TOP,
            Text::LayerTransformAnimatedHint.tr(locale),
            egui::FontId::proportional(10.0),
            theme::TEXT_MUTED,
        );
    }

    if resizable {
        const HANDLE_SIZE: f32 = 10.0;
        let handle_rect =
            egui::Rect::from_center_size(layer_rect.right_bottom(), egui::Vec2::splat(HANDLE_SIZE));
        let resize_resp = ui.interact(
            handle_rect,
            ui.id().with("preview_layer_resize"),
            egui::Sense::drag(),
        );
        painter.rect_filled(handle_rect, 2, theme::ACCENT_2);
        if resize_resp.dragged() {
            let delta = resize_resp.drag_delta();
            let new_layer_w = (layer_w + delta.x).max(4.0);
            let new_layer_h = (layer_h + delta.y).max(4.0);
            app.set_selected_clip_layer_scale(
                layer_scale_x * (new_layer_w / layer_w.max(1.0)),
                layer_scale_y * (new_layer_h / layer_h.max(1.0)),
            );
        }
        if resize_resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
        }
        resize_resp.on_hover_text(Text::LayerTransformResizeHint.tr(locale));
    }
}

/// Click-to-place-vertex surface for `request.md`'s Fase 4 "forma personalizada" — active
/// whenever `app.drawing_shape_points` is `Some` (see [`App::start_drawing_custom_shape`]).
/// Takes over `canvas_rect` entirely in place of [`layer_transform_preview`]'s usual layer
/// drag/resize handling for the duration of the drawing; the two modes are mutually exclusive.
///
/// Draws `texture` stretched to fill `canvas_rect` first (same "stretch to the target rect,
/// don't preserve its own aspect separately" convention [`layer_transform_preview`]'s own image
/// paint and `fullscreen_preview_overlay` already use) so there's an actual frame to trace a
/// shape over, rather than the bare canvas outline this surface used to be limited to.
///
/// Each click on `canvas_rect` appends one point in canvas-fraction coordinates (the same
/// space [`avcore::timeline::ShapeClip::center_x`]/`_y` use) via
/// [`App::push_drawing_shape_point`]. Placed points are drawn as small filled dots connected by
/// straight lines, plus a lighter closing segment back to the first point once there are
/// enough to see the shape taking form. Enter finishes (a no-op below 3 points — the drawing
/// stays active); Escape cancels outright.
fn draw_custom_shape_surface(
    app: &mut App,
    ui: &mut egui::Ui,
    canvas_rect: egui::Rect,
    texture: &egui::TextureHandle,
) {
    let locale = app.locale;
    ui.painter().image(
        texture.id(),
        canvas_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    let to_screen = |p: (f32, f32)| {
        canvas_rect.min + egui::vec2(p.0 * canvas_rect.width(), p.1 * canvas_rect.height())
    };

    let click_resp = ui.interact(
        canvas_rect,
        ui.id().with("shape_draw_surface"),
        egui::Sense::click(),
    );
    if click_resp.clicked() {
        if let Some(pos) = click_resp.interact_pointer_pos() {
            let frac_x = (pos.x - canvas_rect.min.x) / canvas_rect.width().max(1.0);
            let frac_y = (pos.y - canvas_rect.min.y) / canvas_rect.height().max(1.0);
            app.push_drawing_shape_point(frac_x, frac_y);
        }
    }
    if click_resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }

    let points = app.drawing_shape_points.clone().unwrap_or_default();
    let screen_points: Vec<egui::Pos2> = points.iter().copied().map(to_screen).collect();
    for &p in &screen_points {
        ui.painter().circle_filled(p, 4.0, theme::ACCENT);
    }
    if screen_points.len() >= 2 {
        ui.painter().add(egui::Shape::line(
            screen_points.clone(),
            egui::Stroke::new(1.5, theme::ACCENT),
        ));
    }
    if screen_points.len() >= 3 {
        ui.painter().line_segment(
            [screen_points[screen_points.len() - 1], screen_points[0]],
            egui::Stroke::new(1.0, theme::TEXT_MUTED),
        );
    }

    ui.painter().text(
        canvas_rect.center_bottom() + egui::vec2(0.0, -6.0),
        egui::Align2::CENTER_BOTTOM,
        Text::ShapeDrawHint.tr(locale),
        egui::FontId::proportional(11.0),
        theme::TEXT_SECONDARY,
    );

    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.cancel_drawing_custom_shape();
    } else if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        app.finish_drawing_custom_shape();
    }
}

/// Drag-to-select surface for the motion-tracking region — active whenever
/// `app.motion_track_region.picking_motion_track_region` is set (see [`App::start_picking_motion_track_region`]).
/// Takes over `layer_rect` (the preview's already-computed texture rect — see
/// [`layer_transform_preview`]) in place of its usual layer drag/resize handling, mutually
/// exclusive with it the same way [`draw_custom_shape_surface`] is.
///
/// `layer_rect` spans the full source frame at UV `0.0..=1.0` on both axes (however the layer
/// itself is currently positioned/scaled on an overlay track), so a fraction of `layer_rect`
/// maps directly to `motion_track_center_x`/`_y`'s own "fraction of source frame" convention.
/// `motion_track_width`/`_height` are each a fraction of the source frame's *shorter*
/// dimension (`avcore::track_region`'s convention) rather than of `layer_rect` itself, so the
/// on-screen region rect additionally scales by `layer_rect`'s per-axis stretch relative to
/// `tex_size` — `layer_rect`'s aspect only matches `tex_size`'s when `layer_scale_x` equals
/// `layer_scale_y`, otherwise the two diverge and this conversion keeps the drawn region
/// faithful to what `avcore::track_region` will actually sample.
fn draw_motion_track_region_picker(
    app: &mut App,
    ui: &mut egui::Ui,
    layer_rect: egui::Rect,
    tex_size: egui::Vec2,
) {
    let locale = app.locale;
    let short_side = tex_size.x.min(tex_size.y).max(1.0);
    let scale = egui::vec2(
        layer_rect.width() / tex_size.x.max(1.0),
        layer_rect.height() / tex_size.y.max(1.0),
    );

    let center_px = layer_rect.min
        + egui::vec2(
            app.motion_track_region.motion_track_center_x * layer_rect.width(),
            app.motion_track_region.motion_track_center_y * layer_rect.height(),
        );
    let size_px = egui::vec2(
        app.motion_track_region.motion_track_width * short_side * scale.x,
        app.motion_track_region.motion_track_height * short_side * scale.y,
    );
    let region_rect = egui::Rect::from_center_size(center_px, size_px);

    ui.painter().rect_stroke(
        region_rect,
        0,
        egui::Stroke::new(1.5, theme::ACCENT_2),
        egui::StrokeKind::Outside,
    );

    let body_resp = ui.interact(
        region_rect,
        ui.id().with("motion_track_region_body"),
        egui::Sense::drag(),
    );
    if body_resp.dragged() {
        let delta = body_resp.drag_delta();
        app.motion_track_region.motion_track_center_x =
            (app.motion_track_region.motion_track_center_x + delta.x / layer_rect.width().max(1.0))
                .clamp(0.0, 1.0);
        app.motion_track_region.motion_track_center_y =
            (app.motion_track_region.motion_track_center_y
                + delta.y / layer_rect.height().max(1.0))
            .clamp(0.0, 1.0);
    }
    if body_resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }

    const HANDLE_SIZE: f32 = 10.0;
    let handle_rect =
        egui::Rect::from_center_size(region_rect.right_bottom(), egui::Vec2::splat(HANDLE_SIZE));
    ui.painter().rect_filled(handle_rect, 2, theme::ACCENT_2);
    let resize_resp = ui.interact(
        handle_rect,
        ui.id().with("motion_track_region_resize"),
        egui::Sense::drag(),
    );
    if resize_resp.dragged() {
        let delta = resize_resp.drag_delta();
        let new_width_px = (size_px.x + delta.x).max(4.0);
        let new_height_px = (size_px.y + delta.y).max(4.0);
        app.motion_track_region.motion_track_width =
            (new_width_px / (short_side * scale.x).max(0.001)).clamp(
                *MOTION_TRACK_SIZE_RANGE.start(),
                *MOTION_TRACK_SIZE_RANGE.end(),
            );
        app.motion_track_region.motion_track_height =
            (new_height_px / (short_side * scale.y).max(0.001)).clamp(
                *MOTION_TRACK_SIZE_RANGE.start(),
                *MOTION_TRACK_SIZE_RANGE.end(),
            );
    }

    ui.painter().text(
        layer_rect.center_bottom() + egui::vec2(0.0, -6.0),
        egui::Align2::CENTER_BOTTOM,
        Text::MotionTrackRegionPickHint.tr(locale),
        egui::FontId::proportional(11.0),
        theme::TEXT_SECONDARY,
    );

    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.stop_picking_motion_track_region();
    }
}

#[cfg(test)]
mod mod_test;
