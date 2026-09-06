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

use eframe::egui::{self, RichText};

use avcore::media::format_timecode;

use crate::app::App;
use crate::i18n::Text;
use crate::icons;
use crate::theme;

/// Idle time (no pointer movement/click) before the fullscreen preview overlay's controls
/// start fading out — see [`App::fullscreen_controls_opacity`].
pub const FULLSCREEN_CONTROLS_IDLE_SECS: f32 = 2.5;

/// Renders the Editor preview panel as a fullscreen overlay covering the whole window —
/// called instead of the normal nav-rail/breadcrumb/screen chain from
/// `impl eframe::App for App::ui` whenever `app.preview_state.fullscreen_preview` is set, so the overlay
/// truly covers everything rather than sitting inside the Editor's three-column layout.
/// Controls (play/pause/seek/exit) fade out after [`FULLSCREEN_CONTROLS_IDLE_SECS`] of no
/// pointer activity and fade back in on the next movement or click; Esc always exits
/// regardless of control visibility.
pub fn fullscreen_preview_overlay(app: &mut App, ui: &mut egui::Ui) {
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.exit_fullscreen_preview();
        return;
    }
    let pointer_active =
        ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO || i.pointer.any_pressed());
    if pointer_active {
        app.note_fullscreen_controls_activity();
    }
    // Playback and the fade both need a steady repaint cadence, not just the throttled
    // 200ms one the rest of the app falls back to when idle.
    ui.ctx().request_repaint();

    let locale = app.locale;
    let opacity = app.fullscreen_controls_opacity();
    let texture = app.preview_state.preview_texture.clone();
    let timeline_duration = app.active_project().timeline().duration_secs();
    let playhead = app.active_project().timeline().playhead_secs;

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::BLACK))
        .show(ui, |ui| {
            let avail = ui.available_rect_before_wrap();
            if let Some(texture) = &texture {
                let tex_size = texture.size_vec2();
                let tex_aspect = tex_size.x / tex_size.y.max(1.0);
                let mut size = avail.size();
                if size.x / size.y > tex_aspect {
                    size.x = size.y * tex_aspect;
                } else {
                    size.y = size.x / tex_aspect;
                }
                let rect = egui::Rect::from_center_size(avail.center(), size);
                ui.painter().image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new(icons::PLAY_STR)
                            .family(icons::family())
                            .size(64.0)
                            .color(theme::TEXT_MUTED),
                    );
                });
            }

            if opacity <= 0.0 {
                return;
            }

            egui::Area::new(ui.id().with("fullscreen_exit"))
                .fixed_pos(egui::pos2(avail.right() - 56.0, avail.top() + 16.0))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    let clicked = ui
                        .add(
                            egui::Button::new(
                                RichText::new("X")
                                    .size(16.0)
                                    .color(theme::TEXT_SECONDARY.gamma_multiply(opacity)),
                            )
                            .fill(theme::SURFACE_2.gamma_multiply(opacity)),
                        )
                        .on_hover_text(Text::ExitFullscreenPreview.tr(locale))
                        .clicked();
                    if clicked {
                        app.exit_fullscreen_preview();
                    }
                });

            egui::Area::new(ui.id().with("fullscreen_controls"))
                .fixed_pos(egui::pos2(avail.left() + 24.0, avail.bottom() - 76.0))
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    ui.set_width(avail.width() - 48.0);
                    egui::Frame::new()
                        .fill(theme::SURFACE.gamma_multiply(0.85 * opacity))
                        .corner_radius(theme::RADIUS_MD)
                        .inner_margin(egui::Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .small_button(
                                        RichText::new(crate::icons::SKIP_BACK_STR)
                                            .family(crate::icons::family()),
                                    )
                                    .on_hover_text(Text::SeekToStart.tr(locale))
                                    .clicked()
                                {
                                    app.seek_preview(0.0);
                                }
                                let play_icon = if app.preview_state.preview_playing {
                                    crate::icons::PAUSE_STR
                                } else {
                                    crate::icons::PLAY_STR
                                };
                                if ui
                                    .button(
                                        RichText::new(play_icon)
                                            .family(crate::icons::family())
                                            .color(theme::ACCENT.gamma_multiply(opacity)),
                                    )
                                    .on_hover_text(Text::ShortcutPlayPause.tr(locale))
                                    .clicked()
                                {
                                    app.toggle_preview_playback();
                                }
                                if ui
                                    .small_button(
                                        RichText::new(crate::icons::SKIP_FORWARD_STR)
                                            .family(crate::icons::family()),
                                    )
                                    .on_hover_text(Text::SeekToEnd.tr(locale))
                                    .clicked()
                                {
                                    app.seek_preview(timeline_duration);
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "{} / {}",
                                        format_timecode(playhead),
                                        format_timecode(timeline_duration.max(playhead))
                                    ))
                                    .size(12.0)
                                    .color(theme::TEXT_SECONDARY.gamma_multiply(opacity))
                                    .monospace(),
                                );
                            });
                            if timeline_duration > 0.0 {
                                let mut position = playhead;
                                let slider = ui.add(
                                    egui::Slider::new(&mut position, 0.0..=timeline_duration)
                                        .show_value(false),
                                );
                                if slider.changed() {
                                    app.seek_preview(position);
                                }
                            }
                        });
                });
        });
}
