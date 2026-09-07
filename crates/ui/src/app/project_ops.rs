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

//! Project, sequence, and active-selection operations.

use super::*;
use avcore::Project;

impl App {
    /// The project currently open in the Editor/Mídia screens.
    ///
    /// Callers must establish an active project first, usually through
    /// [`App::ensure_active_project`]. The optional state itself lives in [`OpenProjects`]; this
    /// non-optional compatibility accessor remains for editor-only operations that require one.
    pub fn active_project(&self) -> &Project {
        self.open_projects
            .active()
            .expect("active project required by this operation")
    }

    /// Guarantees `active_project()`/`active_project_mut()` resolve — the Editor and Mídia
    /// screens call this before touching either, since a fresh launch's `projects` starts
    /// empty and navigating straight there (nav rail, no "Novo projeto" first) would otherwise
    /// index out of bounds. Creates and opens an untitled project exactly like clicking "Novo
    /// projeto" would, only when none exists yet; a no-op once any project is open.
    ///
    /// `create_new_project` also forces `screen` to `Editor` (what the Home "Novo projeto"
    /// button relies on to navigate away from Home) — restored here so calling this from
    /// Mídia doesn't hijack the user back to the Editor screen mid-render.
    pub fn ensure_active_project(&mut self) {
        if self.open_projects.is_empty() {
            let screen_before = self.screen;
            self.create_new_project(Text::UntitledProject.tr(self.locale).to_string());
            self.screen = screen_before;
        }
    }

    /// Mutable access to the project currently open in the Editor/Mídia screens — for
    /// imports, edits, and anything else that changes the active project in place. Sets the
    /// autosave dirty flag so [`App::pump_autosave`] knows to write the recovery file.
    pub fn active_project_mut(&mut self) -> &mut Project {
        self.project_dirty = true;
        self.last_edit_instant = Some(Instant::now());
        self.active_project_mut_untracked()
    }

    /// Mutably accesses the active project without recording an edit for autosave.
    ///
    /// This is reserved for derived UI state such as preview playhead synchronization. Callers
    /// must establish an active project first, just like [`App::active_project`].
    pub(crate) fn active_project_mut_untracked(&mut self) -> &mut Project {
        self.open_projects
            .active_mut()
            .expect("active project required by this operation")
    }

    /// The asset backing the Editor's "Clipe selecionado" panel, if any is selected.
    pub fn selected_asset(&self) -> Option<&MediaAsset> {
        let id = self.selected_asset_id?;
        self.active_project()
            .media_library
            .iter()
            .find(|a| a.id == id)
    }

    /// The timeline clip backing the properties panel's per-block controls (gain, freeze), if
    /// `selected_clip_id` points at one on the active sequence.
    pub fn selected_clip(&self) -> Option<&avcore::timeline::ClipInstance> {
        let id = self.selected_clip_id?;
        self.active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .find(|c| c.id == id)
    }

    /// The kind of track `selected_clip_id` lives on, if any — lets the properties panel gate
    /// the freeze-frame toggle to video clips only ("Congelar" holds a video frame, audio
    /// clips have none).
    pub fn selected_clip_track_kind(&self) -> Option<avcore::timeline::TrackKind> {
        let id = self.selected_clip_id?;
        self.active_project()
            .timeline()
            .tracks
            .iter()
            .find(|t| t.clips.iter().any(|c| c.id == id))
            .map(|t| t.kind)
    }

    /// The [`AudioRole`] of the track `selected_clip_id` lives on, if any — lets the properties
    /// panel suggest enabling CF-03 voice cleanup when a clip sits on a `Mic`-role track but
    /// wasn't defaulted to it at creation time (e.g. the track's role was reassigned after the
    /// clip already existed — `App::add_asset_to_timeline`'s own "Mic by default" only applies
    /// at creation, never retroactively).
    pub fn selected_clip_track_audio_role(&self) -> Option<AudioRole> {
        let id = self.selected_clip_id?;
        self.active_project()
            .timeline()
            .tracks
            .iter()
            .find(|t| t.clips.iter().any(|c| c.id == id))
            .map(|t| t.audio_role)
    }

    /// Selects a timeline clip and its backing asset together, so the properties panel's
    /// per-block controls and the preview stay in sync with a direct timeline click.
    pub fn select_timeline_clip(&mut self, id: u64) {
        let asset_id = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|track| &track.clips)
            .find(|clip| clip.id == id)
            .map(|clip| clip.asset_id);
        self.selected_clip_id = Some(id);
        self.select_asset(asset_id);
    }

    /// Switches the active project to `index` and navigates to the Editor screen — this is
    /// what a project card click on the Início screen does.
    pub fn open_project(&mut self, index: usize) {
        if !self.open_projects.activate(index) {
            return;
        }
        self.undo_stack.clear();
        let project = self.active_project();
        info!(
            project_id = project.id,
            name = %project.name,
            path = ?project.file_path,
            "project opened"
        );
        let asset_id = project.media_library.first().map(|a| a.id);
        let multicam_group_id = project.timeline().multicam_groups.first().map(|g| g.id);
        self.select_asset(asset_id);
        self.active_multicam_group_id = multicam_group_id;
        self.media_filter = MediaLibraryFilter::All;
        self.load_panel_layout_for_active_project();
        self.screen = Screen::Editor;
    }

    /// Loads the Editor's live panel-layout fields (`lib_panel_width`/`props_panel_width`/
    /// `timeline_height`) for whichever project is now active — called from [`Self::open_project`]
    /// so switching projects doesn't leave the previous project's dragged layout on screen.
    /// Under [`LayoutScope::PerUser`] this is a no-op (the per-user `prefs` values already
    /// apply to every project uniformly). Under [`LayoutScope::PerProject`], falls back to the
    /// current live values (effectively the per-user defaults from `App::new`, unchanged) when
    /// the newly active project has never saved its own layout yet, rather than resetting to
    /// some arbitrary size.
    pub(crate) fn load_panel_layout_for_active_project(&mut self) {
        if self.prefs.layout_scope != LayoutScope::PerProject {
            return;
        }
        if let Some(layout) = self.active_project().panel_layout {
            self.lib_panel_width = layout.lib_panel_width;
            self.props_panel_width = layout.props_panel_width;
            self.timeline_height = layout.timeline_height;
        }
    }

    /// Captures the Editor's live panel-layout fields back into the active project's own
    /// `avcore::Project::panel_layout`, under [`LayoutScope::PerProject`] — called right before
    /// serializing a project (explicit save and autosave both), mirroring how the per-user
    /// scope's `App::save_prefs` already captures the same three fields into `prefs` at its own
    /// save points. A no-op under [`LayoutScope::PerUser`] (nothing to capture — the per-user
    /// scope never touches `Project::panel_layout` at all) or once nothing has changed.
    pub(crate) fn sync_panel_layout_into_active_project(&mut self) {
        if self.prefs.layout_scope != LayoutScope::PerProject {
            return;
        }
        let layout = avcore::PanelLayout {
            lib_panel_width: self.lib_panel_width,
            props_panel_width: self.props_panel_width,
            timeline_height: self.timeline_height,
        };
        if self.active_project().panel_layout != Some(layout) {
            self.active_project_mut().panel_layout = Some(layout);
        }
    }

    /// Selects `id` as the Editor's active clip and clears out whatever pipeline/texture/
    /// playback state belonged to the previous one — what clicking an asset in the media
    /// library panel does, and what [`App::open_project`] uses to select the newly-opened
    /// project's first asset. `None` clears the selection (empty media library). Does *not*
    /// itself open a pipeline for the new selection — [`App::ensure_preview_loaded`] does
    /// that lazily, the next time the Editor's preview panel actually draws, so switching
    /// projects or asking for a project at startup never pays GStreamer's open cost for an
    /// asset nobody's looking at yet.
    pub fn select_asset(&mut self, id: Option<u64>) {
        self.selected_asset_id = id;
    }

    /// Flips `asset_id`'s [`avcore::media::MediaAsset::favorited`] flag — what clicking the
    /// star button on a media library row/tile does. A no-op if `asset_id` isn't in the active
    /// project's media library.
    pub fn toggle_asset_favorite(&mut self, asset_id: u64) {
        if let Some(asset) = self
            .active_project_mut()
            .media_library
            .iter_mut()
            .find(|a| a.id == asset_id)
        {
            asset.favorited = !asset.favorited;
        }
    }

    /// Appends `project` to the project list and opens it — used for both "Novo projeto"
    /// (an empty project) and "Abrir projeto" (one just loaded from disk).
    pub fn add_and_open_project(&mut self, project: Project) {
        let idx = self.open_projects.push_and_activate(project);
        // Track the file path in recent projects before open_project() runs.
        if let Some(path) = self
            .open_projects
            .get(idx)
            .and_then(|project| project.file_path.clone())
        {
            let path_str = path.display().to_string();
            self.prefs.recent_project_paths.retain(|p| p != &path_str);
            self.prefs.recent_project_paths.insert(0, path_str);
            self.prefs.recent_project_paths.truncate(10);
            self.save_prefs();
        }
        self.open_project(idx);
    }

    /// Removes the project at `index` from the in-memory list and from `prefs.recent_project_paths`,
    /// then persists prefs. Adjusts `active_project` so it stays in bounds. Does NOT navigate —
    /// the caller (Home screen) decides whether to switch screens.
    pub fn remove_project(&mut self, index: usize) {
        let Some(project) = self.open_projects.get(index) else {
            return;
        };
        // Remove from recents before dropping the project.
        if let Some(path) = &project.file_path {
            let path_str = path.display().to_string();
            self.prefs.recent_project_paths.retain(|p| p != &path_str);
        }
        self.open_projects.remove(index);
        self.save_prefs();
    }

    /// Builds an empty project with a fresh id and opens it — what "Novo projeto" does.
    pub fn create_new_project(&mut self, name: String) {
        let id = self.open_projects.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        let target_lufs = LUFS_PROFILES
            .get(self.prefs.lufs_profile)
            .map(|(_, target_lufs)| *target_lufs)
            .unwrap_or_else(|| avcore::SequenceExportSettings::default().target_lufs);
        self.add_and_open_project(Project {
            id,
            name,
            last_edited: avcore::Recency::HoursAgo(0),
            summary: String::new(),
            media_library: Vec::new(),
            sequences: vec![avcore::Sequence {
                id: 1,
                name: Text::DefaultSequenceName.tr(self.locale).to_string(),
                timeline: avcore::Timeline {
                    tracks: Vec::new(),
                    playhead_secs: 0.0,
                    markers: Vec::new(),
                    multicam_groups: Vec::new(),
                },
                export_settings: avcore::SequenceExportSettings {
                    aspect_ratio: avcore::ExportAspectRatio::Original,
                    target_lufs,
                },
            }],
            active_sequence: 0,
            file_path: None,
            panel_layout: None,
            smart_bins: Vec::new(),
            recent_asset_ids: Vec::new(),
        });
    }

    /// Appends a new, empty sequence tab to the active project and switches to it — what the
    /// Editor's tab bar "+" button does (per `request.md`'s Fase 3 "abas de projeto" spec).
    /// Named positionally (`Text::DefaultSequenceName` is reserved for a project's first,
    /// non-numbered tab).
    pub fn add_sequence(&mut self) {
        let locale = self.locale;
        let project = self.active_project_mut();
        let n = project.sequences.len() + 1;
        project.new_sequence(crate::i18n::sequence_name(locale, n));
        self.reset_sequence_context();
    }

    /// Switches the active project's tab to `index` — what clicking a tab in the Editor's tab
    /// bar does. A no-op if `index` is out of range.
    pub fn select_sequence(&mut self, index: usize) {
        if index >= self.active_project().sequences.len()
            || index == self.active_project().active_sequence
        {
            return;
        }
        self.active_project_mut().active_sequence = index;
        self.reset_sequence_context();
    }

    /// Duplicates a sequence, including its complete timeline and export defaults, immediately
    /// after the source tab and switches to the duplicate. The localized copy name stays in
    /// the UI layer while [`Project::duplicate_sequence`] owns the data invariants.
    pub fn duplicate_sequence(&mut self, index: usize) {
        let Some(source_name) = self
            .active_project()
            .sequences
            .get(index)
            .map(|sequence| sequence.name.clone())
        else {
            return;
        };
        let name = crate::i18n::sequence_copy_name(self.locale, &source_name);
        if self
            .active_project_mut()
            .duplicate_sequence(index, name)
            .is_some()
        {
            self.reset_sequence_context();
        }
    }

    /// Deletes a sequence by stable id. The core model refuses to remove the project's last
    /// tab; a successful deletion switches to the nearest surviving tab and clears state tied
    /// to the removed/previous sequence.
    pub fn delete_sequence(&mut self, sequence_id: u64) {
        let active_id_before =
            self.active_project().sequences[self.active_project().active_sequence].id;
        let Some(index) = self
            .active_project()
            .sequences
            .iter()
            .position(|sequence| sequence.id == sequence_id)
        else {
            return;
        };
        if self.active_project().sequences.len() <= 1 {
            return;
        }
        if self.active_project_mut().remove_sequence(index) {
            let active_id_after =
                self.active_project().sequences[self.active_project().active_sequence].id;
            if active_id_after != active_id_before {
                self.reset_sequence_context();
            }
        }
    }

    /// Reorders tabs by index while preserving the active sequence's identity. Unlike a tab
    /// switch this deliberately keeps selections/preview alive because their owning sequence
    /// did not change, only its visual position did.
    pub fn move_sequence(&mut self, from_index: usize, target_index: usize) {
        let sequence_count = self.active_project().sequences.len();
        if from_index >= sequence_count
            || target_index >= sequence_count
            || from_index == target_index
        {
            return;
        }
        self.active_project_mut()
            .move_sequence(from_index, target_index);
    }

    /// Clears state whose ids/frames are scoped to the active sequence. Clip ids restart from
    /// one in each tab, so retaining any of these across a switch could target an unrelated
    /// clip with the same numeric id. Reordering does not call this because identity is stable.
    pub(super) fn reset_sequence_context(&mut self) {
        self.selected_clip_id = None;
        self.selected_text_clip_id = None;
        self.text_color_edit = None;
        self.selected_shape_clip_id = None;
        self.multi_selected_clip_ids.clear();
        self.drawing_shape_points = None;
        self.motion_track_region.picking_motion_track_region = false;
        self.preview_state.preview_playing = false;
        self.preview_state.preview_frozen_since = None;
        self.undo_stack.clear();
        self.invalidate_preview_rendering();
    }
}
