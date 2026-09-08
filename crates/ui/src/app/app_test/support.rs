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

//! Shared fixtures for application tests.

use avcore::ExportJobStatus;

use super::*;

pub(super) fn test_project(id: u64, assets: Vec<MediaAsset>) -> Project {
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
        recent_asset_ids: Vec::new(),
    }
}

pub(super) fn test_project_with_tracks(id: u64, tracks: Vec<Track>) -> Project {
    let mut project = test_project(id, Vec::new());
    project.timeline_mut().tracks = tracks;
    project
}

pub(super) fn test_project_with_tracks_and_assets(
    id: u64,
    tracks: Vec<Track>,
    assets: Vec<MediaAsset>,
) -> Project {
    let mut project = test_project(id, assets);
    project.timeline_mut().tracks = tracks;
    project
}

pub(super) fn test_track(id: u64, kind: TrackKind, clips: Vec<ClipInstance>) -> Track {
    Track {
        id,
        name: format!("Track {id}"),
        kind,
        clips,

        text_clips: vec![],
        shape_clips: vec![],

        visible: true,
        audio_role: AudioRole::Unspecified,
        locked: false,
        color_label: None,
    }
}

pub(in crate::app) fn test_clip(
    id: u64,
    start_secs: f64,
    source_in_secs: f64,
    source_out_secs: f64,
) -> ClipInstance {
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
        voice_cleanup_enabled: false,
        voice_cleanup_noise_floor_db: -30.0,
        voice_cleanup_compressor_threshold_db: -18.0,
        voice_cleanup_compressor_ratio: 3.0,
        voice_cleanup_ceiling_linear: 0.95,
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
        blend_mode: avcore::timeline::BlendMode::Normal,
        anchor_x: 0.5,
        anchor_y: 0.5,
        reframe_seed_point: None,
        privacy_blur_enabled: false,
        privacy_blur_mask_path: String::new(),
        privacy_blur_sigma: 15.0,
        privacy_blur_seed_vertices: Vec::new(),
        privacy_blur_seed_center_x_frac: 0.0,
        privacy_blur_seed_center_y_frac: 0.0,
    }
}

pub(super) fn test_composite_clip(id: u64, start_secs: f64, composite_id: u64) -> ClipInstance {
    ClipInstance {
        composite_id: Some(composite_id),
        color_label: None,
        ..test_clip(id, start_secs, 0.0, 10.0)
    }
}

pub(super) fn test_asset(id: u64) -> MediaAsset {
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
        favorited: false,
    }
}

pub(super) fn test_asset_with_kind(id: u64, kind: MediaKind) -> MediaAsset {
    MediaAsset {
        kind,
        ..test_asset(id)
    }
}

pub(super) fn test_canvas() -> avcore::Canvas {
    avcore::Canvas {
        width: 1920,
        height: 1080,
        fps_num: 30,
        fps_den: 1,
        bit_rate_bps: 8_000_000,
    }
}

pub(super) fn test_job(id: u64, status: ExportJobStatus) -> ExportJob {
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

pub(super) fn test_app(projects: Vec<Project>, export_jobs: Vec<ExportJob>) -> App {
    let (render_tx, render_rx) = mpsc::unbounded_channel();
    let (import_tx, import_rx) = mpsc::unbounded_channel();
    let (thumbnail_tx, thumbnail_rx) = mpsc::unbounded_channel();
    let (transcribe_tx, transcribe_rx) = mpsc::unbounded_channel();
    let (auto_reframe_tx, auto_reframe_rx) = mpsc::unbounded_channel();
    let (dynamic_reframe_tx, dynamic_reframe_rx) = mpsc::unbounded_channel();
    let (motion_tracking_tx, motion_tracking_rx) = mpsc::unbounded_channel();
    let (voice_cleanup_preview_tx, voice_cleanup_preview_rx) = mpsc::unbounded_channel();
    let (scene_cut_detection_tx, scene_cut_detection_rx) = mpsc::unbounded_channel();
    let (matte_generation_tx, matte_generation_rx) = mpsc::unbounded_channel();
    let (privacy_blur_generation_tx, privacy_blur_generation_rx) = mpsc::unbounded_channel();
    let (tts_tx, tts_rx) = mpsc::unbounded_channel();
    let (youtube_download_tx, youtube_download_rx) = mpsc::unbounded_channel();
    let (sound_library_tx, sound_library_rx) = mpsc::unbounded_channel();
    let (telemetry_tx, _telemetry_rx) = mpsc::unbounded_channel();
    let (update_check_tx, update_check_rx) = mpsc::unbounded_channel();
    let (watch_folder_tx, watch_folder_rx) = mpsc::unbounded_channel();
    App {
        screen: Screen::Home,
        tool: EditorTool::Select,
        properties_tab: PropertiesTab::default(),
        media_view_mode: MediaViewMode::default(),
        locale: Locale::PtBr,
        open_projects: OpenProjects::new(projects),
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
        export_job_started_at: HashMap::new(),
        export_preview_cache: None,
        nested_sequence_render_cache: std::collections::HashMap::new(),
        nested_sequence_render_state: {
            let (nested_sequence_tx, nested_sequence_rx) = mpsc::unbounded_channel();
            NestedSequenceRenderState {
                nested_sequence_tx,
                nested_sequence_rx,
                nested_sequence_rendering_ids: std::collections::HashSet::new(),
                nested_sequence_last_result: std::collections::HashMap::new(),
                nested_sequence_last_input: std::collections::HashMap::new(),
            }
        },
        preview_state: PreviewState::default(),
        import_state: ImportState {
            import_tx,
            import_rx,
            pending_imports: 0,
            next_import_token: 0,
            pending_enrichment: HashMap::new(),
            auto_add_to_timeline: HashSet::new(),
            auto_add_to_new_track: HashSet::new(),
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
        dynamic_reframe_state: DynamicReframeState {
            dynamic_reframe_tx,
            dynamic_reframe_rx,
            dynamic_reframing_clip_id: None,
        },
        shorts_pack_reframe_state: None,
        pending_graphic_template_apply: None,
        motion_tracking_state: MotionTrackingState {
            motion_tracking_tx,
            motion_tracking_rx,
            motion_tracking_clip_id: None,
        },
        voice_cleanup_preview_state: VoiceCleanupPreviewState {
            tx: voice_cleanup_preview_tx,
            rx: voice_cleanup_preview_rx,
            rendering_clip_id: None,
            result: None,
            player: None,
            player_is_processed: false,
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
        privacy_blur_generation_state: PrivacyBlurGenerationState {
            privacy_blur_generation_tx,
            privacy_blur_generation_rx,
            privacy_blur_generating_clip_id: None,
        },
        privacy_blur_region: PrivacyBlurRegionState {
            privacy_blur_center_x: 0.5,
            privacy_blur_center_y: 0.5,
            privacy_blur_width: 0.2,
            privacy_blur_height: 0.2,
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
        timeline_thumbnail_zoom_settled_px_per_sec: 4.0,
        timeline_zoom_changed_at: None,
        timeline_pan_px: 0.0,
        collapsed_track_ids: std::collections::HashSet::new(),
        snap_enabled: true,
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
        media_search: String::new(),
        clipboard_clip: None,
        formatting_clipboard: None,
        multi_selected_clip_ids: HashSet::new(),
        active_multicam_group_id: None,
        media_filter: MediaLibraryFilter::All,
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
        pending_crash_review: None,
        renaming_project: None::<(usize, String, String)>,
        renaming_sequence: None,
        deleting_sequence: None,
        renaming_track: None,
        deleting_track: None,
        text_tool_pending_empty_clip_id: None,
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
