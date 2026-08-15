use super::*;
use avcore::timeline::{ClipInstance, Track, TrackKind};
use avcore::{LoudnessMetrics, MediaAsset, MediaKind, Recency, Sequence, Timeline};
use eframe::egui;

fn test_project(id: u64, assets: Vec<MediaAsset>) -> Project {
    Project {
        id,
        name: format!("Project {id}"),
        last_edited: Recency::HoursAgo(0),
        summary: String::new(),
        media_library: assets,
        sequences: vec![Sequence {
            id: 1,
            name: "Sequence 1".to_string(),
            timeline: Timeline {
                tracks: Vec::new(),
                playhead_secs: 0.0,
            },
        }],
        active_sequence: 0,
        file_path: None,
    }
}

fn test_project_with_tracks(id: u64, tracks: Vec<Track>) -> Project {
    let mut project = test_project(id, Vec::new());
    project.timeline_mut().tracks = tracks;
    project
}

fn test_track(id: u64, kind: TrackKind, clips: Vec<ClipInstance>) -> Track {
    Track {
        id,
        name: format!("Track {id}"),
        kind,
        clips,

        text_clips: vec![],

        visible: true,
    }
}

fn test_clip(id: u64, start_secs: f64, source_in_secs: f64, source_out_secs: f64) -> ClipInstance {
    ClipInstance {
        id,
        asset_id: 1,
        start_secs,
        source_in_secs,
        source_out_secs,
        composite_id: None,
        gain_db: 0.0,
        frozen: false,
        speed_factor: 1.0,
        crop_x: 0.0,
        crop_y: 0.0,
        crop_w: 1.0,
        crop_h: 1.0,
        mask_shape: avcore::timeline::MaskShape::None,
        mask_corner_radius: 0.0,
        flipped_h: false,
        color_filter: avcore::timeline::ColorFilter::None,
        vignette_intensity: 0.0,
        brightness: 0.0,
        contrast: 1.0,
        saturation: 1.0,
        sharpen: 0.0,
        chroma_key_enabled: false,
        chroma_key_color: [0, 255, 0],
        chroma_key_tolerance: 0.4,
        blur_intensity: 0.0,
        shake_intensity: 0.0,
        glitch_intensity: 0.0,
        pixelize_intensity: 0.0,
        transition_in: avcore::timeline::TransitionType::None,
        transition_duration_secs: 0.5,
        position_keyframes: vec![],
        scale_keyframes: vec![],
        rotation_keyframes: vec![],
        opacity_keyframes: vec![],
        deflicker_enabled: false,
        lut_path: String::new(),
        layer_scale_x: 1.0,
        layer_scale_y: 1.0,
        stabilization_intensity: 0.0,
        background_removal_enabled: false,
        background_removal_mask_path: String::new(),
    }
}

fn test_composite_clip(id: u64, start_secs: f64, composite_id: u64) -> ClipInstance {
    ClipInstance {
        composite_id: Some(composite_id),
        ..test_clip(id, start_secs, 0.0, 10.0)
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
        waveform_peaks: None,
    }
}

fn test_asset_with_kind(id: u64, kind: MediaKind) -> MediaAsset {
    MediaAsset {
        kind,
        ..test_asset(id)
    }
}

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

        track_segments: Vec::new(),

        canvas: test_canvas(),
        target_lufs: -14.0,
        output_path: format!("out-{id}.mp4"),
        status,
    }
}

fn test_app(projects: Vec<Project>, export_jobs: Vec<ExportJob>) -> App {
    let (render_tx, render_rx) = mpsc::unbounded_channel();
    let (import_tx, import_rx) = mpsc::unbounded_channel();
    let (thumbnail_tx, thumbnail_rx) = mpsc::unbounded_channel();
    let (transcribe_tx, transcribe_rx) = mpsc::unbounded_channel();
    let (auto_reframe_tx, auto_reframe_rx) = mpsc::unbounded_channel();
    let (motion_tracking_tx, motion_tracking_rx) = mpsc::unbounded_channel();
    let (tts_tx, tts_rx) = mpsc::unbounded_channel();
    let (model_download_tx, model_download_rx) = mpsc::unbounded_channel();
    let (sound_library_tx, sound_library_rx) = mpsc::unbounded_channel();
    App {
        screen: Screen::Home,
        tool: EditorTool::Select,
        locale: Locale::PtBr,
        projects,
        active_project: 0,
        selected_asset_id: None,
        export_jobs,
        prefs: PrefsState::default(),
        render_tx,
        render_rx,
        active_renders: HashMap::new(),
        preview: None,
        preview_clip_id: None,
        preview_texture: None,
        preview_playing: false,
        preview_frozen_since: None,
        import_tx,
        import_rx,
        pending_imports: 0,
        next_import_token: 0,
        pending_enrichment: HashMap::new(),
        auto_add_to_timeline: HashSet::new(),
        sound_library_tracks: Vec::new(),
        sound_library_tx,
        sound_library_rx,
        transcribe_tx,
        transcribe_rx,
        transcribing_asset_id: None,
        auto_reframe_tx,
        auto_reframe_rx,
        auto_reframing_clip_id: None,
        motion_tracking_tx,
        motion_tracking_rx,
        motion_tracking_clip_id: None,
        tts_tx,
        tts_rx,
        tts_modal_text: None,
        tts_generating: false,
        model_download_tx,
        model_download_rx,
        model_download_progress: None,
        model_download_kind: None,
        cancel_model_download: None,
        selected_clip_id: None,
        selected_text_clip_id: None,
        timeline_px_per_sec: 4.0,
        lib_panel_width: 220.0,
        props_panel_width: 240.0,
        timeline_height: 190.0,
        thumbnail_tx,
        thumbnail_rx,
        thumbnail_textures: HashMap::new(),
        requested_thumbnails: HashSet::new(),
        pending_asset_drop: None,
        clipboard_clip: None,
        formatting_clipboard: None,
        multi_selected_clip_ids: HashSet::new(),
        toasts: Vec::new(),
        prefs_open: false,
        prev_prefs_open: false,
        project_dirty: false,
        last_edit_instant: None,
        last_autosave_instant: None,
        autosave_restore_pending: None,
        crash_detected: false,
        export_aspect_ratio: avcore::ExportAspectRatio::default(),
        renaming_project: None::<(usize, String, String)>,
        renaming_sequence: None,
        saving_layer_template: None,
        applying_layer_template: None,
        layer_templates_menu_open: false,
        binding_capture: None,
        pending_export_conflict: None,
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
fn add_sequence_appends_a_named_tab_and_switches_to_it() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_sequence();

    assert_eq!(app.active_project().sequences.len(), 2);
    assert_eq!(app.active_project().sequences[1].name, "Sequência 2");
    assert_eq!(app.active_project().active_sequence, 1);
    assert!(app.active_project().timeline().tracks.is_empty());
}

#[test]
fn add_sequence_clears_a_stale_clip_selection() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.selected_clip_id = Some(1);

    app.add_sequence();

    assert_eq!(app.selected_clip_id, None);
}

#[test]
fn select_sequence_switches_the_active_index() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_sequence();
    app.select_sequence(0);

    assert_eq!(app.active_project().active_sequence, 0);
}

#[test]
fn select_sequence_is_a_no_op_for_an_out_of_range_index() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.select_sequence(5);

    assert_eq!(app.active_project().active_sequence, 0);
}

#[test]
fn add_asset_to_timeline_creates_a_track_and_appends_a_clip_when_none_exists() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.add_asset_to_timeline(1);

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].name, "V1");
    assert_eq!(tracks[0].kind, TrackKind::Video);
    assert_eq!(tracks[0].clips.len(), 1);
    let clip = &tracks[0].clips[0];
    assert_eq!(clip.asset_id, 1);
    assert_eq!(clip.start_secs, 0.0);
    assert_eq!(clip.source_in_secs, 0.0);
    assert_eq!(clip.source_out_secs, 10.0); // test_asset's fixed duration_secs.
}

#[test]
fn add_asset_to_timeline_creates_an_audio_track_for_an_audio_asset() {
    let mut app = test_app(
        vec![test_project(
            1,
            vec![test_asset_with_kind(1, MediaKind::Audio)],
        )],
        Vec::new(),
    );

    app.add_asset_to_timeline(1);

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks[0].name, "A1");
    assert_eq!(tracks[0].kind, TrackKind::Audio);
}

#[test]
fn add_asset_to_timeline_appends_after_whatever_is_already_on_the_matching_track() {
    let mut project = test_project(1, vec![test_asset(2)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 20.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.add_asset_to_timeline(2);

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1); // Reused the existing Video track, no second one created.
    assert_eq!(tracks[0].clips.len(), 2);
    assert_eq!(tracks[0].clips[1].start_secs, 20.0);
    assert_eq!(tracks[0].clips[1].asset_id, 2);
}

#[test]
fn add_asset_to_timeline_is_a_no_op_for_an_unknown_asset_id() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_asset_to_timeline(99);

    assert!(app.active_project().timeline().tracks.is_empty());
}

#[test]
fn request_thumbnail_marks_the_bucket_requested_and_does_not_duplicate_on_a_second_call() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.request_thumbnail(1, 0);
    assert!(app.requested_thumbnails.contains(&(1, 0)));

    app.request_thumbnail(1, 0);
    assert_eq!(app.requested_thumbnails.len(), 1);
}

#[test]
fn request_thumbnail_still_marks_requested_for_an_unknown_asset_id() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.request_thumbnail(99, 0);

    // Marked requested even though the asset lookup failed, so this bucket isn't retried
    // every frame — it just never gets a texture.
    assert!(app.requested_thumbnails.contains(&(99, 0)));
}

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
        test_canvas(),
        -14.0,
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
fn select_asset_only_updates_selected_asset_id() {
    // Selecting a media-library asset no longer touches preview state — preview follows the
    // timeline playhead now, a separate concept from the library selection (see
    // `current_preview_clip`).
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

    assert_eq!(app.selected_asset_id, Some(2));
    assert!(app.preview_playing);
    assert!(app.preview_texture.is_some());
}

// test_asset()'s source_path is a relative, nonexistent file, so `ensure_preview_loaded`
// always bails out before actually touching GStreamer here (see the `path.exists()` guard in
// app.rs) — these exercise the no-pipeline branches of the preview API, not real playback.
#[test]
fn preview_available_is_false_without_a_live_pipeline() {
    let app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    assert!(!app.preview_available());
}

#[test]
fn toggle_preview_playback_is_a_no_op_without_a_live_pipeline() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.toggle_preview_playback();

    assert!(!app.preview_playing);
}

#[test]
fn seek_preview_updates_the_timeline_playhead_without_a_live_pipeline() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.seek_preview(5.0);

    assert_eq!(app.active_project().timeline().playhead_secs, 5.0);
}

#[test]
fn frozen_playhead_advances_by_elapsed_wall_clock_time() {
    use super::preview::frozen_playhead;

    assert_eq!(frozen_playhead(2.0, 1.5, 0.0, 10.0), 3.5);
}

#[test]
fn frozen_playhead_clamps_to_the_clip_end() {
    use super::preview::frozen_playhead;

    // clip spans [5.0, 5.0+2.0) = [5.0, 7.0); way more than 2s elapsed should still land
    // exactly on the clip's end, not run past it.
    assert_eq!(frozen_playhead(5.0, 100.0, 5.0, 2.0), 7.0);
}

#[test]
fn ensure_preview_loaded_is_a_no_op_with_no_video_track() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.ensure_preview_loaded();

    assert!(!app.preview_clip_present());
    assert!(!app.preview_available());
}

#[test]
fn ensure_preview_loaded_does_not_mark_a_clip_present_when_its_asset_is_missing() {
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

    app.ensure_preview_loaded();

    // test_clip's asset_id (1) isn't in this project's (empty) media_library, so
    // current_preview_clip() actually resolves to None here — covers the "clip exists on the
    // track but its asset can't be found" branch distinctly from "no clip at all".
    assert!(!app.preview_clip_present());
}

#[test]
fn ensure_preview_loaded_resolves_the_clip_covering_the_playhead() {
    let mut project = test_project_with_tracks(
        1,
        vec![test_track(
            1,
            TrackKind::Video,
            vec![test_clip(1, 0.0, 0.0, 10.0)],
        )],
    );
    project.media_library.push(test_asset(1));
    project.timeline_mut().playhead_secs = 3.0;
    let mut app = test_app(vec![project], Vec::new());

    app.ensure_preview_loaded();

    // The clip covers the playhead and its asset resolves, so a pipeline open was attempted
    // (and failed only because test_asset's source_path doesn't exist on disk).
    assert!(app.preview_clip_present());
    assert!(!app.preview_available());
}

#[test]
fn ensure_preview_loaded_clears_state_once_the_playhead_moves_past_every_clip() {
    let mut project = test_project_with_tracks(
        1,
        vec![test_track(
            1,
            TrackKind::Video,
            vec![test_clip(1, 0.0, 0.0, 10.0)],
        )],
    );
    project.media_library.push(test_asset(1));
    project.timeline_mut().playhead_secs = 3.0;
    let mut app = test_app(vec![project], Vec::new());
    app.ensure_preview_loaded();
    assert!(app.preview_clip_present());

    app.active_project_mut().timeline_mut().playhead_secs = 20.0;
    app.preview_playing = true;
    app.ensure_preview_loaded();

    assert!(!app.preview_clip_present());
    assert!(!app.preview_playing);
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
    app.active_project_mut().timeline_mut().playhead_secs = 10.0;

    app.split_at_playhead();

    let tracks = &app.active_project().timeline().tracks;
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
    app.active_project_mut().timeline_mut().playhead_secs = 10.0;

    app.split_at_playhead();

    let tracks = &app.active_project().timeline().tracks;
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
    app.active_project_mut().timeline_mut().playhead_secs = 50.0;

    app.split_at_playhead();

    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
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

    let clip = &app.active_project().timeline().tracks[0].clips[0];
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
        app.active_project().timeline().tracks[0].clips[0].start_secs,
        10.0
    );
}

#[test]
fn trim_clip_end_moves_the_right_edge_and_keeps_the_start_fixed() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 8.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.trim_clip_end(1, 5.0);

    let clip = &app.active_project().timeline().tracks[0].clips[0];
    assert_eq!(clip.start_secs, 0.0);
    assert_eq!(clip.source_out_secs, 5.0);
}

#[test]
fn trim_clip_end_is_bounded_by_the_source_assets_own_duration() {
    // test_asset's duration_secs is a fixed 10.0.
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 8.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.trim_clip_end(1, 50.0);

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].source_out_secs,
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

    let clip = &app.active_project().timeline().tracks[0].clips[0];
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
        app.active_project().timeline().tracks[0].clips[0].start_secs,
        10.0
    );
}

#[test]
fn toggle_multi_select_adds_then_removes() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.toggle_multi_select(1);
    assert!(app.multi_selected_clip_ids.contains(&1));

    app.toggle_multi_select(1);
    assert!(!app.multi_selected_clip_ids.contains(&1));
}

#[test]
fn merge_into_composite_is_a_no_op_with_fewer_than_two_selected() {
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
    app.toggle_multi_select(1);

    app.merge_into_composite();

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].composite_id,
        None
    );
    assert!(app.multi_selected_clip_ids.contains(&1)); // left untouched, not cleared.
}

#[test]
fn merge_into_composite_is_a_no_op_when_ids_span_different_tracks() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]),
                test_track(2, TrackKind::Video, vec![test_clip(2, 10.0, 0.0, 10.0)]),
            ],
        )],
        Vec::new(),
    );
    app.toggle_multi_select(1);
    app.toggle_multi_select(2);

    app.merge_into_composite();

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].composite_id,
        None
    );
    assert_eq!(
        app.active_project().timeline().tracks[1].clips[0].composite_id,
        None
    );
}

#[test]
fn merge_into_composite_assigns_a_shared_id_and_clears_the_multi_selection() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![
                    test_clip(1, 0.0, 0.0, 10.0),
                    test_clip(2, 10.0, 0.0, 10.0),
                    test_clip(3, 20.0, 0.0, 10.0),
                ],
            )],
        )],
        Vec::new(),
    );
    app.toggle_multi_select(1);
    app.toggle_multi_select(2);

    app.merge_into_composite();

    let clips = &app.active_project().timeline().tracks[0].clips;
    let group = clips[0].composite_id;
    assert!(group.is_some());
    assert_eq!(clips[1].composite_id, group);
    assert_eq!(clips[2].composite_id, None); // not selected, left standalone.
    assert!(app.multi_selected_clip_ids.is_empty());
}

#[test]
fn delete_selected_clip_removes_every_composite_member() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![
                    test_composite_clip(1, 0.0, 9),
                    test_composite_clip(2, 10.0, 9),
                    test_clip(3, 20.0, 0.0, 10.0),
                ],
            )],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.delete_selected_clip();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].id, 3);
}

#[test]
fn move_clip_with_group_moves_every_member_by_the_same_delta() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![
                    test_composite_clip(1, 0.0, 9),
                    test_composite_clip(2, 10.0, 9),
                    test_clip(3, 100.0, 0.0, 10.0),
                ],
            )],
        )],
        Vec::new(),
    );

    app.move_clip_with_group(1, 50.0); // +50s delta.

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].start_secs, 50.0);
    assert_eq!(clips[1].start_secs, 60.0);
    assert_eq!(clips[2].start_secs, 100.0); // standalone clip, untouched.
}

#[test]
fn move_clip_with_group_behaves_like_a_single_move_when_not_composite() {
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

    app.move_clip_with_group(1, 30.0);

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].start_secs,
        30.0
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

    assert!(app.active_project().timeline().tracks[0].clips.is_empty());
    let moved = &app.active_project().timeline().tracks[1].clips[0];
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

    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
    assert!(app.active_project().timeline().tracks[1].clips.is_empty());
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

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].id, 2);
    assert_eq!(app.selected_clip_id, None);
}

#[test]
fn set_selected_clip_gain_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_gain(6.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].gain_db, 0.0);
    assert_eq!(clips[1].gain_db, 6.0);
}

#[test]
fn set_selected_clip_gain_clamps_to_gain_db_range() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_gain(999.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].gain_db, *crate::app::GAIN_DB_RANGE.end());
}

#[test]
fn set_selected_clip_gain_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_gain(6.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].gain_db, 0.0);
}

#[test]
fn set_selected_clip_deflicker_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_deflicker(true);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].deflicker_enabled);
    assert!(clips[1].deflicker_enabled);
}

#[test]
fn set_selected_clip_deflicker_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_deflicker(true);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].deflicker_enabled);
}

#[test]
fn set_selected_clip_frozen_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_frozen(true);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].frozen);
    assert!(clips[1].frozen);
}

#[test]
fn set_selected_clip_frozen_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_frozen(true);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].frozen);
}

#[test]
fn set_selected_clip_speed_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_speed(2.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].speed_factor, 1.0);
    assert_eq!(clips[1].speed_factor, 2.0);
}

#[test]
fn set_selected_clip_speed_clamps_to_speed_factor_range() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_speed(999.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].speed_factor, *crate::app::SPEED_FACTOR_RANGE.end());
}

#[test]
fn set_selected_clip_speed_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_speed(2.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].speed_factor, 1.0);
}

#[test]
fn set_selected_clip_crop_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_crop(0.1, 0.2, 0.5, 0.6);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        (
            clips[0].crop_x,
            clips[0].crop_y,
            clips[0].crop_w,
            clips[0].crop_h
        ),
        (0.1, 0.2, 0.5, 0.6)
    );
}

#[test]
fn set_selected_clip_crop_clamps_each_field_independently() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_crop(-1.0, 2.0, 0.0, 999.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].crop_x, 0.0);
    assert_eq!(clips[0].crop_y, 1.0);
    assert_eq!(clips[0].crop_w, crate::app::CROP_MIN_SIZE);
    assert_eq!(clips[0].crop_h, 1.0);
}

#[test]
fn set_selected_clip_crop_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_crop(0.1, 0.2, 0.5, 0.6);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        (
            clips[0].crop_x,
            clips[0].crop_y,
            clips[0].crop_w,
            clips[0].crop_h
        ),
        (0.0, 0.0, 1.0, 1.0)
    );
}

#[test]
fn set_selected_clip_mask_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_mask(avcore::timeline::MaskShape::Circle, 0.3);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].mask_shape, avcore::timeline::MaskShape::None);
    assert_eq!(clips[1].mask_shape, avcore::timeline::MaskShape::Circle);
    assert_eq!(clips[1].mask_corner_radius, 0.3);
}

#[test]
fn set_selected_clip_mask_clamps_corner_radius_to_range() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_mask(avcore::timeline::MaskShape::RoundedRect, 999.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].mask_corner_radius,
        *crate::app::MASK_CORNER_RADIUS_RANGE.end()
    );
}

#[test]
fn set_selected_clip_mask_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_mask(avcore::timeline::MaskShape::Circle, 0.3);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].mask_shape, avcore::timeline::MaskShape::None);
}

#[test]
fn set_selected_clip_flip_h_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_flip_h(true);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].flipped_h);
    assert!(clips[1].flipped_h);
}

#[test]
fn set_selected_clip_flip_h_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_flip_h(true);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].flipped_h);
}

#[test]
fn set_selected_clip_color_filter_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_color_filter(avcore::timeline::ColorFilter::Sepia);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].color_filter, avcore::timeline::ColorFilter::None);
    assert_eq!(clips[1].color_filter, avcore::timeline::ColorFilter::Sepia);
}

#[test]
fn set_selected_clip_color_filter_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_color_filter(avcore::timeline::ColorFilter::Sepia);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].color_filter, avcore::timeline::ColorFilter::None);
}

#[test]
fn set_selected_clip_vignette_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_vignette(0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].vignette_intensity, 0.0);
    assert_eq!(clips[1].vignette_intensity, 0.5);
}

#[test]
fn set_selected_clip_vignette_clamps_to_vignette_intensity_range() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_vignette(5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].vignette_intensity,
        *crate::app::VIGNETTE_INTENSITY_RANGE.end()
    );
}

#[test]
fn set_selected_clip_vignette_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_vignette(0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].vignette_intensity, 0.0);
}

#[test]
fn set_selected_clip_color_adjust_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_color_adjust(0.2, 1.5, 0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        (clips[0].brightness, clips[0].contrast, clips[0].saturation),
        (0.0, 1.0, 1.0)
    );
    assert_eq!(
        (clips[1].brightness, clips[1].contrast, clips[1].saturation),
        (0.2, 1.5, 0.5)
    );
}

#[test]
fn set_selected_clip_color_adjust_clamps_each_field_independently() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_color_adjust(-5.0, 5.0, -5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        (clips[0].brightness, clips[0].contrast, clips[0].saturation),
        (
            *crate::app::BRIGHTNESS_RANGE.start(),
            *crate::app::CONTRAST_RANGE.end(),
            *crate::app::SATURATION_RANGE.start()
        )
    );
}

#[test]
fn set_selected_clip_color_adjust_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_color_adjust(0.2, 1.5, 0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        (clips[0].brightness, clips[0].contrast, clips[0].saturation),
        (0.0, 1.0, 1.0)
    );
}

#[test]
fn set_selected_clip_sharpen_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_sharpen(0.6);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].sharpen, 0.0);
    assert_eq!(clips[1].sharpen, 0.6);
}

#[test]
fn set_selected_clip_sharpen_clamps_to_sharpen_range() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_sharpen(5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].sharpen, *crate::app::SHARPEN_RANGE.end());
}

#[test]
fn set_selected_clip_sharpen_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_sharpen(0.6);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].sharpen, 0.0);
}

#[test]
fn set_selected_clip_transition_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_transition(avcore::timeline::TransitionType::Fade, 1.2);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].transition_in,
        avcore::timeline::TransitionType::None
    );
    assert_eq!(clips[0].transition_duration_secs, 0.5);
    assert_eq!(
        clips[1].transition_in,
        avcore::timeline::TransitionType::Fade
    );
    assert_eq!(clips[1].transition_duration_secs, 1.2);
}

#[test]
fn set_selected_clip_transition_clamps_duration_to_range() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_transition(avcore::timeline::TransitionType::Slide, 10.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].transition_duration_secs,
        *crate::app::TRANSITION_DURATION_RANGE.end()
    );
}

#[test]
fn set_selected_clip_transition_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_transition(avcore::timeline::TransitionType::Fade, 1.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].transition_in,
        avcore::timeline::TransitionType::None
    );
    assert_eq!(clips[0].transition_duration_secs, 0.5);
}

#[test]
fn set_selected_clip_scale_keyframes_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_scale_keyframes(vec![
        avcore::Keyframe {
            time_fraction: 0.0,
            value: 1.5,
        },
        avcore::Keyframe {
            time_fraction: 1.0,
            value: 2.5,
        },
    ]);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(clips[0].scale_keyframes.is_empty());
    assert_eq!(
        clips[1].scale_keyframes,
        vec![
            avcore::Keyframe {
                time_fraction: 0.0,
                value: 1.5
            },
            avcore::Keyframe {
                time_fraction: 1.0,
                value: 2.5
            },
        ]
    );
}

#[test]
fn set_selected_clip_scale_keyframes_clamps_each_value_independently() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_scale_keyframes(vec![
        avcore::Keyframe {
            time_fraction: 0.0,
            value: 0.0,
        },
        avcore::Keyframe {
            time_fraction: 1.0,
            value: 5.0,
        },
    ]);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].scale_keyframes,
        vec![
            avcore::Keyframe {
                time_fraction: 0.0,
                value: *crate::app::SCALE_RANGE.start(),
            },
            avcore::Keyframe {
                time_fraction: 1.0,
                value: *crate::app::SCALE_RANGE.end(),
            },
        ]
    );
}

#[test]
fn set_selected_clip_scale_keyframes_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_scale_keyframes(vec![
        avcore::Keyframe {
            time_fraction: 0.0,
            value: 1.5,
        },
        avcore::Keyframe {
            time_fraction: 1.0,
            value: 2.5,
        },
    ]);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(clips[0].scale_keyframes.is_empty());
}

#[test]
fn set_selected_clip_chroma_key_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_chroma_key(true, [10, 200, 30], 0.7);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].chroma_key_enabled);
    assert!(clips[1].chroma_key_enabled);
    assert_eq!(clips[1].chroma_key_color, [10, 200, 30]);
    assert_eq!(clips[1].chroma_key_tolerance, 0.7);
}

#[test]
fn set_selected_clip_chroma_key_clamps_tolerance_to_range() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_chroma_key(true, [0, 255, 0], 5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].chroma_key_tolerance,
        *crate::app::CHROMA_KEY_TOLERANCE_RANGE.end()
    );
}

#[test]
fn set_selected_clip_chroma_key_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_chroma_key(true, [10, 200, 30], 0.7);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].chroma_key_enabled);
}

#[test]
fn set_selected_clip_blur_updates_and_clamps() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_blur(5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].blur_intensity,
        *crate::app::BLUR_INTENSITY_RANGE.end()
    );
}

#[test]
fn set_selected_clip_blur_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_blur(0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].blur_intensity, 0.0);
}

#[test]
fn set_selected_clip_shake_updates_and_clamps() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_shake(5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].shake_intensity,
        *crate::app::SHAKE_INTENSITY_RANGE.end()
    );
}

#[test]
fn set_selected_clip_shake_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_shake(0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].shake_intensity, 0.0);
}

#[test]
fn set_selected_clip_glitch_updates_and_clamps() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_glitch(5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].glitch_intensity,
        *crate::app::GLITCH_INTENSITY_RANGE.end()
    );
}

#[test]
fn set_selected_clip_glitch_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_glitch(0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].glitch_intensity, 0.0);
}

#[test]
fn set_selected_clip_pixelize_updates_and_clamps() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_pixelize(5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].pixelize_intensity,
        *crate::app::PIXELIZE_INTENSITY_RANGE.end()
    );
}

#[test]
fn set_selected_clip_pixelize_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_pixelize(0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].pixelize_intensity, 0.0);
}

#[test]
fn copy_selected_clip_formatting_is_a_no_op_when_nothing_is_selected() {
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

    app.copy_selected_clip_formatting();

    assert!(!app.has_formatting_clipboard());
}

#[test]
fn paste_selected_clip_formatting_applies_gain_and_frozen_without_touching_position() {
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
    app.set_selected_clip_gain(6.0);
    app.set_selected_clip_frozen(true);
    app.set_selected_clip_speed(2.0);
    app.set_selected_clip_crop(0.1, 0.2, 0.5, 0.6);
    app.set_selected_clip_mask(avcore::timeline::MaskShape::Circle, 0.3);
    app.set_selected_clip_flip_h(true);
    app.set_selected_clip_color_filter(avcore::timeline::ColorFilter::Sepia);
    app.set_selected_clip_vignette(0.5);
    app.set_selected_clip_color_adjust(0.2, 1.5, 0.5);
    app.set_selected_clip_sharpen(0.6);
    app.set_selected_clip_chroma_key(true, [10, 200, 30], 0.7);
    app.set_selected_clip_blur(0.1);
    app.set_selected_clip_shake(0.2);
    app.set_selected_clip_glitch(0.3);
    app.set_selected_clip_pixelize(0.4);
    app.set_selected_clip_transition(avcore::timeline::TransitionType::Fade, 1.2);
    app.set_selected_clip_scale_keyframes(vec![
        avcore::Keyframe {
            time_fraction: 0.0,
            value: 1.5,
        },
        avcore::Keyframe {
            time_fraction: 1.0,
            value: 2.5,
        },
    ]);
    app.copy_selected_clip_formatting();

    app.selected_clip_id = Some(2);
    app.paste_selected_clip_formatting();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[1].gain_db, 6.0);
    assert!(clips[1].frozen);
    assert_eq!(clips[1].speed_factor, 2.0);
    assert_eq!(
        (
            clips[1].crop_x,
            clips[1].crop_y,
            clips[1].crop_w,
            clips[1].crop_h
        ),
        (0.1, 0.2, 0.5, 0.6)
    );
    assert_eq!(clips[1].mask_shape, avcore::timeline::MaskShape::Circle);
    assert_eq!(clips[1].mask_corner_radius, 0.3);
    assert!(clips[1].flipped_h);
    assert_eq!(clips[1].color_filter, avcore::timeline::ColorFilter::Sepia);
    assert_eq!(clips[1].vignette_intensity, 0.5);
    assert_eq!(
        (clips[1].brightness, clips[1].contrast, clips[1].saturation),
        (0.2, 1.5, 0.5)
    );
    assert_eq!(clips[1].sharpen, 0.6);
    assert!(clips[1].chroma_key_enabled);
    assert_eq!(clips[1].chroma_key_color, [10, 200, 30]);
    assert_eq!(clips[1].chroma_key_tolerance, 0.7);
    assert_eq!(
        (
            clips[1].blur_intensity,
            clips[1].shake_intensity,
            clips[1].glitch_intensity,
            clips[1].pixelize_intensity
        ),
        (0.1, 0.2, 0.3, 0.4)
    );
    assert_eq!(
        clips[1].transition_in,
        avcore::timeline::TransitionType::Fade
    );
    assert_eq!(clips[1].transition_duration_secs, 1.2);
    assert_eq!(
        clips[1].scale_keyframes,
        vec![
            avcore::Keyframe {
                time_fraction: 0.0,
                value: 1.5
            },
            avcore::Keyframe {
                time_fraction: 1.0,
                value: 2.5
            },
        ]
    );
    assert_eq!(clips[1].start_secs, 10.0); // position untouched
    assert_eq!(clips[1].source_out_secs, 20.0); // trim untouched
}

#[test]
fn paste_selected_clip_formatting_is_a_no_op_with_an_empty_clipboard() {
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
    app.selected_clip_id = Some(1);

    app.paste_selected_clip_formatting();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].gain_db, 0.0);
    assert!(!clips[0].frozen);
}

#[test]
fn selected_clip_track_kind_reports_the_track_the_clip_lives_on() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]),
                test_track(2, TrackKind::Audio, vec![test_clip(2, 0.0, 0.0, 10.0)]),
            ],
        )],
        Vec::new(),
    );

    app.selected_clip_id = Some(1);
    assert_eq!(app.selected_clip_track_kind(), Some(TrackKind::Video));

    app.selected_clip_id = Some(2);
    assert_eq!(app.selected_clip_track_kind(), Some(TrackKind::Audio));

    app.selected_clip_id = None;
    assert_eq!(app.selected_clip_track_kind(), None);
}

#[test]
fn select_timeline_clip_synchronizes_its_backing_asset() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(7, 0.0, 0.0, 10.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.select_timeline_clip(7);

    assert_eq!(app.selected_clip_id, Some(7));
    assert_eq!(app.selected_asset_id, Some(1));
}

#[test]
fn copy_selected_clip_is_a_no_op_when_nothing_is_selected() {
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

    app.copy_selected_clip();

    assert!(!app.has_clipboard_clip());
}

#[test]
fn paste_clip_at_playhead_is_a_no_op_with_an_empty_clipboard() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(1, TrackKind::Video, vec![])],
        )],
        Vec::new(),
    );

    app.paste_clip_at_playhead();

    assert!(app.active_project().timeline().tracks[0].clips.is_empty());
}

#[test]
fn paste_clip_at_playhead_appends_a_fresh_clip_with_the_copied_trim_range() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(5, 0.0, 2.0, 12.0)],
            )],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(5);
    app.set_selected_clip_gain(4.0);
    app.set_selected_clip_frozen(true);
    app.set_selected_clip_speed(2.0);
    app.set_selected_clip_crop(0.1, 0.2, 0.5, 0.6);
    app.set_selected_clip_mask(avcore::timeline::MaskShape::RoundedRect, 0.4);
    app.set_selected_clip_flip_h(true);
    app.set_selected_clip_color_filter(avcore::timeline::ColorFilter::BlackAndWhite);
    app.set_selected_clip_vignette(0.7);
    app.set_selected_clip_color_adjust(0.3, 1.2, 0.8);
    app.set_selected_clip_sharpen(0.9);
    app.set_selected_clip_chroma_key(true, [5, 180, 5], 0.55);
    app.set_selected_clip_blur(0.1);
    app.set_selected_clip_shake(0.2);
    app.set_selected_clip_glitch(0.3);
    app.set_selected_clip_pixelize(0.4);
    app.set_selected_clip_transition(avcore::timeline::TransitionType::Slide, 0.9);
    app.set_selected_clip_scale_keyframes(vec![
        avcore::Keyframe {
            time_fraction: 0.0,
            value: 1.5,
        },
        avcore::Keyframe {
            time_fraction: 1.0,
            value: 2.5,
        },
    ]);
    app.copy_selected_clip();
    app.active_project_mut().timeline_mut().playhead_secs = 30.0;

    app.paste_clip_at_playhead();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips.len(), 2);
    let pasted = &clips[1];
    assert_ne!(pasted.id, 5); // fresh id, not a duplicate of the copied clip's.
    assert_eq!(pasted.start_secs, 30.0);
    assert_eq!(pasted.source_in_secs, 2.0);
    assert_eq!(pasted.source_out_secs, 12.0);
    assert_eq!(pasted.gain_db, 4.0); // gain travels with the clip through copy/paste.
    assert!(pasted.frozen); // so does frozen.
    assert_eq!(pasted.speed_factor, 2.0); // so does speed.
    assert_eq!(
        (pasted.crop_x, pasted.crop_y, pasted.crop_w, pasted.crop_h),
        (0.1, 0.2, 0.5, 0.6)
    ); // so does crop.
    assert_eq!(pasted.mask_shape, avcore::timeline::MaskShape::RoundedRect); // so does mask.
    assert_eq!(pasted.mask_corner_radius, 0.4);
    assert!(pasted.flipped_h); // so does flip.
    assert_eq!(
        pasted.color_filter,
        avcore::timeline::ColorFilter::BlackAndWhite
    ); // so does the color filter.
    assert_eq!(pasted.vignette_intensity, 0.7); // so does vignette.
    assert_eq!(
        (pasted.brightness, pasted.contrast, pasted.saturation),
        (0.3, 1.2, 0.8)
    ); // so does the color adjustment.
    assert_eq!(pasted.sharpen, 0.9); // so does sharpen.
    assert!(pasted.chroma_key_enabled); // so does chroma key.
    assert_eq!(pasted.chroma_key_color, [5, 180, 5]);
    assert_eq!(pasted.chroma_key_tolerance, 0.55);
    assert_eq!(
        (
            pasted.blur_intensity,
            pasted.shake_intensity,
            pasted.glitch_intensity,
            pasted.pixelize_intensity
        ),
        (0.1, 0.2, 0.3, 0.4)
    ); // so do blur/shake/glitch/pixelize.
    assert_eq!(
        pasted.transition_in,
        avcore::timeline::TransitionType::Slide
    ); // so does the transition.
    assert_eq!(pasted.transition_duration_secs, 0.9);
    assert_eq!(
        pasted.scale_keyframes,
        vec![
            avcore::Keyframe {
                time_fraction: 0.0,
                value: 1.5
            },
            avcore::Keyframe {
                time_fraction: 1.0,
                value: 2.5
            },
        ]
    ); // so do scale keyframes.
}

#[test]
fn paste_clip_at_playhead_survives_a_sequence_switch() {
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
    app.selected_clip_id = Some(1);
    app.copy_selected_clip();

    app.add_sequence(); // switches to a fresh, empty tab.
    app.paste_clip_at_playhead();

    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
}

#[test]
fn cut_selected_clip_copies_then_removes_the_clip() {
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
    app.selected_clip_id = Some(1);

    app.cut_selected_clip();

    assert!(app.active_project().timeline().tracks[0].clips.is_empty());
    assert!(app.has_clipboard_clip());
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

    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
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
            import_token: 0,
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
            import_token: 0,
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
fn pump_import_queue_applies_enrichment_to_the_asset_it_was_assigned() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(5)])], Vec::new());
    let mut ready_asset = test_asset(0);
    ready_asset.file_name = "clip.mp4".to_string();
    app.import_tx
        .send(ImportEvent::AssetReady {
            project_id: 1,
            import_token: 7,
            asset: ready_asset,
        })
        .unwrap();
    app.pump_import_queue();
    let loudness = LoudnessMetrics {
        integrated_lufs: -14.0,
        true_peak_dbtp: -1.0,
        loudness_range_lu: 6.0,
    };

    app.import_tx
        .send(ImportEvent::Enriched {
            project_id: 1,
            import_token: 7,
            loudness: Some(loudness),
            proxy_path: Some(PathBuf::from("proxy.mp4")),
            waveform_peaks: Some(vec![(-0.5, 0.5)]),
        })
        .unwrap();
    app.pump_import_queue();

    let imported = app
        .active_project()
        .media_library
        .iter()
        .find(|a| a.file_name == "clip.mp4")
        .unwrap();
    assert_eq!(imported.loudness, Some(loudness));
    assert_eq!(imported.proxy_path, Some(PathBuf::from("proxy.mp4")));
    assert_eq!(imported.waveform_peaks, Some(vec![(-0.5, 0.5)]));
}

#[test]
fn pump_import_queue_ignores_enrichment_for_an_unknown_token() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(5)])], Vec::new());
    app.import_tx
        .send(ImportEvent::Enriched {
            project_id: 1,
            import_token: 999,
            loudness: None,
            proxy_path: None,
            waveform_peaks: None,
        })
        .unwrap();

    app.pump_import_queue();

    assert_eq!(app.active_project().media_library.len(), 1);
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

#[test]
fn next_available_path_picks_the_first_free_numeric_suffix() {
    let dir = std::env::temp_dir().join("oca_test_next_available_path");
    let _ = std::fs::create_dir_all(&dir);
    let base = dir.join("clip.mp4");
    let taken2 = dir.join("clip (2).mp4");
    std::fs::write(&base, b"").unwrap();
    std::fs::write(&taken2, b"").unwrap();

    let result = super::export::next_available_path(&base);

    assert_eq!(result, dir.join("clip (3).mp4"));

    let _ = std::fs::remove_file(&base);
    let _ = std::fs::remove_file(&taken2);
}

#[test]
fn next_available_path_uses_suffix_two_when_only_the_base_name_exists() {
    let dir = std::env::temp_dir().join("oca_test_next_available_path_2");
    let _ = std::fs::create_dir_all(&dir);
    let base = dir.join("clip.mp4");
    std::fs::write(&base, b"").unwrap();

    let result = super::export::next_available_path(&base);

    assert_eq!(result, dir.join("clip (2).mp4"));

    let _ = std::fs::remove_file(&base);
}

#[test]
fn pending_export_conflict_overwrite_queues_with_the_original_path() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let dir = std::env::temp_dir().join("oca_test_export_conflict_overwrite");
    let _ = std::fs::create_dir_all(&dir);
    let output = dir.join("out.mp4");
    std::fs::write(&output, b"").unwrap();

    app.pending_export_conflict = Some(super::export::PendingExportConflict {
        title: "Export".to_string(),
        track_segments: Vec::new(),
        text_segments: vec![],
        canvas: test_canvas(),
        target_lufs: -14.0,
        output_path: output.clone(),
    });

    // Simulates the modal's Overwrite branch directly — queues with output_path unchanged.
    let pending = app.pending_export_conflict.take().unwrap();
    app.queue_export(
        pending.title,
        pending.track_segments,
        pending.text_segments,
        pending.canvas,
        pending.target_lufs,
        pending.output_path.display().to_string(),
    );

    assert_eq!(app.export_jobs.len(), 1);
    assert_eq!(app.export_jobs[0].output_path, output.display().to_string());
    assert!(app.pending_export_conflict.is_none());

    let _ = std::fs::remove_file(&output);
}

#[test]
fn pump_transcribe_creates_a_text_track_with_rebased_word_timings() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.active_project_mut().timeline_mut().playhead_secs = 5.0;

    let segments = vec![avcore::transcribe::TranscribeSegment {
        start_secs: 100.0,
        end_secs: 101.0,
        text: "Hello World".to_string(),
        words: vec![
            avcore::transcribe::TranscribeWord {
                text: "Hello".to_string(),
                start_secs: 100.0,
                end_secs: 100.5,
            },
            avcore::transcribe::TranscribeWord {
                text: "World".to_string(),
                start_secs: 100.5,
                end_secs: 101.0,
            },
        ],
    }];
    app.transcribe_tx
        .send(TranscribeEvent::Done {
            asset_id: 1,
            segments,
        })
        .unwrap();

    app.pump_transcribe();

    let text_track = app
        .active_project()
        .timeline()
        .tracks
        .iter()
        .find(|t| t.kind == TrackKind::Text)
        .expect("no text track created");
    assert_eq!(text_track.text_clips.len(), 1);
    let tc = &text_track.text_clips[0];
    // Anchored at the playhead (5.0) plus the segment's own start_secs (100.0) - apply_
    // transcription preserves each segment's offset from the playhead rather than resetting
    // every segment to start exactly at it (see its doc comment).
    assert_eq!(tc.start_secs, 105.0);
    assert!(tc.highlight_enabled);
    assert_eq!(tc.words.len(), 2);
    // Rebased relative to the clip's own start, not the segment's absolute 100.0.
    assert_eq!(tc.words[0].start_secs, 0.0);
    assert_eq!(tc.words[0].end_secs, 0.5);
    assert_eq!(tc.words[1].start_secs, 0.5);
    assert_eq!(tc.words[1].end_secs, 1.0);
}

#[test]
fn begin_save_layer_template_is_a_no_op_when_nothing_is_multi_selected() {
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

    app.begin_save_layer_template();

    assert!(app.saving_layer_template.is_none());
}

#[test]
fn begin_save_layer_template_snapshots_the_multi_selection_ordered_by_track_then_start() {
    let mut c1 = test_clip(1, 5.0, 0.0, 10.0);
    c1.gain_db = 3.0;
    let mut c2 = test_clip(2, 0.0, 0.0, 10.0);
    c2.gain_db = -3.0;
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![c1, c2]),
                test_track(2, TrackKind::Audio, vec![test_clip(3, 0.0, 0.0, 5.0)]),
            ],
        )],
        Vec::new(),
    );
    app.multi_selected_clip_ids.insert(1);
    app.multi_selected_clip_ids.insert(2);

    app.begin_save_layer_template();

    let (layers, name) = app.saving_layer_template.expect("should be staged");
    assert_eq!(name, "");
    assert_eq!(layers.len(), 2);
    // clip 2 (start_secs 0.0) sorts before clip 1 (start_secs 5.0) on the same track.
    assert_eq!(layers[0].0, TrackKind::Video);
    assert_eq!(layers[0].1.gain_db, -3.0);
    assert_eq!(layers[1].1.gain_db, 3.0);
}

#[test]
fn commit_save_layer_template_is_a_no_op_when_the_name_is_blank() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.saving_layer_template = Some((
        vec![(TrackKind::Video, test_clip(1, 0.0, 0.0, 1.0).formatting())],
        "   ".to_string(),
    ));

    app.commit_save_layer_template();

    assert!(app.saving_layer_template.is_some());
    assert!(app.prefs.saved_layer_templates.is_empty());
}

#[test]
fn commit_save_layer_template_pushes_a_named_template_and_clears_the_pending_state() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let layers = vec![(TrackKind::Video, test_clip(1, 0.0, 0.0, 1.0).formatting())];
    app.saving_layer_template = Some((layers, "Webcam corner".to_string()));

    app.commit_save_layer_template();

    assert!(app.saving_layer_template.is_none());
    assert_eq!(app.prefs.saved_layer_templates.len(), 1);
    assert_eq!(app.prefs.saved_layer_templates[0].name, "Webcam corner");
}

#[test]
fn delete_layer_template_removes_the_entry_at_index() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.saved_layer_templates = vec![
        avcore::LayerTemplate {
            name: "A".to_string(),
            layers: vec![],
        },
        avcore::LayerTemplate {
            name: "B".to_string(),
            layers: vec![],
        },
    ];

    app.delete_layer_template(0);

    assert_eq!(app.prefs.saved_layer_templates.len(), 1);
    assert_eq!(app.prefs.saved_layer_templates[0].name, "B");
}

#[test]
fn delete_layer_template_is_a_no_op_for_an_out_of_range_index() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.saved_layer_templates = vec![avcore::LayerTemplate {
        name: "A".to_string(),
        layers: vec![],
    }];

    app.delete_layer_template(5);

    assert_eq!(app.prefs.saved_layer_templates.len(), 1);
}

#[test]
fn begin_apply_layer_template_stages_one_empty_slot_per_layer() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.saved_layer_templates = vec![avcore::LayerTemplate {
        name: "Group".to_string(),
        layers: vec![
            (TrackKind::Video, test_clip(1, 0.0, 0.0, 1.0).formatting()),
            (TrackKind::Audio, test_clip(2, 0.0, 0.0, 1.0).formatting()),
        ],
    }];
    app.layer_templates_menu_open = true;

    app.begin_apply_layer_template(0);

    let (index, slots) = app.applying_layer_template.expect("should be staged");
    assert_eq!(index, 0);
    assert_eq!(slots, vec![None, None]);
    assert!(!app.layer_templates_menu_open);
}

#[test]
fn confirm_apply_layer_template_creates_one_clip_per_filled_layer_with_its_formatting() {
    let mut styled = test_clip(99, 0.0, 0.0, 1.0);
    styled.gain_db = 6.0;
    styled.flipped_h = true;
    let mut project = test_project(1, vec![test_asset(1), test_asset(2)]);
    project.timeline_mut().tracks = vec![];
    project.timeline_mut().playhead_secs = 2.5;
    let mut app = test_app(vec![project], Vec::new());
    app.prefs.saved_layer_templates = vec![avcore::LayerTemplate {
        name: "Group".to_string(),
        layers: vec![(TrackKind::Video, styled.formatting())],
    }];
    app.applying_layer_template = Some((0, vec![Some(1)]));

    app.confirm_apply_layer_template();

    assert!(app.applying_layer_template.is_none());
    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks.len(), 1);
    let track = &timeline.tracks[0];
    assert_eq!(track.kind, TrackKind::Video);
    assert_eq!(track.clips.len(), 1);
    let clip = &track.clips[0];
    assert_eq!(clip.asset_id, 1);
    assert_eq!(clip.start_secs, 2.5);
    assert_eq!(clip.gain_db, 6.0);
    assert!(clip.flipped_h);
}

#[test]
fn confirm_apply_layer_template_puts_each_layer_on_its_own_new_track() {
    let mut project = test_project(1, vec![test_asset(1), test_asset(1)]);
    project.timeline_mut().tracks = vec![];
    let mut app = test_app(vec![project], Vec::new());
    app.prefs.saved_layer_templates = vec![avcore::LayerTemplate {
        name: "Two video layers".to_string(),
        layers: vec![
            (TrackKind::Video, test_clip(1, 0.0, 0.0, 1.0).formatting()),
            (TrackKind::Video, test_clip(2, 0.0, 0.0, 1.0).formatting()),
        ],
    }];
    app.applying_layer_template = Some((0, vec![Some(1), Some(1)]));

    app.confirm_apply_layer_template();

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks.len(), 2);
    assert_eq!(timeline.tracks[0].clips.len(), 1);
    assert_eq!(timeline.tracks[1].clips.len(), 1);
}

#[test]
fn confirm_apply_layer_template_skips_layers_left_without_an_asset() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![];
    let mut app = test_app(vec![project], Vec::new());
    app.prefs.saved_layer_templates = vec![avcore::LayerTemplate {
        name: "Group".to_string(),
        layers: vec![
            (TrackKind::Video, test_clip(1, 0.0, 0.0, 1.0).formatting()),
            (TrackKind::Video, test_clip(2, 0.0, 0.0, 1.0).formatting()),
        ],
    }];
    app.applying_layer_template = Some((0, vec![Some(1), None]));

    app.confirm_apply_layer_template();

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].clips.len(), 1);
}
