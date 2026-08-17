# Graph Report - oca  (2026-08-17)

## Corpus Check
- 154 files · ~207,665 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1971 nodes · 3915 edges · 96 communities (87 shown, 9 thin omitted)
- Extraction: 94% EXTRACTED · 6% INFERRED · 0% AMBIGUOUS · INFERRED: 246 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `b558c578`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- keyframe_test.rs
- Project
- app_test.rs
- avbridge/src/lib.rs
- timeline_test.rs
- PreviewError
- timeline.rs
- encode_write_packet
- App
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
- App
- track_region
- timeline_with
- src/background_removal.rs
- text_metrics_test.rs
- App
- test_asset
- Locale
- Update Process
- src/auto_reframe.rs
- test_canvas
- Keyframe
- render.rs
- bridge_internal.h
- App
- pcm_extract.c
- ShapeKind
- open_input
- Timeline
- waveform.c
- avbridge_measure_loudness
- gpu_encoder.c
- theme.rs
- app/mod.rs
- MediaAsset
- text_to_speech.rs
- Position
- timeline_panel.rs
- preview_test.rs
- editor/mod.rs
- core/build.rs
- PrefsState
- .add_and_open_project
- subtitles.rs
- UpdateCheckError
- generate_tts_one
- enum_combo
- Watch-Gameplay.ps1
- generate_matte_one
- ImportEvent
- App
- yt2mp3.sh
- .pump_preview_frame
- App
- yt2mp4.sh
- ytbridge/build.rs

## God Nodes (most connected - your core abstractions)
1. `test_app()` - 172 edges
2. `App` - 65 edges
3. `clip()` - 62 edges
4. `ClipInstance` - 58 edges
5. `track_with()` - 37 edges
6. `Project` - 32 edges
7. `App` - 30 edges
8. `MediaAsset` - 29 edges
9. `App` - 27 edges
10. `render_timeline_export()` - 26 edges

## Surprising Connections (you probably didn't know these)
- `split_keyframes_at_drops_keyframes_that_land_on_the_other_side()` --calls--> `split_keyframes_at()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `split_keyframes_at_is_empty_for_an_empty_input()` --calls--> `split_keyframes_at()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `opacity_alpha_ramp_expr_clamps_a_single_keyframe()` --calls--> `opacity_alpha_ramp_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `avbridge_encode_export()` --calls--> `open_input()`  [INFERRED]
  crates/avbridge/csrc/export.c → crates/avbridge/csrc/common.c
- `avbridge_measure_loudness()` --calls--> `open_input()`  [INFERRED]
  crates/avbridge/csrc/loudness.c → crates/avbridge/csrc/common.c

## Import Cycles
- 2-file cycle: `crates/ui/src/app/mod.rs -> crates/ui/src/i18n.rs -> crates/ui/src/app/mod.rs`

## Communities (96 total, 9 thin omitted)

### Community 0 - "keyframe_test.rs"
Cohesion: 0.11
Nodes (3): opacity_alpha_ramp_expr_clamps_a_single_keyframe(), split_keyframes_at_drops_keyframes_that_land_on_the_other_side(), split_keyframes_at_is_empty_for_an_empty_input()

### Community 1 - "Project"
Cohesion: 0.07
Nodes (46): bench_parse_loudnorm_stderr(), bench_project_ocproj_round_trip(), bench_timeline_duration(), large_project(), from_ocproj_bytes(), load_project_from_file(), PersistError, Display (+38 more)

### Community 2 - "app_test.rs"
Cohesion: 0.03
Nodes (151): add_asset_to_timeline_creates_a_track_and_appends_a_clip_when_none_exists(), add_asset_to_timeline_creates_an_audio_track_for_an_audio_asset(), add_asset_to_timeline_is_a_no_op_for_an_unknown_asset_id(), add_opacity_marker_at_playhead_defaults_to_fully_opaque_with_no_existing_keyframes(), add_opacity_marker_at_playhead_is_a_no_op_outside_the_clips_own_span(), add_opacity_marker_at_playhead_is_a_no_op_when_nothing_is_selected(), add_opacity_marker_at_playhead_preserves_the_currently_interpolated_value(), add_sequence_appends_a_named_tab_and_switches_to_it() (+143 more)

### Community 3 - "avbridge/src/lib.rs"
Cohesion: 0.06
Nodes (76): c_char, c_int, c_longlong, apply_shape_overlays(), apply_text_overlays(), encode_export(), encode_matte_video(), encode_timeline_export() (+68 more)

### Community 4 - "timeline_test.rs"
Cohesion: 0.06
Nodes (45): apply_formatting_overwrites_all_effect_fields(), clip(), formatting_roundtrip_preserves_all_fields(), has_vignette_is_true_above_zero(), is_color_filtered_is_true_for_any_filter_but_none(), is_cropped_is_true_when_any_crop_field_differs_from_the_full_frame(), is_masked_is_true_for_any_shape_but_none(), negative_gain_db_scales_linear_gain_below_unity() (+37 more)

### Community 5 - "PreviewError"
Cohesion: 0.12
Nodes (27): AppSink, BoolError, build_chroma_key_element(), build_composite_branch(), build_static_overlay_branch(), build_video_filter_bin(), CompositeBranch, connect_decodebin_video_pad() (+19 more)

### Community 6 - "timeline.rs"
Cohesion: 0.12
Nodes (11): text_clip(), ClipFormatting, ColorFilter, LayerTemplate, MaskShape, String, Vec, TextClip (+3 more)

### Community 7 - "encode_write_packet"
Cohesion: 0.19
Nodes (24): AVStream, avbridge_encode_export(), EncodeStatus, ProgressCallback, AudioFilterChain, AVCodecContext, AVFormatContext, AVFrame (+16 more)

### Community 8 - "App"
Cohesion: 0.14
Nodes (12): TrackKind, App, AtomicBool, Context, HashMap, Option, TextureHandle, UnboundedReceiver (+4 more)

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
Cohesion: 0.09
Nodes (5): App, next_clip_id(), resolve_or_create_track(), FnOnce, Option

### Community 14 - "Task Breakdown — Fase 1 (Motor central)"
Cohesion: 0.12
Nodes (15): Task Breakdown — Fase 1 (Motor central), task_id: chore-001, task_id: chore-002, task_id: docs-001, task_id: docs-002, task_id: impl-002a, task_id: impl-002b, task_id: impl-003 (+7 more)

### Community 15 - "Reviewer Agent"
Cohesion: 0.29
Nodes (6): Constraints, Decision rules, Expected inputs, Required output: `review_report.md`, Reviewer Agent, Role

### Community 16 - "record_event"
Cohesion: 0.11
Nodes (22): record_event(), rotate_if_oversized(), Display, Error, Formatter, Path, Result, String (+14 more)

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
Cohesion: 0.08
Nodes (35): abort_trampoline(), collect_segments(), collect_words(), Arc, AtomicBool, c_void, Display, Error (+27 more)

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
Cohesion: 0.17
Nodes (12): add_and_open_project_appends_and_opens_it(), add_asset_to_timeline_appends_after_whatever_is_already_on_the_matching_track(), begin_save_layer_template_snapshots_the_multi_selection_ordered_by_track_then_start(), confirm_apply_layer_template_creates_one_clip_per_filled_layer_with_its_formatting(), confirm_apply_layer_template_puts_each_layer_on_its_own_new_track(), confirm_apply_layer_template_skips_layers_left_without_an_asset(), select_timeline_clip_synchronizes_its_backing_asset(), test_clip() (+4 more)

### Community 41 - "avbridge_encode_timeline_export_multi"
Cohesion: 0.20
Nodes (15): AVFilterContext, AVFilterGraph, advance_overlay_decoder(), avbridge_encode_timeline_export_multi(), build_overlay_vfilter(), build_vfilter_descr(), AVCodecContext, AVFrame (+7 more)

### Community 43 - "LibraryTrack"
Cohesion: 0.14
Nodes (13): LibraryTrack, Path, PathBuf, String, Vec, scan_library_dir(), SoundCategory, fixture() (+5 more)

### Community 44 - "App"
Cohesion: 0.16
Nodes (6): App, autosave_is_newer(), media_kind_matches(), Context, Path, String

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
Cohesion: 0.18
Nodes (12): default_font(), Option, Vec, text_width_px_grows_with_more_characters(), text_width_px_scales_up_with_font_size(), word_x_offsets_px_is_empty_for_no_words(), word_x_offsets_px_is_strictly_increasing(), word_x_offsets_px_matches_word_count() (+4 more)

### Community 49 - "App"
Cohesion: 0.09
Nodes (3): App, String, Vec

### Community 50 - "test_asset"
Cohesion: 0.22
Nodes (10): add_shape_clip_ids_stay_unique_past_an_existing_high_shape_clip_id(), ensure_preview_loaded_clears_state_once_the_playhead_moves_past_every_clip(), ensure_preview_loaded_resolves_the_clip_covering_the_playhead(), pump_import_queue_adds_the_asset_to_its_target_project_with_a_fresh_id(), pump_import_queue_applies_enrichment_to_the_asset_it_was_assigned(), pump_import_queue_targets_the_project_by_id_not_the_active_index(), Vec, test_asset() (+2 more)

### Community 51 - "Locale"
Cohesion: 0.08
Nodes (41): Screen, job(), job_detail_line_flips_the_negative_lufs_sign_for_display(), job_detail_line_shows_translated_error_prefix_on_failure(), job_detail_line(), job_status_label(), Locale, nav_label() (+33 more)

### Community 52 - "Update Process"
Cohesion: 0.13
Nodes (14): 1. Choose a Release, 2. Download Assets and Compute Checksums, 3. Update Workflow Environment Variables, 4. Update Cache Keys, 5. Test the Changes, Checksum Mismatch Error, Download Failures, FFmpeg Update Guide (+6 more)

### Community 53 - "src/auto_reframe.rs"
Cohesion: 0.08
Nodes (32): approx(), main_subject_picks_highest_score(), nms_collapses_overlapping_boxes(), no_subject_falls_back_to_center_crop(), same_aspect_ratio_keeps_full_frame(), subject_near_edge_clamps_without_overflow(), subject_off_center_shifts_crop_toward_it(), wider_target_keeps_full_width() (+24 more)

### Community 54 - "test_canvas"
Cohesion: 0.50
Nodes (4): pending_export_conflict_overwrite_queues_with_the_original_path(), queue_export_appends_a_queued_job_with_the_next_id(), queue_export_starts_at_one_when_no_jobs_exist(), test_canvas()

### Community 55 - "Keyframe"
Cohesion: 0.20
Nodes (22): clamp_scale(), evaluate_keyframes(), Keyframe, position_overlay_xy_expr_builds_a_t_based_ramp_for_an_animated_axis(), position_overlay_xy_expr_scales_a_single_keyframe_by_canvas_dimensions(), rotation_filter_angle_expr_converts_a_single_keyframe_to_radians(), rotation_filter_angle_expr_uses_t_for_an_animated_ramp(), scale_filter_expr_builds_a_geq_expression_for_an_animated_ramp() (+14 more)

### Community 56 - "render.rs"
Cohesion: 0.05
Nodes (93): Canvas, ClipSegment, PathBuf, String, ShapeSegment, TextSegment, ExportJob, ExportJobStatus (+85 more)

### Community 57 - "bridge_internal.h"
Cohesion: 0.25
Nodes (5): AVCodec, loudness_log_callback(), avbridge_encode_matte_video(), MatteStatus, va_list

### Community 58 - "App"
Cohesion: 0.18
Nodes (4): App, Option, PathBuf, Vec

### Community 59 - "pcm_extract.c"
Cohesion: 0.29
Nodes (10): append_pcm_frame(), avbridge_extract_pcm_16k_mono(), AudioFilterChain, AVCodecContext, AVFrame, drain_pcm_frame(), init_pcm_filter_chain(), pcm_buffer_reserve() (+2 more)

### Community 60 - "ShapeKind"
Cohesion: 0.07
Nodes (38): ellipse_inside(), ellipse_center_pixel_is_opaque_and_corners_are_transparent(), empty_text_produces_a_fully_transparent_buffer(), non_empty_text_draws_at_least_one_opaque_pixel(), rectangle_fills_its_whole_local_square_not_just_the_center(), sample_shape(), sample_text(), stroke_only_shape_leaves_its_own_center_transparent() (+30 more)

### Community 61 - "open_input"
Cohesion: 0.17
Nodes (11): AVFormatContext, open_input(), avbridge_probe(), avbridge_remux_copy(), avbridge_apply_shape_overlays(), TextOverlayStatus, avbridge_apply_text_overlays(), TextOverlayStatus (+3 more)

### Community 62 - "Timeline"
Cohesion: 0.15
Nodes (3): Timeline, App, create_new_track()

### Community 63 - "waveform.c"
Cohesion: 0.33
Nodes (9): accumulate_waveform_frame(), avbridge_generate_waveform(), AudioFilterChain, AVCodecContext, AVFrame, drain_waveform_frame(), init_waveform_filter_chain(), WaveformAccumulator (+1 more)

### Community 64 - "avbridge_measure_loudness"
Cohesion: 0.33
Nodes (7): avbridge_measure_loudness(), AudioFilterChain, AVCodecContext, AVFrame, drain_measure_frame(), init_measure_filter_chain(), LoudnessStatus

### Community 65 - "gpu_encoder.c"
Cohesion: 0.60
Nodes (5): AVRational, AVCodecContext, open_video_encoder(), pix_fmt_for_encoder_name(), try_open_encoder()

### Community 66 - "theme.rs"
Cohesion: 0.06
Nodes (41): card_frame(), Frame, property_block(), property_section(), property_toggle(), FnOnce, Ui, Ui (+33 more)

### Community 68 - "app/mod.rs"
Cohesion: 0.19
Nodes (18): AvailableUpdate, BindableAction, EditorTool, MatteGenerationEvent, ModelDownloadEvent, ModelKind, prefs_path(), RenderEvent (+10 more)

### Community 69 - "MediaAsset"
Cohesion: 0.08
Nodes (41): format_timecode(), MediaAsset, MediaKind, Option, PathBuf, String, Vec, ProbedMedia (+33 more)

### Community 70 - "text_to_speech.rs"
Cohesion: 0.11
Nodes (32): default_length_scale(), default_noise_scale(), default_noise_w(), io_from_hound(), load_voice_config(), phonemes_to_ids(), PiperAudioConfig, PiperEspeakConfig (+24 more)

### Community 71 - "Position"
Cohesion: 0.14
Nodes (8): f32, Lerp, Position, MotionTrackEvent, App, motion_track_one(), Path, Vec

### Community 73 - "timeline_panel.rs"
Cohesion: 0.21
Nodes (19): color_filter_tint(), draw_filmstrip(), draw_frozen_poster(), draw_keyframe_markers(), draw_playhead(), draw_waveform(), App, Color32 (+11 more)

### Community 74 - "preview_test.rs"
Cohesion: 0.20
Nodes (21): a_cropped_clip_shrinks_the_decoded_frame(), a_flipped_clip_still_opens_and_decodes(), a_neutral_clip_applies_no_filter_bin_and_frame_size_is_unchanged(), a_pixelized_clip_still_opens_and_decodes_at_full_size(), a_shaken_clip_still_opens_and_decodes_at_full_size(), a_zoomed_clip_still_opens_and_decodes_at_full_size(), clip(), current_frame_is_none_for_audio_only_input() (+13 more)

### Community 75 - "editor/mod.rs"
Cohesion: 0.33
Nodes (18): draw_custom_shape_surface(), draw_motion_track_region_picker(), export_srt_for_active_sequence(), fullscreen_preview_overlay(), layer_transform_preview(), media_library_panel(), preview_panel(), resizable_divider() (+10 more)

### Community 76 - "core/build.rs"
Cohesion: 0.44
Nodes (8): copy_dir_recursive(), find_espeak_data_source(), find_profile_dir(), main(), Option, Path, PathBuf, Result

### Community 77 - "PrefsState"
Cohesion: 0.17
Nodes (12): default_add_opacity_marker_binding(), default_lib_panel_width(), default_props_panel_width(), default_timeline_height(), KeyBindings, KeyCombo, load_prefs(), PrefsState (+4 more)

### Community 79 - "subtitles.rs"
Cohesion: 0.29
Nodes (10): export_srt(), exports_two_clips_as_sequential_numbered_entries(), format_srt_timestamp(), ignores_non_text_tracks(), orders_entries_by_start_time_regardless_of_input_order(), returns_an_empty_string_for_a_timeline_with_no_text_clips(), String, Vec (+2 more)

### Community 80 - "UpdateCheckError"
Cohesion: 0.24
Nodes (9): fetch_latest_release(), GithubRelease, LatestRelease, Display, Error, Formatter, Result, String (+1 more)

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

### Community 87 - "ImportEvent"
Cohesion: 0.29
Nodes (7): ImportEvent, Vec, ThumbnailReady, TranscribeEvent, Path, UnboundedSender, transcribe_one()

### Community 98 - "ytbridge/build.rs"
Cohesion: 0.90
Nodes (4): copy_dir_all(), copy_file(), main(), Path

## Knowledge Gaps
- **122 isolated node(s):** `TrimEdge`, `setup-python-runtime.sh script`, `yt2mp3.sh script`, `yt2mp4.sh script`, `Security Context` (+117 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **9 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `App` connect `App` to `Project`, `app/mod.rs`, `PreviewError`, `timeline.rs`, `Position`, `ClipInstance`, `LibraryTrack`, `PrefsState`, `.add_and_open_project`, `record_event`, `Locale`, `src/auto_reframe.rs`, `ImportEvent`, `src/youtube_download.rs`, `render.rs`?**
  _High betweenness centrality (0.183) - this node is a cross-community bridge._
- **Why does `ClipInstance` connect `ClipInstance` to `Project`, `timeline_test.rs`, `PreviewError`, `timeline.rs`, `Position`, `MediaAsset`, `test_project`, `preview_test.rs`, `track_with`, `App`, `App`, `timeline_panel.rs`, `.duration_secs`, `test_asset`, `Keyframe`, `render.rs`, `App`?**
  _High betweenness centrality (0.102) - this node is a cross-community bridge._
- **Why does `Project` connect `Project` to `app_test.rs`, `MediaAsset`, `test_project`, `App`, `ensure_proxy`, `.add_and_open_project`, `src/background_removal.rs`, `test_asset`, `render.rs`, `Timeline`?**
  _High betweenness centrality (0.073) - this node is a cross-community bridge._
- **What connects `TrimEdge`, `setup-python-runtime.sh script`, `yt2mp3.sh script` to the rest of the system?**
  _122 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `keyframe_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.10526315789473684 - nodes in this community are weakly interconnected._
- **Should `Project` be split into smaller, more focused modules?**
  _Cohesion score 0.0677555958862674 - nodes in this community are weakly interconnected._
- **Should `app_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.025144747725392887 - nodes in this community are weakly interconnected._