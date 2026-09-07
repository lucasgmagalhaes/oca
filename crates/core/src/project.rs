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

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::export::ExportAspectRatio;
use crate::media::MediaAsset;
use crate::smart_bins::SmartBin;
use crate::timeline::Timeline;

/// How long ago a project was last edited. Locale-neutral by design — the UI layer is
/// responsible for turning this into a translated label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Recency {
    HoursAgo(u32),
    Yesterday,
    DaysAgo(u32),
}

/// One editable cut within a project — its own timeline and export defaults, independently
/// zoomable/playable and shown as its own tab in the Editor (per `request.md`'s Fase 3 "abas de
/// projeto" spec, e.g. one sequence for trimmed highlights, another for the full unedited
/// recording). Every project has at least one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sequence {
    pub id: u64,
    pub name: String,
    pub timeline: Timeline,
    /// Export defaults owned by this tab. A queued [`crate::ExportJob`] still snapshots the
    /// resolved values, so later edits here never change a job already in flight.
    #[serde(default)]
    pub export_settings: SequenceExportSettings,
}

/// Persisted export choices for one [`Sequence`]. Output path and GPU preference remain
/// app/job-level concerns; aspect ratio and normalization target describe the sequence's
/// intended presentation and therefore follow the tab across saves and project switches.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SequenceExportSettings {
    #[serde(default)]
    pub aspect_ratio: ExportAspectRatio,
    #[serde(default = "default_target_lufs")]
    pub target_lufs: f32,
}

impl Default for SequenceExportSettings {
    fn default() -> Self {
        Self {
            aspect_ratio: ExportAspectRatio::Original,
            target_lufs: default_target_lufs(),
        }
    }
}

fn default_target_lufs() -> f32 {
    -14.0
}

/// A single edit project: its imported media, its sequences, and display metadata for the
/// Início screen's project list. This is the root of everything a user works on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,
    pub name: String,
    pub last_edited: Recency,
    /// Short user-facing description shown on the project card (e.g. "Cortes dos boss
    /// fights, um vídeo por chefe.") — free text, not a translatable UI label.
    pub summary: String,
    pub media_library: Vec<MediaAsset>,
    /// Every sequence (tab) in this project — always non-empty; [`Project::new_sequence`]
    /// and every construction site guarantee at least one.
    pub sequences: Vec<Sequence>,
    /// Index into `sequences` of the tab currently shown in the Editor.
    pub active_sequence: usize,
    /// Where this project was last saved to/loaded from, if anywhere. Not serialized — the
    /// file *contains* this data, it doesn't need to know its own path, and a copied/renamed
    /// project file shouldn't carry a stale path forward.
    #[serde(skip)]
    pub file_path: Option<PathBuf>,
    /// The Editor's panel widths/timeline height, when `ui`'s `LayoutScope::PerProject` is
    /// active — `request.md`'s Fase 3 "layout salvo por projeto ou por usuário" spec, the
    /// per-project half (per-user persistence already lives in `ui::PrefsState`). `None` for
    /// a project that's never been saved under the per-project scope (including every older
    /// saved project, via `#[serde(default)]`) — `ui::App` falls back to the per-user prefs
    /// values in that case rather than resetting to some arbitrary default.
    #[serde(default)]
    pub panel_layout: Option<PanelLayout>,
    /// Rule-based media-pool folders (P4 item 22, "Smart bins") — `#[serde(default)]` so a
    /// project saved before this field existed still loads (empty bin list).
    #[serde(default)]
    pub smart_bins: Vec<SmartBin>,
    /// Media-library asset ids most recently added to this project's timeline, most-recent
    /// first, deduplicated (re-adding an already-present id moves it back to the front rather
    /// than appearing twice) and capped at [`RECENT_ASSET_CAPACITY`] — the Media library
    /// panel's Recent filter's backing data. Deliberately tracks *timeline usage*, not import
    /// time or library-browsing: an asset just sitting in the library isn't "recent" until it's
    /// actually been edited with. `#[serde(default)]` so a project saved before this field
    /// existed loads with an empty recent list rather than failing to deserialize.
    #[serde(default)]
    pub recent_asset_ids: Vec<u64>,
}

/// Cap on [`Project::recent_asset_ids`] — same order of magnitude as `ui::PrefsState::
/// recent_project_paths`' own cap (10), a little larger since library assets are used more
/// often per session than whole projects are opened.
pub const RECENT_ASSET_CAPACITY: usize = 20;

/// See [`Project::panel_layout`]. A plain value type — `core` has no opinion on layout scope
/// itself (that's `ui::LayoutScope`), it just knows how to carry these three numbers along
/// with the rest of a saved project.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PanelLayout {
    pub lib_panel_width: f32,
    pub props_panel_width: f32,
    pub timeline_height: f32,
}

impl Project {
    /// The active tab's timeline — what the Editor screen and every clip-editing `ui::App`
    /// method actually read. Panics if `active_sequence` is out of bounds, which shouldn't
    /// happen given `sequences` is never empty and every mutation keeps the index in range —
    /// same trust-the-invariant style as `ui::App::active_project`'s own indexing.
    pub fn timeline(&self) -> &Timeline {
        &self.sequences[self.active_sequence].timeline
    }

    /// Mutable counterpart of [`Project::timeline`].
    pub fn timeline_mut(&mut self) -> &mut Timeline {
        &mut self.sequences[self.active_sequence].timeline
    }

    /// The active tab itself — for whole-sequence operations (undo/redo snapshots) that
    /// `timeline()` doesn't cover, since [`Sequence`] also carries `export_settings`.
    pub fn active_sequence(&self) -> &Sequence {
        &self.sequences[self.active_sequence]
    }

    /// Mutable counterpart of [`Project::active_sequence`].
    pub fn active_sequence_mut(&mut self) -> &mut Sequence {
        &mut self.sequences[self.active_sequence]
    }

    /// Appends a new, empty sequence named `name` and switches `active_sequence` to it —
    /// what the Editor's tab bar "+" button does. Returns the new sequence's id.
    pub fn new_sequence(&mut self, name: String) -> u64 {
        let id = self.sequences.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        let export_settings = self
            .sequences
            .get(self.active_sequence)
            .map(|sequence| sequence.export_settings)
            .unwrap_or_default();
        self.sequences.push(Sequence {
            id,
            name,
            timeline: Timeline {
                tracks: Vec::new(),
                playhead_secs: 0.0,
                markers: Vec::new(),
                multicam_groups: Vec::new(),
            },
            export_settings,
        });
        self.active_sequence = self.sequences.len() - 1;
        id
    }

    /// Clones `sequences[index]` into a new tab immediately after it, assigns a fresh sequence
    /// id, replaces the clone's name with `name`, and switches to it. Clip/track ids remain
    /// unchanged because they are scoped to their owning sequence. Returns the fresh id, or
    /// `None` when `index` is out of range.
    pub fn duplicate_sequence(&mut self, index: usize, name: String) -> Option<u64> {
        let mut duplicate = self.sequences.get(index)?.clone();
        let id = self.sequences.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        duplicate.id = id;
        duplicate.name = name;
        let insert_at = index + 1;
        self.sequences.insert(insert_at, duplicate);
        self.active_sequence = insert_at;
        Some(id)
    }

    /// Removes one sequence while preserving the invariant that every project has at least
    /// one. If the active tab is removed, the tab now occupying that position becomes active,
    /// falling back to the previous tab when the removed one was last. Returns whether a tab
    /// was actually removed.
    pub fn remove_sequence(&mut self, index: usize) -> bool {
        if self.sequences.len() <= 1 || index >= self.sequences.len() {
            return false;
        }
        self.sequences.remove(index);
        self.active_sequence = match self.active_sequence.cmp(&index) {
            std::cmp::Ordering::Less => self.active_sequence,
            std::cmp::Ordering::Equal => index.min(self.sequences.len() - 1),
            std::cmp::Ordering::Greater => self.active_sequence - 1,
        };
        true
    }

    /// Every sequence in this project (`sequence_id` itself excluded — a sequence can't nest
    /// itself, see `nested_sequence::materialize_nested_sequences`'s own cycle guard) containing
    /// at least one clip whose `ClipInstance::nested_sequence_id` points at `sequence_id` —
    /// i.e. every place `sequence_id` is used as a compound clip elsewhere in the project.
    /// [`Project::remove_sequence`] itself doesn't check this: a dangling `nested_sequence_id`
    /// left behind by a delete degrades gracefully at render time
    /// (`RenderError::MissingNestedSequence`, clip skipped, not a crash) rather than erroring —
    /// but a caller that wants to warn the user *before* deleting can call this first.
    pub fn sequences_referencing_as_compound_clip(&self, sequence_id: u64) -> Vec<&Sequence> {
        self.sequences
            .iter()
            .filter(|s| s.id != sequence_id)
            .filter(|s| {
                s.timeline.tracks.iter().any(|t| {
                    t.clips
                        .iter()
                        .any(|c| c.nested_sequence_id == Some(sequence_id))
                })
            })
            .collect()
    }

    /// Moves one sequence to `target_index`, keeping the currently active sequence active even
    /// when indices shift around it. Returns `false` for invalid indices or a no-op move.
    pub fn move_sequence(&mut self, from_index: usize, target_index: usize) -> bool {
        if from_index >= self.sequences.len()
            || target_index >= self.sequences.len()
            || from_index == target_index
        {
            return false;
        }
        let active_id = self.sequences[self.active_sequence].id;
        let sequence = self.sequences.remove(from_index);
        self.sequences.insert(target_index, sequence);
        self.active_sequence = self
            .sequences
            .iter()
            .position(|sequence| sequence.id == active_id)
            .expect("the active sequence is only reordered, never removed");
        true
    }

    /// Adds a new, unnamed-rule [`SmartBin`] (see [`SmartBin::new`]) and returns its freshly
    /// assigned id — "max + 1", the same convention every other entity id in this codebase uses.
    pub fn add_smart_bin(&mut self, name: String) -> u64 {
        let id = self.smart_bins.iter().map(|b| b.id).max().unwrap_or(0) + 1;
        self.smart_bins.push(SmartBin::new(id, name));
        id
    }

    /// Removes the smart bin with `bin_id`, if any. `true` if a bin was actually removed.
    pub fn remove_smart_bin(&mut self, bin_id: u64) -> bool {
        let before = self.smart_bins.len();
        self.smart_bins.retain(|b| b.id != bin_id);
        self.smart_bins.len() != before
    }

    /// Mutable access to the smart bin with `bin_id`, if it exists.
    pub fn smart_bin_mut(&mut self, bin_id: u64) -> Option<&mut SmartBin> {
        self.smart_bins.iter_mut().find(|b| b.id == bin_id)
    }

    /// Moves `asset_id` to the front of [`Self::recent_asset_ids`] (inserting it if not
    /// already present), then truncates to [`RECENT_ASSET_CAPACITY`] — what `ui`'s
    /// `App::add_asset_to_timeline`/`add_asset_to_timeline_at` call every time an asset lands
    /// on the timeline.
    pub fn record_recent_asset(&mut self, asset_id: u64) {
        self.recent_asset_ids.retain(|&id| id != asset_id);
        self.recent_asset_ids.insert(0, asset_id);
        self.recent_asset_ids.truncate(RECENT_ASSET_CAPACITY);
    }
}
