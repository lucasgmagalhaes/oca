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
