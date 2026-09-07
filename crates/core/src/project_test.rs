use super::*;
use crate::timeline::Timeline;

fn empty_timeline() -> Timeline {
    Timeline {
        tracks: Vec::new(),
        playhead_secs: 0.0,
        markers: Vec::new(),
        multicam_groups: Vec::new(),
    }
}

fn sequence(id: u64, name: &str) -> Sequence {
    Sequence {
        id,
        name: name.to_string(),
        timeline: empty_timeline(),
        export_settings: SequenceExportSettings::default(),
    }
}

fn project_with_sequences(names: &[&str]) -> Project {
    Project {
        id: 1,
        name: "Test".to_string(),
        last_edited: Recency::HoursAgo(1),
        summary: String::new(),
        media_library: Vec::new(),
        sequences: names
            .iter()
            .enumerate()
            .map(|(i, name)| sequence(i as u64 + 1, name))
            .collect(),
        active_sequence: 0,
        file_path: None,
        panel_layout: None,
        smart_bins: Vec::new(),
        recent_asset_ids: Vec::new(),
    }
}

#[test]
fn new_sequence_appends_and_activates_it_with_a_fresh_id() {
    let mut project = project_with_sequences(&["First"]);
    let id = project.new_sequence("Second".to_string());
    assert_eq!(id, 2);
    assert_eq!(project.sequences.len(), 2);
    assert_eq!(project.active_sequence, 1);
    assert_eq!(project.active_sequence().name, "Second");
}

#[test]
fn new_sequence_inherits_the_previously_active_sequences_export_settings() {
    let mut project = project_with_sequences(&["First"]);
    project.active_sequence_mut().export_settings.target_lufs = -9.0;
    project.new_sequence("Second".to_string());
    assert_eq!(project.active_sequence().export_settings.target_lufs, -9.0);
}

#[test]
fn duplicate_sequence_inserts_right_after_the_source_with_a_fresh_id_and_name() {
    let mut project = project_with_sequences(&["A", "B"]);
    let id = project.duplicate_sequence(0, "A copy".to_string());
    assert_eq!(id, Some(3));
    assert_eq!(project.sequences.len(), 3);
    assert_eq!(project.sequences[1].name, "A copy");
    assert_eq!(project.sequences[1].id, 3);
    // The original at index 0 and the pushed-along "B" (now index 2) are untouched.
    assert_eq!(project.sequences[0].name, "A");
    assert_eq!(project.sequences[2].name, "B");
    assert_eq!(project.active_sequence, 1);
}

#[test]
fn duplicate_sequence_returns_none_for_an_out_of_range_index() {
    let mut project = project_with_sequences(&["A"]);
    assert_eq!(project.duplicate_sequence(5, "X".to_string()), None);
    assert_eq!(project.sequences.len(), 1);
}

#[test]
fn remove_sequence_refuses_to_drop_the_last_sequence() {
    let mut project = project_with_sequences(&["Only"]);
    assert!(!project.remove_sequence(0));
    assert_eq!(project.sequences.len(), 1);
}

#[test]
fn remove_sequence_refuses_an_out_of_range_index() {
    let mut project = project_with_sequences(&["A", "B"]);
    assert!(!project.remove_sequence(5));
    assert_eq!(project.sequences.len(), 2);
}

#[test]
fn remove_sequence_shifts_the_active_index_left_when_a_track_before_it_is_removed() {
    let mut project = project_with_sequences(&["A", "B", "C"]);
    project.active_sequence = 2; // "C"
    assert!(project.remove_sequence(0)); // removes "A"
    assert_eq!(project.active_sequence, 1);
    assert_eq!(project.active_sequence().name, "C");
}

#[test]
fn remove_sequence_leaves_the_active_index_alone_when_a_track_after_it_is_removed() {
    let mut project = project_with_sequences(&["A", "B", "C"]);
    project.active_sequence = 0; // "A"
    assert!(project.remove_sequence(2)); // removes "C"
    assert_eq!(project.active_sequence, 0);
    assert_eq!(project.active_sequence().name, "A");
}

#[test]
fn remove_sequence_falls_back_to_the_previous_tab_when_the_active_one_was_last() {
    let mut project = project_with_sequences(&["A", "B", "C"]);
    project.active_sequence = 2; // "C", the last tab
    assert!(project.remove_sequence(2));
    assert_eq!(project.active_sequence, 1);
    assert_eq!(project.active_sequence().name, "B");
}

#[test]
fn remove_sequence_keeps_the_active_index_in_range_when_the_active_tab_itself_is_removed_from_the_middle(
) {
    let mut project = project_with_sequences(&["A", "B", "C"]);
    project.active_sequence = 1; // "B"
    assert!(project.remove_sequence(1));
    assert_eq!(project.active_sequence, 1);
    assert_eq!(project.active_sequence().name, "C");
}

#[test]
fn move_sequence_keeps_the_active_sequence_active_across_the_reorder() {
    let mut project = project_with_sequences(&["A", "B", "C"]);
    project.active_sequence = 0; // "A"
    assert!(project.move_sequence(0, 2));
    assert_eq!(
        project
            .sequences
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        vec!["B", "C", "A"]
    );
    assert_eq!(project.active_sequence, 2);
    assert_eq!(project.active_sequence().name, "A");
}

#[test]
fn move_sequence_rejects_a_no_op_or_out_of_range_move() {
    let mut project = project_with_sequences(&["A", "B"]);
    assert!(!project.move_sequence(0, 0));
    assert!(!project.move_sequence(0, 5));
    assert!(!project.move_sequence(5, 0));
}

#[test]
fn add_and_remove_smart_bin_assigns_max_plus_one_ids() {
    let mut project = project_with_sequences(&["A"]);
    let first = project.add_smart_bin("Bosses".to_string());
    let second = project.add_smart_bin("Intros".to_string());
    assert_eq!(first, 1);
    assert_eq!(second, 2);
    assert_eq!(project.smart_bins.len(), 2);

    assert!(project.remove_smart_bin(first));
    assert_eq!(project.smart_bins.len(), 1);
    assert!(!project.remove_smart_bin(first));
}

#[test]
fn smart_bin_mut_finds_the_right_bin() {
    let mut project = project_with_sequences(&["A"]);
    let id = project.add_smart_bin("Bosses".to_string());
    project.smart_bin_mut(id).unwrap().name_contains = "boss".to_string();
    assert_eq!(
        project
            .smart_bins
            .iter()
            .find(|b| b.id == id)
            .unwrap()
            .name_contains,
        "boss"
    );
    assert!(project.smart_bin_mut(999).is_none());
}

#[test]
fn record_recent_asset_moves_an_existing_id_to_the_front_instead_of_duplicating_it() {
    let mut project = project_with_sequences(&["A"]);
    project.record_recent_asset(1);
    project.record_recent_asset(2);
    project.record_recent_asset(3);
    project.record_recent_asset(1);
    assert_eq!(project.recent_asset_ids, vec![1, 3, 2]);
}

#[test]
fn record_recent_asset_truncates_at_the_capacity() {
    let mut project = project_with_sequences(&["A"]);
    for id in 0..(RECENT_ASSET_CAPACITY as u64 + 5) {
        project.record_recent_asset(id);
    }
    assert_eq!(project.recent_asset_ids.len(), RECENT_ASSET_CAPACITY);
    // Most recently recorded id is at the front.
    assert_eq!(
        project.recent_asset_ids[0],
        RECENT_ASSET_CAPACITY as u64 + 4
    );
}

#[test]
fn sequences_referencing_as_compound_clip_excludes_the_sequence_itself() {
    let project = project_with_sequences(&["A", "B"]);
    // No clips at all yet, so nothing references sequence 1 as a compound clip — including
    // sequence 1 not referencing itself.
    assert!(project.sequences_referencing_as_compound_clip(1).is_empty());
}
