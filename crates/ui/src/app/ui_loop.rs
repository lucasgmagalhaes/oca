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

//! The top-level eframe frame loop.

use std::time::Duration;

use eframe::egui;

use crate::screens;

use super::*;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump_export_queue();
        self.pump_nested_sequence_renders();
        self.pump_import_queue();
        self.pump_sound_library_queue();
        self.pump_transcribe();
        self.pump_auto_reframe();
        self.pump_dynamic_reframe();
        self.pump_motion_tracking();
        self.pump_voice_cleanup_preview();
        self.pump_scene_cut_detection();
        self.pump_matte_generation();
        self.pump_privacy_blur_generation();
        self.pump_text_to_speech();
        self.pump_youtube_download();
        self.pump_watch_folder();
        self.pump_update_check();
        self.pump_thumbnail_queue(ui.ctx());
        self.pump_preview_frame(ui.ctx());
        self.pump_autosave();
        self.pump_autosave_restore(ui.ctx());
        self.handle_dropped_files(ui.ctx());
        // Detect the prefs modal closing (true → false) and persist the new settings.
        if self.prev_prefs_open && !self.prefs_open {
            self.save_prefs();
        }
        self.prev_prefs_open = self.prefs_open;
        if self.crash_detected {
            self.crash_detected = false;
            tracing::warn!("crash sentinel found — previous session did not exit cleanly");
            self.push_toast(crate::i18n::Text::CrashDetected.tr(self.locale).to_string());
        }
        if self.preview_state.preview_playing {
            // Smooth video needs every-frame repaints; the 200ms throttle below would show
            // it as a slideshow.
            ui.ctx().request_repaint();
            self.sample_preview_frame_telemetry(ui.ctx());
        } else {
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }

        if self.preview_state.fullscreen_preview {
            screens::editor::fullscreen_preview_overlay(self, ui);
            return;
        }

        // Must run before any panel narrows `ui`'s rect — see its own doc comment.
        screens::breadcrumb::handle_resize_borders(ui);

        screens::nav_rail::show(self, ui);

        // Must run after `nav_rail::show` (which applies the click that changes `self.screen`)
        // and before `breadcrumb::show` — the breadcrumb reads `active_project()` too (project
        // name + unsaved-changes dot) whenever `screen == Editor`, and it renders before the
        // `editor`/`library` screens' own `ensure_active_project()` call gets a chance to.
        if matches!(self.screen, Screen::Editor | Screen::Library) {
            self.ensure_active_project();
        }

        screens::breadcrumb::show(self, ui);

        egui::CentralPanel::default().show(ui, |ui| match self.screen {
            Screen::Home => screens::home::show(self, ui),
            Screen::Editor => screens::editor::show(self, ui),
            Screen::Library => screens::library::show(self, ui),
            Screen::SoundLibrary => screens::sound_library::show(self, ui),
            Screen::Queue => screens::queue::show(self, ui),
            Screen::WatchFolder => screens::watch_folder::show(self, ui),
        });
        self.show_prefs_modal(ui.ctx());
        self.show_about_modal(ui.ctx());
        self.show_rename_project_modal(ui.ctx());
        self.show_rename_sequence_modal(ui.ctx());
        self.show_rename_track_modal(ui.ctx());
        self.show_delete_track_modal(ui.ctx());
        self.show_speed_ramp_modal(ui.ctx());
        self.show_delete_sequence_modal(ui.ctx());
        self.show_text_color_modal(ui.ctx());
        self.show_export_conflict_modal(ui.ctx());
        self.show_crash_review_modal(ui.ctx());
        self.show_save_layer_template_modal(ui.ctx());
        self.show_layer_templates_menu(ui.ctx());
        self.show_apply_layer_template_modal(ui.ctx());
        self.show_apply_graphic_template_modal(ui.ctx());
        self.show_tts_modal(ui.ctx());
        self.show_youtube_download_modal(ui.ctx());
        self.show_timeline_index_panel(ui.ctx());
        self.show_transcript_panel(ui.ctx());
        self.show_silence_review_modal(ui.ctx());
        self.show_transcript_proposals_modal(ui.ctx());
        self.show_smart_bin_modal(ui.ctx());
        self.show_toasts(ui.ctx());
        self.show_drop_hint_overlay(ui.ctx());
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Persist synchronously — see save_prefs_sync's doc comment on why the normal
        // background-thread save_prefs can't be trusted to finish before the process exits.
        self.save_prefs_sync();
        // Clean exit — remove the crash sentinel so the next launch doesn't think we crashed.
        let _ = std::fs::remove_file(sentinel_path());
        tracing::info!("clean exit — crash sentinel removed");
    }
}
