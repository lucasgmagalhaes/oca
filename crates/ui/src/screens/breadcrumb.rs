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

use avcore::media::format_timecode;
use eframe::egui::{self, RichText};

use crate::app::{App, Screen};
use crate::i18n::{self, Text};
use crate::theme;

/// Width, in points, of the invisible strip along each window edge/corner used to resize by
/// dragging — needed because `main.rs`'s `with_decorations(false)` (this bar replaces the OS
/// chrome it removes) takes the OS's own resize handles with it.
const RESIZE_BORDER: f32 = 6.0;
/// Width of one window-control button (minimize/maximize/close).
const WINDOW_BUTTON_WIDTH: f32 = 44.0;
const BAR_HEIGHT: f32 = 40.0;

/// Renders oca's custom title bar: app name, current screen title, the active project's name
/// (in the Editor) with an "unsaved changes" dot, and — since `main.rs` disables OS window
/// decorations — the window's own drag-to-move region and minimize/maximize/close buttons,
/// styled with the app's own theme instead of the OS's default chrome. Double-clicking the
/// draggable area toggles maximize, the same as double-clicking a normal OS title bar.
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    egui::Panel::top("breadcrumb")
        .exact_size(BAR_HEIGHT)
        .frame(
            egui::Frame::new()
                .fill(theme::BG)
                .stroke(egui::Stroke::new(1.0, theme::BORDER))
                // See nav_rail.rs's identical note: explicit per the doc's "Panels | 0px", a
                // no-op today since `Frame::new()` already defaults to zero radius.
                .corner_radius(theme::RADIUS_NONE)
                .inner_margin(egui::Margin::symmetric(18, 0)),
        )
        .show(ui, |ui| {
            let is_maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
            let full_rect = ui.max_rect();

            // Draggable "move window" region: the whole bar minus a fixed strip on the right
            // reserved for the three window-control buttons, computed up front (not by reading
            // back an actual widget rect) so nothing here depends on draw order. Nothing else
            // in this bar wants clicks or drags (the labels below are hover/click-inert, and
            // the unsaved-changes dot only ever wants hover), so this can safely claim the
            // whole rect without stealing input from anything.
            let controls_width = WINDOW_BUTTON_WIDTH * 3.0;
            let drag_rect = egui::Rect::from_min_max(
                full_rect.min,
                egui::pos2(
                    (full_rect.max.x - controls_width).max(full_rect.min.x),
                    full_rect.max.y,
                ),
            );
            let drag_resp = ui.interact(
                drag_rect,
                ui.id().with("title_bar_drag"),
                egui::Sense::click_and_drag(),
            );
            if drag_resp.double_clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
            } else if drag_resp.drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }

            ui.horizontal_centered(|ui| {
                ui.label(
                    RichText::new(Text::AppName.tr(app.locale))
                        .size(13.0)
                        .color(theme::TEXT_MUTED),
                );
                ui.label(RichText::new("›").color(theme::TEXT_MUTED));
                let title = if app.prefs_open {
                    Text::ScreenTitlePrefs.tr(app.locale)
                } else {
                    i18n::screen_title(app.locale, app.screen)
                };
                ui.label(RichText::new(title).size(13.0).strong());

                if app.screen == Screen::Editor {
                    ui.label(RichText::new("›").color(theme::TEXT_MUTED));
                    ui.label(
                        RichText::new(&app.active_project().name)
                            .size(13.0)
                            .color(theme::TEXT_SECONDARY),
                    );
                    ui.add_space(4.0);
                    egui::Frame::new()
                        .fill(theme::ACCENT)
                        .corner_radius(theme::RADIUS_PILL)
                        .show(ui, |ui| {
                            ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                        });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(Text::UnsavedChanges.tr(app.locale))
                            .size(11.0)
                            .color(theme::TEXT_MUTED),
                    );
                    // Fused with the breadcrumb into one top strip, per the OCA mockup's single
                    // dense top bar (`spec/architecture/editor-ui-visual-redesign.md`'s Top bar
                    // section) — this used to be its own full-width row below the breadcrumb,
                    // which read as two visually separate bars instead of one.
                    ui.add_space(16.0);
                    crate::screens::editor::menu_bar::menu_bar(app, ui);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Flush against each other and the right edge — `drag_rect` above reserves
                    // exactly `WINDOW_BUTTON_WIDTH * 3.0` for these three buttons, which only
                    // holds if there's no gap between them eating into that reserved strip.
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let locale = app.locale;
                    // "✕" (U+2715) read as blank/tofu in a real screenshot of this app's
                    // titlebar — swapped for a plain ASCII "X", guaranteed to render in any font.
                    if window_button(ui, "X", Text::WindowClose.tr(locale), theme::ERROR).clicked()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    let (icon, label) = if is_maximized {
                        ("🗗", Text::WindowRestore.tr(locale))
                    } else {
                        ("🗖", Text::WindowMaximize.tr(locale))
                    };
                    if window_button(ui, icon, label, theme::SURFACE_2).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                    }
                    if window_button(ui, "🗕", Text::WindowMinimize.tr(locale), theme::SURFACE_2)
                        .clicked()
                    {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(locale.short_code())
                            .size(10.5)
                            .color(theme::TEXT_MUTED)
                            .monospace(),
                    );

                    // Editor-only: timecode/fps/resolution + Export, matching the OCA mockup's
                    // top-bar cluster (`spec/architecture/editor-ui-visual-redesign.md`'s Top bar
                    // section) — same data the preview panel's own HUD overlay already reads
                    // (`app.current_preview_fps()`, `preview_state.preview_texture`), just
                    // additionally surfaced here since the mockup puts a live readout in the top
                    // bar itself, not only on the video frame.
                    if app.screen == Screen::Editor {
                        ui.add_space(12.0);
                        if crate::components::primary_button(ui, Text::Export.tr(app.locale))
                            .clicked()
                        {
                            app.screen = Screen::Queue;
                        }
                        ui.add_space(12.0);
                        if let Some([w, h]) =
                            app.preview_state.preview_texture.as_ref().map(|t| t.size())
                        {
                            ui.label(
                                RichText::new(format!("{w}×{h}"))
                                    .size(11.0)
                                    .color(theme::TEXT_SECONDARY)
                                    .monospace(),
                            );
                            ui.add_space(8.0);
                        }
                        if let Some(fps) = app.current_preview_fps() {
                            ui.label(
                                RichText::new(format!("{fps:.2} FPS"))
                                    .size(11.0)
                                    .color(theme::TEXT_SECONDARY)
                                    .monospace(),
                            );
                            ui.add_space(8.0);
                        }
                        let playhead = app.active_project().timeline().playhead_secs;
                        ui.label(
                            RichText::new(format_timecode(playhead))
                                .size(11.0)
                                .color(theme::TEXT_PRIMARY)
                                .monospace(),
                        );
                    }
                });
            });
        });
}

/// One window-control button (minimize/maximize/close) — hand-painted like `nav_rail`'s
/// `rail_button` rather than a real `ui.button()`, so it reads as a slim, icon-only titlebar
/// control instead of a normal bordered button. `hover_bg` fills the whole button on hover
/// (`theme::ERROR` for close, matching the OS convention of a red close-button hover).
fn window_button(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    hover_bg: egui::Color32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(WINDOW_BUTTON_WIDTH, BAR_HEIGHT),
        egui::Sense::click(),
    );
    // Hand-painted below, so without this it carries no accessible name — screen readers and
    // UI-Automation-driven e2e tests alike would only ever see an unlabeled clickable rect.
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(rect, 0, hover_bg);
        }
        let color = if response.hovered() {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_SECONDARY
        };
        ui.painter_at(rect).text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(13.0),
            color,
        );
    }
    response
}

/// Lets the user resize the window by dragging within [`RESIZE_BORDER`] points of its edge —
/// the custom-titlebar counterpart to the OS resize handles `main.rs`'s `with_decorations(false)`
/// removes. A no-op while maximized (nothing to resize) or if the window is too small for the
/// border regions to make sense. Must run before any panel narrows `ui`'s rect — call this
/// first thing in [`crate::app::App::ui`], against the full, unclipped root `Ui`.
///
/// Deliberately covers only the south/west/east edges and the two bottom corners — not the top
/// edge or its corners. [`show`]'s title bar already claims that whole strip (horizontally
/// inset by its own frame margin) as its own drag-to-move region, so a north-facing resize zone
/// there would compete with it for the same pointer input. The west/east strips still reach all
/// the way up to the physical top of the window (outside the title bar's inset content area, so
/// no actual overlap) — only straight-up resizing is unavailable, same tradeoff several
/// real-world custom-titlebar apps make for the same reason.
pub fn handle_resize_borders(ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    if ctx.input(|i| i.viewport().maximized).unwrap_or(false) {
        return;
    }
    let rect = ui.max_rect();
    let b = RESIZE_BORDER;
    if rect.width() <= b * 2.0 || rect.height() <= b * 2.0 {
        return;
    }

    use egui::{pos2, vec2, CursorIcon, Rect, ViewportCommand};

    let regions = [
        (
            "resize_sw",
            Rect::from_min_size(pos2(rect.min.x, rect.max.y - b), vec2(b, b)),
            ViewportCommand::BeginResize(egui::ResizeDirection::SouthWest),
            CursorIcon::ResizeNeSw,
        ),
        (
            "resize_se",
            Rect::from_min_size(pos2(rect.max.x - b, rect.max.y - b), vec2(b, b)),
            ViewportCommand::BeginResize(egui::ResizeDirection::SouthEast),
            CursorIcon::ResizeNwSe,
        ),
        (
            "resize_s",
            Rect::from_min_max(
                pos2(rect.min.x + b, rect.max.y - b),
                pos2(rect.max.x - b, rect.max.y),
            ),
            ViewportCommand::BeginResize(egui::ResizeDirection::South),
            CursorIcon::ResizeVertical,
        ),
        (
            "resize_w",
            Rect::from_min_max(
                pos2(rect.min.x, rect.min.y),
                pos2(rect.min.x + b, rect.max.y - b),
            ),
            ViewportCommand::BeginResize(egui::ResizeDirection::West),
            CursorIcon::ResizeHorizontal,
        ),
        (
            "resize_e",
            Rect::from_min_max(
                pos2(rect.max.x - b, rect.min.y),
                pos2(rect.max.x, rect.max.y - b),
            ),
            ViewportCommand::BeginResize(egui::ResizeDirection::East),
            CursorIcon::ResizeHorizontal,
        ),
    ];

    for (salt, region_rect, begin_resize, cursor) in regions {
        let response = ui.interact(region_rect, ui.id().with(salt), egui::Sense::drag());
        if response.hovered() || response.dragged() {
            ctx.set_cursor_icon(cursor);
        }
        if response.drag_started() {
            ctx.send_viewport_cmd(begin_resize);
        }
    }
}
