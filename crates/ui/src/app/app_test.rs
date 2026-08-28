// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use super::*;
use avcore::timeline::{AudioRole, ClipInstance, Track, TrackKind};
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

fn test_project_with_tracks(id: u64, tracks: Vec<Track>) -> Project {
    let mut project = test_project(id, Vec::new());
    project.timeline_mut().tracks = tracks;
    project
}

fn test_project_with_tracks_and_assets(
    id: u64,
    tracks: Vec<Track>,
    assets: Vec<MediaAsset>,
) -> Project {
    let mut project = test_project(id, assets);
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
        shape_clips: vec![],

        visible: true,
        audio_role: AudioRole::Unspecified,
        color_label: None,
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
        color_label: None,
        gain_db: 0.0,
        frozen: false,
        speed_factor: 1.0,
        speed_ramp_end_factor: None,
        nested_sequence_id: None,
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
        gain_keyframes: vec![],
        brightness_keyframes: vec![],
        contrast_keyframes: vec![],
        saturation_keyframes: vec![],
        crop_x_keyframes: vec![],
        crop_y_keyframes: vec![],
        crop_w_keyframes: vec![],
        crop_h_keyframes: vec![],
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
        color_label: None,
        ..test_clip(id, start_secs, 0.0, 10.0)
    }
}

fn test_asset(id: u64) -> MediaAsset {
    MediaAsset {
        id,
        file_name: format!("asset-{id}.mp4"),
        source_path: PathBuf::from(format!("asset-{id}.mp4")),
        kind: MediaKind::Video,
        has_audio: true,
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
        shape_segments: vec![],

        track_segments: Vec::new(),
        audio_segments: Vec::new(),

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
    let (scene_cut_detection_tx, scene_cut_detection_rx) = mpsc::unbounded_channel();
    let (matte_generation_tx, matte_generation_rx) = mpsc::unbounded_channel();
    let (tts_tx, tts_rx) = mpsc::unbounded_channel();
    let (youtube_download_tx, youtube_download_rx) = mpsc::unbounded_channel();
    let (sound_library_tx, sound_library_rx) = mpsc::unbounded_channel();
    let (telemetry_tx, _telemetry_rx) = mpsc::unbounded_channel();
    let (update_check_tx, update_check_rx) = mpsc::unbounded_channel();
    let (watch_folder_tx, watch_folder_rx) = mpsc::unbounded_channel();
    App {
        screen: Screen::Home,
        tool: EditorTool::Select,
        locale: Locale::PtBr,
        projects,
        active_project: 0,
        selected_asset_id: None,
        export_jobs,
        prefs: PrefsState::default(),
        error_reporter: None,
        telemetry_state: TelemetryState {
            telemetry_tx,
            telemetry_enabled_flag: Arc::new(AtomicBool::new(true)),
            last_preview_frame_telemetry: None,
        },
        render_tx,
        render_rx,
        active_renders: HashMap::new(),
        export_preview_cache: None,
        nested_sequence_render_cache: std::collections::HashMap::new(),
        preview_state: PreviewState::default(),
        import_state: ImportState {
            import_tx,
            import_rx,
            pending_imports: 0,
            next_import_token: 0,
            pending_enrichment: HashMap::new(),
            auto_add_to_timeline: HashSet::new(),
        },
        sound_library_tracks: Vec::new(),
        sound_library_tx,
        sound_library_rx,
        transcribe_state: TranscribeState {
            transcribe_tx,
            transcribe_rx,
            transcribing_asset_id: None,
        },
        auto_reframe_state: AutoReframeState {
            auto_reframe_tx,
            auto_reframe_rx,
            auto_reframing_clip_id: None,
        },
        motion_tracking_state: MotionTrackingState {
            motion_tracking_tx,
            motion_tracking_rx,
            motion_tracking_clip_id: None,
        },
        scene_cut_detection_state: SceneCutDetectionState {
            scene_cut_detection_tx,
            scene_cut_detection_rx,
            scene_cut_detection_clip_id: None,
        },
        motion_track_region: MotionTrackRegionState {
            motion_track_center_x: 0.5,
            motion_track_center_y: 0.5,
            motion_track_width: 0.2,
            motion_track_height: 0.2,
            motion_track_search_radius: 0.08,
            picking_motion_track_region: false,
        },
        matte_generation_state: MatteGenerationState {
            matte_generation_tx,
            matte_generation_rx,
            matte_generating_clip_id: None,
        },
        tts_state: TtsState {
            tts_tx,
            tts_rx,
            tts_modal_text: None,
            tts_generating: false,
        },
        youtube_download_state: YoutubeDownloadState {
            youtube_download_tx,
            youtube_download_rx,
            youtube_modal_url: None,
            youtube_modal_format: crate::app::YoutubeFormatChoice::Mp4,
            youtube_modal_mp4_quality: avcore::Mp4Quality::P720,
            youtube_modal_mp3_bitrate: avcore::Mp3Bitrate::K192,
            youtube_downloading: false,
            youtube_download_progress: 0.0,
            youtube_download_error: None,
            youtube_download_cancel: None,
        },
        watch_folder_state: crate::app::WatchFolderState {
            tx: watch_folder_tx,
            rx: watch_folder_rx,
            watch_path: None,
            running: false,
            stop: None,
            files: Vec::new(),
        },
        selected_clip_id: None,
        undo_stack: avcore::undo::UndoStack::new(),
        undo_drag_active: false,
        selected_text_clip_id: None,
        text_color_edit: None,
        selected_shape_clip_id: None,
        drawing_shape_points: None,
        timeline_px_per_sec: 4.0,
        lib_panel_width: 220.0,
        props_panel_width: 240.0,
        timeline_height: 190.0,
        thumbnail_state: ThumbnailState {
            thumbnail_tx,
            thumbnail_rx,
            thumbnail_textures: HashMap::new(),
            pending_thumbnails: HashSet::new(),
            thumbnail_last_used: HashMap::new(),
            failed_thumbnails: HashMap::new(),
            thumbnail_usage_clock: 0,
        },
        pending_asset_drop: None,
        clipboard_clip: None,
        formatting_clipboard: None,
        multi_selected_clip_ids: HashSet::new(),
        active_multicam_group_id: None,
        active_smart_bin_id: None,
        editing_smart_bin: None,
        toasts: Vec::new(),
        prefs_open: false,
        prev_prefs_open: false,
        about_open: false,
        project_dirty: false,
        last_edit_instant: None,
        last_autosave_instant: None,
        autosave_restore_pending: None,
        crash_detected: false,
        renaming_project: None::<(usize, String, String)>,
        renaming_sequence: None,
        deleting_sequence: None,
        speed_ramp_dialog: None,
        saving_layer_template: None,
        applying_layer_template: None,
        layer_templates_menu_open: false,
        timeline_index_open: false,
        marker_search: String::new(),
        transcript_panel_open: false,
        transcript_search: String::new(),
        transcript_panel_state: TranscriptPanelState::default(),
        silence_review: None,
        transcript_review: None,
        transcript_search_project: false,
        binding_capture: None,
        update_check_tx,
        update_check_rx,
        update_check_status: UpdateCheckStatus::Checking,
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
fn create_new_project_uses_the_preferred_loudness_as_its_sequence_default() {
    let mut app = test_app(Vec::new(), Vec::new());
    app.prefs.lufs_profile = 2;

    app.create_new_project("Broadcast".to_string());

    assert_eq!(
        app.active_sequence_export_settings().aspect_ratio,
        avcore::ExportAspectRatio::Original
    );
    assert_eq!(app.active_sequence_export_settings().target_lufs, -23.0);
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
fn duplicate_sequence_copies_the_tab_switches_to_it_and_clears_sequence_state() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.active_project_mut().timeline_mut().playhead_secs = 17.0;
    app.selected_clip_id = Some(1);
    app.selected_text_clip_id = Some(2);
    app.selected_shape_clip_id = Some(3);
    app.multi_selected_clip_ids.extend([1, 4]);
    app.preview_state.preview_clip_id = Some(1);
    app.preview_state.preview_overlay_clip_ids.push(2);
    app.preview_state.preview_playing = true;

    app.duplicate_sequence(0);

    assert_eq!(app.active_project().sequences.len(), 2);
    assert_eq!(app.active_project().active_sequence, 1);
    assert_eq!(app.active_project().sequences[1].id, 2);
    assert_eq!(
        app.active_project().sequences[1].name,
        "Cópia de Sequence 1"
    );
    assert_eq!(app.active_project().timeline().playhead_secs, 17.0);
    assert_eq!(app.selected_clip_id, None);
    assert_eq!(app.selected_text_clip_id, None);
    assert_eq!(app.selected_shape_clip_id, None);
    assert!(app.multi_selected_clip_ids.is_empty());
    assert_eq!(app.preview_state.preview_clip_id, None);
    assert!(app.preview_state.preview_overlay_clip_ids.is_empty());
    assert!(!app.preview_state.preview_playing);
}

#[test]
fn delete_sequence_removes_the_target_and_selects_a_surviving_neighbor() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_sequence();
    let second_id = app.active_project().sequences[1].id;
    app.selected_clip_id = Some(9);
    app.selected_text_clip_id = Some(8);
    app.selected_shape_clip_id = Some(7);

    app.delete_sequence(second_id);

    assert_eq!(app.active_project().sequences.len(), 1);
    assert_eq!(app.active_project().active_sequence, 0);
    assert_eq!(app.active_project().sequences[0].name, "Sequence 1");
    assert_eq!(app.selected_clip_id, None);
    assert_eq!(app.selected_text_clip_id, None);
    assert_eq!(app.selected_shape_clip_id, None);
}

#[test]
fn delete_sequence_refuses_to_remove_the_last_tab() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let only_id = app.active_project().sequences[0].id;

    app.delete_sequence(only_id);

    assert_eq!(app.active_project().sequences.len(), 1);
    assert_eq!(app.active_project().active_sequence, 0);
}

#[test]
fn deleting_an_inactive_sequence_preserves_the_active_sequence_state() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_sequence();
    let inactive_id = app.active_project().sequences[0].id;
    let active_id = app.active_project().sequences[1].id;
    app.selected_clip_id = Some(42);
    app.preview_state.preview_clip_id = Some(42);

    app.delete_sequence(inactive_id);

    assert_eq!(app.active_project().sequences.len(), 1);
    assert_eq!(app.active_project().sequences[0].id, active_id);
    assert_eq!(app.active_project().active_sequence, 0);
    assert_eq!(app.selected_clip_id, Some(42));
    assert_eq!(app.preview_state.preview_clip_id, Some(42));
}

#[test]
fn move_sequence_reorders_tabs_while_preserving_active_identity_and_selection() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_sequence();
    app.add_sequence();
    app.select_sequence(0);
    let active_id = app.active_project().sequences[0].id;
    app.selected_clip_id = Some(42);

    app.move_sequence(0, 2);

    let names: Vec<&str> = app
        .active_project()
        .sequences
        .iter()
        .map(|sequence| sequence.name.as_str())
        .collect();
    assert_eq!(names, vec!["Sequência 2", "Sequência 3", "Sequence 1"]);
    assert_eq!(app.active_project().active_sequence, 2);
    assert_eq!(
        app.active_project().sequences[app.active_project().active_sequence].id,
        active_id
    );
    assert_eq!(app.selected_clip_id, Some(42));
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
fn export_settings_follow_the_active_sequence_without_leaking_between_tabs() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.set_active_sequence_export_aspect_ratio(avcore::ExportAspectRatio::Landscape);
    app.set_active_sequence_target_lufs(-16.0);
    app.add_sequence();
    assert_eq!(
        app.active_sequence_export_settings().aspect_ratio,
        avcore::ExportAspectRatio::Landscape
    );
    assert_eq!(app.active_sequence_export_settings().target_lufs, -16.0);

    app.set_active_sequence_export_aspect_ratio(avcore::ExportAspectRatio::Portrait);
    app.set_active_sequence_target_lufs(-23.0);
    app.select_sequence(0);

    assert_eq!(
        app.active_sequence_export_settings().aspect_ratio,
        avcore::ExportAspectRatio::Landscape
    );
    assert_eq!(app.active_sequence_export_settings().target_lufs, -16.0);
    assert_eq!(
        app.active_project().sequences[1]
            .export_settings
            .aspect_ratio,
        avcore::ExportAspectRatio::Portrait
    );
    assert_eq!(
        app.active_project().sequences[1]
            .export_settings
            .target_lufs,
        -23.0
    );
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
fn request_thumbnail_marks_the_frame_pending_and_does_not_duplicate_on_a_second_call() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.request_thumbnail(1, 0);
    assert!(app.thumbnail_state.pending_thumbnails.contains(&(1, 0)));

    app.request_thumbnail(1, 0);
    assert_eq!(app.thumbnail_state.pending_thumbnails.len(), 1);
}

#[test]
fn request_thumbnail_remembers_an_unknown_asset_as_a_bounded_failure() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.request_thumbnail(99, 0);

    assert!(app.thumbnail_state.failed_thumbnails.contains_key(&(99, 0)));
    assert!(app.thumbnail_state.pending_thumbnails.is_empty());
}

#[test]
fn thumbnail_frame_keys_follow_source_fps_and_have_a_safe_fallback() {
    assert_eq!(thumbnail_frame_index(1.5, Some(60.0)), 90);
    assert_eq!(thumbnail_frame_index(1.5, None), 45);
    assert_eq!(thumbnail_frame_index(-2.0, Some(30.0)), 0);
    assert_eq!(thumbnail_frame_time(90, Some(60.0)), 1.5);
}

#[test]
fn thumbnail_requests_cap_concurrent_extractions() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    for frame_index in 0..(THUMBNAIL_MAX_PENDING as i64 + 10) {
        app.request_thumbnail(1, frame_index);
    }

    assert_eq!(
        app.thumbnail_state.pending_thumbnails.len(),
        THUMBNAIL_MAX_PENDING
    );
}

#[test]
fn failed_thumbnail_suppression_is_bounded() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    for frame_index in 0..(THUMBNAIL_FAILURE_CAPACITY as i64 + 1) {
        app.request_thumbnail(99, frame_index);
    }

    assert_eq!(
        app.thumbnail_state.failed_thumbnails.len(),
        THUMBNAIL_FAILURE_CAPACITY
    );
    assert!(!app.thumbnail_state.failed_thumbnails.contains_key(&(99, 0)));
    assert!(app
        .thumbnail_state
        .failed_thumbnails
        .contains_key(&(99, THUMBNAIL_FAILURE_CAPACITY as i64)));
}

#[test]
fn thumbnail_texture_cache_evicts_the_least_recently_used_entry() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let ctx = egui::Context::default();
    for frame_index in 0..THUMBNAIL_CACHE_CAPACITY as i64 {
        app.thumbnail_state
            .thumbnail_tx
            .send(ThumbnailReady::Ready {
                asset_id: 1,
                frame_index,
                width: 1,
                height: 1,
                rgba: vec![0, 0, 0, 255],
            })
            .unwrap();
    }
    app.pump_thumbnail_queue(&ctx);
    app.touch_thumbnails(&[(1, 0)]);

    app.thumbnail_state
        .thumbnail_tx
        .send(ThumbnailReady::Ready {
            asset_id: 1,
            frame_index: THUMBNAIL_CACHE_CAPACITY as i64,
            width: 1,
            height: 1,
            rgba: vec![255, 255, 255, 255],
        })
        .unwrap();
    app.pump_thumbnail_queue(&ctx);

    assert_eq!(
        app.thumbnail_state.thumbnail_textures.len(),
        THUMBNAIL_CACHE_CAPACITY
    );
    assert!(app.thumbnail_state.thumbnail_textures.contains_key(&(1, 0)));
    assert!(!app.thumbnail_state.thumbnail_textures.contains_key(&(1, 1)));
    assert!(app
        .thumbnail_state
        .thumbnail_textures
        .contains_key(&(1, THUMBNAIL_CACHE_CAPACITY as i64)));
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
        .insert(1, Arc::new(export::RenderControl::new()));
    app.render_tx
        .send(RenderEvent::Failed {
            job_id: 1,
            message: "disk full".to_string(),
            duration_ms: 500,
            output_duration_secs: 10.0,
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
        .insert(1, Arc::new(export::RenderControl::new()));
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
    app.preview_state.preview_playing = true;
    let ctx = egui::Context::default();
    let image = egui::ColorImage::new([1, 1], vec![egui::Color32::BLACK]);
    app.preview_state.preview_texture =
        Some(ctx.load_texture("test", image, egui::TextureOptions::default()));

    app.select_asset(Some(2));

    assert_eq!(app.selected_asset_id, Some(2));
    assert!(app.preview_state.preview_playing);
    assert!(app.preview_state.preview_texture.is_some());
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

    assert!(!app.preview_state.preview_playing);
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
fn preview_hardware_decode_is_enabled_by_default() {
    assert!(PrefsState::default().preview_hardware_decode);
}

#[test]
fn gpu_encoder_defaults_to_automatic_hardware_detection() {
    assert_eq!(
        PrefsState::default().gpu_encoder,
        avcore::GpuEncoderPreference::Auto
    );
}

#[test]
fn vaapi_gpu_encoder_preference_roundtrips_through_preferences() {
    let prefs = PrefsState {
        gpu_encoder: avcore::GpuEncoderPreference::Vaapi,
        ..PrefsState::default()
    };

    let encoded = serde_json::to_vec(&prefs).unwrap();
    let decoded: PrefsState = serde_json::from_slice(&encoded).unwrap();

    assert_eq!(decoded.gpu_encoder, avcore::GpuEncoderPreference::Vaapi);
}

#[test]
fn prefs_without_preview_hardware_decode_migrate_to_enabled() {
    let mut legacy = serde_json::to_value(PrefsState::default()).unwrap();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("preview_hardware_decode");

    let migrated: PrefsState = serde_json::from_value(legacy).unwrap();

    assert!(migrated.preview_hardware_decode);
}

#[test]
fn changing_preview_hardware_decode_invalidates_preview_state() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.preview_state.preview_clip_id = Some(7);
    app.preview_state.preview_overlay_clip_ids = vec![8];
    app.preview_state.preview_audio_clip_ids = vec![9];

    app.set_preview_hardware_decode(false);

    assert!(!app.prefs.preview_hardware_decode);
    assert_eq!(app.preview_state.preview_clip_id, None);
    assert!(app.preview_state.preview_overlay_clip_ids.is_empty());
    assert!(app.preview_state.preview_audio_clip_ids.is_empty());
}

#[test]
fn invalidate_preview_rendering_drops_the_scope_textures_too() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let ctx = egui::Context::default();
    let image = egui::ColorImage::new([1, 1], vec![egui::Color32::BLACK]);
    app.preview_state.waveform_texture = Some(ctx.load_texture(
        "waveform-test",
        image.clone(),
        egui::TextureOptions::default(),
    ));
    app.preview_state.vectorscope_texture =
        Some(ctx.load_texture("vectorscope-test", image, egui::TextureOptions::default()));

    app.invalidate_preview_rendering();

    assert!(app.preview_state.waveform_texture.is_none());
    assert!(app.preview_state.vectorscope_texture.is_none());
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
    app.preview_state.preview_playing = true;
    app.ensure_preview_loaded();

    assert!(!app.preview_clip_present());
    assert!(!app.preview_state.preview_playing);
}

#[test]
fn toggle_transcript_panel_flips_the_open_flag() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    assert!(!app.transcript_panel_open);
    app.toggle_transcript_panel();
    assert!(app.transcript_panel_open);
    app.toggle_transcript_panel();
    assert!(!app.transcript_panel_open);
}

#[test]
fn ensure_transcript_loaded_for_preview_clears_state_with_no_clip() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.transcript_panel_state.loaded_asset_id = Some(99);
    app.ensure_transcript_loaded_for_preview();
    assert_eq!(app.transcript_panel_state.loaded_asset_id, None);
    assert!(app.transcript_panel_state.document.is_none());
}

#[test]
fn ensure_transcript_loaded_for_preview_finds_no_document_when_none_saved() {
    let dir = tempfile::tempdir().unwrap();
    let mut project = test_project_with_tracks_and_assets(
        1,
        vec![test_track(
            1,
            TrackKind::Video,
            vec![test_clip(1, 0.0, 0.0, 10.0)],
        )],
        vec![test_asset(1)],
    );
    project.file_path = Some(dir.path().join("proj.ocproj"));
    project.timeline_mut().playhead_secs = 3.0;
    let mut app = test_app(vec![project], Vec::new());

    app.ensure_transcript_loaded_for_preview();

    assert_eq!(app.transcript_panel_state.loaded_asset_id, Some(1));
    assert!(app.transcript_panel_state.document.is_none());
}

#[test]
fn ensure_transcript_loaded_for_preview_loads_a_saved_document() {
    let dir = tempfile::tempdir().unwrap();
    let mut project = test_project_with_tracks_and_assets(
        1,
        vec![test_track(
            1,
            TrackKind::Video,
            vec![test_clip(1, 0.0, 0.0, 10.0)],
        )],
        vec![test_asset(1)],
    );
    project.file_path = Some(dir.path().join("proj.ocproj"));
    project.timeline_mut().playhead_secs = 3.0;

    let cache_dir = avcore::transcript_cache_dir_for_project(&project);
    let segments = vec![avcore::transcribe::TranscribeSegment {
        start_secs: 0.0,
        end_secs: 1.0,
        text: "hi".to_string(),
        words: vec![avcore::transcribe::TranscribeWord {
            text: "hi".to_string(),
            start_secs: 0.0,
            end_secs: 0.5,
            confidence: 0.9,
        }],
    }];
    let document = avcore::TranscriptDocument::from_transcribe_segments(1, None, &segments);
    avcore::save_transcript_document(&cache_dir, &document).unwrap();

    let mut app = test_app(vec![project], Vec::new());
    app.ensure_transcript_loaded_for_preview();

    assert_eq!(app.transcript_panel_state.loaded_asset_id, Some(1));
    assert_eq!(app.transcript_panel_state.document, Some(document));
}

#[test]
fn save_transcript_document_for_refreshes_an_already_open_panel() {
    let dir = tempfile::tempdir().unwrap();
    let mut project = test_project(1, vec![test_asset(1)]);
    project.file_path = Some(dir.path().join("proj.ocproj"));
    let mut app = test_app(vec![project], Vec::new());
    app.transcript_panel_state.loaded_asset_id = Some(1);
    assert!(app.transcript_panel_state.document.is_none());

    let segments = vec![avcore::transcribe::TranscribeSegment {
        start_secs: 0.0,
        end_secs: 1.0,
        text: "hi".to_string(),
        words: vec![avcore::transcribe::TranscribeWord {
            text: "hi".to_string(),
            start_secs: 0.0,
            end_secs: 0.5,
            confidence: 0.9,
        }],
    }];
    app.save_transcript_document_for(1, &segments);

    let document = app
        .transcript_panel_state
        .document
        .as_ref()
        .expect("panel already showed asset 1, so saving its transcript should refresh it");
    assert_eq!(document.asset_id, 1);
    assert_eq!(document.words.len(), 1);
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
fn a_continuous_effect_property_drag_pushes_only_one_undo_step() {
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
    app.selected_clip_id = Some(1);
    assert!(!app.can_undo());

    // Simulates egui re-firing the slider's setter every frame while the pointer stays down
    // (see App::push_undo_snapshot_for_drag's doc comment) — three "frames" of the same drag.
    app.set_selected_clip_gain(1.0);
    app.set_selected_clip_gain(2.0);
    app.set_selected_clip_gain(3.0);
    assert!(app.can_undo());
    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].gain_db,
        3.0
    );

    app.undo();
    // One undo step undoes the whole drag, back to the value before it started, not just the
    // last frame's increment.
    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].gain_db,
        0.0
    );
    assert!(!app.can_undo());
}

#[test]
fn releasing_the_pointer_starts_a_fresh_undo_step_for_the_next_drag() {
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
    app.selected_clip_id = Some(1);

    app.set_selected_clip_gain(1.0);
    app.end_undo_drag_tracking_if_pointer_released(false);
    app.set_selected_clip_gain(2.0);

    assert!(app.can_undo());
    app.undo();
    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].gain_db,
        1.0
    );
    assert!(app.can_undo());
    app.undo();
    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].gain_db,
        0.0
    );
    assert!(!app.can_undo());
}

#[test]
fn undo_after_split_at_playhead_restores_the_unsplit_clip() {
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
    app.active_project_mut().timeline_mut().playhead_secs = 10.0;
    assert!(!app.can_undo());

    app.split_at_playhead();
    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 2);
    assert!(app.can_undo());
    assert!(!app.can_redo());

    app.undo();
    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
    assert!(!app.can_undo());
    assert!(app.can_redo());

    app.redo();
    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 2);
    assert!(app.can_undo());
    assert!(!app.can_redo());
}

#[test]
fn undo_is_a_no_op_with_empty_history() {
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
    app.undo();
    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
}

#[test]
fn selecting_a_sequence_clears_undo_history_from_the_previous_one() {
    let mut project = test_project_with_tracks(
        1,
        vec![test_track(
            1,
            TrackKind::Video,
            vec![test_clip(1, 0.0, 0.0, 20.0)],
        )],
    );
    project.new_sequence("Second".to_string());
    project.active_sequence = 0;
    let mut app = test_app(vec![project], Vec::new());
    app.active_project_mut().timeline_mut().playhead_secs = 10.0;

    app.split_at_playhead();
    assert!(app.can_undo());

    app.select_sequence(1);
    assert!(!app.can_undo());
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
fn ripple_trim_clip_start_shifts_only_later_clips() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 10.0), test_clip(2, 10.0, 0.0, 5.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.ripple_trim_clip_start(2, 12.0);

    let tracks = &app.active_project().timeline().tracks[0];
    assert_eq!(tracks.clips[0].start_secs, 0.0);
    let clip2 = tracks.clips.iter().find(|c| c.id == 2).unwrap();
    assert_eq!(clip2.start_secs, 12.0);
    assert_eq!(clip2.source_in_secs, 2.0);
}

#[test]
fn ripple_trim_clip_end_is_bounded_by_the_source_assets_own_duration() {
    // test_asset's duration_secs is a fixed 10.0, so trimming clip 1's end past it must
    // refuse -- same bound App::trim_clip_end already enforces.
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 8.0), test_clip(2, 8.0, 0.0, 5.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.ripple_trim_clip_end(1, 50.0);

    let tracks = &app.active_project().timeline().tracks[0];
    assert_eq!(tracks.clips[0].source_out_secs, 8.0);
    assert_eq!(
        tracks.clips[1].start_secs, 8.0,
        "a refused trim must not shift the later clip either"
    );
}

#[test]
fn roll_edit_clip_moves_the_shared_boundary() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 10.0), test_clip(2, 10.0, 2.0, 7.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.roll_edit_clip(1, 8.0);

    let tracks = &app.active_project().timeline().tracks[0];
    let clip1 = tracks.clips.iter().find(|c| c.id == 1).unwrap();
    let clip2 = tracks.clips.iter().find(|c| c.id == 2).unwrap();
    assert_eq!(clip1.start_secs + clip1.duration_secs(), 8.0);
    assert_eq!(clip2.start_secs, 8.0);
}

#[test]
fn roll_edit_from_start_edge_resolves_the_previous_neighbor() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 10.0), test_clip(2, 10.0, 2.0, 7.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    // Dragging clip 2's own start edge should produce the same roll as dragging clip 1's end.
    app.roll_edit_from_start_edge(2, 8.0);

    let tracks = &app.active_project().timeline().tracks[0];
    let clip1 = tracks.clips.iter().find(|c| c.id == 1).unwrap();
    let clip2 = tracks.clips.iter().find(|c| c.id == 2).unwrap();
    assert_eq!(clip1.start_secs + clip1.duration_secs(), 8.0);
    assert_eq!(clip2.start_secs, 8.0);
}

#[test]
fn slip_clip_shifts_source_range_without_moving_on_the_timeline() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 5.0, 1.0, 6.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.slip_clip(1, 1.0);

    let clip = &app.active_project().timeline().tracks[0].clips[0];
    assert_eq!(clip.start_secs, 5.0);
    assert_eq!(clip.source_in_secs, 2.0);
    assert_eq!(clip.source_out_secs, 7.0);
}

#[test]
fn slide_clip_absorbs_the_move_into_both_neighbors() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![
            test_clip(1, 0.0, 0.0, 6.0),
            test_clip(2, 6.0, 0.0, 9.0),
            test_clip(3, 15.0, 0.0, 5.0),
        ],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.slide_clip(2, 8.0);

    let tracks = &app.active_project().timeline().tracks[0];
    let clip1 = tracks.clips.iter().find(|c| c.id == 1).unwrap();
    let clip2 = tracks.clips.iter().find(|c| c.id == 2).unwrap();
    let clip3 = tracks.clips.iter().find(|c| c.id == 3).unwrap();
    assert_eq!(clip2.start_secs, 8.0);
    assert_eq!(clip1.start_secs + clip1.duration_secs(), 8.0);
    assert_eq!(clip3.start_secs, 17.0);
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
fn preview_seek_parameters_keep_composited_branches_in_order() {
    let mut background = test_clip(1, 2.0, 1.0, 9.0);
    background.speed_factor = 0.5;
    let mut overlay = test_clip(2, 3.0, 0.25, 9.0);
    overlay.speed_factor = 2.0;
    let mut audio = test_clip(3, 1.0, 4.0, 9.0);
    audio.frozen = true;
    audio.speed_factor = 1.5;

    let (offsets, rates) = App::preview_seek_parameters([&background, &overlay, &audio], 5.0);

    assert_eq!(offsets, vec![2.5, 4.25, 4.0]);
    assert_eq!(rates, vec![0.5, 2.0, 1.5]);
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
fn spawn_motion_track_selected_clip_is_a_no_op_when_nothing_is_selected() {
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
    app.selected_clip_id = None;

    app.spawn_motion_track_selected_clip();

    // No background job started — the busy flag stays clear rather than getting stuck "in
    // progress" forever with nothing to ever complete it.
    assert_eq!(app.motion_tracking_state.motion_tracking_clip_id, None);
}

#[test]
fn spawn_motion_track_selected_clip_is_a_no_op_while_a_run_is_already_in_flight() {
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
    app.motion_tracking_state.motion_tracking_clip_id = Some(99);

    app.spawn_motion_track_selected_clip();

    // Stays pinned to the already-running clip's id, not overwritten by this second call.
    assert_eq!(app.motion_tracking_state.motion_tracking_clip_id, Some(99));
}

#[test]
fn open_youtube_modal_starts_with_an_empty_url() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.open_youtube_modal();

    assert_eq!(
        app.youtube_download_state.youtube_modal_url,
        Some(String::new())
    );
}

#[test]
fn open_youtube_modal_is_a_no_op_while_already_open() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.youtube_download_state.youtube_modal_url = Some("https://example.com/x".to_string());

    app.open_youtube_modal();

    assert_eq!(
        app.youtube_download_state.youtube_modal_url,
        Some("https://example.com/x".to_string())
    );
}

#[test]
fn open_youtube_modal_is_a_no_op_while_downloading() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.youtube_download_state.youtube_downloading = true;

    app.open_youtube_modal();

    assert_eq!(app.youtube_download_state.youtube_modal_url, None);
}

#[test]
fn spawn_youtube_download_is_a_no_op_with_a_blank_url() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.youtube_download_state.youtube_modal_url = Some("   ".to_string());

    app.spawn_youtube_download();

    assert!(!app.youtube_download_state.youtube_downloading);
}

#[test]
fn spawn_youtube_download_is_a_no_op_while_already_downloading() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.youtube_download_state.youtube_modal_url = Some("https://example.com/x".to_string());
    app.youtube_download_state.youtube_downloading = true;
    app.youtube_download_state.youtube_download_progress = 0.4;

    app.spawn_youtube_download();

    // Progress isn't reset by this second, ignored call.
    assert_eq!(app.youtube_download_state.youtube_download_progress, 0.4);
}

#[test]
fn close_youtube_modal_is_a_no_op_while_downloading() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.youtube_download_state.youtube_modal_url = Some("https://example.com/x".to_string());
    app.youtube_download_state.youtube_downloading = true;

    app.close_youtube_modal();

    assert!(app.youtube_download_state.youtube_modal_url.is_some());
}

#[test]
fn close_youtube_modal_clears_the_url_when_idle() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.youtube_download_state.youtube_modal_url = Some("https://example.com/x".to_string());

    app.close_youtube_modal();

    assert_eq!(app.youtube_download_state.youtube_modal_url, None);
}

#[test]
fn request_cancel_youtube_download_is_a_no_op_when_nothing_is_downloading() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    // Just needs to not panic without a live download.
    app.request_cancel_youtube_download();
}

#[test]
fn motion_track_region_defaults_to_a_centered_region() {
    let app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    assert_eq!(app.motion_track_region.motion_track_center_x, 0.5);
    assert_eq!(app.motion_track_region.motion_track_center_y, 0.5);
}

#[test]
fn start_picking_motion_track_region_is_a_no_op_without_a_loaded_preview() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.preview_state.preview_texture = None;

    app.start_picking_motion_track_region();

    assert!(!app.motion_track_region.picking_motion_track_region);
}

#[test]
fn start_picking_motion_track_region_activates_with_a_loaded_preview() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let ctx = egui::Context::default();
    let image = egui::ColorImage::new([1, 1], vec![egui::Color32::BLACK]);
    app.preview_state.preview_texture =
        Some(ctx.load_texture("test", image, egui::TextureOptions::default()));

    app.start_picking_motion_track_region();

    assert!(app.motion_track_region.picking_motion_track_region);
}

#[test]
fn stop_picking_motion_track_region_clears_the_flag() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.motion_track_region.picking_motion_track_region = true;

    app.stop_picking_motion_track_region();

    assert!(!app.motion_track_region.picking_motion_track_region);
}

#[test]
fn spawn_generate_matte_for_selected_clip_is_a_no_op_when_no_model_is_configured() {
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
    assert!(app.prefs.background_removal_model_path.trim().is_empty());

    app.spawn_generate_matte_for_selected_clip();

    // No background job started, and the user is told why.
    assert_eq!(app.matte_generation_state.matte_generating_clip_id, None);
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn spawn_generate_matte_for_selected_clip_is_a_no_op_while_a_run_is_already_in_flight() {
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
    app.prefs.background_removal_model_path = "/models/modnet.onnx".to_string();
    app.selected_clip_id = Some(1);
    app.matte_generation_state.matte_generating_clip_id = Some(99);

    app.spawn_generate_matte_for_selected_clip();

    // Stays pinned to the already-running clip's id, not overwritten by this second call.
    assert_eq!(
        app.matte_generation_state.matte_generating_clip_id,
        Some(99)
    );
}

#[test]
fn set_selected_clip_background_removal_mask_path_updates_the_selected_clip() {
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

    app.set_selected_clip_background_removal_mask_path("/cache/clip_1_matte.mp4".to_string());

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].background_removal_mask_path,
        "/cache/clip_1_matte.mp4"
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
fn add_opacity_marker_at_playhead_is_a_no_op_when_nothing_is_selected() {
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
    app.active_project_mut().timeline_mut().playhead_secs = 4.0;

    app.add_opacity_marker_at_playhead();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(clips[0].opacity_keyframes.is_empty());
}

#[test]
fn add_opacity_marker_at_playhead_is_a_no_op_outside_the_clips_own_span() {
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
    app.active_project_mut().timeline_mut().playhead_secs = 15.0; // past the clip's 10s span.

    app.add_opacity_marker_at_playhead();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(clips[0].opacity_keyframes.is_empty());
}

#[test]
fn add_opacity_marker_at_playhead_defaults_to_fully_opaque_with_no_existing_keyframes() {
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
    app.active_project_mut().timeline_mut().playhead_secs = 4.0; // 4s into a 10s clip -> 0.4.

    app.add_opacity_marker_at_playhead();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].opacity_keyframes.len(), 1);
    let added = clips[0].opacity_keyframes[0];
    assert!((added.time_fraction - 0.4).abs() < 1e-6);
    assert_eq!(added.value, 1.0);
}

#[test]
fn add_opacity_marker_at_playhead_preserves_the_currently_interpolated_value() {
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
    app.set_selected_clip_opacity_keyframes(vec![
        avcore::Keyframe {
            time_fraction: 0.0,
            value: 0.2,
        },
        avcore::Keyframe {
            time_fraction: 1.0,
            value: 1.0,
        },
    ]);
    app.active_project_mut().timeline_mut().playhead_secs = 5.0; // halfway -> time_fraction 0.5.

    app.add_opacity_marker_at_playhead();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].opacity_keyframes.len(), 3);
    let added = clips[0]
        .opacity_keyframes
        .iter()
        .find(|kf| (kf.time_fraction - 0.5).abs() < 1e-6)
        .expect("the new marker at time_fraction 0.5");
    // Halfway between 0.2 and 1.0 is 0.6 - adding the marker shouldn't itself change the
    // clip's current opacity.
    assert!((added.value - 0.6).abs() < 1e-6);
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
fn copy_selected_clip_captures_every_member_of_a_composite_group() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 5.0), test_clip(2, 5.0, 0.0, 5.0)],
            )],
        )],
        Vec::new(),
    );
    app.multi_selected_clip_ids = HashSet::from([1, 2]);
    app.merge_into_composite();
    app.selected_clip_id = Some(1);

    app.copy_selected_clip();

    let (copied, _kind) = app.clipboard_clip.as_ref().unwrap();
    assert_eq!(copied.len(), 2);
    let mut copied_ids: Vec<u64> = copied.iter().map(|c| c.id).collect();
    copied_ids.sort();
    assert_eq!(copied_ids, vec![1, 2]);
}

#[test]
fn paste_clip_at_playhead_pastes_a_composite_group_as_one_block() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 5.0), test_clip(2, 5.0, 0.0, 5.0)],
            )],
        )],
        Vec::new(),
    );
    app.multi_selected_clip_ids = HashSet::from([1, 2]);
    app.merge_into_composite();
    app.selected_clip_id = Some(1);
    app.copy_selected_clip();
    app.active_project_mut().timeline_mut().playhead_secs = 100.0;

    app.paste_clip_at_playhead();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips.len(), 4);
    let pasted: Vec<_> = clips.iter().filter(|c| c.id != 1 && c.id != 2).collect();
    assert_eq!(pasted.len(), 2);
    // The two originals started at 0.0 and 5.0 (a 5s relative offset) - that offset survives
    // the paste, anchored at the new playhead instead of at 0.0.
    let mut pasted_starts: Vec<f64> = pasted.iter().map(|c| c.start_secs).collect();
    pasted_starts.sort_by(f64::total_cmp);
    assert_eq!(pasted_starts, vec![100.0, 105.0]);
    // Fresh ids, distinct from both the originals and each other.
    assert_ne!(pasted[0].id, pasted[1].id);
    assert!(pasted[0].id != 1 && pasted[0].id != 2);
    // Both pasted clips share one new composite_id, distinct from the original group's.
    let original_group = app.active_project().timeline().tracks[0]
        .clips
        .iter()
        .find(|c| c.id == 1)
        .unwrap()
        .composite_id;
    assert!(pasted[0].composite_id.is_some());
    assert_eq!(pasted[0].composite_id, pasted[1].composite_id);
    assert_ne!(pasted[0].composite_id, original_group);
}

#[test]
fn paste_clip_at_playhead_keeps_a_lone_copied_clip_standalone() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 5.0)],
            )],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);
    app.copy_selected_clip();
    app.active_project_mut().timeline_mut().playhead_secs = 20.0;

    app.paste_clip_at_playhead();

    let pasted = &app.active_project().timeline().tracks[0].clips[1];
    assert_eq!(pasted.composite_id, None);
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
    app.import_state.pending_imports = 1;
    let mut ready_asset = test_asset(0);
    ready_asset.file_name = "clip.mp4".to_string();
    app.import_state
        .import_tx
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
    assert_eq!(app.import_state.pending_imports, 0);
}

#[test]
fn pump_import_queue_targets_the_project_by_id_not_the_active_index() {
    let mut app = test_app(
        vec![test_project(1, Vec::new()), test_project(2, Vec::new())],
        Vec::new(),
    );
    app.active_project = 1; // Simulates the user switching projects mid-import.
    app.import_state
        .import_tx
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
    app.import_state
        .import_tx
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

    app.import_state
        .import_tx
        .send(ImportEvent::Enriched {
            project_id: 1,
            import_token: 7,
            loudness: Some(loudness),
            proxy_path: Some(PathBuf::from("proxy.mp4")),
            waveform_peaks: Some(vec![(-0.5, 0.5)]),
            duration_ms: 250,
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
    app.import_state
        .import_tx
        .send(ImportEvent::Enriched {
            project_id: 1,
            import_token: 999,
            loudness: None,
            proxy_path: None,
            waveform_peaks: None,
            duration_ms: 0,
        })
        .unwrap();

    app.pump_import_queue();

    assert_eq!(app.active_project().media_library.len(), 1);
}

#[test]
fn pump_import_queue_drops_a_failed_import_without_panicking() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.import_state.pending_imports = 1;
    app.import_state
        .import_tx
        .send(ImportEvent::Failed {
            path: PathBuf::from("missing.mp4"),
            message: "not found".to_string(),
        })
        .unwrap();

    app.pump_import_queue();

    assert_eq!(app.import_state.pending_imports, 0);
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
        audio_segments: vec![],
        text_segments: vec![],
        shape_segments: vec![],
        canvas: test_canvas(),
        target_lufs: -14.0,
        output_path: output.clone(),
    });

    // Simulates the modal's Overwrite branch directly — queues with output_path unchanged.
    let pending = app.pending_export_conflict.take().unwrap();
    app.queue_export(
        pending.title,
        pending.track_segments,
        pending.audio_segments,
        pending.text_segments,
        pending.shape_segments,
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
                confidence: 0.9,
            },
            avcore::transcribe::TranscribeWord {
                text: "World".to_string(),
                start_secs: 100.5,
                end_secs: 101.0,
                confidence: 0.9,
            },
        ],
    }];
    app.transcribe_state
        .transcribe_tx
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

#[test]
fn add_shape_track_appends_an_empty_shape_track() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_shape_track();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].kind, TrackKind::Shape);
    assert!(tracks[0].shape_clips.is_empty());
}

#[test]
fn add_text_clip_auto_creates_a_text_track_at_the_playhead_and_selects_it() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.active_project_mut().timeline_mut().playhead_secs = 12.5;
    app.selected_clip_id = Some(90);
    app.selected_shape_clip_id = Some(91);

    app.add_text_clip();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].kind, TrackKind::Text);
    assert_eq!(tracks[0].text_clips.len(), 1);
    let clip = &tracks[0].text_clips[0];
    assert_eq!(clip.start_secs, 12.5);
    assert_eq!(clip.duration_secs, 3.0);
    assert_eq!(clip.text, "Texto");
    assert_eq!(app.selected_text_clip_id, Some(clip.id));
    assert_eq!(app.selected_clip_id, None);
    assert_eq!(app.selected_shape_clip_id, None);
}

#[test]
fn add_text_clip_reuses_the_existing_text_track() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_text_clip();
    app.add_text_clip();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].text_clips.len(), 2);
    assert_ne!(tracks[0].text_clips[0].id, tracks[0].text_clips[1].id);
}

#[test]
fn text_color_edit_commits_each_supported_target_only_on_confirmation() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_text_clip();
    let clip_id = app.selected_text_clip_id.unwrap();

    app.begin_text_color_edit(clip_id, TextColorTarget::Foreground, [255, 255, 255, 255]);
    app.text_color_edit.as_mut().unwrap().rgba = [1, 2, 3, 4];
    assert_eq!(
        app.active_project().timeline().tracks[0].text_clips[0].color_rgba,
        [255, 255, 255, 255]
    );
    assert!(app.confirm_text_color_edit());

    for (target, rgba) in [
        (TextColorTarget::Background, [5, 6, 7, 8]),
        (TextColorTarget::Highlight, [9, 10, 11, 12]),
    ] {
        app.begin_text_color_edit(clip_id, target, [255, 255, 255, 255]);
        app.text_color_edit.as_mut().unwrap().rgba = rgba;
        assert!(app.confirm_text_color_edit());
    }

    let clip = &app.active_project().timeline().tracks[0].text_clips[0];
    assert_eq!(clip.color_rgba, [1, 2, 3, 4]);
    assert_eq!(clip.background_rgba, [5, 6, 7, 8]);
    assert_eq!(clip.highlight_color_rgba, [9, 10, 11, 12]);
}

#[test]
fn confirming_a_text_color_edit_pushes_an_undo_step() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_text_clip();
    let clip_id = app.selected_text_clip_id.unwrap();
    app.undo_stack.clear(); // discard the snapshot add_text_clip() itself pushed.

    app.begin_text_color_edit(clip_id, TextColorTarget::Foreground, [255, 255, 255, 255]);
    app.text_color_edit.as_mut().unwrap().rgba = [1, 2, 3, 4];
    assert!(app.confirm_text_color_edit());

    assert!(app.can_undo());
    app.undo();
    assert_eq!(
        app.active_project().timeline().tracks[0].text_clips[0].color_rgba,
        [255, 255, 255, 255]
    );
}

#[test]
fn switching_sequences_discards_an_open_text_color_edit() {
    let mut project = test_project(1, Vec::new());
    project.new_sequence("Sequence 2".to_string());
    project.active_sequence = 0;
    let mut app = test_app(vec![project], Vec::new());
    app.add_text_clip();
    let clip_id = app.selected_text_clip_id.unwrap();
    app.begin_text_color_edit(clip_id, TextColorTarget::Foreground, [255, 255, 255, 255]);

    app.select_sequence(1);

    assert_eq!(app.text_color_edit, None);
}

#[test]
fn add_shape_clip_auto_creates_a_shape_track_and_selects_the_new_clip() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_shape_clip();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].kind, TrackKind::Shape);
    assert_eq!(tracks[0].shape_clips.len(), 1);
    let clip_id = tracks[0].shape_clips[0].id;
    assert_eq!(app.selected_shape_clip_id, Some(clip_id));
    assert_eq!(app.selected_clip_id, None);
    assert_eq!(app.selected_text_clip_id, None);
}

#[test]
fn add_shape_clip_ids_stay_unique_past_an_existing_high_shape_clip_id() {
    let mut track = test_track(1, TrackKind::Shape, Vec::new());
    track.shape_clips.push(avcore::timeline::ShapeClip {
        id: 100,
        start_secs: 0.0,
        duration_secs: 1.0,
        shape_kind: avcore::timeline::ShapeKind::rectangle(),
        center_x: 0.5,
        center_y: 0.5,
        center_x_keyframes: vec![],
        center_y_keyframes: vec![],
        width_keyframes: vec![],
        height_keyframes: vec![],
        rotation_keyframes: vec![],
        width: 0.3,
        height: 0.3,
        rotation_deg: 0.0,
        color_rgba: [255, 255, 255, 255],
        stroke_thickness_px: 0.0,
    });
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());

    app.add_shape_clip();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks[0].shape_clips.len(), 2);
    assert_eq!(tracks[0].shape_clips[1].id, 101);
}

#[test]
fn add_shape_clip_reuses_the_existing_shape_track_on_a_second_call() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_shape_clip();
    app.add_shape_clip();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].shape_clips.len(), 2);
    // Ids are unique even across the two calls.
    assert_ne!(tracks[0].shape_clips[0].id, tracks[0].shape_clips[1].id);
}

#[test]
fn start_drawing_custom_shape_begins_an_empty_point_list() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.start_drawing_custom_shape();

    assert_eq!(app.drawing_shape_points, Some(Vec::new()));
}

#[test]
fn cancel_drawing_custom_shape_discards_the_in_progress_drawing() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.start_drawing_custom_shape();
    app.push_drawing_shape_point(0.2, 0.3);

    app.cancel_drawing_custom_shape();

    assert_eq!(app.drawing_shape_points, None);
}

#[test]
fn push_drawing_shape_point_is_a_no_op_outside_drawing_mode() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.push_drawing_shape_point(0.2, 0.3);

    assert_eq!(app.drawing_shape_points, None);
}

#[test]
fn push_drawing_shape_point_clamps_to_the_canvas() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.start_drawing_custom_shape();

    app.push_drawing_shape_point(-0.5, 1.5);

    assert_eq!(app.drawing_shape_points, Some(vec![(0.0, 1.0)]));
}

#[test]
fn finish_drawing_custom_shape_is_a_no_op_below_three_points() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.start_drawing_custom_shape();
    app.push_drawing_shape_point(0.2, 0.2);
    app.push_drawing_shape_point(0.4, 0.2);

    app.finish_drawing_custom_shape();

    assert_eq!(app.drawing_shape_points, Some(vec![(0.2, 0.2), (0.4, 0.2)]));
    assert!(app.active_project().timeline().tracks.is_empty());
}

#[test]
fn finish_drawing_custom_shape_creates_a_polygon_clip_and_clears_drawing_mode() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.start_drawing_custom_shape();
    app.push_drawing_shape_point(0.2, 0.2);
    app.push_drawing_shape_point(0.6, 0.2);
    app.push_drawing_shape_point(0.4, 0.6);

    app.finish_drawing_custom_shape();

    assert_eq!(app.drawing_shape_points, None);
    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].kind, TrackKind::Shape);
    assert_eq!(tracks[0].shape_clips.len(), 1);
    let clip = &tracks[0].shape_clips[0];
    assert_eq!(app.selected_shape_clip_id, Some(clip.id));
    // Bounding box of the three points above: x in [0.2, 0.6], y in [0.2, 0.6].
    assert!((clip.center_x - 0.4).abs() < 1e-6);
    assert!((clip.center_y - 0.4).abs() < 1e-6);
    assert!((clip.width - 0.4).abs() < 1e-6);
    assert!((clip.height - 0.4).abs() < 1e-6);
    assert_eq!(clip.rotation_deg, 0.0);
    let avcore::timeline::ShapeKind::Polygon(vertices) = &clip.shape_kind else {
        panic!("expected a Polygon shape kind");
    };
    assert_eq!(vertices.len(), 3);
    // First point (0.2, 0.2) is the box's top-left corner -> local (-0.5, -0.5).
    assert!((vertices[0].0 - (-0.5)).abs() < 1e-6);
    assert!((vertices[0].1 - (-0.5)).abs() < 1e-6);
}

#[test]
fn finish_drawing_custom_shape_floors_a_degenerate_bounding_box() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.start_drawing_custom_shape();
    // Three collinear points on the same vertical line - a zero-width bounding box.
    app.push_drawing_shape_point(0.5, 0.2);
    app.push_drawing_shape_point(0.5, 0.4);
    app.push_drawing_shape_point(0.5, 0.6);

    app.finish_drawing_custom_shape();

    let clip = &app.active_project().timeline().tracks[0].shape_clips[0];
    assert!(clip.width > 0.0);
    let avcore::timeline::ShapeKind::Polygon(vertices) = &clip.shape_kind else {
        panic!("expected a Polygon shape kind");
    };
    assert!(vertices.iter().all(|v| v.0.is_finite() && v.1.is_finite()));
}

#[test]
fn pump_update_check_sets_available_status_on_a_newer_version_event() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.update_check_tx
        .send(UpdateCheckEvent::NewerVersionAvailable {
            version: "99.0.0".to_string(),
            html_url: "https://github.com/lucasgmagalhaes/oca/releases/tag/v99.0.0".to_string(),
            auto_update_available: true,
        })
        .unwrap();

    app.pump_update_check();

    assert_eq!(
        app.update_check_status,
        UpdateCheckStatus::Available(AvailableUpdate {
            version: "99.0.0".to_string(),
            html_url: "https://github.com/lucasgmagalhaes/oca/releases/tag/v99.0.0".to_string(),
            auto_update_available: true,
        })
    );
}

#[test]
fn pump_update_check_is_a_no_op_with_no_pending_events() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.pump_update_check();

    assert_eq!(app.update_check_status, UpdateCheckStatus::Checking);
}

#[test]
fn pump_update_check_distinguishes_up_to_date_from_failure() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.update_check_tx
        .send(UpdateCheckEvent::UpToDate)
        .unwrap();

    app.pump_update_check();

    assert_eq!(app.update_check_status, UpdateCheckStatus::UpToDate);

    app.update_check_tx.send(UpdateCheckEvent::Failed).unwrap();
    app.pump_update_check();

    assert_eq!(app.update_check_status, UpdateCheckStatus::Failed);
}

#[test]
fn pump_update_check_marks_an_installed_update_ready_to_restart() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let update = AvailableUpdate {
        version: "99.0.0".to_string(),
        html_url: "https://github.com/lucasgmagalhaes/oca/releases/tag/v99.0.0".to_string(),
        auto_update_available: true,
    };
    app.update_check_tx
        .send(UpdateCheckEvent::Installed {
            update: update.clone(),
        })
        .unwrap();

    app.pump_update_check();

    assert_eq!(
        app.update_check_status,
        UpdateCheckStatus::RestartRequired(update)
    );
}

#[test]
fn pump_update_check_preserves_release_details_after_install_failure() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let update = AvailableUpdate {
        version: "99.0.0".to_string(),
        html_url: "https://github.com/lucasgmagalhaes/oca/releases/tag/v99.0.0".to_string(),
        auto_update_available: true,
    };
    app.update_check_tx
        .send(UpdateCheckEvent::InstallFailed {
            update: update.clone(),
        })
        .unwrap();

    app.pump_update_check();

    assert_eq!(
        app.update_check_status,
        UpdateCheckStatus::InstallFailed(update)
    );
}

#[test]
fn open_about_closes_preferences_and_close_about_clears_the_modal() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs_open = true;

    app.open_about();

    assert!(!app.prefs_open);
    assert!(app.about_open);

    app.close_about();

    assert!(!app.about_open);
}

#[test]
fn prefs_snapshot_captures_live_locale_and_panel_layout() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.locale = Locale::En;
    app.lib_panel_width = 321.0;
    app.props_panel_width = 456.0;
    app.timeline_height = 111.0;
    // Deliberately different from the live values above, so the snapshot can only match by
    // actually reading the live App fields, not by coincidentally already matching prefs.
    app.prefs.locale = Locale::PtBr;
    app.prefs.lib_panel_width = 1.0;
    app.prefs.props_panel_width = 2.0;
    app.prefs.timeline_height = 3.0;

    let snapshot = app.prefs_snapshot();

    assert_eq!(snapshot.locale, Locale::En);
    assert_eq!(snapshot.lib_panel_width, 321.0);
    assert_eq!(snapshot.props_panel_width, 456.0);
    assert_eq!(snapshot.timeline_height, 111.0);
}

#[test]
fn prefs_snapshot_prunes_recent_paths_that_no_longer_exist() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.recent_project_paths = vec![
        "/definitely/does/not/exist/project.ocproj".to_string(),
        env!("CARGO_MANIFEST_DIR").to_string(), // this directory does exist.
    ];

    let snapshot = app.prefs_snapshot();

    assert_eq!(
        snapshot.recent_project_paths,
        vec![env!("CARGO_MANIFEST_DIR").to_string()]
    );
}

#[test]
fn sync_panel_layout_is_a_no_op_under_per_user_scope() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.layout_scope = LayoutScope::PerUser;
    app.lib_panel_width = 999.0;

    app.sync_panel_layout_into_active_project();

    assert_eq!(app.active_project().panel_layout, None);
}

#[test]
fn sync_panel_layout_captures_live_values_under_per_project_scope() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.layout_scope = LayoutScope::PerProject;
    app.lib_panel_width = 321.0;
    app.props_panel_width = 456.0;
    app.timeline_height = 111.0;

    app.sync_panel_layout_into_active_project();

    assert_eq!(
        app.active_project().panel_layout,
        Some(avcore::PanelLayout {
            lib_panel_width: 321.0,
            props_panel_width: 456.0,
            timeline_height: 111.0,
        })
    );
}

#[test]
fn load_panel_layout_is_a_no_op_under_per_user_scope() {
    let mut project = test_project(1, Vec::new());
    project.panel_layout = Some(avcore::PanelLayout {
        lib_panel_width: 10.0,
        props_panel_width: 20.0,
        timeline_height: 30.0,
    });
    let mut app = test_app(vec![project], Vec::new());
    app.prefs.layout_scope = LayoutScope::PerUser;
    app.lib_panel_width = 999.0;

    app.load_panel_layout_for_active_project();

    assert_eq!(app.lib_panel_width, 999.0);
}

#[test]
fn load_panel_layout_applies_the_active_projects_saved_layout() {
    let mut project = test_project(1, Vec::new());
    project.panel_layout = Some(avcore::PanelLayout {
        lib_panel_width: 10.0,
        props_panel_width: 20.0,
        timeline_height: 30.0,
    });
    let mut app = test_app(vec![project], Vec::new());
    app.prefs.layout_scope = LayoutScope::PerProject;
    app.lib_panel_width = 999.0;

    app.load_panel_layout_for_active_project();

    assert_eq!(app.lib_panel_width, 10.0);
    assert_eq!(app.props_panel_width, 20.0);
    assert_eq!(app.timeline_height, 30.0);
}

#[test]
fn load_panel_layout_keeps_live_values_when_project_has_no_saved_layout() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.layout_scope = LayoutScope::PerProject;
    app.lib_panel_width = 999.0;

    app.load_panel_layout_for_active_project();

    assert_eq!(app.lib_panel_width, 999.0);
}

#[test]
fn toggle_timeline_index_flips_the_open_flag() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    assert!(!app.timeline_index_open);

    app.toggle_timeline_index();
    assert!(app.timeline_index_open);

    app.toggle_timeline_index();
    assert!(!app.timeline_index_open);
}

#[test]
fn add_marker_at_playhead_places_it_at_the_current_playhead() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.active_project_mut().timeline_mut().playhead_secs = 12.5;

    let id = app.add_marker_at_playhead(avcore::MarkerKind::Chapter);

    let timeline = app.active_project().timeline();
    let marker = timeline.markers.iter().find(|m| m.id == id).unwrap();
    assert_eq!(marker.position_secs, 12.5);
    assert_eq!(marker.kind, avcore::MarkerKind::Chapter);
}

#[test]
fn remove_marker_deletes_it() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let id = app.add_marker_at_playhead(avcore::MarkerKind::Standard);

    app.remove_marker(id);

    assert!(app.active_project().timeline().markers.is_empty());
}

#[test]
fn set_marker_label_updates_the_right_marker() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let id = app.add_marker_at_playhead(avcore::MarkerKind::Standard);

    app.set_marker_label(id, "Needs a re-take".to_string());

    let timeline = app.active_project().timeline();
    assert_eq!(
        timeline.markers.iter().find(|m| m.id == id).unwrap().label,
        "Needs a re-take"
    );
}

#[test]
fn set_marker_kind_updates_the_right_marker() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let id = app.add_marker_at_playhead(avcore::MarkerKind::Standard);

    app.set_marker_kind(id, avcore::MarkerKind::ToDo);

    let timeline = app.active_project().timeline();
    assert_eq!(
        timeline.markers.iter().find(|m| m.id == id).unwrap().kind,
        avcore::MarkerKind::ToDo
    );
}

#[test]
fn toggle_marker_completed_flips_the_flag() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let id = app.add_marker_at_playhead(avcore::MarkerKind::ToDo);

    app.toggle_marker_completed(id);
    assert!(
        app.active_project()
            .timeline()
            .markers
            .iter()
            .find(|m| m.id == id)
            .unwrap()
            .completed
    );

    app.toggle_marker_completed(id);
    assert!(
        !app.active_project()
            .timeline()
            .markers
            .iter()
            .find(|m| m.id == id)
            .unwrap()
            .completed
    );
}

#[test]
fn marker_mutations_are_no_ops_for_an_unknown_id() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.remove_marker(404);
    app.set_marker_label(404, "x".to_string());
    app.set_marker_kind(404, avcore::MarkerKind::Chapter);
    app.toggle_marker_completed(404);

    assert!(app.active_project().timeline().markers.is_empty());
}

fn silence_review_asset() -> MediaAsset {
    // 20 one-second buckets, loud except a silent run [8, 12).
    let mut peaks = vec![(-0.8, 0.8); 20];
    for p in &mut peaks[8..12] {
        *p = (0.0, 0.0);
    }
    MediaAsset {
        duration_secs: 20.0,
        waveform_peaks: Some(peaks),
        ..test_asset(1)
    }
}

#[test]
fn begin_silence_review_maps_a_detected_gap_into_timeline_coordinates() {
    // Clip shows source 5..15 starting at timeline 100 -- same setup as
    // avcore::silence_detection's own clip_silence_gaps test, exercised here end-to-end
    // through the App wrapper.
    let track = test_track(1, TrackKind::Audio, vec![test_clip(1, 100.0, 5.0, 15.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![silence_review_asset()],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.begin_silence_review();

    let review = app.silence_review.as_ref().expect("review should open");
    assert_eq!(review.track_id, 1);
    assert_eq!(review.gaps.len(), 1);
    assert_eq!(review.gaps[0].clip_id, 1);
    assert_eq!(review.gaps[0].gap.start_secs, 103.0);
    assert_eq!(review.gaps[0].gap.end_secs, 107.0);
    assert!(review.gaps[0].accepted, "gaps default to accepted");
}

#[test]
fn begin_silence_review_without_a_selected_clip_toasts_instead_of_opening() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.begin_silence_review();

    assert!(app.silence_review.is_none());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn begin_silence_review_skips_clips_whose_asset_has_no_cached_waveform() {
    let track = test_track(1, TrackKind::Audio, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![test_asset(1)],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.begin_silence_review();

    assert!(app.silence_review.as_ref().unwrap().gaps.is_empty());
}

#[test]
fn toggle_silence_gap_accepted_flips_only_the_targeted_entry() {
    let track = test_track(1, TrackKind::Audio, vec![test_clip(1, 100.0, 5.0, 15.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![silence_review_asset()],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);
    app.begin_silence_review();

    app.toggle_silence_gap_accepted(0);

    assert!(!app.silence_review.as_ref().unwrap().gaps[0].accepted);
}

#[test]
fn apply_silence_review_ripple_deletes_only_accepted_gaps_and_closes_the_modal() {
    // Two clips, each with its own silent run, on the same track.
    let track = test_track(
        1,
        TrackKind::Audio,
        vec![test_clip(1, 0.0, 0.0, 20.0), test_clip(2, 20.0, 0.0, 20.0)],
    );
    let mut asset2 = silence_review_asset();
    asset2.id = 2;
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![silence_review_asset(), asset2],
        )],
        Vec::new(),
    );
    // clip 1 uses asset 1, clip 2 uses asset 2 -- fix up asset_id on the second clip.
    app.active_project_mut().timeline_mut().tracks[0].clips[1].asset_id = 2;
    app.selected_clip_id = Some(1);
    app.begin_silence_review();
    assert_eq!(app.silence_review.as_ref().unwrap().gaps.len(), 2);

    // Reject the first clip's gap; only the second clip's 4s gap should actually be cut.
    app.toggle_silence_gap_accepted(0);
    app.apply_silence_review();

    assert!(app.silence_review.is_none(), "modal closes after apply");
    let track = &app.active_project().timeline().tracks[0];
    assert_eq!(
        track.clips.len(),
        3,
        "clip 1 kept whole, clip 2 split in two"
    );
    let total_duration: f64 = track.clips.iter().map(|c| c.duration_secs()).sum();
    assert_eq!(
        total_duration, 36.0,
        "only the second clip's 4s silent run was removed (20 + 20 - 4)"
    );
}

#[test]
fn apply_silence_review_with_nothing_accepted_is_a_no_op() {
    let track = test_track(1, TrackKind::Audio, vec![test_clip(1, 100.0, 5.0, 15.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![silence_review_asset()],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);
    app.begin_silence_review();
    app.toggle_silence_gap_accepted(0);

    app.apply_silence_review();

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].duration_secs(),
        10.0,
        "nothing was accepted -- the clip is untouched"
    );
}

// --- CF-01 slices 4-5: speech-edit proposal review/apply ---

/// One transcript word at the given media-relative times, with a stable id.
fn transcript_word(id: u64, text: &str, start_secs: f64, end_secs: f64) -> avcore::TranscriptWord {
    avcore::TranscriptWord {
        id,
        text: text.to_string(),
        start_secs,
        end_secs,
        confidence: 0.9,
        speaker: None,
    }
}

/// A transcript that yields exactly two non-overlapping proposals: a dead-air cut in `[0.5,2.0)`
/// and a retake (the second "the") in `[3.5,4.0)`.
fn two_proposal_document() -> avcore::TranscriptDocument {
    avcore::TranscriptDocument {
        schema_version: 1,
        asset_id: 1,
        language: Some("en".to_string()),
        words: vec![
            transcript_word(1, "hello", 0.0, 0.5),
            transcript_word(2, "world", 2.0, 2.5),
            transcript_word(3, "the", 3.0, 3.5),
            transcript_word(4, "the", 3.5, 4.0),
        ],
    }
}

/// Builds an App whose active project holds `track` + `assets` and whose asset 1's transcript
/// sidecar is already written on disk (under `project.file_path`'s cache dir). Returns the
/// `TempDir` alongside so the sidecar survives the test body.
fn transcript_proposals_app_with(
    track: Track,
    assets: Vec<MediaAsset>,
    doc: &avcore::TranscriptDocument,
) -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut project = test_project_with_tracks_and_assets(1, vec![track], assets);
    project.file_path = Some(dir.path().join("proj.ocproj"));
    let cache_dir = avcore::transcript_cache_dir_for_project(&project);
    avcore::save_transcript_document(&cache_dir, doc).unwrap();
    (test_app(vec![project], Vec::new()), dir)
}

#[test]
fn begin_transcript_proposals_stages_detected_edits_for_the_previewed_clip() {
    let doc = two_proposal_document();
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let (mut app, _dir) = transcript_proposals_app_with(track, vec![test_asset(1)], &doc);
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;

    app.begin_transcript_proposals();

    let review = app.transcript_review.as_ref().expect("review should open");
    assert_eq!(review.track_id, 1);
    assert_eq!(review.clip_id, 1);
    assert_eq!(review.proposals.len(), 2);
    assert_eq!(
        review.proposals[0].proposal.kind,
        avcore::TranscriptEditKind::DeadAir,
        "staged earliest-first"
    );
    assert_eq!(
        review.proposals[1].proposal.kind,
        avcore::TranscriptEditKind::Retake
    );
    assert!(
        review.proposals.iter().all(|e| e.accepted),
        "proposals default to accepted"
    );
}

#[test]
fn begin_transcript_proposals_without_a_previewed_clip_toasts() {
    let track = test_track(1, TrackKind::Video, vec![]);
    let (mut app, _dir) =
        transcript_proposals_app_with(track, vec![test_asset(1)], &two_proposal_document());
    app.active_project_mut().timeline_mut().playhead_secs = 0.0;

    app.begin_transcript_proposals();

    assert!(app.transcript_review.is_none());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn begin_transcript_proposals_without_a_saved_transcript_toasts() {
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let dir = tempfile::tempdir().unwrap();
    let mut project = test_project_with_tracks_and_assets(1, vec![track], vec![test_asset(1)]);
    project.file_path = Some(dir.path().join("proj.ocproj"));
    let mut app = test_app(vec![project], Vec::new());
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;

    app.begin_transcript_proposals();

    assert!(app.transcript_review.is_none());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn begin_transcript_proposals_with_nothing_detected_toasts_without_opening() {
    // Contiguous words, no fillers/repeats/gaps -- nothing to propose.
    let doc = avcore::TranscriptDocument {
        schema_version: 1,
        asset_id: 1,
        language: Some("en".to_string()),
        words: vec![
            transcript_word(1, "one", 0.0, 0.4),
            transcript_word(2, "two", 0.4, 0.8),
            transcript_word(3, "three", 0.8, 1.2),
        ],
    };
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let (mut app, _dir) = transcript_proposals_app_with(track, vec![test_asset(1)], &doc);
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;

    app.begin_transcript_proposals();

    assert!(app.transcript_review.is_none());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn toggle_transcript_proposal_flips_only_the_targeted_entry() {
    let doc = two_proposal_document();
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let (mut app, _dir) = transcript_proposals_app_with(track, vec![test_asset(1)], &doc);
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;
    app.begin_transcript_proposals();

    app.toggle_transcript_proposal(0);

    assert!(!app.transcript_review.as_ref().unwrap().proposals[0].accepted);
    assert!(app.transcript_review.as_ref().unwrap().proposals[1].accepted);
}

#[test]
fn apply_transcript_proposals_ripple_deletes_only_accepted_and_closes() {
    let doc = two_proposal_document();
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let (mut app, _dir) = transcript_proposals_app_with(track, vec![test_asset(1)], &doc);
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;
    app.begin_transcript_proposals();
    assert_eq!(app.transcript_review.as_ref().unwrap().proposals.len(), 2);

    // Reject the retake (index 1); only the 1.5s dead air should actually be cut.
    app.toggle_transcript_proposal(1);
    app.apply_transcript_proposals();

    assert!(app.transcript_review.is_none(), "modal closes after apply");
    let track = &app.active_project().timeline().tracks[0];
    assert_eq!(track.clips.len(), 2, "dead-air cut splits the clip in two");
    let total: f64 = track.clips.iter().map(|c| c.duration_secs()).sum();
    assert!(
        (total - 8.5).abs() < 1e-6,
        "only the 1.5s dead air was removed; got {total}"
    );
}

#[test]
fn apply_transcript_proposals_with_nothing_accepted_is_a_no_op() {
    let doc = two_proposal_document();
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let (mut app, _dir) = transcript_proposals_app_with(track, vec![test_asset(1)], &doc);
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;
    app.begin_transcript_proposals();
    app.toggle_transcript_proposal(0);
    app.toggle_transcript_proposal(1);

    app.apply_transcript_proposals();

    let track = &app.active_project().timeline().tracks[0];
    assert_eq!(track.clips.len(), 1);
    assert_eq!(track.clips[0].duration_secs(), 10.0);
}

#[test]
fn apply_transcript_proposals_when_the_clip_was_deleted_is_a_no_op() {
    let doc = two_proposal_document();
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let (mut app, _dir) = transcript_proposals_app_with(track, vec![test_asset(1)], &doc);
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;
    app.begin_transcript_proposals();
    assert!(app.transcript_review.is_some());
    app.active_project_mut().timeline_mut().tracks[0]
        .clips
        .clear();

    app.apply_transcript_proposals();

    assert!(
        app.transcript_review.is_none(),
        "apply still closes the modal even though nothing could be cut"
    );
}

#[test]
fn close_silence_review_discards_the_staged_review() {
    let track = test_track(1, TrackKind::Audio, vec![test_clip(1, 100.0, 5.0, 15.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![silence_review_asset()],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);
    app.begin_silence_review();

    app.close_silence_review();

    assert!(app.silence_review.is_none());
}

fn collab_bundle_scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oca_app_collab_bundle_test_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn export_collab_bundle_writes_a_zip_and_toasts_success() {
    let dir = collab_bundle_scratch_dir("export_ok");
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let output_path = dir.join("handoff.zip");

    app.export_collab_bundle(output_path.clone());

    assert!(output_path.exists());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn export_collab_bundle_toasts_on_failure_instead_of_panicking() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    // A parent directory that doesn't exist -- File::create fails.
    let output_path = PathBuf::from("/nonexistent-oca-test-dir/handoff.zip");

    app.export_collab_bundle(output_path);

    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn import_collab_bundle_opens_the_project_with_its_new_file_path() {
    let dir = collab_bundle_scratch_dir("import_ok");
    let bundle_path = dir.join("handoff.zip");
    let mut sender = test_app(vec![test_project(1, Vec::new())], Vec::new());
    sender.export_collab_bundle(bundle_path.clone());

    let mut recipient = test_app(Vec::new(), Vec::new());
    let dest_project_path = dir.join("recipient/project.ocproj");

    recipient.import_collab_bundle(bundle_path, dest_project_path.clone());

    assert_eq!(recipient.projects.len(), 1);
    assert_eq!(
        recipient.projects[0].file_path,
        Some(dest_project_path.clone())
    );
    assert_eq!(
        recipient.screen,
        Screen::Editor,
        "opens the imported project"
    );
    assert!(dest_project_path.exists());
}

#[test]
fn import_collab_bundle_toasts_on_failure_instead_of_panicking() {
    let mut app = test_app(Vec::new(), Vec::new());
    let missing_zip = PathBuf::from("/nonexistent-oca-test-dir/handoff.zip");
    let dest_project_path =
        std::env::temp_dir().join("oca_app_collab_bundle_test_never_written.ocproj");

    app.import_collab_bundle(missing_zip, dest_project_path);

    assert!(app.projects.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn apply_detected_scene_cuts_adds_numbered_chapter_markers_at_timeline_coordinates() {
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 100.0, 5.0, 15.0)]);
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());
    app.locale = Locale::En;

    app.apply_detected_scene_cuts(
        1,
        vec![
            avcore::SceneCut {
                at_secs: 8.0,
                score: 0.5,
            },
            avcore::SceneCut {
                at_secs: 12.0,
                score: 0.6,
            },
        ],
    );

    let markers = app.active_project().timeline().markers_sorted();
    assert_eq!(markers.len(), 2);
    assert_eq!(markers[0].position_secs, 103.0);
    assert_eq!(markers[0].label, "Chapter 1");
    assert_eq!(markers[0].kind, avcore::MarkerKind::Chapter);
    assert_eq!(markers[1].position_secs, 107.0);
    assert_eq!(markers[1].label, "Chapter 2");
}

#[test]
fn apply_detected_scene_cuts_numbering_continues_from_existing_chapters() {
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());
    app.locale = Locale::En;
    app.active_project_mut()
        .timeline_mut()
        .add_marker(1.0, avcore::MarkerKind::Chapter);

    app.apply_detected_scene_cuts(
        1,
        vec![avcore::SceneCut {
            at_secs: 5.0,
            score: 0.5,
        }],
    );

    let markers = app.active_project().timeline().markers_sorted();
    let new_marker = markers.iter().find(|m| m.position_secs == 5.0).unwrap();
    assert_eq!(new_marker.label, "Chapter 2");
}

#[test]
fn apply_detected_scene_cuts_is_a_no_op_for_an_unknown_clip() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.apply_detected_scene_cuts(
        404,
        vec![avcore::SceneCut {
            at_secs: 1.0,
            score: 0.5,
        }],
    );

    assert!(app.active_project().timeline().markers.is_empty());
}

#[test]
fn export_chapters_txt_writes_sorted_timecode_lines() {
    let dir = std::env::temp_dir().join("oca_app_export_chapters_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let output_path = dir.join("chapters.txt");

    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let timeline = app.active_project_mut().timeline_mut();
    let later_id = timeline.add_marker(65.0, avcore::MarkerKind::Chapter);
    timeline.marker_mut(later_id).unwrap().label = "Boss fight".to_string();
    let earlier_id = timeline.add_marker(0.0, avcore::MarkerKind::Chapter);
    timeline.marker_mut(earlier_id).unwrap().label = "Intro".to_string();
    // A non-Chapter marker should never show up in the export.
    timeline.add_marker(30.0, avcore::MarkerKind::Standard);

    app.export_chapters_txt(output_path.clone());

    let contents = std::fs::read_to_string(&output_path).unwrap();
    assert_eq!(contents, "00:00 Intro\n01:05 Boss fight\n");
}

#[test]
fn export_chapters_txt_toasts_instead_of_writing_when_no_chapters_exist() {
    let dir = std::env::temp_dir().join("oca_app_export_chapters_test_empty");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let output_path = dir.join("chapters.txt");

    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.export_chapters_txt(output_path.clone());

    assert!(!output_path.exists());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn set_track_audio_role_writes_the_role_on_the_targeted_track() {
    let track = test_track(1, TrackKind::Audio, Vec::new());
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());

    app.set_track_audio_role(1, avcore::AudioRole::Mic);

    assert_eq!(
        app.active_project().timeline().tracks[0].audio_role,
        avcore::AudioRole::Mic
    );
}

#[test]
fn set_track_audio_role_is_a_no_op_for_an_unknown_track() {
    let track = test_track(1, TrackKind::Audio, Vec::new());
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());

    app.set_track_audio_role(404, avcore::AudioRole::GameAudio);

    assert_eq!(
        app.active_project().timeline().tracks[0].audio_role,
        avcore::AudioRole::Unspecified
    );
}

#[test]
fn set_track_color_label_writes_the_label_on_the_targeted_track() {
    let track = test_track(1, TrackKind::Audio, Vec::new());
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());

    app.set_track_color_label(1, Some([229, 83, 83]));

    assert_eq!(
        app.active_project().timeline().tracks[0].color_label,
        Some([229, 83, 83])
    );
}

#[test]
fn set_track_color_label_is_a_no_op_for_an_unknown_track() {
    let track = test_track(1, TrackKind::Audio, Vec::new());
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());

    app.set_track_color_label(404, Some([229, 83, 83]));

    assert_eq!(app.active_project().timeline().tracks[0].color_label, None);
}

#[test]
fn set_clip_color_label_writes_the_label_on_the_targeted_clip_regardless_of_selection() {
    let clip = test_clip(1, 0.0, 0.0, 10.0);
    let track = test_track(1, TrackKind::Video, vec![clip]);
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());
    app.selected_clip_id = None;

    app.set_clip_color_label(1, Some([86, 156, 214]));

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].color_label,
        Some([86, 156, 214])
    );
}

#[test]
fn detach_audio_mutes_the_video_clip_and_adds_a_synced_audio_clip() {
    let video_clip = test_clip(1, 5.0, 1.0, 4.0);
    let video_track = test_track(1, TrackKind::Video, vec![video_clip]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![video_track],
            vec![test_asset(1)],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.detach_audio_from_selected_clip();

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks[0].clips[0].gain_db, *GAIN_DB_RANGE.start());
    let audio_track = timeline
        .tracks
        .iter()
        .find(|t| t.kind == TrackKind::Audio)
        .expect("an audio track should have been created");
    assert_eq!(audio_track.clips.len(), 1);
    let detached = &audio_track.clips[0];
    assert_eq!(detached.asset_id, 1);
    assert_eq!(detached.start_secs, 5.0);
    assert_eq!(detached.source_in_secs, 1.0);
    assert_eq!(detached.source_out_secs, 4.0);
    assert_eq!(detached.gain_db, 0.0);
}

#[test]
fn detach_audio_is_a_no_op_when_the_asset_has_no_audio() {
    let video_clip = test_clip(1, 5.0, 1.0, 4.0);
    let video_track = test_track(1, TrackKind::Video, vec![video_clip]);
    let mut silent_asset = test_asset(1);
    silent_asset.has_audio = false;
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![video_track],
            vec![silent_asset],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.detach_audio_from_selected_clip();

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].clips[0].gain_db, 0.0);
}

#[test]
fn detach_audio_is_a_no_op_for_a_non_video_clip() {
    let audio_clip = test_clip(1, 5.0, 1.0, 4.0);
    let audio_track = test_track(1, TrackKind::Audio, vec![audio_clip]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![audio_track],
            vec![test_asset(1)],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.detach_audio_from_selected_clip();

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].clips[0].gain_db, 0.0);
}

#[test]
fn apply_speed_ramp_splits_into_contiguous_steps_with_interpolated_speed() {
    // A clip from 10s..20s (10s long at 1.0x, the default speed_factor test_clip already uses).
    let video_clip = test_clip(1, 10.0, 0.0, 10.0);
    let video_track = test_track(1, TrackKind::Video, vec![video_clip]);
    let mut app = test_app(
        vec![test_project_with_tracks(1, vec![video_track])],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.apply_speed_ramp_to_selected_clip(0.5, 2.0, 4);

    let mut clips = app.active_project().timeline().tracks[0].clips.clone();
    clips.sort_by(|a, b| a.start_secs.total_cmp(&b.start_secs));
    assert_eq!(clips.len(), 4);

    // Speeds interpolate linearly from 0.5 to 2.0 across the 4 pieces.
    let speeds: Vec<f32> = clips.iter().map(|c| c.speed_factor).collect();
    assert_eq!(speeds, vec![0.5, 1.0, 1.5, 2.0]);

    // The pieces stay contiguous (no gaps/overlaps) even though each one's duration_secs now
    // differs from the others, since speed_factor changed per piece.
    let mut cursor = 10.0;
    for clip in &clips {
        assert_eq!(clip.start_secs, cursor);
        cursor += clip.duration_secs();
    }

    // Splitting at equal ORIGINAL (unramped) 2.5s boundaries means each piece's own trimmed
    // source range is 2.5s wide, so its post-ramp duration is 2.5 / speed_factor.
    for (clip, speed) in clips.iter().zip(&speeds) {
        assert!((clip.duration_secs() - 2.5 / *speed as f64).abs() < 1e-9);
    }
}

#[test]
fn apply_speed_ramp_is_a_no_op_with_fewer_than_two_steps() {
    let video_clip = test_clip(1, 10.0, 0.0, 10.0);
    let video_track = test_track(1, TrackKind::Video, vec![video_clip]);
    let mut app = test_app(
        vec![test_project_with_tracks(1, vec![video_track])],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.apply_speed_ramp_to_selected_clip(0.5, 2.0, 1);

    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
}

#[test]
fn apply_speed_ramp_clamps_to_speed_factor_range() {
    let video_clip = test_clip(1, 10.0, 0.0, 10.0);
    let video_track = test_track(1, TrackKind::Video, vec![video_clip]);
    let mut app = test_app(
        vec![test_project_with_tracks(1, vec![video_track])],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    // 10.0x and 0.01x are both outside SPEED_FACTOR_RANGE (0.25..=4.0).
    app.apply_speed_ramp_to_selected_clip(10.0, 0.01, 2);

    let mut clips = app.active_project().timeline().tracks[0].clips.clone();
    clips.sort_by(|a, b| a.start_secs.total_cmp(&b.start_secs));
    assert_eq!(clips[0].speed_factor, *SPEED_FACTOR_RANGE.end());
    assert_eq!(clips[1].speed_factor, *SPEED_FACTOR_RANGE.start());
}

#[test]
fn set_clip_color_label_none_clears_an_existing_label() {
    let mut clip = test_clip(1, 0.0, 0.0, 10.0);
    clip.color_label = Some([86, 156, 214]);
    let track = test_track(1, TrackKind::Video, vec![clip]);
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());

    app.set_clip_color_label(1, None);

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].color_label,
        None
    );
}

// 20 half-second buckets over a 10s asset -- matches DEFAULT_HIGHLIGHT_GRID_SECS (0.5s) exactly
// so each spiking bucket lands in its own grid cell instead of several buckets sharing one.
fn spiky_peaks(spike_range: std::ops::Range<usize>) -> Vec<(f32, f32)> {
    let mut peaks = vec![(-0.1, 0.1); 20];
    for p in &mut peaks[spike_range] {
        *p = (-0.9, 0.9);
    }
    peaks
}

fn highlight_test_project() -> Project {
    let game_asset = MediaAsset {
        waveform_peaks: Some(spiky_peaks(6..12)),
        duration_secs: 10.0,
        ..test_asset(1)
    };
    let mic_asset = MediaAsset {
        id: 2,
        waveform_peaks: Some(spiky_peaks(6..12)),
        duration_secs: 10.0,
        ..test_asset(2)
    };
    let mut game_track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    game_track.audio_role = avcore::AudioRole::GameAudio;

    let mic_clip = ClipInstance {
        asset_id: 2,
        ..test_clip(2, 0.0, 0.0, 10.0)
    };
    let mut mic_track = test_track(2, TrackKind::Audio, vec![mic_clip]);
    mic_track.audio_role = avcore::AudioRole::Mic;

    test_project_with_tracks_and_assets(1, vec![game_track, mic_track], vec![game_asset, mic_asset])
}

#[test]
fn detect_highlights_adds_a_marker_at_the_simultaneous_spike() {
    let mut app = test_app(vec![highlight_test_project()], Vec::new());

    app.detect_highlights();

    let markers = app.active_project().timeline().markers_sorted();
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].kind, avcore::MarkerKind::Highlight);
    assert_eq!(markers[0].position_secs, 3.0);
}

#[test]
fn detect_highlights_toasts_when_a_role_is_missing() {
    // Only a game-audio track, no mic track tagged.
    let track = {
        let mut t = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
        t.audio_role = avcore::AudioRole::GameAudio;
        t
    };
    let asset = MediaAsset {
        waveform_peaks: Some(spiky_peaks(3..6)),
        duration_secs: 10.0,
        ..test_asset(1)
    };
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![asset],
        )],
        Vec::new(),
    );

    app.detect_highlights();

    assert!(app.active_project().timeline().markers.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn detect_highlights_toasts_when_nothing_spikes_together() {
    let project = {
        let mut p = highlight_test_project();
        // Mic never spikes.
        p.media_library[1].waveform_peaks = Some(vec![(-0.1, 0.1); 20]);
        p
    };
    let mut app = test_app(vec![project], Vec::new());

    app.detect_highlights();

    assert!(app.active_project().timeline().markers.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

fn shorts_pack_scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oca_app_shorts_pack_test_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn shorts_pack_test_project() -> Project {
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 60.0)]);
    let mut project = test_project_with_tracks_and_assets(1, vec![track], vec![test_asset(1)]);
    project
        .timeline_mut()
        .add_marker(10.0, avcore::MarkerKind::Highlight);
    project
        .timeline_mut()
        .add_marker(40.0, avcore::MarkerKind::Highlight);
    project
}

#[test]
fn spawn_shorts_pack_queues_one_job_per_highlight_marker() {
    let dir = shorts_pack_scratch_dir("basic");
    let mut app = test_app(vec![shorts_pack_test_project()], Vec::new());

    app.spawn_shorts_pack(dir.clone());

    assert_eq!(app.export_jobs.len(), 2);
    assert!(app.export_jobs[0].title.contains("short_1"));
    assert!(app.export_jobs[1].title.contains("short_2"));
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn spawn_shorts_pack_uses_portrait_aspect_ratio() {
    let dir = shorts_pack_scratch_dir("portrait");
    let mut app = test_app(vec![shorts_pack_test_project()], Vec::new());

    app.spawn_shorts_pack(dir);

    assert_eq!(app.export_jobs[0].canvas.width, 1080);
    assert_eq!(app.export_jobs[0].canvas.height, 1920);
}

#[test]
fn spawn_shorts_pack_toasts_when_there_are_no_highlight_markers() {
    let dir = shorts_pack_scratch_dir("none");
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 60.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![test_asset(1)],
        )],
        Vec::new(),
    );

    app.spawn_shorts_pack(dir);

    assert!(app.export_jobs.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn spawn_shorts_pack_skips_a_window_landing_entirely_in_a_gap() {
    let dir = shorts_pack_scratch_dir("gap");
    // A single short clip [0, 5); a highlight far past it has no clip content in its window.
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 5.0)]);
    let mut project = test_project_with_tracks_and_assets(1, vec![track], vec![test_asset(1)]);
    project
        .timeline_mut()
        .add_marker(500.0, avcore::MarkerKind::Highlight);
    let mut app = test_app(vec![project], Vec::new());

    app.spawn_shorts_pack(dir);

    assert!(
        app.export_jobs.is_empty(),
        "the only candidate's window had no clip content"
    );
}

#[test]
fn start_watching_folder_is_a_no_op_with_no_path_set() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.start_watching_folder();

    assert!(!app.watch_folder_state.running);
}

#[test]
fn start_watching_folder_is_a_no_op_while_already_running() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.watch_folder_state.watch_path = Some(std::path::PathBuf::from("E:/records"));
    app.watch_folder_state.running = true;
    app.watch_folder_state
        .files
        .push(crate::app::WatchedFileRow {
            path: std::path::PathBuf::from("E:/records/a.mp4"),
            status: crate::app::WatchFolderFileStatus::Processing,
            percent: 40,
            error: None,
            before: None,
            after: None,
        });

    app.start_watching_folder();

    // The already-running session's file list isn't cleared by a second, ignored call.
    assert_eq!(app.watch_folder_state.files.len(), 1);
}

#[test]
fn stop_watching_folder_is_a_no_op_when_nothing_is_running() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    // Just needs to not panic without a live watch session.
    app.stop_watching_folder();

    assert!(!app.watch_folder_state.running);
}

#[test]
fn stop_watching_folder_clears_the_running_flag_and_signals_the_stop_flag() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    app.watch_folder_state.running = true;
    app.watch_folder_state.stop = Some(std::sync::Arc::clone(&stop));

    app.stop_watching_folder();

    assert!(!app.watch_folder_state.running);
    assert!(stop.load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn pump_watch_folder_inserts_a_newly_detected_file_at_the_front() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let path = std::path::PathBuf::from("E:/records/newest.mp4");
    app.watch_folder_state
        .files
        .push(crate::app::WatchedFileRow {
            path: std::path::PathBuf::from("E:/records/older.mp4"),
            status: crate::app::WatchFolderFileStatus::Done,
            percent: 100,
            error: None,
            before: None,
            after: None,
        });
    let _ = app
        .watch_folder_state
        .tx
        .send(crate::app::WatchFolderEvent::Detected(path.clone()));

    app.pump_watch_folder();

    assert_eq!(app.watch_folder_state.files.len(), 2);
    assert_eq!(app.watch_folder_state.files[0].path, path);
    assert_eq!(
        app.watch_folder_state.files[0].status,
        crate::app::WatchFolderFileStatus::Stabilizing
    );
}

#[test]
fn pump_watch_folder_applies_progress_and_completion_to_the_matching_row() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let path = std::path::PathBuf::from("E:/records/a.mp4");
    app.watch_folder_state
        .files
        .push(crate::app::WatchedFileRow {
            path: path.clone(),
            status: crate::app::WatchFolderFileStatus::Processing,
            percent: 10,
            error: None,
            before: None,
            after: None,
        });
    let before = avcore::LoudnessMetrics {
        integrated_lufs: -22.0,
        true_peak_dbtp: -3.0,
        loudness_range_lu: 8.0,
    };
    let after = avcore::LoudnessMetrics {
        integrated_lufs: -16.0,
        true_peak_dbtp: -1.0,
        loudness_range_lu: 6.0,
    };
    let _ = app
        .watch_folder_state
        .tx
        .send(crate::app::WatchFolderEvent::Progress(path.clone(), 55));
    let _ = app
        .watch_folder_state
        .tx
        .send(crate::app::WatchFolderEvent::Done {
            path: path.clone(),
            before,
            after,
        });

    app.pump_watch_folder();

    let row = &app.watch_folder_state.files[0];
    assert_eq!(row.status, crate::app::WatchFolderFileStatus::Done);
    assert_eq!(row.percent, 100);
    assert_eq!(row.before.unwrap().integrated_lufs, -22.0);
    assert_eq!(row.after.unwrap().integrated_lufs, -16.0);
}

#[test]
fn pump_watch_folder_applies_a_failure_to_the_matching_row() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let path = std::path::PathBuf::from("E:/records/a.mp4");
    app.watch_folder_state
        .files
        .push(crate::app::WatchedFileRow {
            path: path.clone(),
            status: crate::app::WatchFolderFileStatus::Processing,
            percent: 10,
            error: None,
            before: None,
            after: None,
        });
    let _ = app
        .watch_folder_state
        .tx
        .send(crate::app::WatchFolderEvent::Failed {
            path: path.clone(),
            message: "ffmpeg exited with code 1".to_string(),
        });

    app.pump_watch_folder();

    let row = &app.watch_folder_state.files[0];
    assert_eq!(row.status, crate::app::WatchFolderFileStatus::Error);
    assert_eq!(row.error.as_deref(), Some("ffmpeg exited with code 1"));
}

/// Captures everything [`App::report_error`] hands to its reporter, for asserting on the
/// delivered reports in tests.
#[derive(Default)]
struct CapturingReporter {
    reports: std::sync::Mutex<Vec<avcore::ErrorReport>>,
}

impl avcore::ErrorReporter for CapturingReporter {
    fn report(&self, report: avcore::ErrorReport) {
        self.reports.lock().unwrap().push(report);
    }
}

#[test]
fn report_error_is_a_noop_without_a_reporter() {
    let app = test_app(Vec::new(), Vec::new());
    assert!(!app.report_error(
        avcore::ErrorCode::Import,
        avcore::ErrorSeverity::Error,
        avcore::Operation::Import,
        avcore::RecoveryOutcome::RequiresUserAction,
        false,
    ));
}

#[test]
fn report_error_delivers_a_valid_sanitized_report() {
    let reporter = std::sync::Arc::new(CapturingReporter::default());
    let mut app = test_app(Vec::new(), Vec::new());
    app.error_reporter =
        Some(std::sync::Arc::clone(&reporter) as std::sync::Arc<dyn avcore::ErrorReporter>);

    assert!(app.report_error(
        avcore::ErrorCode::ExportEncode,
        avcore::ErrorSeverity::Error,
        avcore::Operation::Export,
        avcore::RecoveryOutcome::Aborted,
        false,
    ));
    assert!(app.report_error(
        avcore::ErrorCode::Import,
        avcore::ErrorSeverity::Error,
        avcore::Operation::Import,
        avcore::RecoveryOutcome::RequiresUserAction,
        true,
    ));

    let reports = reporter.reports.lock().unwrap();
    assert_eq!(reports.len(), 2);
    let encoded = &reports[0];
    let imported = &reports[1];
    assert_eq!(encoded.error_code, avcore::ErrorCode::ExportEncode);
    assert_eq!(encoded.severity, avcore::ErrorSeverity::Error);
    assert_eq!(encoded.operation, avcore::Operation::Export);
    assert_eq!(encoded.recovery_outcome, avcore::RecoveryOutcome::Aborted);
    assert!(!encoded.retried);
    assert_eq!(imported.error_code, avcore::ErrorCode::Import);
    assert!(imported.retried);
    assert_eq!(
        encoded.session_id, imported.session_id,
        "session id is stable across the launch"
    );
    assert_ne!(
        encoded.event_id, imported.event_id,
        "event id is unique per report"
    );
    assert!(avcore::validate_report(encoded).is_ok());
    for report in reports.iter() {
        let stack = report
            .sanitized_stack_trace
            .as_deref()
            .expect("a stack trace is captured for every report");
        assert!(!stack.is_empty());
        assert!(
            !avcore::contains_forbidden_content(stack),
            "the captured backtrace must not survive sanitization with forbidden content"
        );
    }
}

#[test]
fn seed_error_reporting_seeds_launch_identity_and_breadcrumb() {
    let reporter = crate::app::error_reporting::seed_error_reporting(crate::i18n::Locale::En);
    assert!(
        reporter.is_none(),
        "ER-01A holds no reporter (consent disabled)"
    );

    let capturer = std::sync::Arc::new(CapturingReporter::default());
    let mut app = test_app(Vec::new(), Vec::new());
    let capturer_for_app = std::sync::Arc::clone(&capturer);
    app.error_reporter = Some(capturer_for_app as std::sync::Arc<dyn avcore::ErrorReporter>);
    assert!(app.report_error(
        avcore::ErrorCode::ProjectSave,
        avcore::ErrorSeverity::Error,
        avcore::Operation::ProjectSave,
        avcore::RecoveryOutcome::RequiresUserAction,
        false,
    ));

    let report = capturer.reports.lock().unwrap().pop().unwrap();
    assert_eq!(report.locale, "en");
    assert!(
        report.breadcrumbs.iter().any(
            |b| matches!(b, avcore::Breadcrumb::StateTransition { state } if state == "app_started")
        ),
        "seed_error_reporting must record the launch breadcrumb"
    );
    assert!(report.release.starts_with("oca-"));
}
