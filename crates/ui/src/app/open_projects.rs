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

//! In-memory project collection and its optional active selection.

use avcore::Project;

/// Projects currently open in the UI and the one selected for editor operations.
///
/// The collection preserves insertion order for the Home screen. `active_index` is optional so
/// a fresh Home session is represented faithfully instead of relying on an invalid sentinel
/// index. All collection mutations are kept here so removing a project cannot leave selection
/// outside the vector's bounds.
#[derive(Debug, Default)]
pub(crate) struct OpenProjects {
    projects: Vec<Project>,
    active_index: Option<usize>,
}

impl OpenProjects {
    /// Creates an ordered collection and selects its first project, if one was supplied.
    pub(crate) fn new(projects: Vec<Project>) -> Self {
        let active_index = (!projects.is_empty()).then_some(0);
        Self {
            projects,
            active_index,
        }
    }

    /// Returns whether no project is currently open.
    pub(crate) fn is_empty(&self) -> bool {
        self.projects.is_empty()
    }

    /// Returns the number of projects open in this UI session.
    pub(crate) fn len(&self) -> usize {
        self.projects.len()
    }

    /// Iterates over open projects in their Home-screen display order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Project> {
        self.projects.iter()
    }

    /// Returns the project at a Home-screen display index, if it exists.
    pub(crate) fn get(&self, index: usize) -> Option<&Project> {
        self.projects.get(index)
    }

    /// Returns a mutable project at a Home-screen display index, if it exists.
    pub(crate) fn get_mut(&mut self, index: usize) -> Option<&mut Project> {
        self.projects.get_mut(index)
    }

    /// Finds a mutable project by its persisted project id for a correlated background event.
    pub(crate) fn find_mut_by_id(&mut self, project_id: u64) -> Option<&mut Project> {
        self.projects
            .iter_mut()
            .find(|project| project.id == project_id)
    }

    /// Returns the selected project, or `None` when the session has no active project.
    pub(crate) fn active(&self) -> Option<&Project> {
        self.active_index.and_then(|index| self.projects.get(index))
    }

    /// Returns the selected project mutably, or `None` when the session has no active project.
    pub(crate) fn active_mut(&mut self) -> Option<&mut Project> {
        self.active_index
            .and_then(|index| self.projects.get_mut(index))
    }

    /// Returns the selected Home-screen display index, if a project is selected.
    pub(crate) fn active_index(&self) -> Option<usize> {
        self.active_index
    }

    /// Selects an existing project by display index and reports whether the selection changed.
    ///
    /// An out-of-range index leaves the current selection untouched and returns `false`.
    pub(crate) fn activate(&mut self, index: usize) -> bool {
        if index >= self.projects.len() {
            return false;
        }
        self.active_index = Some(index);
        true
    }

    /// Appends `project`, selects it, and returns its display index.
    pub(crate) fn push_and_activate(&mut self, project: Project) -> usize {
        self.projects.push(project);
        let index = self.projects.len() - 1;
        self.active_index = Some(index);
        index
    }

    /// Removes the project at `index` and keeps selection valid.
    ///
    /// Removing the active project selects the project that takes its slot, or the preceding
    /// project when it was last. Removing the final project clears selection. An invalid index
    /// changes nothing and returns `None`.
    pub(crate) fn remove(&mut self, index: usize) -> Option<Project> {
        if index >= self.projects.len() {
            return None;
        }

        let removed = self.projects.remove(index);
        self.active_index = match self.active_index {
            Some(_) if self.projects.is_empty() => None,
            Some(active) if index < active => Some(active - 1),
            Some(active) if index == active => Some(active.min(self.projects.len() - 1)),
            selection => selection,
        };
        Some(removed)
    }
}
