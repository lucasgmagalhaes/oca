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

//! Export queue and export-preview tests.

use super::support::*;
use super::*;

#[test]
fn queue_export_appends_a_queued_job_with_the_next_id() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(3, ExportJobStatus::Done)],
    );

    app.queue_export(
        "Export".to_string(),
        Vec::new(),
        vec![],
        vec![],
        vec![],
        test_canvas(),
        -14.0,
        "out.mp4".to_string(),
    );

    assert_eq!(app.export_jobs.len(), 2);
    let job = app.export_jobs.last().unwrap();
    assert_eq!(job.id, 4);
    assert_eq!(job.status, ExportJobStatus::Queued);
}

#[test]
fn queue_export_starts_at_one_when_no_jobs_exist() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.queue_export(
        "Export".to_string(),
        Vec::new(),
        vec![],
        vec![],
        vec![],
        test_canvas(),
        -14.0,
        "out.mp4".to_string(),
    );

    assert_eq!(app.export_jobs[0].id, 1);
}

#[test]
fn resolved_active_sequence_export_preview_resolves_the_active_sequences_clips() {
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 4.0)]);
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());
    app.active_project_mut().media_library = vec![test_asset(1)];

    let (track_segments, _audio_segments, canvas) =
        app.resolved_active_sequence_export_preview().unwrap();

    assert_eq!(track_segments.len(), 1);
    assert_eq!(track_segments[0].len(), 1);
    assert_eq!(canvas.width, 1920);
}

#[test]
fn resolved_active_sequence_export_preview_ignores_a_playhead_only_change() {
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 4.0)]);
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());
    app.active_project_mut().media_library = vec![test_asset(1)];

    let (before, _, _) = app.resolved_active_sequence_export_preview().unwrap();
    app.active_project_mut().timeline_mut().playhead_secs = 2.5;
    let (after, _, _) = app.resolved_active_sequence_export_preview().unwrap();

    assert_eq!(before.len(), after.len());
    assert_eq!(
        before[0][0].source_out_secs, after[0][0].source_out_secs,
        "scrubbing must not change the resolved segments"
    );
}

#[test]
fn resolved_active_sequence_export_preview_picks_up_a_later_clip_edit() {
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 4.0)]);
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());
    app.active_project_mut().media_library = vec![test_asset(1)];

    let (before, ..) = app.resolved_active_sequence_export_preview().unwrap();
    assert_eq!(before[0][0].source_out_secs, 4.0);

    app.active_project_mut()
        .timeline_mut()
        .clip_mut(1)
        .unwrap()
        .source_out_secs = 6.0;
    let (after, ..) = app.resolved_active_sequence_export_preview().unwrap();

    assert_eq!(
        after[0][0].source_out_secs, 6.0,
        "a real clip edit must not be served a stale cached result"
    );
}

#[test]
fn queued_job_keeps_the_sequence_export_snapshot_after_settings_change() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.set_active_sequence_export_aspect_ratio(avcore::ExportAspectRatio::Portrait);
    app.set_active_sequence_target_lufs(-23.0);
    let settings = app.active_sequence_export_settings();
    let canvas = avcore::apply_export_aspect_ratio(test_canvas(), settings.aspect_ratio);
    app.queue_export(
        "Short".to_string(),
        Vec::new(),
        vec![],
        vec![],
        vec![],
        canvas,
        settings.target_lufs,
        "short.mp4".to_string(),
    );

    app.set_active_sequence_export_aspect_ratio(avcore::ExportAspectRatio::Square);
    app.set_active_sequence_target_lufs(-16.0);

    assert_eq!(app.export_jobs[0].canvas.width, 1080);
    assert_eq!(app.export_jobs[0].canvas.height, 1920);
    assert_eq!(app.export_jobs[0].target_lufs, -23.0);
}

#[test]
fn apply_platform_export_preset_sets_both_aspect_ratio_and_loudness_target() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    // Starts on defaults distinct from every preset's own settings, so this test can't pass by
    // coincidence.
    app.set_active_sequence_export_aspect_ratio(avcore::ExportAspectRatio::Landscape);
    app.set_active_sequence_target_lufs(-23.0);

    app.apply_platform_export_preset(avcore::PlatformExportPreset::TikTok);

    let settings = app.active_sequence_export_settings();
    assert_eq!(settings.aspect_ratio, avcore::ExportAspectRatio::Portrait);
    assert_eq!(settings.target_lufs, -14.0);
}

#[test]
fn apply_platform_export_preset_leaves_the_pickers_free_to_fine_tune_afterward() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.apply_platform_export_preset(avcore::PlatformExportPreset::InstagramReels);
    app.set_active_sequence_export_aspect_ratio(avcore::ExportAspectRatio::Square);

    assert_eq!(
        app.active_sequence_export_settings().aspect_ratio,
        avcore::ExportAspectRatio::Square,
        "a manual pick after a preset must still take effect, not be locked by the preset"
    );
}

#[test]
fn cancel_export_job_removes_a_job_that_has_not_started_rendering() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Queued)],
    );

    app.cancel_export_job(1);

    assert!(app.export_jobs.is_empty());
}

#[test]
fn match_loudness_across_queued_jobs_only_touches_queued_jobs() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![
            test_job(1, ExportJobStatus::Queued),
            test_job(2, ExportJobStatus::Rendering { percent: 40 }),
            test_job(3, ExportJobStatus::Done),
        ],
    );

    app.match_loudness_across_queued_jobs(-23.0);

    assert_eq!(app.export_jobs[0].target_lufs, -23.0);
    assert_eq!(
        app.export_jobs[1].target_lufs, -14.0,
        "an in-flight render already captured its own target -- changing it now would be a no-op lie"
    );
    assert_eq!(
        app.export_jobs[2].target_lufs, -14.0,
        "a finished job is done -- changing it would just be misleading"
    );
}

#[test]
fn cancel_export_job_flags_an_active_render_instead_of_removing_it() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Rendering { percent: 40 })],
    );
    let control = Arc::new(export::RenderControl::new());
    app.active_renders.insert(1, Arc::clone(&control));

    app.cancel_export_job(1);

    assert!(control.is_cancelled());
    assert_eq!(app.export_jobs.len(), 1);
}

#[test]
fn pause_export_job_holds_a_queued_job_before_it_starts() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Queued)],
    );

    app.pause_export_job(1);

    assert_eq!(
        app.export_jobs[0].status,
        ExportJobStatus::Paused { percent: 0 }
    );
    assert!(app.active_renders.is_empty());
}

#[test]
fn pause_and_resume_export_job_controls_an_active_worker() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Rendering { percent: 40 })],
    );
    let control = Arc::new(export::RenderControl::new());
    app.active_renders.insert(1, Arc::clone(&control));

    app.pause_export_job(1);

    assert!(control.is_paused());
    assert_eq!(
        app.export_jobs[0].status,
        ExportJobStatus::Paused { percent: 40 }
    );

    app.resume_export_job(1);

    assert!(!control.is_paused());
    assert_eq!(
        app.export_jobs[0].status,
        ExportJobStatus::Rendering { percent: 40 }
    );
}

#[test]
fn resume_export_job_requeues_a_paused_job_without_a_live_worker() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Paused { percent: 0 })],
    );

    app.resume_export_job(1);

    assert_eq!(app.export_jobs[0].status, ExportJobStatus::Queued);
}

#[test]
fn persisted_queue_keeps_paused_jobs_paused_but_resets_their_progress() {
    let jobs = vec![
        test_job(1, ExportJobStatus::Paused { percent: 65 }),
        test_job(2, ExportJobStatus::Rendering { percent: 30 }),
    ];

    let snapshot = export::persisted_queue_snapshot(&jobs);

    assert_eq!(snapshot[0].status, ExportJobStatus::Paused { percent: 0 });
    assert_eq!(snapshot[1].status, ExportJobStatus::Queued);
}

#[test]
fn loading_a_queue_preserves_a_paused_job_until_the_user_resumes_it() {
    let jobs = vec![
        test_job(1, ExportJobStatus::Paused { percent: 65 }),
        test_job(2, ExportJobStatus::Rendering { percent: 30 }),
    ];

    let normalized = export::normalize_loaded_queue(jobs);

    assert_eq!(normalized[0].status, ExportJobStatus::Paused { percent: 0 });
    assert_eq!(normalized[1].status, ExportJobStatus::Queued);
}

#[test]
fn ocqueue_file_atomically_replaces_the_previous_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("queue.ocqueue");
    export::save_queue_to_path(&[test_job(1, ExportJobStatus::Done)], &path).unwrap();
    export::save_queue_to_path(
        &[test_job(2, ExportJobStatus::Rendering { percent: 55 })],
        &path,
    )
    .unwrap();

    let bytes = std::fs::read(&path).unwrap();
    let jobs: Vec<ExportJob> = avcore::from_ocqueue_bytes(&bytes).unwrap();

    assert_eq!(&bytes[..4], b"OCQU");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, 2);
    assert_eq!(jobs[0].status, ExportJobStatus::Queued);
}

#[test]
fn loading_migrates_the_legacy_json_queue_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("queue.ocqueue");
    let legacy_path = dir.path().join("queue.json");
    let legacy_jobs = vec![
        test_job(1, ExportJobStatus::Paused { percent: 65 }),
        test_job(2, ExportJobStatus::Rendering { percent: 30 }),
    ];
    std::fs::write(&legacy_path, serde_json::to_vec(&legacy_jobs).unwrap()).unwrap();

    let loaded = export::load_queue_from_paths(&path, &legacy_path);

    assert_eq!(loaded[0].status, ExportJobStatus::Paused { percent: 0 });
    assert_eq!(loaded[1].status, ExportJobStatus::Queued);
    assert!(path.exists());
    assert!(!legacy_path.exists());
    let migrated: Vec<ExportJob> =
        avcore::from_ocqueue_bytes(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(migrated.len(), loaded.len());
    assert_eq!(migrated[0].id, loaded[0].id);
    assert_eq!(migrated[0].status, loaded[0].status);
    assert_eq!(migrated[1].id, loaded[1].id);
    assert_eq!(migrated[1].status, loaded[1].status);
}

#[test]
fn a_corrupt_ocqueue_does_not_resurrect_an_older_json_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("queue.ocqueue");
    let legacy_path = dir.path().join("queue.json");
    std::fs::write(&path, b"corrupt").unwrap();
    std::fs::write(
        &legacy_path,
        serde_json::to_vec(&vec![test_job(1, ExportJobStatus::Done)]).unwrap(),
    )
    .unwrap();

    let loaded = export::load_queue_from_paths(&path, &legacy_path);

    assert!(loaded.is_empty());
    assert!(legacy_path.exists());
}

#[test]
fn cancelling_a_paused_control_releases_its_waiter() {
    let control = Arc::new(export::RenderControl::new());
    control.pause();
    let waiter_control = Arc::clone(&control);
    let (tx, rx) = std::sync::mpsc::channel();
    let waiter = std::thread::spawn(move || {
        tx.send(waiter_control.wait_if_paused()).unwrap();
    });

    assert!(rx.recv_timeout(Duration::from_millis(25)).is_err());
    control.cancel();

    assert!(!rx.recv_timeout(Duration::from_secs(1)).unwrap());
    waiter.join().unwrap();
}

#[test]
fn pump_export_queue_applies_a_progress_event_to_the_matching_job() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Rendering { percent: 0 })],
    );
    app.active_renders
        .insert(1, Arc::new(export::RenderControl::new()));
    app.render_tx
        .send(RenderEvent::Progress {
            job_id: 1,
            percent: 42,
        })
        .unwrap();

    app.pump_export_queue();

    assert_eq!(
        app.export_jobs[0].status,
        ExportJobStatus::Rendering { percent: 42 }
    );
}

#[test]
fn pump_export_queue_updates_progress_without_resuming_a_paused_job() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Paused { percent: 20 })],
    );
    app.active_renders
        .insert(1, Arc::new(export::RenderControl::new()));
    app.render_tx
        .send(RenderEvent::Progress {
            job_id: 1,
            percent: 42,
        })
        .unwrap();

    app.pump_export_queue();

    assert_eq!(
        app.export_jobs[0].status,
        ExportJobStatus::Paused { percent: 42 }
    );
}

#[test]
fn pump_export_queue_does_not_dispatch_a_paused_queued_job() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Paused { percent: 0 })],
    );

    app.pump_export_queue();

    assert!(app.active_renders.is_empty());
    assert_eq!(
        app.export_jobs[0].status,
        ExportJobStatus::Paused { percent: 0 }
    );
}

#[test]
fn pump_export_queue_marks_a_job_done_and_frees_its_render_slot() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Rendering { percent: 90 })],
    );
    app.active_renders
        .insert(1, Arc::new(export::RenderControl::new()));
    app.render_tx
        .send(RenderEvent::Done {
            job_id: 1,
            duration_ms: 1000,
            output_duration_secs: 10.0,
        })
        .unwrap();

    app.export_job_started_at
        .insert(1, std::time::Instant::now());

    app.pump_export_queue();

    assert_eq!(app.export_jobs[0].status, ExportJobStatus::Done);
    assert!(!app.active_renders.contains_key(&1));
    assert!(!app.export_job_started_at.contains_key(&1));
}

#[test]
fn pump_export_queue_records_a_failure_message() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Rendering { percent: 10 })],
    );
    app.active_renders
        .insert(1, Arc::new(export::RenderControl::new()));
    app.render_tx
        .send(RenderEvent::Failed {
            job_id: 1,
            message: "disk full".to_string(),
            duration_ms: 500,
            output_duration_secs: 10.0,
        })
        .unwrap();

    app.export_job_started_at
        .insert(1, std::time::Instant::now());

    app.pump_export_queue();

    assert_eq!(
        app.export_jobs[0].status,
        ExportJobStatus::Failed {
            message: "disk full".to_string()
        }
    );
    assert!(!app.active_renders.contains_key(&1));
    assert!(!app.export_job_started_at.contains_key(&1));
}

#[test]
fn pump_export_queue_removes_a_cancelled_job_entirely() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Rendering { percent: 10 })],
    );
    app.active_renders
        .insert(1, Arc::new(export::RenderControl::new()));
    app.render_tx
        .send(RenderEvent::Cancelled { job_id: 1 })
        .unwrap();

    app.export_job_started_at
        .insert(1, std::time::Instant::now());

    app.pump_export_queue();

    assert!(app.export_jobs.is_empty());
    assert!(!app.active_renders.contains_key(&1));
    assert!(!app.export_job_started_at.contains_key(&1));
}
