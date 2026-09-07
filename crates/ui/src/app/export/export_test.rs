use super::*;

fn test_canvas() -> avcore::Canvas {
    avcore::Canvas {
        width: 1920,
        height: 1080,
        fps_num: 30,
        fps_den: 1,
        bit_rate_bps: 8_000_000,
    }
}

fn test_job(id: u64, status: ExportJobStatus) -> ExportJob {
    ExportJob {
        id,
        title: format!("Job {id}"),
        segments: Vec::new(),
        text_segments: vec![],
        shape_segments: vec![],
        privacy_blur_segments: vec![],
        track_segments: Vec::new(),
        audio_segments: Vec::new(),
        canvas: test_canvas(),
        target_lufs: -14.0,
        output_path: format!("out-{id}.mp4"),
        status,
    }
}

fn test_dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "oca_export_queue_test_{tag}_{}",
        std::process::id()
    ))
}

#[test]
fn next_available_path_appends_a_parenthesized_counter() {
    let dir = test_dir("next_available_basic");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let original = dir.join("clip.mp4");
    std::fs::write(&original, b"x").unwrap();

    assert_eq!(next_available_path(&original), dir.join("clip (2).mp4"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn next_available_path_skips_every_taken_counter() {
    let dir = test_dir("next_available_skip");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("clip.mp4"), b"x").unwrap();
    std::fs::write(dir.join("clip (2).mp4"), b"x").unwrap();
    std::fs::write(dir.join("clip (3).mp4"), b"x").unwrap();

    assert_eq!(
        next_available_path(&dir.join("clip.mp4")),
        dir.join("clip (4).mp4")
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn next_available_path_preserves_a_missing_extension() {
    let dir = test_dir("next_available_no_ext");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("README"), b"x").unwrap();

    assert_eq!(
        next_available_path(&dir.join("README")),
        dir.join("README (2)")
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_and_load_queue_round_trips_through_the_binary_path() {
    let dir = test_dir("round_trip");
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("queue.ocqueue");
    let legacy_path = dir.join("queue.json");
    let jobs = vec![
        test_job(1, ExportJobStatus::Queued),
        test_job(2, ExportJobStatus::Done),
    ];

    save_queue_to_path(&jobs, &path).unwrap();
    let loaded = load_queue_from_paths(&path, &legacy_path);

    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].id, 1);
    assert_eq!(loaded[0].status, ExportJobStatus::Queued);
    assert_eq!(loaded[1].id, 2);
    assert_eq!(loaded[1].status, ExportJobStatus::Done);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_queue_from_paths_is_empty_when_neither_file_exists() {
    let dir = test_dir("neither_exists");
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("queue.ocqueue");
    let legacy_path = dir.join("queue.json");

    let loaded = load_queue_from_paths(&path, &legacy_path);

    assert!(loaded.is_empty());
}

#[test]
fn load_queue_from_paths_migrates_a_legacy_json_queue_once() {
    let dir = test_dir("migrates_legacy");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("queue.ocqueue");
    let legacy_path = dir.join("queue.json");
    let legacy_jobs = vec![test_job(7, ExportJobStatus::Rendering { percent: 42 })];
    std::fs::write(&legacy_path, serde_json::to_vec(&legacy_jobs).unwrap()).unwrap();

    let loaded = load_queue_from_paths(&path, &legacy_path);

    // Rendering never survives a save/load boundary -- a worker can't still be running after
    // a restart, so migration must normalize it to Queued just like an ordinary load does.
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].status, ExportJobStatus::Queued);
    // The migration must have actually written the new binary file...
    assert!(path.exists());
    // ...and removed the old JSON file so migration never re-runs on the next launch.
    assert!(!legacy_path.exists());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_queue_from_paths_ignores_a_corrupt_legacy_queue() {
    let dir = test_dir("corrupt_legacy");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("queue.ocqueue");
    let legacy_path = dir.join("queue.json");
    std::fs::write(&legacy_path, b"not valid json").unwrap();

    let loaded = load_queue_from_paths(&path, &legacy_path);

    assert!(loaded.is_empty());
    // A legacy file that failed to parse is left alone rather than silently deleted -- it
    // might be recoverable by hand, or worth investigating.
    assert!(legacy_path.exists());
    assert!(!path.exists());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_queue_from_paths_never_falls_back_to_legacy_when_the_binary_file_is_corrupt() {
    let dir = test_dir("corrupt_binary");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("queue.ocqueue");
    let legacy_path = dir.join("queue.json");
    std::fs::write(&path, b"not a valid ocqueue file").unwrap();
    let legacy_jobs = vec![test_job(9, ExportJobStatus::Queued)];
    std::fs::write(&legacy_path, serde_json::to_vec(&legacy_jobs).unwrap()).unwrap();

    let loaded = load_queue_from_paths(&path, &legacy_path);

    // A corrupt binary file must never silently resurrect an older JSON snapshot -- that
    // could reintroduce jobs the user believed were long gone.
    assert!(loaded.is_empty());
    assert!(legacy_path.exists(), "legacy file must be left untouched");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn normalize_loaded_queue_resets_rendering_to_queued() {
    let jobs = vec![test_job(1, ExportJobStatus::Rendering { percent: 55 })];
    let normalized = normalize_loaded_queue(jobs);
    assert_eq!(normalized[0].status, ExportJobStatus::Queued);
}

#[test]
fn normalize_loaded_queue_resets_paused_progress_to_zero_but_keeps_it_paused() {
    let jobs = vec![test_job(1, ExportJobStatus::Paused { percent: 80 })];
    let normalized = normalize_loaded_queue(jobs);
    assert_eq!(normalized[0].status, ExportJobStatus::Paused { percent: 0 });
}

#[test]
fn normalize_loaded_queue_leaves_terminal_statuses_untouched() {
    let jobs = vec![
        test_job(1, ExportJobStatus::Queued),
        test_job(2, ExportJobStatus::Done),
        test_job(
            3,
            ExportJobStatus::Failed {
                message: "boom".to_string(),
            },
        ),
    ];
    let normalized = normalize_loaded_queue(jobs);
    assert_eq!(normalized[0].status, ExportJobStatus::Queued);
    assert_eq!(normalized[1].status, ExportJobStatus::Done);
    assert_eq!(
        normalized[2].status,
        ExportJobStatus::Failed {
            message: "boom".to_string()
        }
    );
}

#[test]
fn persisted_queue_snapshot_matches_normalize_loaded_queues_rules() {
    // save/load must agree on what "survives a restart" means -- a job persisted mid-render
    // and one loaded mid-render should both come back the same way.
    let jobs = vec![
        test_job(1, ExportJobStatus::Rendering { percent: 10 }),
        test_job(2, ExportJobStatus::Paused { percent: 30 }),
    ];
    let snapshot = persisted_queue_snapshot(&jobs);
    assert_eq!(snapshot[0].status, ExportJobStatus::Queued);
    assert_eq!(snapshot[1].status, ExportJobStatus::Paused { percent: 0 });
}
