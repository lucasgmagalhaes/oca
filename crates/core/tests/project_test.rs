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

use avcore::project::{Project, Sequence};
use avcore::timeline::Timeline;
use avcore::{ExportAspectRatio, MediaAsset, Recency};

fn test_project() -> Project {
    Project {
        id: 1,
        name: "Test".to_string(),
        last_edited: Recency::HoursAgo(0),
        summary: String::new(),
        media_library: Vec::<MediaAsset>::new(),
        sequences: vec![Sequence {
            id: 1,
            name: "Main".to_string(),
            timeline: Timeline {
                tracks: Vec::new(),
                playhead_secs: 5.0,
                markers: Vec::new(),
                multicam_groups: Vec::new(),
            },
            export_settings: Default::default(),
        }],
        active_sequence: 0,
        file_path: None,
        panel_layout: None,
        smart_bins: Vec::new(),
    }
}

#[test]
fn timeline_reads_the_active_sequence() {
    let project = test_project();
    assert_eq!(project.timeline().playhead_secs, 5.0);
}

#[test]
fn timeline_mut_writes_the_active_sequence() {
    let mut project = test_project();
    project.timeline_mut().playhead_secs = 42.0;
    assert_eq!(project.sequences[0].timeline.playhead_secs, 42.0);
}

#[test]
fn new_sequence_appends_and_switches_to_it() {
    let mut project = test_project();
    let id = project.new_sequence("Second".to_string());

    assert_eq!(project.sequences.len(), 2);
    assert_eq!(id, 2);
    assert_eq!(project.sequences[1].id, 2);
    assert_eq!(project.sequences[1].name, "Second");
    assert_eq!(project.active_sequence, 1);
    assert!(project.timeline().tracks.is_empty());
}

#[test]
fn new_sequence_ids_keep_increasing_after_multiple_calls() {
    let mut project = test_project();
    project.new_sequence("A".to_string());
    let third_id = project.new_sequence("B".to_string());

    assert_eq!(third_id, 3);
}

#[test]
fn new_sequence_inherits_the_active_sequences_export_settings() {
    let mut project = test_project();
    project.sequences[0].export_settings.aspect_ratio = ExportAspectRatio::Portrait;
    project.sequences[0].export_settings.target_lufs = -23.0;

    project.new_sequence("Short variant".to_string());

    assert_eq!(
        project.sequences[1].export_settings.aspect_ratio,
        ExportAspectRatio::Portrait
    );
    assert_eq!(project.sequences[1].export_settings.target_lufs, -23.0);
}

#[test]
fn duplicate_sequence_copies_the_complete_tab_with_a_fresh_id_and_name() {
    let mut project = test_project();
    project.sequences[0].timeline.playhead_secs = 12.5;
    project.sequences[0].export_settings.aspect_ratio = ExportAspectRatio::Portrait;

    let id = project
        .duplicate_sequence(0, "Main copy".to_string())
        .unwrap();

    assert_eq!(id, 2);
    assert_eq!(project.sequences.len(), 2);
    assert_eq!(project.active_sequence, 1);
    assert_eq!(project.sequences[1].id, 2);
    assert_eq!(project.sequences[1].name, "Main copy");
    assert_eq!(project.sequences[1].timeline.playhead_secs, 12.5);
    assert_eq!(
        project.sequences[1].export_settings.aspect_ratio,
        ExportAspectRatio::Portrait
    );
}

#[test]
fn duplicate_sequence_is_a_no_op_for_an_out_of_range_index() {
    let mut project = test_project();

    assert_eq!(project.duplicate_sequence(4, "Copy".to_string()), None);
    assert_eq!(project.sequences.len(), 1);
    assert_eq!(project.active_sequence, 0);
}

#[test]
fn remove_sequence_never_removes_the_projects_last_tab() {
    let mut project = test_project();

    assert!(!project.remove_sequence(0));
    assert_eq!(project.sequences.len(), 1);
    assert_eq!(project.active_sequence, 0);
}

#[test]
fn removing_the_active_last_sequence_selects_its_previous_neighbor() {
    let mut project = test_project();
    project.new_sequence("Second".to_string());
    project.new_sequence("Third".to_string());

    assert!(project.remove_sequence(2));
    assert_eq!(project.active_sequence, 1);
    assert_eq!(project.sequences[1].name, "Second");
}

#[test]
fn removing_a_sequence_before_the_active_one_preserves_the_active_identity() {
    let mut project = test_project();
    project.new_sequence("Second".to_string());
    project.new_sequence("Third".to_string());
    let active_id = project.sequences[2].id;

    assert!(project.remove_sequence(0));
    assert_eq!(project.active_sequence, 1);
    assert_eq!(project.sequences[project.active_sequence].id, active_id);
}

#[test]
fn move_sequence_reorders_tabs_without_changing_the_active_identity() {
    let mut project = test_project();
    project.new_sequence("Second".to_string());
    project.new_sequence("Third".to_string());
    project.active_sequence = 1;
    let active_id = project.sequences[1].id;

    assert!(project.move_sequence(0, 2));

    let names: Vec<&str> = project
        .sequences
        .iter()
        .map(|sequence| sequence.name.as_str())
        .collect();
    assert_eq!(names, vec!["Second", "Third", "Main"]);
    assert_eq!(project.sequences[project.active_sequence].id, active_id);
}

#[test]
fn move_sequence_rejects_invalid_or_unchanged_positions() {
    let mut project = test_project();
    project.new_sequence("Second".to_string());

    assert!(!project.move_sequence(0, 0));
    assert!(!project.move_sequence(0, 5));
    assert!(!project.move_sequence(5, 0));
    assert_eq!(project.sequences[0].name, "Main");
    assert_eq!(project.sequences[1].name, "Second");
}
