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

//! CF-03 slice 3 (`spec/architecture/competitive-feature-plan.md`): the properties panel's
//! "Prévia A/B" button for the voice-cleanup effect. Thin `App` wrapper around
//! `avcore::voice_cleanup_preview` — this module only turns the result into background-job
//! state, a toast on failure, or a loaded playback pipeline, the same "build the real payload in
//! `core`, `ui` just drives it" shape every other background feature in this app already uses
//! (e.g. `motion_tracking.rs`).

use std::sync::atomic::AtomicBool;

use avcore::voice_cleanup_preview::{render_voice_cleanup_preview, VoiceCleanupParams};

use super::{App, VoiceCleanupPreviewEvent};

/// Where rendered A/B samples land — a single fixed scratch directory, not project- or
/// clip-scoped, since only the most recent render is ever kept around at all (each new render
/// overwrites the previous one, per `render_voice_cleanup_preview`'s own doc comment).
fn preview_out_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("oca_voice_cleanup_preview")
}

impl App {
    /// Renders both A/B samples for the selected clip's own voice-cleanup parameters on a
    /// background thread — what the properties panel's "🔊 Prévia A/B" button does. A no-op if
    /// nothing is selected, the selected clip has no resolvable source asset, or a render is
    /// already in flight.
    pub fn spawn_voice_cleanup_preview(&mut self) {
        if self.voice_cleanup_preview_state.rendering_clip_id.is_some() {
            return;
        }
        let Some(clip) = self.selected_clip() else {
            return;
        };
        let clip_id = clip.id;
        let source_in_secs = clip.source_in_secs;
        let source_out_secs = clip.source_out_secs;
        let params = VoiceCleanupParams {
            noise_floor_db: clip.voice_cleanup_noise_floor_db,
            compressor_threshold_db: clip.voice_cleanup_compressor_threshold_db,
            compressor_ratio: clip.voice_cleanup_compressor_ratio,
            ceiling_linear: clip.voice_cleanup_ceiling_linear,
        };
        let asset_id = clip.asset_id;
        let Some(asset) = self
            .active_project()
            .media_library
            .iter()
            .find(|a| a.id == asset_id)
        else {
            return;
        };
        let source_path = asset.source_path.clone();
        // The sequence's own configured export target, not a hardcoded value, so the preview's
        // overall loudness matches what a real export of this clip would actually produce (see
        // `render_voice_cleanup_preview`'s own doc comment).
        let target_lufs = self
            .active_project()
            .active_sequence()
            .export_settings
            .target_lufs;
        let out_dir = preview_out_dir();

        self.voice_cleanup_preview_state.rendering_clip_id = Some(clip_id);
        let tx = self.voice_cleanup_preview_state.tx.clone();
        std::thread::spawn(move || {
            let result = render_voice_cleanup_preview(
                &source_path,
                source_in_secs,
                source_out_secs,
                params,
                target_lufs,
                &out_dir,
                &AtomicBool::new(false),
            )
            .map_err(|e| e.to_string());
            let _ = tx.send(VoiceCleanupPreviewEvent::Done { clip_id, result });
        });
    }

    /// Applies a finished voice-cleanup preview render. Called once per frame from
    /// [`eframe::App::ui`], same as [`App::pump_motion_tracking`].
    pub(super) fn pump_voice_cleanup_preview(&mut self) {
        while let Ok(event) = self.voice_cleanup_preview_state.rx.try_recv() {
            match event {
                VoiceCleanupPreviewEvent::Done { clip_id, result } => {
                    self.voice_cleanup_preview_state.rendering_clip_id = None;
                    match result {
                        Ok(result) => {
                            self.voice_cleanup_preview_state.result = Some((clip_id, result));
                        }
                        Err(e) => {
                            self.push_toast(format!(
                                "{}: {e}",
                                crate::i18n::Text::VoiceCleanupPreviewFailed.tr(self.locale)
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Plays back the bypassed (`processed: false`) or processed (`true`) A/B sample from the
    /// last completed render — what the properties panel's "▶ Original"/"▶ Tratado" buttons do.
    /// A no-op if no render has completed yet. Replaces whatever sample was previously loaded
    /// (dropping that `Preview` stops it), so only one of the two samples ever plays at a time,
    /// and never disturbs the main timeline preview pipeline (`PreviewState::preview`), which
    /// this deliberately doesn't touch.
    pub fn play_voice_cleanup_preview_sample(&mut self, processed: bool) {
        let Some((_, result)) = &self.voice_cleanup_preview_state.result else {
            return;
        };
        let path = if processed {
            &result.processed.path
        } else {
            &result.bypassed.path
        };
        match avcore::preview::Preview::open(path, None) {
            Ok(preview) => {
                if preview.play().is_ok() {
                    self.voice_cleanup_preview_state.player = Some(preview);
                    self.voice_cleanup_preview_state.player_is_processed = processed;
                }
            }
            Err(e) => self.push_toast(format!(
                "{}: {e}",
                crate::i18n::Text::VoiceCleanupPreviewPlaybackFailed.tr(self.locale)
            )),
        }
    }
}
