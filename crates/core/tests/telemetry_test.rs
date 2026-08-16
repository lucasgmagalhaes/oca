use std::fs;

use avcore::{record_event, TelemetryEvent};

#[test]
fn record_event_appends_one_json_line_per_call() {
    let dir = std::env::temp_dir().join("oca_telemetry_test_append");
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("telemetry.jsonl");

    record_event(&path, &TelemetryEvent::ImportCompleted { duration_ms: 120 }).unwrap();
    record_event(
        &path,
        &TelemetryEvent::ExportCompleted {
            duration_ms: 4500,
            output_duration_secs: 30.0,
            success: true,
        },
    )
    .unwrap();

    let contents = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = contents.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"event\":\"import_completed\""));
    assert!(lines[0].contains("\"duration_ms\":120"));
    assert!(lines[1].contains("\"event\":\"export_completed\""));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn record_event_includes_a_timestamp() {
    let dir = std::env::temp_dir().join("oca_telemetry_test_timestamp");
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("telemetry.jsonl");

    record_event(
        &path,
        &TelemetryEvent::Error {
            context: "import".to_string(),
            message: "boom".to_string(),
        },
    )
    .unwrap();

    let contents = fs::read_to_string(&path).unwrap();
    assert!(contents.contains("\"timestamp\":"));
    assert!(contents.contains("\"context\":\"import\""));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn record_event_creates_missing_parent_directories() {
    let dir = std::env::temp_dir().join("oca_telemetry_test_missing_parent");
    let _ = fs::remove_dir_all(&dir);
    let path = dir.join("nested").join("telemetry.jsonl");
    assert!(!dir.exists());

    record_event(
        &path,
        &TelemetryEvent::PreviewFrameTime {
            frame_time_ms: 16.7,
        },
    )
    .unwrap();

    assert!(path.exists());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn record_event_rotates_an_oversized_file_to_a_backup() {
    let dir = std::env::temp_dir().join("oca_telemetry_test_rotation");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("telemetry.jsonl");
    let backup_path = dir.join("telemetry.jsonl.1");

    // One byte past the private ROTATE_AT_BYTES threshold (10 MiB) - rotation itself never
    // parses the file's contents, so filler bytes are fine.
    let oversized = vec![b'x'; 10 * 1024 * 1024 + 1];
    fs::write(&path, &oversized).unwrap();

    record_event(&path, &TelemetryEvent::ImportCompleted { duration_ms: 1 }).unwrap();

    assert!(backup_path.exists());
    assert_eq!(
        fs::metadata(&backup_path).unwrap().len(),
        oversized.len() as u64
    );
    let new_contents = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = new_contents.lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("\"event\":\"import_completed\""));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn record_event_does_not_rotate_a_file_under_the_threshold() {
    let dir = std::env::temp_dir().join("oca_telemetry_test_no_rotation");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("telemetry.jsonl");
    let backup_path = dir.join("telemetry.jsonl.1");
    fs::write(&path, b"pre-existing line\n").unwrap();

    record_event(&path, &TelemetryEvent::ImportCompleted { duration_ms: 1 }).unwrap();

    assert!(!backup_path.exists());
    let contents = fs::read_to_string(&path).unwrap();
    assert_eq!(contents.lines().count(), 2);
    assert!(contents.starts_with("pre-existing line"));

    let _ = fs::remove_dir_all(&dir);
}
