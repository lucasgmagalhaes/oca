use super::*;
use avcore::timeline::{ClipInstance, Track, TrackKind};
use avcore::{MediaAsset, MediaKind, Recency, Timeline};
use eframe::egui;

fn test_project(id: u64, assets: Vec<MediaAsset>) -> Project {
    Project {
        id,
        name: format!("Project {id}"),
        last_edited: Recency::HoursAgo(0),
        summary: String::new(),
        media_library: assets,
        timeline: Timeline {
            tracks: Vec::new(),
            playhead_secs: 0.0,
        },
        file_path: None,
    }
}

fn test_project_with_tracks(id: u64, tracks: Vec<Track>) -> Project {
    let mut project = test_project(id, Vec::new());
    project.timeline.tracks = tracks;
    project
}

fn test_track(id: u64, kind: TrackKind, clips: Vec<ClipInstance>) -> Track {
    Track {
        id,
        name: format!("Track {id}"),
        kind,
        clips,
    }
}

fn test_clip(id: u64, start_secs: f64, source_in_secs: f64, source_out_secs: f64) -> ClipInstance {
    ClipInstance {
        id,
        asset_id: 1,
        start_secs,
        source_in_secs,
        source_out_secs,
    }
}

fn test_asset(id: u64) -> MediaAsset {
    MediaAsset {
        id,
        file_name: format!("asset-{id}.mp4"),
        source_path: PathBuf::from(format!("asset-{id}.mp4")),
        kind: MediaKind::Video,
        duration_secs: 10.0,
        codec: "h264".to_string(),
        source_bitrate_mbps: 8.0,
        resolution: Some((1920, 1080)),
        fps: Some(30.0),
        sample_rate_khz: None,
        loudness: None,
        proxy_path: None,
    }
}

fn test_job(id: u64, status: ExportJobStatus) -> ExportJob {
    ExportJob {
        id,
        title: format!("Job {id}"),
        source_path: PathBuf::from(format!("job-{id}.mp4")),
        target_lufs: -14.0,
        bitrate_mbps: 8.0,
        output_path: format!("out-{id}.mp4"),
        status,
    }
}

fn test_app(projects: Vec<Project>, export_jobs: Vec<ExportJob>) -> OcaApp {
    let (render_tx, render_rx) = mpsc::unbounded_channel();
    let (import_tx, import_rx) = mpsc::unbounded_channel();
    OcaApp {
        screen: Screen::Home,
        tool: EditorTool::Select,
        locale: Locale::PtBr,
        projects,
        active_project: 0,
        selected_asset_id: None,
        export_jobs,
        queue_workers: 1,
        prefs: PrefsState::default(),
        render_tx,
        render_rx,
        active_renders: HashMap::new(),
        preview: None,
        preview_texture: None,
        preview_playing: false,
        import_tx,
        import_rx,
        pending_imports: 0,
        selected_clip_id: None,
        timeline_px_per_sec: 4.0,
    }
}

#[test]
fn open_project_switches_active_project_and_selects_its_first_asset() {
    let mut app = test_app(
        vec![
            test_project(1, Vec::new()),
            test_project(2, vec![test_asset(42), test_asset(43)]),
        ],
        Vec::new(),
    );

    app.open_project(1);

    assert_eq!(app.active_project, 1);
    assert_eq!(app.selected_asset_id, Some(42));
    assert_eq!(app.screen, Screen::Editor);
}

#[test]
fn open_project_with_an_empty_media_library_selects_nothing() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.open_project(0);

    assert_eq!(app.selected_asset_id, None);
}

#[test]
fn add_and_open_project_appends_and_opens_it() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_and_open_project(test_project(99, vec![test_asset(7)]));

    assert_eq!(app.projects.len(), 2);
    assert_eq!(app.active_project, 1);
    assert_eq!(app.active_project().id, 99);
    assert_eq!(app.selected_asset_id, Some(7));
}

#[test]
fn create_new_project_assigns_the_next_id_after_the_highest_existing_one() {
    let mut app = test_app(
        vec![test_project(1, Vec::new()), test_project(5, Vec::new())],
        Vec::new(),
    );

    app.create_new_project("New".to_string());

    assert_eq!(app.active_project().id, 6);
    assert_eq!(app.active_project().name, "New");
    assert_eq!(app.screen, Screen::Editor);
}

#[test]
fn create_new_project_starts_at_one_when_no_projects_exist() {
    let mut app = test_app(Vec::new(), Vec::new());

    app.create_new_project("First".to_string());

    assert_eq!(app.active_project().id, 1);
}

#[test]
fn queue_export_appends_a_queued_job_with_the_next_id() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(3, ExportJobStatus::Done)],
    );

    app.queue_export(
        "Export".to_string(),
        PathBuf::from("in.mp4"),
        -14.0,
        8.0,
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
        PathBuf::from("in.mp4"),
        -14.0,
        8.0,
        "out.mp4".to_string(),
    );

    assert_eq!(app.export_jobs[0].id, 1);
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
fn cancel_export_job_flags_an_active_render_instead_of_removing_it() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Rendering { percent: 40 })],
    );
    let cancel_flag = Arc::new(AtomicBool::new(false));
    app.active_renders.insert(1, Arc::clone(&cancel_flag));

    app.cancel_export_job(1);

    assert!(cancel_flag.load(Ordering::Relaxed));
    assert_eq!(app.export_jobs.len(), 1);
}

#[test]
fn pump_export_queue_applies_a_progress_event_to_the_matching_job() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Rendering { percent: 0 })],
    );
    app.active_renders
        .insert(1, Arc::new(AtomicBool::new(false)));
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
fn pump_export_queue_marks_a_job_done_and_frees_its_render_slot() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Rendering { percent: 90 })],
    );
    app.active_renders
        .insert(1, Arc::new(AtomicBool::new(false)));
    app.render_tx.send(RenderEvent::Done { job_id: 1 }).unwrap();

    app.pump_export_queue();

    assert_eq!(app.export_jobs[0].status, ExportJobStatus::Done);
    assert!(!app.active_renders.contains_key(&1));
}

#[test]
fn pump_export_queue_records_a_failure_message() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Rendering { percent: 10 })],
    );
    app.active_renders
        .insert(1, Arc::new(AtomicBool::new(false)));
    app.render_tx
        .send(RenderEvent::Failed {
            job_id: 1,
            message: "disk full".to_string(),
        })
        .unwrap();

    app.pump_export_queue();

    assert_eq!(
        app.export_jobs[0].status,
        ExportJobStatus::Failed {
            message: "disk full".to_string()
        }
    );
    assert!(!app.active_renders.contains_key(&1));
}

#[test]
fn pump_export_queue_removes_a_cancelled_job_entirely() {
    let mut app = test_app(
        vec![test_project(1, Vec::new())],
        vec![test_job(1, ExportJobStatus::Rendering { percent: 10 })],
    );
    app.active_renders
        .insert(1, Arc::new(AtomicBool::new(false)));
    app.render_tx
        .send(RenderEvent::Cancelled { job_id: 1 })
        .unwrap();

    app.pump_export_queue();

    assert!(app.export_jobs.is_empty());
    assert!(!app.active_renders.contains_key(&1));
}

#[test]
fn selected_asset_is_none_when_no_asset_id_is_selected() {
    let app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    assert!(app.selected_asset().is_none());
}

#[test]
fn selected_asset_finds_the_asset_in_the_active_project() {
    let mut app = test_app(
        vec![test_project(1, vec![test_asset(1), test_asset(2)])],
        Vec::new(),
    );
    app.selected_asset_id = Some(2);

    assert_eq!(app.selected_asset().unwrap().id, 2);
}

#[test]
fn select_asset_updates_the_selection() {
    let mut app = test_app(
        vec![test_project(1, vec![test_asset(1), test_asset(2)])],
        Vec::new(),
    );

    app.select_asset(Some(2));

    assert_eq!(app.selected_asset_id, Some(2));
}

#[test]
fn select_asset_with_none_clears_the_selection() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());
    app.selected_asset_id = Some(1);

    app.select_asset(None);

    assert_eq!(app.selected_asset_id, None);
}

#[test]
fn select_asset_resets_playback_state_and_the_uploaded_texture() {
    let mut app = test_app(
        vec![test_project(1, vec![test_asset(1), test_asset(2)])],
        Vec::new(),
    );
    app.selected_asset_id = Some(1);
    app.preview_playing = true;
    let ctx = egui::Context::default();
    let image = egui::ColorImage::new([1, 1], vec![egui::Color32::BLACK]);
    app.preview_texture = Some(ctx.load_texture("test", image, egui::TextureOptions::default()));

    app.select_asset(Some(2));

    assert!(!app.preview_playing);
    assert!(app.preview_texture.is_none());
}

// test_asset()'s source_path is a relative, nonexistent file, so `reload_preview` always
// bails out before actually touching GStreamer here (see the `path.exists()` guard in
// app.rs) — these exercise the no-pipeline branches of the preview API, not real playback.
#[test]
fn preview_accessors_are_none_without_a_live_pipeline() {
    let app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    assert!(!app.preview_available());
    assert_eq!(app.preview_duration_secs(), None);
    assert_eq!(app.preview_position_secs(), None);
}

#[test]
fn toggle_preview_playback_is_a_no_op_without_a_live_pipeline() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.toggle_preview_playback();

    assert!(!app.preview_playing);
}

#[test]
fn seek_preview_is_a_no_op_without_a_live_pipeline() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.seek_preview(5.0);
}

#[test]
fn split_at_playhead_splits_the_covering_clip_on_every_track_that_has_one() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 20.0)]),
                test_track(2, TrackKind::Audio, vec![test_clip(2, 0.0, 0.0, 20.0)]),
            ],
        )],
        Vec::new(),
    );
    app.active_project_mut().timeline.playhead_secs = 10.0;

    app.split_at_playhead();

    let tracks = &app.active_project().timeline.tracks;
    assert_eq!(tracks[0].clips.len(), 2);
    assert_eq!(tracks[1].clips.len(), 2);
    assert_eq!(tracks[0].clips[1].start_secs, 10.0);
    assert_eq!(tracks[1].clips[1].start_secs, 10.0);
    // Each track's new half got its own fresh id — no collision between them.
    assert_ne!(tracks[0].clips[1].id, tracks[1].clips[1].id);
}

#[test]
fn split_at_playhead_only_splits_tracks_the_playhead_actually_covers() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 20.0)]),
                test_track(2, TrackKind::Audio, vec![test_clip(2, 0.0, 0.0, 5.0)]),
            ],
        )],
        Vec::new(),
    );
    app.active_project_mut().timeline.playhead_secs = 10.0;

    app.split_at_playhead();

    let tracks = &app.active_project().timeline.tracks;
    assert_eq!(tracks[0].clips.len(), 2);
    assert_eq!(tracks[1].clips.len(), 1);
}

#[test]
fn split_at_playhead_is_a_no_op_when_nothing_covers_the_playhead() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 20.0)],
            )],
        )],
        Vec::new(),
    );
    app.active_project_mut().timeline.playhead_secs = 50.0;

    app.split_at_playhead();

    assert_eq!(app.active_project().timeline.tracks[0].clips.len(), 1);
}

#[test]
fn trim_clip_start_moves_the_left_edge_and_keeps_the_end_fixed() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 10.0, 5.0, 30.0)],
            )],
        )],
        Vec::new(),
    );

    app.trim_clip_start(1, 15.0);

    let clip = &app.active_project().timeline.tracks[0].clips[0];
    assert_eq!(clip.start_secs, 15.0);
    assert_eq!(clip.source_in_secs, 10.0);
    assert_eq!(clip.source_out_secs, 30.0);
}

#[test]
fn trim_clip_start_ignores_an_unknown_clip_id() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 10.0, 5.0, 30.0)],
            )],
        )],
        Vec::new(),
    );

    app.trim_clip_start(99, 15.0);

    assert_eq!(
        app.active_project().timeline.tracks[0].clips[0].start_secs,
        10.0
    );
}

#[test]
fn trim_clip_end_moves_the_right_edge_and_keeps_the_start_fixed() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline.tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 8.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.trim_clip_end(1, 5.0);

    let clip = &app.active_project().timeline.tracks[0].clips[0];
    assert_eq!(clip.start_secs, 0.0);
    assert_eq!(clip.source_out_secs, 5.0);
}

#[test]
fn trim_clip_end_is_bounded_by_the_source_assets_own_duration() {
    // test_asset's duration_secs is a fixed 10.0.
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline.tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 8.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.trim_clip_end(1, 50.0);

    assert_eq!(
        app.active_project().timeline.tracks[0].clips[0].source_out_secs,
        8.0
    );
}

#[test]
fn move_clip_repositions_it_on_its_own_track() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 5.0, 15.0)],
            )],
        )],
        Vec::new(),
    );

    app.move_clip(1, 40.0);

    let clip = &app.active_project().timeline.tracks[0].clips[0];
    assert_eq!(clip.start_secs, 40.0);
    assert_eq!(clip.source_in_secs, 5.0);
    assert_eq!(clip.source_out_secs, 15.0);
}

#[test]
fn move_clip_ignores_a_negative_position() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 10.0, 0.0, 20.0)],
            )],
        )],
        Vec::new(),
    );

    app.move_clip(1, -5.0);

    assert_eq!(
        app.active_project().timeline.tracks[0].clips[0].start_secs,
        10.0
    );
}

#[test]
fn move_clip_to_track_relocates_a_clip_to_a_same_kind_track() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]),
                test_track(2, TrackKind::Video, Vec::new()),
            ],
        )],
        Vec::new(),
    );

    app.move_clip_to_track(1, 2, 5.0);

    assert!(app.active_project().timeline.tracks[0].clips.is_empty());
    let moved = &app.active_project().timeline.tracks[1].clips[0];
    assert_eq!(moved.id, 1);
    assert_eq!(moved.start_secs, 5.0);
}

#[test]
fn move_clip_to_track_ignores_a_mismatched_kind() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]),
                test_track(2, TrackKind::Audio, Vec::new()),
            ],
        )],
        Vec::new(),
    );

    app.move_clip_to_track(1, 2, 5.0);

    assert_eq!(app.active_project().timeline.tracks[0].clips.len(), 1);
    assert!(app.active_project().timeline.tracks[1].clips.is_empty());
}

#[test]
fn delete_selected_clip_removes_it_from_its_track_and_clears_the_selection() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 10.0), test_clip(2, 10.0, 0.0, 20.0)],
            )],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.delete_selected_clip();

    let clips = &app.active_project().timeline.tracks[0].clips;
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].id, 2);
    assert_eq!(app.selected_clip_id, None);
}

#[test]
fn delete_selected_clip_is_a_no_op_when_nothing_is_selected() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 10.0)],
            )],
        )],
        Vec::new(),
    );

    app.delete_selected_clip();

    assert_eq!(app.active_project().timeline.tracks[0].clips.len(), 1);
}

#[test]
fn pump_import_queue_adds_the_asset_to_its_target_project_with_a_fresh_id() {
    let mut app = test_app(
        vec![
            test_project(1, vec![test_asset(5)]),
            test_project(2, Vec::new()),
        ],
        Vec::new(),
    );
    app.pending_imports = 1;
    let mut ready_asset = test_asset(0);
    ready_asset.file_name = "clip.mp4".to_string();
    app.import_tx
        .send(ImportEvent::AssetReady {
            project_id: 1,
            asset: ready_asset,
        })
        .unwrap();

    app.pump_import_queue();

    let project = app.projects.iter().find(|p| p.id == 1).unwrap();
    assert_eq!(project.media_library.len(), 2);
    let imported = project
        .media_library
        .iter()
        .find(|a| a.file_name == "clip.mp4")
        .unwrap();
    assert_eq!(imported.id, 6);
    assert_eq!(app.pending_imports, 0);
}

#[test]
fn pump_import_queue_targets_the_project_by_id_not_the_active_index() {
    let mut app = test_app(
        vec![test_project(1, Vec::new()), test_project(2, Vec::new())],
        Vec::new(),
    );
    app.active_project = 1; // Simulates the user switching projects mid-import.
    app.import_tx
        .send(ImportEvent::AssetReady {
            project_id: 1,
            asset: test_asset(0),
        })
        .unwrap();

    app.pump_import_queue();

    assert_eq!(
        app.projects
            .iter()
            .find(|p| p.id == 1)
            .unwrap()
            .media_library
            .len(),
        1
    );
    assert!(app
        .projects
        .iter()
        .find(|p| p.id == 2)
        .unwrap()
        .media_library
        .is_empty());
}

#[test]
fn pump_import_queue_drops_a_failed_import_without_panicking() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.pending_imports = 1;
    app.import_tx
        .send(ImportEvent::Failed {
            path: PathBuf::from("missing.mp4"),
            message: "not found".to_string(),
        })
        .unwrap();

    app.pump_import_queue();

    assert_eq!(app.pending_imports, 0);
    assert!(app.active_project().media_library.is_empty());
}
