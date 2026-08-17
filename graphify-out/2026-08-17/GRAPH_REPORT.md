# Graph Report - oca  (2026-08-17)

## Corpus Check
- 157 files · ~226,280 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 2159 nodes · 4460 edges · 103 communities (96 shown, 7 thin omitted)
- Extraction: 94% EXTRACTED · 6% INFERRED · 0% AMBIGUOUS · INFERRED: 282 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `2d3d328c`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- keyframe_test.rs
- persistence_test.rs
- app_test.rs
- avbridge/src/lib.rs
- timeline_test.rs
- PreviewError
- timeline.rs
- encode_write_packet
- overlay_render_test.rs
- Plano de Execução — oca (PacoPaçoca)
- track_with
- ensure_proxy
- parse_loudnorm_stderr
- App
- Task Breakdown — Fase 1 (Motor central)
- Reviewer Agent
- record_event
- Manager Agent
- task_id: impl-004b
- Docs Specialist
- Git Agent
- Impl Specialist
- Test Specialist
- src/youtube_download.rs
- workflow/README.md
- Required checklist
- main.rs
- Commit Plan — Fase 1
- test_layer_transform.py
- generate_waveform
- setup-python-runtime.sh script
- transcribe
- run
- download_whisper_model
- Security Fix: FFmpeg Download Verification
- test_project
- avbridge_encode_timeline_export_multi
- ClipInstance
- LibraryTrack
- timeline_export_multi_test.rs
- track_region
- timeline_with
- src/background_removal.rs
- text_metrics_test.rs
- App
- theme.rs
- Locale
- Update Process
- src/auto_reframe.rs
- test_canvas
- Keyframe
- MediaAsset
- bridge_internal.h
- App
- pcm_extract.c
- ShapeKind
- open_input
- Project
- avbridge_generate_waveform
- avbridge_measure_loudness
- gpu_encoder.c
- render.rs
- App
- audio_mix.c
- text_to_speech.rs
- motion_track_one
- render_timeline_export
- preview_test.rs
- Sequence
- core/build.rs
- subtitles.rs
- persistence.rs
- tests/background_removal_test.rs
- UpdateCheckError
- ExportJob
- generate_tts_one
- enum_combo
- Watch-Gameplay.ps1
- generate_matte_one
- project_test.rs
- core/tests/probe_test.rs
- yt2mp3.sh
- auto_reframe_one
- App
- yt2mp4.sh
- .pump_preview_frame
- parsing.rs
- fonts/README.md
- ytbridge/build.rs
- App
- Position
- test_asset

## God Nodes (most connected - your core abstractions)
1. `test_app()` - 185 edges
2. `App` - 70 edges
3. `clip()` - 62 edges
4. `ClipInstance` - 60 edges
5. `track_with()` - 37 edges
6. `Project` - 34 edges
7. `MediaAsset` - 32 edges
8. `App` - 30 edges
9. `fixture()` - 29 edges
10. `render_timeline_export()` - 27 edges

## Surprising Connections (you probably didn't know these)
- `split_keyframes_at_drops_keyframes_that_land_on_the_other_side()` --calls--> `split_keyframes_at()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `split_keyframes_at_is_empty_for_an_empty_input()` --calls--> `split_keyframes_at()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `scale_filter_expr_builds_a_geq_expression_for_an_animated_ramp()` --calls--> `scale_filter_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `scale_filter_expr_is_static_crop_scale_for_a_single_non_unity_keyframe()` --calls--> `scale_filter_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `rotation_filter_angle_expr_converts_a_single_keyframe_to_radians()` --calls--> `rotation_filter_angle_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs

## Import Cycles
- 2-file cycle: `crates/ui/src/app/mod.rs -> crates/ui/src/i18n.rs -> crates/ui/src/app/mod.rs`

## Communities (103 total, 7 thin omitted)

### Community 0 - "keyframe_test.rs"
Cohesion: 0.08
Nodes (9): opacity_alpha_ramp_expr_clamps_a_single_keyframe(), position_overlay_xy_expr_builds_a_t_based_ramp_for_an_animated_axis(), position_overlay_xy_expr_scales_a_single_keyframe_by_canvas_dimensions(), rotation_filter_angle_expr_converts_a_single_keyframe_to_radians(), rotation_filter_angle_expr_uses_t_for_an_animated_ramp(), scale_filter_expr_builds_a_geq_expression_for_an_animated_ramp(), scale_filter_expr_is_static_crop_scale_for_a_single_non_unity_keyframe(), split_keyframes_at_drops_keyframes_that_land_on_the_other_side() (+1 more)

### Community 1 - "persistence_test.rs"
Cohesion: 0.23
Nodes (20): from_ocproj_bytes(), to_ocproj_bytes(), as_array_field_mut(), as_map_field_mut(), empty_project(), file_path_is_not_part_of_the_serialized_bytes(), fixture_project(), from_ocproj_bytes_rejects_an_unsupported_version() (+12 more)

### Community 2 - "app_test.rs"
Cohesion: 0.02
Nodes (161): add_asset_to_timeline_creates_a_track_and_appends_a_clip_when_none_exists(), add_asset_to_timeline_creates_an_audio_track_for_an_audio_asset(), add_asset_to_timeline_is_a_no_op_for_an_unknown_asset_id(), add_opacity_marker_at_playhead_defaults_to_fully_opaque_with_no_existing_keyframes(), add_opacity_marker_at_playhead_is_a_no_op_outside_the_clips_own_span(), add_opacity_marker_at_playhead_is_a_no_op_when_nothing_is_selected(), add_opacity_marker_at_playhead_preserves_the_currently_interpolated_value(), add_sequence_appends_a_named_tab_and_switches_to_it() (+153 more)

### Community 3 - "avbridge/src/lib.rs"
Cohesion: 0.05
Nodes (86): c_char, c_int, c_longlong, apply_shape_overlays(), apply_text_overlays(), AudioMixError, AudioMixOutcome, encode_export() (+78 more)

### Community 4 - "timeline_test.rs"
Cohesion: 0.06
Nodes (45): apply_formatting_overwrites_all_effect_fields(), clip(), formatting_roundtrip_preserves_all_fields(), has_vignette_is_true_above_zero(), is_color_filtered_is_true_for_any_filter_but_none(), is_cropped_is_true_when_any_crop_field_differs_from_the_full_frame(), is_masked_is_true_for_any_shape_but_none(), negative_gain_db_scales_linear_gain_below_unity() (+37 more)

### Community 5 - "PreviewError"
Cohesion: 0.12
Nodes (33): AppSink, BoolError, attach_audio_mix_branch(), build_audio_filter_bin(), build_audio_mix_output(), build_chroma_key_element(), build_composite_branch(), build_mask_shape_stage() (+25 more)

### Community 6 - "timeline.rs"
Cohesion: 0.10
Nodes (10): text_clip(), ClipFormatting, ColorFilter, LayerTemplate, MaskShape, String, Vec, TextClip (+2 more)

### Community 7 - "encode_write_packet"
Cohesion: 0.21
Nodes (22): AVStream, avbridge_encode_export(), EncodeStatus, ProgressCallback, AudioFilterChain, AVCodecContext, AVFormatContext, AVFrame (+14 more)

### Community 8 - "overlay_render_test.rs"
Cohesion: 0.10
Nodes (37): blend_pixel(), draw_laid_out_text(), draw_rounded_background(), draw_text_segment_onto(), ellipse_inside(), a_local_time_covered_by_no_word_leaves_only_the_base_color(), circle_mask_center_is_visible_and_corners_are_masked(), ellipse_center_pixel_is_opaque_and_corners_are_transparent() (+29 more)

### Community 9 - "Plano de Execução — oca (PacoPaçoca)"
Cohesion: 0.06
Nodes (29): Approach, Architecture, Commands, graphify, What this is, Approach, Architecture, Commands (+21 more)

### Community 10 - "track_with"
Cohesion: 0.06
Nodes (34): clip_at_finds_the_clip_covering_a_position(), clip_at_is_exclusive_of_a_clips_end(), clip_at_is_inclusive_of_a_clips_start(), clip_at_is_none_for_a_gap_between_clips(), clip_mut_finds_a_clip_by_id(), clip_mut_returns_none_for_an_unknown_id(), move_clip_is_a_no_op_for_a_negative_position(), move_clip_is_a_no_op_for_an_unknown_clip_id() (+26 more)

### Community 11 - "ensure_proxy"
Cohesion: 0.08
Nodes (31): cache_dir_for_project(), ensure_proxy(), is_up_to_date(), PreviewQuality, proxy_path_for(), a_proxy_newer_than_its_source_is_up_to_date(), a_proxy_older_than_its_source_is_not_up_to_date(), missing_proxy_is_not_up_to_date() (+23 more)

### Community 12 - "parse_loudnorm_stderr"
Cohesion: 0.14
Nodes (20): extract_first_json_object(), LoudnessError, LoudnormReport, measure_loudness(), parse_loudnorm_stderr(), Display, Error, Formatter (+12 more)

### Community 13 - "App"
Cohesion: 0.06
Nodes (9): TrackKind, App, App, create_new_track(), next_clip_id(), resolve_or_create_track(), FnOnce, Option (+1 more)

### Community 14 - "Task Breakdown — Fase 1 (Motor central)"
Cohesion: 0.12
Nodes (15): Task Breakdown — Fase 1 (Motor central), task_id: chore-001, task_id: chore-002, task_id: docs-001, task_id: docs-002, task_id: impl-002a, task_id: impl-002b, task_id: impl-003 (+7 more)

### Community 15 - "Reviewer Agent"
Cohesion: 0.29
Nodes (6): Constraints, Decision rules, Expected inputs, Required output: `review_report.md`, Reviewer Agent, Role

### Community 16 - "record_event"
Cohesion: 0.08
Nodes (31): record_event(), ResourceSampler, rotate_if_oversized(), Default, Display, Error, Formatter, Path (+23 more)

### Community 17 - "Manager Agent"
Cohesion: 0.20
Nodes (10): Behavior rules, `commit_plan.md`, Expected inputs, Granularity rules (HARD RULES), Manager Agent, Project stack, Required outputs, Role (+2 more)

### Community 18 - "task_id: impl-004b"
Cohesion: 0.22
Nodes (8): Bugs found and fixed during implementation (not just typos — genuine correctness issues), Checklist, Decision, Diff sizes, Review Report, task_id: impl-004b, Verification performed (real files, not just `cargo test` passing), Why this is flagged, not silently split

### Community 19 - "Docs Specialist"
Cohesion: 0.25
Nodes (7): CHANGELOG.md — expected format, Constraints, Docs Specialist, Expected inputs, Output, Required workflow, Role

### Community 20 - "Git Agent"
Cohesion: 0.25
Nodes (8): Commit message format, Expected inputs, Final validation (after all commits), Git Agent, HARD RULES — never violate, In case of problems, Required workflow, Role

### Community 21 - "Impl Specialist"
Cohesion: 0.25
Nodes (8): Code rules — C, Code rules — Rust, Constraints, Expected inputs, Impl Specialist, Output, Required workflow, Role

### Community 22 - "Test Specialist"
Cohesion: 0.29
Nodes (7): Constraints, Expected inputs, Output, Required workflow, Role, Test Specialist, What makes a good test here

### Community 23 - "src/youtube_download.rs"
Cohesion: 0.11
Nodes (18): download_youtube(), Event, is_yt_dlp_available(), Mp3Bitrate, Mp4Quality, AtomicBool, Display, Error (+10 more)

### Community 24 - "workflow/README.md"
Cohesion: 0.22
Nodes (4): Agents (in pipeline order), Hard rules across the pipeline, How to run it, Workflow — multi-agent feature pipeline

### Community 25 - "Required checklist"
Cohesion: 0.29
Nodes (7): 1. Diff size (granularity check), 2. acceptance_criteria coverage, 3. Quality — Rust, 4. Quality — C, 5. Consistency with the plan, 6. Documentation, Required checklist

### Community 26 - "main.rs"
Cohesion: 0.48
Nodes (6): init_logging(), install_panic_hook(), main(), platform_log_dir(), PathBuf, Result

### Community 32 - "test_layer_transform.py"
Cohesion: 0.15
Nodes (23): oca_window(), poll_for_descendants(), Repeatedly re-queries `window`'s accessibility tree for a descendant control…, Launches a fresh ui.exe, waits for its main window, yields it, then tears it…, _wait_for_window(), _set_reframe_model_path(), test_auto_reframe_button_runs_detection_without_crashing(), test_importing_a_file_adds_it_to_the_library_quickly() (+15 more)

### Community 34 - "generate_waveform"
Cohesion: 0.19
Nodes (14): generate_waveform(), Display, Error, Formatter, Path, Result, Vec, WaveformError (+6 more)

### Community 36 - "transcribe"
Cohesion: 0.07
Nodes (38): abort_trampoline(), collect_segments(), collect_words(), Arc, AtomicBool, c_void, Display, Error (+30 more)

### Community 37 - "run"
Cohesion: 0.28
Nodes (13): Bound, Args, emit(), Event, extract_final_path(), main(), Option, PathBuf (+5 more)

### Community 38 - "download_whisper_model"
Cohesion: 0.11
Nodes (27): BackgroundRemovalModel, download_background_removal_model(), download_file(), download_reframe_model(), download_tts_voice(), download_whisper_model(), DownloadError, DownloadOutcome (+19 more)

### Community 39 - "Security Fix: FFmpeg Download Verification"
Cohesion: 0.12
Nodes (16): 1. `.github/workflows/ci.yml`, 2. `.github/FFMPEG_UPDATE.md` (new file), After, Attack Surface Reduction, Before, Changes Made, Environment Variables (lines 9-16), Implementation Notes (+8 more)

### Community 40 - "test_project"
Cohesion: 0.14
Nodes (14): add_and_open_project_appends_and_opens_it(), add_asset_to_timeline_appends_after_whatever_is_already_on_the_matching_track(), begin_save_layer_template_snapshots_the_multi_selection_ordered_by_track_then_start(), confirm_apply_layer_template_creates_one_clip_per_filled_layer_with_its_formatting(), confirm_apply_layer_template_puts_each_layer_on_its_own_new_track(), confirm_apply_layer_template_skips_layers_left_without_an_asset(), load_panel_layout_applies_the_active_projects_saved_layout(), load_panel_layout_is_a_no_op_under_per_user_scope() (+6 more)

### Community 41 - "avbridge_encode_timeline_export_multi"
Cohesion: 0.19
Nodes (22): AVFilterInOut, add_graph_input(), advance_overlay_decoder(), avbridge_encode_timeline_export_multi(), build_overlay_vfilter(), build_vfilter_descr(), AVCodecContext, AVFilterContext (+14 more)

### Community 42 - "ClipInstance"
Cohesion: 0.10
Nodes (4): ClipInstance, Option, Track, clip()

### Community 43 - "LibraryTrack"
Cohesion: 0.14
Nodes (13): LibraryTrack, Path, PathBuf, String, Vec, scan_library_dir(), SoundCategory, fixture() (+5 more)

### Community 44 - "timeline_export_multi_test.rs"
Cohesion: 0.27
Nodes (28): audio_asset(), cancelling_mid_multi_track_export_reports_cancelled(), clip(), fixture(), layer_scale_at_native_size_adds_no_filter_stage(), layer_scale_is_appended_to_the_overlay_tracks_video_filter(), overlay_track_animated_opacity_keyframes_composites_without_error(), overlay_track_animated_position_keyframes_composites_without_error() (+20 more)

### Community 45 - "track_region"
Cohesion: 0.21
Nodes (17): extract_patch(), GrayFrame, a_single_frame_produces_one_clamped_position(), frame_with_block(), initial_center_near_the_edge_is_clamped_so_the_template_fits(), keyframes_ride_the_delta_from_the_first_tracked_frame(), stays_put_when_the_block_does_not_move(), template_width_and_height_clamp_independently_to_each_frame_dimension() (+9 more)

### Community 46 - "timeline_with"
Cohesion: 0.25
Nodes (8): move_clip_to_track_is_a_no_op_across_mismatched_kinds(), move_clip_to_track_is_a_no_op_for_a_negative_position(), move_clip_to_track_is_a_no_op_for_an_unknown_target_track(), move_clip_to_track_relocates_the_clip_to_a_same_kind_track(), Vec, timeline_clip_mut_finds_a_clip_across_tracks(), timeline_clip_mut_returns_none_for_an_unknown_id(), timeline_with()

### Community 47 - "src/background_removal.rs"
Cohesion: 0.13
Nodes (26): approx(), model_input_size_leaves_frames_already_spanning_target_alone_then_rounds(), model_input_size_rounds_down_to_multiple_of_32(), model_input_size_scales_constrained_side_toward_target(), model_input_size_upscales_tiny_frames_toward_target(), preprocess_normalizes_black_pixel_to_negative_one(), preprocess_normalizes_white_pixel_to_one(), resize_matte_downsamples_without_out_of_bounds() (+18 more)

### Community 48 - "text_metrics_test.rs"
Cohesion: 0.15
Nodes (20): bundled_font(), parse_font(), Font, Option, Vec, every_bundled_font_face_parses_and_is_cached(), selecting_another_family_changes_text_metrics(), text_width_px_grows_with_more_characters() (+12 more)

### Community 49 - "App"
Cohesion: 0.09
Nodes (3): App, String, Vec

### Community 50 - "theme.rs"
Cohesion: 0.06
Nodes (53): card_frame(), Frame, property_block(), property_section(), property_toggle(), FnOnce, Ui, Ui (+45 more)

### Community 51 - "Locale"
Cohesion: 0.09
Nodes (47): Recency, Screen, job_detail_line(), Locale, nav_label(), recency_label(), String, screen_title() (+39 more)

### Community 52 - "Update Process"
Cohesion: 0.13
Nodes (14): 1. Choose a Release, 2. Download Assets and Compute Checksums, 3. Update Workflow Environment Variables, 4. Update Cache Keys, 5. Test the Changes, Checksum Mismatch Error, Download Failures, FFmpeg Update Guide (+6 more)

### Community 53 - "src/auto_reframe.rs"
Cohesion: 0.12
Nodes (24): approx(), main_subject_picks_highest_score(), nms_collapses_overlapping_boxes(), no_subject_falls_back_to_center_crop(), same_aspect_ratio_keeps_full_frame(), subject_near_edge_clamps_without_overflow(), subject_off_center_shifts_crop_toward_it(), wider_target_keeps_full_width() (+16 more)

### Community 54 - "test_canvas"
Cohesion: 0.29
Nodes (7): ocqueue_file_atomically_replaces_the_previous_snapshot(), pending_export_conflict_overwrite_queues_with_the_original_path(), queue_export_appends_a_queued_job_with_the_next_id(), queue_export_starts_at_one_when_no_jobs_exist(), queued_job_keeps_the_sequence_export_snapshot_after_settings_change(), test_canvas(), test_job()

### Community 55 - "Keyframe"
Cohesion: 0.33
Nodes (16): clamp_scale(), evaluate_keyframes(), Keyframe, split_keyframes_at_rescales_and_inserts_a_matching_boundary_point(), opacity_alpha_ramp_expr(), piecewise_expr(), position_overlay_xy_expr(), rotation_filter_angle_expr() (+8 more)

### Community 56 - "MediaAsset"
Cohesion: 0.10
Nodes (20): format_timecode(), MediaAsset, MediaKind, Option, PathBuf, String, Vec, ProbedMedia (+12 more)

### Community 57 - "bridge_internal.h"
Cohesion: 0.20
Nodes (7): AVCodec, loudness_log_callback(), avbridge_encode_matte_video(), AVFrame, scale_video_frame(), MatteStatus, va_list

### Community 58 - "App"
Cohesion: 0.14
Nodes (6): ShapeClip, shape_clip(), App, Option, PathBuf, Vec

### Community 59 - "pcm_extract.c"
Cohesion: 0.29
Nodes (10): append_pcm_frame(), avbridge_extract_pcm_16k_mono(), AudioFilterChain, AVCodecContext, AVFrame, drain_pcm_frame(), init_pcm_filter_chain(), pcm_buffer_reserve() (+2 more)

### Community 60 - "ShapeKind"
Cohesion: 0.07
Nodes (41): build_shape_filter_desc(), ellipse_inside_expr(), inside_expr(), polygon_inside_expr(), rgb_to_ycbcr(), String, build_shape_filter_desc_bakes_in_the_visible_time_window(), build_shape_filter_desc_ellipse_uses_lte_not_mod() (+33 more)

### Community 61 - "open_input"
Cohesion: 0.17
Nodes (11): AVFormatContext, open_input(), avbridge_probe(), avbridge_remux_copy(), avbridge_apply_shape_overlays(), TextOverlayStatus, avbridge_apply_text_overlays(), TextOverlayStatus (+3 more)

### Community 62 - "Project"
Cohesion: 0.16
Nodes (11): default_target_lufs(), PanelLayout, Project, Default, Option, PathBuf, Self, String (+3 more)

### Community 63 - "avbridge_generate_waveform"
Cohesion: 0.31
Nodes (9): accumulate_waveform_frame(), avbridge_generate_waveform(), AudioFilterChain, AVCodecContext, AVFrame, drain_waveform_frame(), init_waveform_filter_chain(), WaveformAccumulator (+1 more)

### Community 64 - "avbridge_measure_loudness"
Cohesion: 0.33
Nodes (7): avbridge_measure_loudness(), AudioFilterChain, AVCodecContext, AVFrame, drain_measure_frame(), init_measure_filter_chain(), LoudnessStatus

### Community 65 - "gpu_encoder.c"
Cohesion: 0.60
Nodes (5): AVRational, AVCodecContext, open_video_encoder(), pix_fmt_for_encoder_name(), try_open_encoder()

### Community 66 - "render.rs"
Cohesion: 0.19
Nodes (33): Canvas, ClipSegment, String, ShapeSegment, apply_audio_mix_pass(), apply_export_aspect_ratio(), apply_shape_overlay_pass(), apply_text_overlay_pass() (+25 more)

### Community 68 - "App"
Cohesion: 0.05
Nodes (50): App, AutoReframeEvent, AvailableUpdate, BindableAction, default_add_opacity_marker_binding(), default_lib_panel_width(), default_props_panel_width(), default_timeline_height() (+42 more)

### Community 69 - "audio_mix.c"
Cohesion: 0.24
Nodes (14): AudioMixStatus, avbridge_mix_audio_timeline(), avbridge_mux_video_audio(), build_mix_graph(), AVFilterContext, AVFilterGraph, AVFormatContext, AVPacket (+6 more)

### Community 70 - "text_to_speech.rs"
Cohesion: 0.11
Nodes (32): default_length_scale(), default_noise_scale(), default_noise_w(), io_from_hound(), load_voice_config(), phonemes_to_ids(), PiperAudioConfig, PiperEspeakConfig (+24 more)

### Community 71 - "motion_track_one"
Cohesion: 0.25
Nodes (4): App, motion_track_one(), Path, Vec

### Community 73 - "render_timeline_export"
Cohesion: 0.34
Nodes (20): probe_media(), render_timeline_export(), animated_rotation_keyframes_export_without_error(), animated_scale_keyframes_export_without_error(), cancelling_mid_timeline_export_reports_cancelled(), clip(), deflicker_exports_without_error(), fade_transition_exports_without_error() (+12 more)

### Community 74 - "preview_test.rs"
Cohesion: 0.16
Nodes (30): a_cropped_clip_shrinks_the_decoded_frame(), a_fade_transition_clip_still_opens_and_decodes(), a_flipped_clip_still_opens_and_decodes(), a_gained_clip_opens_with_a_real_audio_sink_and_still_decodes_video(), a_neutral_clip_applies_no_filter_bin_and_frame_size_is_unchanged(), a_pixelized_clip_still_opens_and_decodes_at_full_size(), a_shaken_clip_still_opens_and_decodes_at_full_size(), a_slide_transition_clip_still_opens_and_decodes_at_full_size() (+22 more)

### Community 75 - "Sequence"
Cohesion: 0.23
Nodes (19): Sequence, resolve_shape_segments(), resolve_text_segments(), cancelling_mid_render_reports_cancelled(), fixture(), rejects_a_missing_source(), renders_and_normalizes_loudness_toward_target(), resolve_shape_segments_returns_empty_when_no_shape_track_exists() (+11 more)

### Community 76 - "core/build.rs"
Cohesion: 0.44
Nodes (8): copy_dir_recursive(), find_espeak_data_source(), find_profile_dir(), main(), Option, Path, PathBuf, Result

### Community 77 - "subtitles.rs"
Cohesion: 0.29
Nodes (10): export_srt(), exports_two_clips_as_sequential_numbered_entries(), format_srt_timestamp(), ignores_non_text_tracks(), orders_entries_by_start_time_regardless_of_input_order(), returns_an_empty_string_for_a_timeline_with_no_text_clips(), String, Vec (+2 more)

### Community 78 - "persistence.rs"
Cohesion: 0.23
Nodes (17): from_framed_bytes(), from_ocqueue_bytes(), load_project_from_file(), PersistError, Display, Error, Formatter, Path (+9 more)

### Community 79 - "tests/background_removal_test.rs"
Cohesion: 0.32
Nodes (5): mask_cache_dir_falls_back_to_a_temp_dir_for_an_unsaved_project(), mask_cache_dir_is_a_hidden_sibling_of_a_saved_project_file(), Option, PathBuf, test_project()

### Community 80 - "UpdateCheckError"
Cohesion: 0.24
Nodes (9): fetch_latest_release(), GithubRelease, LatestRelease, Display, Error, Formatter, Result, String (+1 more)

### Community 81 - "ExportJob"
Cohesion: 0.06
Nodes (38): Box, Condvar, AudioSegment, PathBuf, ExportAspectRatio, ExportJob, ExportJobStatus, String (+30 more)

### Community 82 - "generate_tts_one"
Cohesion: 0.22
Nodes (6): App, generate_tts_one(), Path, PathBuf, Result, String

### Community 84 - "enum_combo"
Cohesion: 0.33
Nodes (5): enum_combo(), Fn, String, T, Ui

### Community 85 - "Watch-Gameplay.ps1"
Cohesion: 0.47
Nodes (3): Format-Arg(), Get-AudioAnalysis(), Invoke-FfmpegWithProgress()

### Community 86 - "generate_matte_one"
Cohesion: 0.25
Nodes (6): App, generate_matte_one(), Path, PathBuf, Result, String

### Community 87 - "project_test.rs"
Cohesion: 0.52
Nodes (6): new_sequence_appends_and_switches_to_it(), new_sequence_ids_keep_increasing_after_multiple_calls(), new_sequence_inherits_the_active_sequences_export_settings(), test_project(), timeline_mut_writes_the_active_sequence(), timeline_reads_the_active_sequence()

### Community 88 - "core/tests/probe_test.rs"
Cohesion: 0.36
Nodes (7): converts_container_bitrate_from_bps_to_mbps(), falls_back_to_the_audio_stream_when_there_is_no_video(), fixture(), into_media_asset_carries_the_probed_fields_through(), parses_frame_rate(), probes_a_video_stream_as_the_primary_track(), PathBuf

### Community 90 - "auto_reframe_one"
Cohesion: 0.27
Nodes (7): App, auto_reframe_one(), extract_frame(), Option, Path, UnboundedSender, Vec

### Community 96 - "parsing.rs"
Cohesion: 0.67
Nodes (5): bench_parse_loudnorm_stderr(), bench_project_ocproj_round_trip(), bench_timeline_duration(), large_project(), Criterion

### Community 98 - "ytbridge/build.rs"
Cohesion: 0.90
Nodes (4): copy_dir_all(), copy_file(), main(), Path

### Community 100 - "App"
Cohesion: 0.17
Nodes (5): App, autosave_is_newer(), Context, Path, String

### Community 102 - "Position"
Cohesion: 0.33
Nodes (3): f32, Lerp, Position

### Community 103 - "test_asset"
Cohesion: 0.20
Nodes (11): add_shape_clip_ids_stay_unique_past_an_existing_high_shape_clip_id(), ensure_preview_loaded_clears_state_once_the_playhead_moves_past_every_clip(), ensure_preview_loaded_resolves_the_clip_covering_the_playhead(), pump_import_queue_adds_the_asset_to_its_target_project_with_a_fresh_id(), pump_import_queue_applies_enrichment_to_the_asset_it_was_assigned(), pump_import_queue_targets_the_project_by_id_not_the_active_index(), Vec, test_asset() (+3 more)

## Knowledge Gaps
- **123 isolated node(s):** `TrimEdge`, `setup-python-runtime.sh script`, `yt2mp3.sh script`, `yt2mp4.sh script`, `Security Context` (+118 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **7 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `App` connect `App` to `PreviewError`, `timeline.rs`, `ClipInstance`, `LibraryTrack`, `App`, `record_event`, `ExportJob`, `Locale`, `src/youtube_download.rs`, `Project`?**
  _High betweenness centrality (0.203) - this node is a cross-community bridge._
- **Why does `ClipInstance` connect `ClipInstance` to `render.rs`, `timeline_test.rs`, `PreviewError`, `Position`, `timeline.rs`, `test_project`, `render_timeline_export`, `preview_test.rs`, `track_with`, `timeline_export_multi_test.rs`, `test_asset`, `App`, `App`, `Keyframe`, `App`, `ShapeKind`?**
  _High betweenness centrality (0.121) - this node is a cross-community bridge._
- **Why does `Project` connect `Project` to `parsing.rs`, `persistence_test.rs`, `app_test.rs`, `App`, `test_asset`, `test_project`, `Sequence`, `ensure_proxy`, `persistence.rs`, `src/background_removal.rs`, `tests/background_removal_test.rs`, `Locale`, `project_test.rs`, `MediaAsset`?**
  _High betweenness centrality (0.094) - this node is a cross-community bridge._
- **What connects `TrimEdge`, `setup-python-runtime.sh script`, `yt2mp3.sh script` to the rest of the system?**
  _123 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `keyframe_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.08 - nodes in this community are weakly interconnected._
- **Should `app_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.02263501891984864 - nodes in this community are weakly interconnected._
- **Should `avbridge/src/lib.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.05131578947368421 - nodes in this community are weakly interconnected._