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

//! Dialogs that edit the currently selected timeline clip.

use eframe::egui;

use crate::components;
use crate::i18n::Text;

use super::App;

impl App {
    /// Shows the custom-speed-ramp dialog when `speed_ramp_dialog` is `Some` (the fixed 0.5x/2x
    /// presets already apply directly with no dialog). Lets the user pick a start speed, end
    /// speed, step count, and a "smooth" toggle — unchecked calls
    /// [`App::apply_speed_ramp_to_selected_clip`] (the stepped approximation), checked calls
    /// [`App::apply_smooth_speed_ramp_to_selected_clip`] (the continuous curve,
    /// `spec/ROADMAP.md` item 29's later follow-up) and hides the now-irrelevant step count
    /// field. `steps_buf` is parsed on confirm; an invalid or sub-2 value is clamped to 2 rather
    /// than rejected, since [`App::apply_speed_ramp_to_selected_clip`] itself already no-ops
    /// below 2.
    pub(super) fn show_speed_ramp_modal(&mut self, ctx: &egui::Context) {
        let Some((clip_id, _, _, _, _)) = self.speed_ramp_dialog.as_ref() else {
            return;
        };
        let clip_id = *clip_id;
        let locale = self.locale;
        let modal = egui::Modal::new(egui::Id::new("speed_ramp_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(320.0);
            components::modal_title(ui, Text::SpeedRampCustomTitle.tr(locale));
            ui.add_space(8.0);
            let (_, start_speed, end_speed, steps_buf, smooth) =
                self.speed_ramp_dialog.as_mut().unwrap();
            ui.label(Text::SpeedRampStartSpeedLabel.tr(locale));
            ui.add(
                egui::DragValue::new(start_speed)
                    .range(0.1..=20.0)
                    .speed(0.01),
            );
            ui.add_space(4.0);
            ui.label(Text::SpeedRampEndSpeedLabel.tr(locale));
            ui.add(
                egui::DragValue::new(end_speed)
                    .range(0.1..=20.0)
                    .speed(0.01),
            );
            ui.add_space(4.0);
            ui.checkbox(smooth, Text::SpeedRampSmoothToggle.tr(locale));
            let smooth = *smooth;
            let mut steps_edit_lost_focus_enter = false;
            if !smooth {
                ui.add_space(4.0);
                ui.label(Text::SpeedRampStepsLabel.tr(locale));
                let steps_edit = ui.add(
                    egui::TextEdit::singleline(steps_buf)
                        .desired_width(60.0)
                        .hint_text(Text::SpeedRampStepsHint.tr(locale)),
                );
                steps_edit_lost_focus_enter =
                    steps_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            }
            if steps_edit_lost_focus_enter {
                confirmed = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if components::primary_button(ui, Text::SpeedRampApply.tr(locale)).clicked() {
                    confirmed = true;
                }
                if ui.button(Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.speed_ramp_dialog = None;
            return;
        }
        if confirmed {
            if let Some((_, start_speed, end_speed, steps_buf, smooth)) =
                self.speed_ramp_dialog.take()
            {
                self.selected_clip_id = Some(clip_id);
                if smooth {
                    self.apply_smooth_speed_ramp_to_selected_clip(start_speed, end_speed);
                } else {
                    let steps = steps_buf.trim().parse::<usize>().unwrap_or(0).max(2);
                    self.apply_speed_ramp_to_selected_clip(start_speed, end_speed, steps);
                }
            }
        }
    }
}
