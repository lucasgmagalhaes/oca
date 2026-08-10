use super::*;
use avcore::{MediaAsset, MediaKind, Recency, Timeline};

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
    app.active_renders.insert(1, Arc::new(AtomicBool::new(false)));
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
    app.active_renders.insert(1, Arc::new(AtomicBool::new(false)));
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
    app.active_renders.insert(1, Arc::new(AtomicBool::new(false)));
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
    app.active_renders.insert(1, Arc::new(AtomicBool::new(false)));
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
    let mut app = test_app(vec![test_project(1, vec![test_asset(1), test_asset(2)])], Vec::new());
    app.selected_asset_id = Some(2);

    assert_eq!(app.selected_asset().unwrap().id, 2);
}
