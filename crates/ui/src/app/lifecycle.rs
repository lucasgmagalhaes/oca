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

//! Application construction and startup recovery.

use super::*;
use crate::theme;

impl App {
    /// Builds the initial app state: applies the theme and starts with an empty project list
    /// and export queue — every project, asset, and job comes from the user via "Novo
    /// projeto"/"Abrir projeto" and real imports, not mock data.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        crate::icons::install(&cc.egui_ctx);
        let sentinel = sentinel_path();
        let crash_detected = sentinel.exists();
        if let Some(dir) = sentinel.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Write the sentinel — deleted on clean exit via on_exit(). Survives a crash.
        let _ = std::fs::write(&sentinel, b"");
        let mut prefs = load_prefs();
        apply_bundled_model_defaults(&mut prefs);
        // Captured before `prefs` itself is moved into the struct literal below.
        let lib_panel_width = prefs.lib_panel_width;
        let props_panel_width = prefs.props_panel_width;
        let timeline_height = prefs.timeline_height;
        // Reload projects from the last session. Failures (moved/deleted files) are silently
        // skipped — the missing path will be pruned from recents next time prefs are saved.
        let projects: Vec<avcore::Project> = prefs
            .recent_project_paths
            .iter()
            .filter_map(|p| {
                let path = PathBuf::from(p);
                avcore::load_project_from_file(&path).ok().map(|mut proj| {
                    proj.file_path = Some(path);
                    proj
                })
            })
            .collect();
        let (render_tx, render_rx) = mpsc::unbounded_channel();
        let (import_tx, import_rx) = mpsc::unbounded_channel();
        let (thumbnail_tx, thumbnail_rx) = mpsc::unbounded_channel();
        let (transcribe_tx, transcribe_rx) = mpsc::unbounded_channel();
        let (auto_reframe_tx, auto_reframe_rx) = mpsc::unbounded_channel();
        let (dynamic_reframe_tx, dynamic_reframe_rx) = mpsc::unbounded_channel();
        let (nested_sequence_tx, nested_sequence_rx) = mpsc::unbounded_channel();
        let (motion_tracking_tx, motion_tracking_rx) = mpsc::unbounded_channel();
        let (voice_cleanup_preview_tx, voice_cleanup_preview_rx) = mpsc::unbounded_channel();
        let (scene_cut_detection_tx, scene_cut_detection_rx) = mpsc::unbounded_channel();
        let (matte_generation_tx, matte_generation_rx) = mpsc::unbounded_channel();
        let (privacy_blur_generation_tx, privacy_blur_generation_rx) = mpsc::unbounded_channel();
        let (tts_tx, tts_rx) = mpsc::unbounded_channel();
        let (youtube_download_tx, youtube_download_rx) = mpsc::unbounded_channel();
        let (watch_folder_tx, watch_folder_rx) = mpsc::unbounded_channel();
        let (sound_library_tx, sound_library_rx) = mpsc::unbounded_channel();
        let (telemetry_tx, telemetry_rx) = mpsc::unbounded_channel();
        telemetry::spawn_telemetry_writer(telemetry_rx, telemetry::telemetry_path());
        let telemetry_enabled_flag = Arc::new(AtomicBool::new(prefs.telemetry_enabled));
        telemetry::spawn_resource_sampler(
            telemetry_tx.clone(),
            Arc::clone(&telemetry_enabled_flag),
        );
        telemetry::spawn_gpu_sampler(telemetry_tx.clone(), Arc::clone(&telemetry_enabled_flag));
        let (update_check_tx, update_check_rx) = mpsc::unbounded_channel();
        // ER-01B: seeds the process-wide report builder and, only when the user has opted in
        // (`prefs.error_reporting_consent`, default disabled), spawns the delivery worker.
        let error_reporter =
            error_reporting::seed_error_reporting(prefs.locale, prefs.error_reporting_consent);
        // ER-01B's post-crash review offer: only set when a previous launch's panic hook left a
        // `crash_<unix>.txt` newer than the last one the user already reviewed.
        let pending_crash_review = crash_review::find_latest_unreviewed_crash(
            &crate::platform_log_dir(),
            prefs.last_reviewed_crash_unix,
        );
        let mut app = Self {
            screen: Screen::Home,
            tool: EditorTool::Select,
            properties_tab: PropertiesTab::default(),
            media_view_mode: MediaViewMode::default(),
            locale: prefs.locale,
            open_projects: OpenProjects::new(projects),
            selected_asset_id: None,
            export_jobs: export::load_queue(),
            prefs,
            error_reporter,
            telemetry_state: TelemetryState {
                telemetry_tx,
                telemetry_enabled_flag,
                last_preview_frame_telemetry: None,
            },
            render_tx,
            render_rx,
            active_renders: HashMap::new(),
            export_job_started_at: HashMap::new(),
            export_preview_cache: None,
            nested_sequence_render_cache: HashMap::new(),
            nested_sequence_render_state: NestedSequenceRenderState {
                nested_sequence_tx,
                nested_sequence_rx,
                nested_sequence_rendering_ids: HashSet::new(),
                nested_sequence_last_result: HashMap::new(),
                nested_sequence_last_input: HashMap::new(),
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
                youtube_modal_format: YoutubeFormatChoice::Mp4,
                youtube_modal_mp4_quality: avcore::Mp4Quality::P720,
                youtube_modal_mp3_bitrate: avcore::Mp3Bitrate::K192,
                youtube_downloading: false,
                youtube_download_progress: 0.0,
                youtube_download_error: None,
                youtube_download_cancel: None,
            },
            watch_folder_state: WatchFolderState {
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
            lib_panel_width,
            props_panel_width,
            timeline_height,
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
            crash_detected,
            pending_crash_review,
            pending_export_conflict: None,
            renaming_project: None,
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
        };
        if !app.prefs.sound_library_path.is_empty() {
            app.rescan_sound_library();
        }
        if !app.open_projects.is_empty() {
            app.load_panel_layout_for_active_project();
        }
        app.spawn_update_check();
        app
    }
}
