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

use avcore::MarkerKind;
use eframe::egui;

use crate::components;
use crate::i18n::Text;
use crate::theme;

use super::App;

impl App {
    /// The searchable Timeline Index panel (`ROADMAP.md` P2 item 9) — every marker on the
    /// active sequence, filterable by [`App::marker_search`], click-to-seek, inline label
    /// editing, add/remove/toggle-complete. Shown while [`App::timeline_index_open`] is set;
    /// a no-op otherwise. Same "read everything needed into locals, mutate `self` only after
    /// `modal.show` returns" shape every other modal in this file uses, since the closure can't
    /// safely re-borrow `self` from inside itself.
    pub(super) fn show_timeline_index_panel(&mut self, ctx: &egui::Context) {
        if !self.timeline_index_open {
            return;
        }
        let locale = self.locale;
        let mut search = self.marker_search.clone();
        let markers: Vec<avcore::Marker> = self
            .active_project()
            .timeline()
            .markers_sorted()
            .into_iter()
            .cloned()
            .collect();

        let mut seek_to: Option<f64> = None;
        let mut remove_id: Option<u64> = None;
        let mut toggle_id: Option<u64> = None;
        let mut label_edit: Option<(u64, String)> = None;
        let mut kind_edit: Option<(u64, avcore::MarkerKind)> = None;
        let mut add_kind: Option<avcore::MarkerKind> = None;
        let mut close = false;

        let modal = egui::Modal::new(egui::Id::new("timeline_index_panel"));
        let response = modal.show(ctx, |ui| {
            ui.set_width(420.0);
            components::modal_title(ui, Text::TimelineIndexTitle.tr(locale));
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut search)
                    .desired_width(f32::INFINITY)
                    .hint_text(Text::TimelineIndexSearchHint.tr(locale)),
            );
            ui.add_space(8.0);

            let query = search.to_lowercase();
            let visible: Vec<&avcore::Marker> = markers
                .iter()
                .filter(|m| query.is_empty() || m.label.to_lowercase().contains(&query))
                .collect();

            egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    if visible.is_empty() {
                        ui.label(
                            egui::RichText::new(Text::TimelineIndexEmpty.tr(locale))
                                .color(theme::TEXT_MUTED),
                        );
                    }
                    for marker in &visible {
                        ui.horizontal(|ui| {
                            if marker.kind == avcore::MarkerKind::ToDo {
                                let mut completed = marker.completed;
                                if ui.checkbox(&mut completed, "").changed() {
                                    toggle_id = Some(marker.id);
                                }
                            }
                            let mut kind = marker.kind;
                            egui::ComboBox::from_id_salt(("marker_kind", marker.id))
                                .selected_text(marker_kind_icon(kind))
                                .width(36.0)
                                .show_ui(ui, |ui| {
                                    for candidate in avcore::MarkerKind::ALL {
                                        ui.selectable_value(
                                            &mut kind,
                                            *candidate,
                                            marker_kind_icon(*candidate),
                                        );
                                    }
                                });
                            if kind != marker.kind {
                                kind_edit = Some((marker.id, kind));
                            }
                            if ui
                                .button(avcore::media::format_timecode(marker.position_secs))
                                .clicked()
                            {
                                seek_to = Some(marker.position_secs);
                            }
                            let mut label = marker.label.clone();
                            let label_resp = ui.add(
                                egui::TextEdit::singleline(&mut label)
                                    .desired_width(180.0)
                                    .hint_text(Text::TimelineIndexLabelHint.tr(locale)),
                            );
                            if label_resp.changed() {
                                label_edit = Some((marker.id, label));
                            }
                            if components::icon_button(
                                ui,
                                "X",
                                Text::RemoveMarker.tr(locale),
                                components::IconButtonOpts::default(),
                            )
                            .clicked()
                            {
                                remove_id = Some(marker.id);
                            }
                        });
                    }
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .button(Text::TimelineIndexAddStandard.tr(locale))
                    .clicked()
                {
                    add_kind = Some(avcore::MarkerKind::Standard);
                }
                if ui.button(Text::TimelineIndexAddToDo.tr(locale)).clicked() {
                    add_kind = Some(avcore::MarkerKind::ToDo);
                }
                if ui
                    .button(Text::TimelineIndexAddChapter.tr(locale))
                    .clicked()
                {
                    add_kind = Some(avcore::MarkerKind::Chapter);
                }
            });
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                close = true;
            }
            ui.add_space(4.0);
            if ui.button(Text::WindowClose.tr(locale)).clicked() {
                close = true;
            }
        });

        self.marker_search = search;
        if response.should_close() || close {
            self.timeline_index_open = false;
        }
        if let Some(kind) = add_kind {
            self.add_marker_at_playhead(kind);
        }
        if let Some(marker_id) = remove_id {
            self.remove_marker(marker_id);
        }
        if let Some(marker_id) = toggle_id {
            self.toggle_marker_completed(marker_id);
        }
        if let Some((marker_id, label)) = label_edit {
            self.set_marker_label(marker_id, label);
        }
        if let Some((marker_id, kind)) = kind_edit {
            self.set_marker_kind(marker_id, kind);
        }
        if let Some(position_secs) = seek_to {
            self.seek_preview(position_secs);
        }
    }
    /// Opens/closes the Timeline Index panel — what the toolbar's "🏷" button does.
    pub fn toggle_timeline_index(&mut self) {
        self.timeline_index_open = !self.timeline_index_open;
    }

    /// Adds a new marker of `kind` at the active sequence's current playhead position — what
    /// the Timeline Index panel's "Add" buttons do. Returns the new marker's id.
    pub fn add_marker_at_playhead(&mut self, kind: MarkerKind) -> u64 {
        self.push_undo_snapshot();
        let playhead = self.active_project().timeline().playhead_secs;
        self.active_project_mut()
            .timeline_mut()
            .add_marker(playhead, kind)
    }

    /// Removes `marker_id`, if it exists — what the Timeline Index panel's trash-can button
    /// does.
    pub fn remove_marker(&mut self, marker_id: u64) {
        self.push_undo_snapshot();
        self.active_project_mut()
            .timeline_mut()
            .remove_marker(marker_id);
    }

    /// Sets `marker_id`'s label, if it exists — what typing into the Timeline Index panel's
    /// inline text field does. Uses [`App::push_undo_snapshot_for_drag`]'s coalescing, same as
    /// every other text-field write-back in this codebase, so each keystroke doesn't become its
    /// own undo step.
    pub fn set_marker_label(&mut self, marker_id: u64, label: String) {
        self.push_undo_snapshot_for_drag();
        if let Some(marker) = self
            .active_project_mut()
            .timeline_mut()
            .marker_mut(marker_id)
        {
            marker.label = label;
        }
    }

    /// Sets `marker_id`'s [`MarkerKind`], if it exists.
    pub fn set_marker_kind(&mut self, marker_id: u64, kind: MarkerKind) {
        self.push_undo_snapshot();
        if let Some(marker) = self
            .active_project_mut()
            .timeline_mut()
            .marker_mut(marker_id)
        {
            marker.kind = kind;
        }
    }

    /// Flips `marker_id`'s `completed` flag, if it exists — only meaningful for a
    /// [`MarkerKind::ToDo`] marker, but harmless (if pointless) to call on any other kind.
    pub fn toggle_marker_completed(&mut self, marker_id: u64) {
        self.push_undo_snapshot();
        if let Some(marker) = self
            .active_project_mut()
            .timeline_mut()
            .marker_mut(marker_id)
        {
            marker.completed = !marker.completed;
        }
    }
}

/// A short, locale-neutral icon for one [`avcore::MarkerKind`] — the Timeline Index panel's
/// per-marker kind picker and its own selected-value label both use this, so a marker's kind
/// always reads the same glyph whether it's the picked value or a dropdown option.
fn marker_kind_icon(kind: avcore::MarkerKind) -> &'static str {
    match kind {
        // "🔹"/"☐"/"📖" are all confirmed-tofu classes (emoji-presentation / Geometric Shapes)
        // per this session's other fixes — plain ASCII instead. "⭐" swapped for "*": both call
        // sites render this as a plain &str with no font-family override, so the vendored
        // Lucide icon font (which STAR_STR needs) never gets applied here — using it would
        // just trade one tofu glyph for another private-use-area one.
        avcore::MarkerKind::Standard => "M",
        avcore::MarkerKind::ToDo => "[]",
        avcore::MarkerKind::Chapter => "C",
        avcore::MarkerKind::Highlight => "*",
    }
}

#[cfg(test)]
#[path = "markers/markers_test.rs"]
mod tests;
