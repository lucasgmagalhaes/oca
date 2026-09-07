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

//! Invariants for the UI's open-project collection.

use super::support::test_project;
use super::*;

#[test]
fn empty_collection_has_no_active_project() {
    let projects = OpenProjects::new(Vec::new());

    assert!(projects.is_empty());
    assert_eq!(projects.len(), 0);
    assert_eq!(projects.active_index(), None);
    assert!(projects.active().is_none());
}

#[test]
fn collection_selects_the_first_initial_project() {
    let projects = OpenProjects::new(vec![
        test_project(10, Vec::new()),
        test_project(20, Vec::new()),
    ]);

    assert_eq!(projects.active_index(), Some(0));
    assert_eq!(projects.active().map(|project| project.id), Some(10));
    assert_eq!(projects.get(1).map(|project| project.id), Some(20));
}

#[test]
fn pushing_a_project_activates_it_and_preserves_display_order() {
    let mut projects = OpenProjects::default();

    assert_eq!(projects.push_and_activate(test_project(10, Vec::new())), 0);
    assert_eq!(projects.push_and_activate(test_project(20, Vec::new())), 1);
    projects.active_mut().unwrap().name = "Current".to_string();

    assert_eq!(projects.active_index(), Some(1));
    assert_eq!(
        projects.active().map(|project| project.name.as_str()),
        Some("Current")
    );
    assert_eq!(
        projects
            .iter()
            .map(|project| project.id)
            .collect::<Vec<_>>(),
        vec![10, 20]
    );
}

#[test]
fn invalid_activation_preserves_the_existing_selection() {
    let mut projects = OpenProjects::new(vec![test_project(10, Vec::new())]);

    assert!(!projects.activate(1));
    assert_eq!(projects.active_index(), Some(0));
    assert_eq!(projects.active().map(|project| project.id), Some(10));
    assert!(projects.remove(2).is_none());
}

#[test]
fn removing_before_the_active_project_rebases_its_index() {
    let mut projects = OpenProjects::new(vec![
        test_project(10, Vec::new()),
        test_project(20, Vec::new()),
        test_project(30, Vec::new()),
    ]);
    assert!(projects.activate(2));

    let removed = projects.remove(0);

    assert_eq!(removed.map(|project| project.id), Some(10));
    assert_eq!(projects.active_index(), Some(1));
    assert_eq!(projects.active().map(|project| project.id), Some(30));
}

#[test]
fn removing_the_active_project_selects_its_successor_or_predecessor() {
    let mut projects = OpenProjects::new(vec![
        test_project(10, Vec::new()),
        test_project(20, Vec::new()),
        test_project(30, Vec::new()),
    ]);
    assert!(projects.activate(1));

    projects.remove(1);
    assert_eq!(projects.active().map(|project| project.id), Some(30));

    projects.remove(1);
    assert_eq!(projects.active().map(|project| project.id), Some(10));

    projects.remove(0);
    assert_eq!(projects.active_index(), None);
    assert!(projects.active().is_none());
}

#[test]
fn mutable_lookup_updates_only_the_matched_project() {
    let mut projects = OpenProjects::new(vec![
        test_project(10, Vec::new()),
        test_project(20, Vec::new()),
    ]);

    projects.get_mut(0).unwrap().name = "Renamed".to_string();
    projects.find_mut_by_id(20).unwrap().summary = "Updated".to_string();

    assert_eq!(
        projects.get(0).map(|project| project.name.as_str()),
        Some("Renamed")
    );
    assert_eq!(
        projects.get(1).map(|project| project.summary.as_str()),
        Some("Updated")
    );
    assert!(projects.find_mut_by_id(99).is_none());
}
