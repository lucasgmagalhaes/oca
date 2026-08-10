use std::fs;

use avcore::persistence::{
    from_json, load_project_from_file, save_project_to_file, to_json, PersistError,
};
use avcore::sample::sample_projects;

#[test]
fn round_trips_a_project_through_json() {
    let original = sample_projects().into_iter().next().unwrap();
    let json = to_json(&original).unwrap();
    let restored = from_json(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn round_trips_a_project_with_an_empty_timeline_and_library() {
    let original = sample_projects().into_iter().nth(2).unwrap();
    assert!(original.media_library.is_empty());
    assert!(original.timeline.tracks.is_empty());
    let json = to_json(&original).unwrap();
    let restored = from_json(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn from_json_rejects_malformed_input() {
    assert!(from_json("not json").is_err());
}

#[test]
fn file_path_is_not_part_of_the_serialized_json() {
    let mut project = sample_projects().into_iter().next().unwrap();
    project.file_path = Some("/tmp/whatever.json".into());
    let json = to_json(&project).unwrap();
    assert!(!json.contains("whatever.json"));
    assert!(!json.contains("file_path"));
}

#[test]
fn save_then_load_round_trips_through_a_real_file() {
    let original = sample_projects().into_iter().next().unwrap();
    let path = std::env::temp_dir().join(format!("oca_persist_test_{}.json", original.id));

    save_project_to_file(&original, &path).unwrap();
    let loaded = load_project_from_file(&path).unwrap();
    let _ = fs::remove_file(&path);

    assert_eq!(original, loaded);
}

#[test]
fn load_project_from_file_errors_on_a_missing_file() {
    let path = std::env::temp_dir().join("oca_persist_test_does_not_exist.json");
    assert!(matches!(
        load_project_from_file(&path),
        Err(PersistError::Io(_))
    ));
}
