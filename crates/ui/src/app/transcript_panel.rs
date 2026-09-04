// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! CF-01 slice 2 (`spec/architecture/competitive-feature-plan.md`) — a searchable panel over
//! the previewed clip's own [`avcore::TranscriptDocument`] (slice 1), seeking on word selection
//! and highlighting the word currently under the playhead. Same modal-panel shape
//! `App::show_timeline_index_panel` already established for markers — read everything needed
//! into locals, mutate `self` only after `modal.show` returns, since the closure can't safely
//! re-borrow `self` from inside itself.

use eframe::egui;

use crate::components;
use crate::i18n::Text;
use crate::theme;

use super::App;

impl App {
    /// Opens/closes the Transcript panel — what the toolbar's "📝" button does.
    pub fn toggle_transcript_panel(&mut self) {
        self.transcript_panel_open = !self.transcript_panel_open;
    }

    /// Loads (or clears) `transcript_panel_state` for whichever asset the previewed clip
    /// currently plays from, if that's changed since the last call — called once per frame
    /// alongside [`App::ensure_preview_loaded`], so the panel always reflects "the clip you're
    /// looking at right now" without re-reading the transcript sidecar off disk every frame.
    /// Compound clips (nested sequences, `ClipInstance::nested_sequence_id`) have no `asset_id`
    /// to transcribe at all — treated the same as "no transcript exists," not an error.
    pub fn ensure_transcript_loaded_for_preview(&mut self) {
        let asset_id = self
            .current_preview_video_clip()
            .filter(|clip| clip.nested_sequence_id.is_none())
            .map(|clip| clip.asset_id);
        if asset_id == self.transcript_panel_state.loaded_asset_id {
            return;
        }
        self.transcript_panel_state.loaded_asset_id = asset_id;
        self.transcript_panel_state.document = asset_id.and_then(|id| {
            let dir = avcore::transcript_cache_dir_for_project(self.active_project());
            match avcore::load_transcript_document(&dir, id) {
                Ok(document) => document,
                Err(error) => {
                    tracing::warn!(asset_id = id, ?error, "failed to load transcript document");
                    None
                }
            }
        });
    }

    /// The searchable Transcript panel. Shown while [`App::transcript_panel_open`] is set; a
    /// no-op otherwise. Two modes: with [`App::transcript_search_project`] off (default) it
    /// filters the previewed clip's own document (slice 2); turned on with a query it searches
    /// every asset in the project library that has a transcript (slice 3) and shows hits grouped
    /// by asset. Clicking a word seeks the timeline — within the previewed asset through that
    /// clip, from any other asset through the first timeline clip that uses it.
    pub(super) fn show_transcript_panel(&mut self, ctx: &egui::Context) {
        if !self.transcript_panel_open {
            return;
        }
        let locale = self.locale;
        let mut search = self.transcript_search.clone();
        let mut search_project = self.transcript_search_project;

        let query = search.trim().to_lowercase();
        let project_search = search_project && !query.is_empty();

        let clip = self
            .current_preview_video_clip()
            .filter(|clip| clip.nested_sequence_id.is_none());
        let document = self.transcript_panel_state.document.clone();

        let playhead_secs = self.active_project().timeline().playhead_secs;
        // Media-relative position of the playhead inside the previewed clip (see seek below for
        // the inverse) — the word covering it is the one highlighted.
        let current_source_secs = clip.as_ref().map(|clip| {
            if clip.frozen {
                clip.source_in_secs
            } else {
                clip.source_in_secs
                    + (playhead_secs - clip.start_secs) * clip.speed_factor.max(0.01) as f64
            }
        });
        let current_word_id = document.as_ref().and_then(|d| {
            current_source_secs.and_then(|s| {
                d.words
                    .iter()
                    .find(|w| s >= w.start_secs && s < w.end_secs)
                    .map(|w| w.id)
            })
        });

        // Project-wide hits, when that mode is active.
        let project_hits = if project_search {
            let dir = avcore::transcript_cache_dir_for_project(self.active_project());
            avcore::search_transcripts_in_project(
                &dir,
                &self.active_project().media_library,
                &query,
            )
        } else {
            Vec::new()
        };

        let mut seek_to: Option<(u64, f64)> = None;
        let mut close = false;
        let mut help_message: Option<&str> = None;

        let modal = egui::Modal::new(egui::Id::new("transcript_panel"));
        let response = modal.show(ctx, |ui| {
            ui.set_width(420.0);
            ui.horizontal(|ui| {
                components::modal_title(ui, Text::TranscriptPanelTitle.tr(locale));
            });
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut search)
                    .desired_width(f32::INFINITY)
                    .hint_text(if search_project {
                        Text::TranscriptProjectSearchHint.tr(locale)
                    } else {
                        Text::TranscriptPanelSearchHint.tr(locale)
                    }),
            );
            ui.add_space(4.0);
            if ui
                .selectable_label(search_project, Text::TranscriptSearchProject.tr(locale))
                .clicked()
            {
                search_project = !search_project;
            }
            ui.add_space(8.0);

            if project_search {
                // Group hits by asset (order of first appearance), then source-time within.
                let mut seen_assets: Vec<u64> = Vec::new();
                for hit in &project_hits {
                    if !seen_assets.contains(&hit.asset_id) {
                        seen_assets.push(hit.asset_id);
                    }
                }
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        if project_hits.is_empty() {
                            ui.label(
                                egui::RichText::new(Text::TranscriptProjectEmpty.tr(locale))
                                    .color(theme::TEXT_MUTED),
                            );
                        } else {
                            for asset_id in &seen_assets {
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(
                                        project_hits
                                            .iter()
                                            .find(|h| h.asset_id == *asset_id)
                                            .map(|h| h.asset_name.as_str())
                                            .unwrap_or(""),
                                    )
                                    .color(theme::ACCENT)
                                    .strong(),
                                );
                                for hit in project_hits.iter().filter(|h| h.asset_id == *asset_id) {
                                    let is_current = Some(hit.word.id) == current_word_id;
                                    let label = format!(
                                        "{} - {}",
                                        avcore::media::format_timecode(hit.word.start_secs),
                                        hit.word.text
                                    );
                                    let response =
                                        ui.selectable_label(is_current, egui::RichText::new(label));
                                    response.clone().on_hover_text(format!(
                                        "{} ({:.0}%)",
                                        avcore::media::format_timecode(hit.word.start_secs),
                                        hit.word.confidence * 100.0
                                    ));
                                    if response.clicked() {
                                        seek_to = Some((hit.asset_id, hit.word.start_secs));
                                    }
                                }
                            }
                        }
                    });
            } else {
                let Some(document) = &document else {
                    let message = if clip.is_some() {
                        Text::TranscriptPanelNoTranscript.tr(locale)
                    } else {
                        Text::TranscriptPanelNoClip.tr(locale)
                    };
                    help_message = Some(message);
                    return;
                };
                let visible: Vec<&avcore::TranscriptWord> = document
                    .words
                    .iter()
                    .filter(|w| query.is_empty() || w.text.to_lowercase().contains(&query))
                    .collect();

                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        if visible.is_empty() {
                            ui.label(
                                egui::RichText::new(Text::TranscriptPanelEmpty.tr(locale))
                                    .color(theme::TEXT_MUTED),
                            );
                        }
                        ui.horizontal_wrapped(|ui| {
                            for word in &visible {
                                let is_current = Some(word.id) == current_word_id;
                                let text = egui::RichText::new(&word.text).size(13.0).color(
                                    if is_current {
                                        theme::ACCENT
                                    } else if word.confidence < 0.5 {
                                        theme::TEXT_MUTED
                                    } else {
                                        theme::TEXT_PRIMARY
                                    },
                                );
                                let response = ui.selectable_label(is_current, text);
                                response.clone().on_hover_text(format!(
                                    "{} ({:.0}%)",
                                    avcore::media::format_timecode(word.start_secs),
                                    word.confidence * 100.0
                                ));
                                if response.clicked() {
                                    seek_to = Some((document.asset_id, word.start_secs));
                                }
                            }
                        });
                    });
            }

            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
            ui.add_space(4.0);
            if ui.button(Text::WindowClose.tr(locale)).clicked() {
                close = true;
            }
        });

        self.transcript_search = search;
        self.transcript_search_project = search_project;
        if let Some(message) = help_message {
            self.show_empty_transcript_panel(ctx, message);
            return;
        }
        if response.should_close() || close {
            self.transcript_panel_open = false;
        }
        if let Some((asset_id, source_secs)) = seek_to {
            if let Some(timeline_secs) = self.map_asset_source_to_timeline(asset_id, source_secs) {
                self.active_project_mut().timeline_mut().playhead_secs = timeline_secs;
                self.seek_preview(timeline_secs);
            }
        }
    }

    /// Maps a media-relative `source_secs` of `asset_id` back onto the timeline through the
    /// first clip that uses that asset (preferring the currently previewed one), so a word from
    /// any asset's transcript can be located on the timeline. `None` if no clip on the timeline
    /// uses the asset — the search hit stays informational.
    fn map_asset_source_to_timeline(&self, asset_id: u64, source_secs: f64) -> Option<f64> {
        let timeline = self.active_project().timeline();
        let prefer = self
            .current_preview_video_clip()
            .filter(|clip| clip.asset_id == asset_id);
        if let Some(clip) = prefer {
            return Some(self.clip_media_to_timeline(&clip, source_secs));
        }
        for track in &timeline.tracks {
            if let Some(clip) = track.clips.iter().find(|c| c.asset_id == asset_id) {
                return Some(self.clip_media_to_timeline(clip, source_secs));
            }
        }
        None
    }

    fn clip_media_to_timeline(&self, clip: &avcore::ClipInstance, source_secs: f64) -> f64 {
        clip.start_secs + (source_secs - clip.source_in_secs) / clip.speed_factor.max(0.01) as f64
    }

    /// Builds a [`avcore::TranscriptDocument`] from a just-finished transcription and saves it
    /// as `asset_id`'s sidecar (CF-01 slice 1) — called from [`App::pump_transcribe`] right
    /// after a transcription completes, so the existing "Transcrever" flow (subtitle `TextClip`
    /// generation, unchanged) now also produces the persisted editing-surface document the
    /// Transcript panel reads, with no separate action needed. A save failure is logged and
    /// otherwise swallowed — the subtitle `TextClip`s this transcription also produces are the
    /// half of the result actually visible/usable in the timeline regardless, so a failed
    /// sidecar write shouldn't surface as if the whole transcription failed.
    pub(super) fn save_transcript_document_for(
        &mut self,
        asset_id: u64,
        segments: &[avcore::TranscribeSegment],
    ) {
        let document =
            avcore::TranscriptDocument::from_transcribe_segments(asset_id, None, segments);
        let dir = avcore::transcript_cache_dir_for_project(self.active_project());
        match avcore::save_transcript_document(&dir, &document) {
            Ok(_) => {
                // Refresh the panel immediately if it's already showing this asset, rather than
                // waiting for the previewed clip to change and re-trigger the lazy load.
                if self.transcript_panel_state.loaded_asset_id == Some(asset_id) {
                    self.transcript_panel_state.document = Some(document);
                }
            }
            Err(error) => {
                tracing::warn!(asset_id, ?error, "failed to save transcript document");
            }
        }
    }

    fn show_empty_transcript_panel(&mut self, ctx: &egui::Context, message: &str) {
        let locale = self.locale;
        let mut close = false;
        let modal = egui::Modal::new(egui::Id::new("transcript_panel"));
        let response = modal.show(ctx, |ui| {
            ui.set_width(360.0);
            components::modal_title(ui, Text::TranscriptPanelTitle.tr(locale));
            ui.add_space(8.0);
            ui.label(egui::RichText::new(message).color(theme::TEXT_MUTED));
            ui.add_space(8.0);
            if ui.button(Text::WindowClose.tr(locale)).clicked() {
                close = true;
            }
        });
        if response.should_close() || close {
            self.transcript_panel_open = false;
        }
    }
}
