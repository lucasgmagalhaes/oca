# Graph Report - oca  (2026-08-28)

## Corpus Check
- 275 files · ~369,502 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 3649 nodes · 7250 edges · 236 communities (205 shown, 31 thin omitted)
- Extraction: 94% EXTRACTED · 6% INFERRED · 0% AMBIGUOUS · INFERRED: 428 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `b3604007`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- keyframe_test.rs
- persistence_test.rs
- app_test.rs
- avbridge/src/lib.rs
- timeline_test.rs
- src/preview.rs
- encode_write_packet
- UndoStack
- Plano de Execução — oca (PacoPaçoca)
- track_with
- ensure_proxy
- MediaAsset
- next_clip_id
- Task Breakdown — Fase 1 (Motor central)
- Required checklist
- record_event
- Manager Agent
- task_id: impl-004b
- Docs Specialist
- Git Agent
- Impl Specialist
- Test Specialist
- src/youtube_download.rs
- transcript_proposals_test.rs
- Built-in Font Catalog
- main.rs
- Commit Plan — Fase 1
- poll_for_descendants
- generate_waveform
- setup-python-runtime.sh script
- transcribe
- run
- Project
- Security Fix: FFmpeg Download Verification
- test_project
- avbridge_encode_timeline_export_multi
- ClipInstance
- Preview
- timeline_export_multi_test.rs
- track_region
- timeline_with
- src/background_removal.rs
- text_metrics_test.rs
- Keyframe
- PHASE 1 — Application UI Inventory
- Locale
- Update Process
- src/auto_reframe.rs
- test_canvas
- keyframe.rs
- app/mod.rs
- bridge_internal.h
- App
- pcm_extract.c
- overlay_render_test.rs
- probe_media
- parse_loudnorm_stderr
- waveform.c
- avbridge_measure_loudness
- gpu_encoder.c
- render.rs
- transcript.rs
- App
- audio_mix.c
- text_to_speech.rs
- motion_track_one
- nested_sequence_test.rs
- preview_test.rs
- render_test.rs
- core/build.rs
- split_keyframes_at
- draw.rs
- INDEX.md
- src/update_check.rs
- ExportJob
- generate_tts_one
- enum_combo
- Watch-Gameplay.ps1
- Duration
- silence_detection.rs
- Auto-update release contract
- yt2mp3.sh script
- shape_render_test.rs
- App
- yt2mp4.sh script
- preview_effects.rs
- String
- Complex Text Shaping and Bidirectional Layout
- ytbridge/build.rs
- App
- bundle_macos_dylibs.py
- auto_reframe_one
- test_asset
- main
- dpkg_field
- Differentiator Features (beyond `features/request.md`)
- build_dmg.sh
- Client Error Reporting
- test_track
- PathBuf
- Performance & Caching Patterns (ported from nimble)
- AGENTS.md
- CLAUDE.md
- Effects, Keyframes, Color
- Roadmap — Actionable Queue
- EGUI.md
- Runtime dependency bundle
- linux-launcher.sh
- main
- digest
- main
- sign_macos_app.sh
- assemble_linux.sh
- assemble_macos.sh
- build_deb.sh
- oca
- Undo/Redo (`ROADMAP.md` P0 item 1)
- Competitor Parity Survey
- timeline-and-editing.md
- Mandatory Rules & Definition of Done
- spec/ — Start Here
- bundle.rs
- encode_timeline_export
- subtitles.rs
- properties_panel
- changelog.md
- editor/mod.rs
- Track
- LibraryTrack
- save_transcript_document
- src/highlight_detection.rs
- timeline_window_test.rs
- transcript_test.rs
- package.json
- project_test.rs
- App
- ProbeError
- CURRENTLY IMPLEMENTED
- SmartBin
- TranscribeSegment
- apply_text_overlays
- Option
- mix_audio_timeline
- open_input
- extract_pcm_16k_mono
- overlay_render.rs
- shorts_pack_scratch_dir
- Ordered implementation backlog
- shape_clip_properties
- text_clip_properties
- code-quality/README.md
- Rust code quality
- Mobile Support (Android + iOS) — ADR
- generate_waveform
- Path
- prefs.rs
- queue.rs
- Performance quality
- Code review checklist
- Security and trust boundaries
- TranscribeError
- tests/frame_sampler_test.rs
- App
- nav_rail.rs
- SOLID adapted to Rust
- Testing quality
- PathBuf
- Architecture
- workflow/README.md
- .spawn_shorts_pack
- probe
- App
- property.rs
- .apply_transcription
- breadcrumb.rs
- ShapeKind
- generate_matte_one
- Competitive Feature Implementation Plan
- Timeline
- Design System
- tag.rs
- UX Principles
- ai-features.md
- packaging.md
- matrix/performance.md
- preview-pipeline.md
- UI Implementation Plan — oca
- solid_rgba
- build_video_filter_bin
- App
- multicam_sync.rs
- Project Instructions
- Timeline — Domain and Interaction Rules
- Timeline Performance
- TextClip
- egui Implementation Guide
- Core Workflows
- Design System Consolidation Review
- UI Review Checklist
- render_shape_clip_rgba
- Claude Code Configuration for Rust + egui Video Editor
- PreviewState
- PreviewError
- Position
- egui-specialist.md
- ui-performance.md
- video-editor-ux.md
- visual-designer.md
- feature.md
- PROFESSIONAL_UI.md
- Quality Gates
- engine.md
- .add_and_open_project
- shape_render.rs
- smooth_speed_ramp_duration_secs

## God Nodes (most connected - your core abstractions)
1. `test_app()` - 270 edges
2. `ClipInstance` - 89 edges
3. `App` - 85 edges
4. `clip()` - 75 edges
5. `Project` - 57 edges
6. `track_with()` - 57 edges
7. `MediaAsset` - 53 edges
8. `Keyframe` - 44 edges
9. `fixture()` - 43 edges
10. `test_track()` - 42 edges

## Surprising Connections (you probably didn't know these)
- `rotation_filter_angle_expr_converts_a_single_keyframe_to_radians()` --calls--> `rotation_filter_angle_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `rotation_filter_angle_expr_uses_t_for_an_animated_ramp()` --calls--> `rotation_filter_angle_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `crop_filter_expr_animates_only_the_keyframed_axis()` --calls--> `crop_filter_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `crop_filter_expr_builds_a_geq_expression_for_a_static_crop_with_no_keyframes()` --calls--> `crop_filter_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `gain_filter_db_expr_converts_a_single_keyframe_to_a_linear_multiplier()` --calls--> `gain_filter_db_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs

## Import Cycles
- 2-file cycle: `crates/ui/src/app/mod.rs -> crates/ui/src/i18n.rs -> crates/ui/src/app/mod.rs`

## Communities (236 total, 31 thin omitted)

### Community 0 - "keyframe_test.rs"
Cohesion: 0.04
Nodes (16): color_balance_filter_expr_animates_only_the_keyframed_axis(), color_balance_filter_expr_uses_constants_for_unanimated_axes(), crop_filter_expr_animates_only_the_keyframed_axis(), crop_filter_expr_builds_a_geq_expression_for_a_static_crop_with_no_keyframes(), gain_filter_db_expr_converts_a_single_keyframe_to_a_linear_multiplier(), gain_filter_db_expr_uses_t_for_an_animated_ramp(), opacity_alpha_ramp_expr_clamps_a_single_keyframe(), rotation_filter_angle_expr_converts_a_single_keyframe_to_radians() (+8 more)

### Community 1 - "persistence_test.rs"
Cohesion: 0.07
Nodes (64): bench_parse_loudnorm_stderr(), bench_project_ocproj_round_trip(), bench_timeline_duration(), large_project(), CollabBundleError, export_collab_bundle(), import_collab_bundle(), Display (+56 more)

### Community 2 - "app_test.rs"
Cohesion: 0.02
Nodes (201): a_continuous_effect_property_drag_pushes_only_one_undo_step(), add_asset_to_timeline_creates_a_track_and_appends_a_clip_when_none_exists(), add_asset_to_timeline_creates_an_audio_track_for_an_audio_asset(), add_asset_to_timeline_is_a_no_op_for_an_unknown_asset_id(), add_marker_at_playhead_places_it_at_the_current_playhead(), add_opacity_marker_at_playhead_defaults_to_fully_opaque_with_no_existing_keyframes(), add_opacity_marker_at_playhead_is_a_no_op_outside_the_clips_own_span(), add_opacity_marker_at_playhead_is_a_no_op_when_nothing_is_selected() (+193 more)

### Community 3 - "avbridge/src/lib.rs"
Cohesion: 0.15
Nodes (24): c_char, c_int, c_longlong, AudioMixError, EncodeError, LoudnessError, MatteError, MediaMuxError (+16 more)

### Community 4 - "timeline_test.rs"
Cohesion: 0.05
Nodes (57): apply_formatting_overwrites_all_effect_fields(), clip(), clip_at_finds_the_clip_covering_a_position(), formatting_roundtrip_preserves_all_fields(), has_vignette_is_true_above_zero(), is_color_filtered_is_true_for_any_filter_but_none(), is_cropped_is_true_when_any_crop_field_differs_from_the_full_frame(), is_masked_is_true_for_any_shape_but_none() (+49 more)

### Community 5 - "src/preview.rs"
Cohesion: 0.21
Nodes (21): AppSrc, attach_audio_mix_branch(), build_audio_filter_bin(), build_chroma_key_element(), build_composite_branch(), build_mask_shape_stage(), build_static_overlay_branch(), build_uri_decodebin() (+13 more)

### Community 7 - "encode_write_packet"
Cohesion: 0.18
Nodes (24): AVStream, avbridge_encode_export(), EncodeStatus, ProgressCallback, AudioFilterChain, AVCodecContext, AVFormatContext, AVFrame (+16 more)

### Community 8 - "UndoStack"
Cohesion: 0.13
Nodes (11): Default, Option, Self, Vec, UndoStack, a_new_push_after_undo_clears_the_redo_branch(), capacity_bounds_the_undo_stack_by_dropping_the_oldest_entry(), clear_drops_both_stacks() (+3 more)

### Community 9 - "Plano de Execução — oca (PacoPaçoca)"
Cohesion: 0.17
Nodes (12): Decisões já tomadas, Fase 0 — Escopo do MVP, Fase 1 — Motor central, Fase 2 — Ajuste automático de áudio, Fase 3 — Timeline, edição e organização do projeto, Fase 4 — Ferramentas e efeitos de edição, Fase 5 — Exportação e fila em background, Fase 6 — Robustez (+4 more)

### Community 10 - "track_with"
Cohesion: 0.04
Nodes (49): clip_at_is_exclusive_of_a_clips_end(), clip_at_is_inclusive_of_a_clips_start(), clip_at_is_none_for_a_gap_between_clips(), clip_mut_finds_a_clip_by_id(), clip_mut_returns_none_for_an_unknown_id(), move_clip_is_a_no_op_for_a_negative_position(), move_clip_is_a_no_op_for_an_unknown_clip_id(), move_clip_repositions_start_secs_and_leaves_the_source_range_untouched() (+41 more)

### Community 11 - "ensure_proxy"
Cohesion: 0.07
Nodes (32): cache_dir_for_project(), ensure_proxy(), is_up_to_date(), PreviewQuality, proxy_path_for(), a_proxy_newer_than_its_source_is_up_to_date(), a_proxy_older_than_its_source_is_not_up_to_date(), missing_proxy_is_not_up_to_date() (+24 more)

### Community 12 - "MediaAsset"
Cohesion: 0.13
Nodes (25): format_timecode(), MediaAsset, MediaKind, Option, PathBuf, String, Vec, cache_dir_for_project() (+17 more)

### Community 13 - "next_clip_id"
Cohesion: 0.05
Nodes (12): App, App, App, App, create_new_track(), default_clip_instance(), next_clip_id(), resolve_or_create_track() (+4 more)

### Community 14 - "Task Breakdown — Fase 1 (Motor central)"
Cohesion: 0.12
Nodes (15): Task Breakdown — Fase 1 (Motor central), task_id: chore-001, task_id: chore-002, task_id: docs-001, task_id: docs-002, task_id: impl-002a, task_id: impl-002b, task_id: impl-003 (+7 more)

### Community 15 - "Required checklist"
Cohesion: 0.14
Nodes (13): 1. Diff size (granularity check), 2. acceptance_criteria coverage, 3. Quality — Rust, 4. Quality — C, 5. Consistency with the plan, 6. Documentation, Constraints, Decision rules (+5 more)

### Community 16 - "record_event"
Cohesion: 0.07
Nodes (36): GpuSampler, record_event(), ResourceSampler, rotate_if_oversized(), Default, Display, Error, Formatter (+28 more)

### Community 17 - "Manager Agent"
Cohesion: 0.18
Nodes (10): Behavior rules, `commit_plan.md`, Expected inputs, Granularity rules (HARD RULES), Manager Agent, Project stack, Required outputs, Role (+2 more)

### Community 18 - "task_id: impl-004b"
Cohesion: 0.22
Nodes (8): Bugs found and fixed during implementation (not just typos — genuine correctness issues), Checklist, Decision, Diff sizes, Review Report, task_id: impl-004b, Verification performed (real files, not just `cargo test` passing), Why this is flagged, not silently split

### Community 19 - "Docs Specialist"
Cohesion: 0.29
Nodes (7): CHANGELOG.md — expected format, Constraints, Docs Specialist, Expected inputs, Output, Required workflow, Role

### Community 20 - "Git Agent"
Cohesion: 0.25
Nodes (8): Commit message format, Expected inputs, Final validation (after all commits), Git Agent, HARD RULES — never violate, In case of problems, Required workflow, Role

### Community 21 - "Impl Specialist"
Cohesion: 0.25
Nodes (8): Code rules — C, Code rules — Rust, Constraints, Expected inputs, Impl Specialist, Output, Required workflow, Role

### Community 22 - "Test Specialist"
Cohesion: 0.25
Nodes (7): Constraints, Expected inputs, Output, Required workflow, Role, Test Specialist, What makes a good test here

### Community 23 - "src/youtube_download.rs"
Cohesion: 0.11
Nodes (19): download_youtube(), Event, is_yt_dlp_available(), Mp3Bitrate, Mp4Quality, python_home_for_bridge(), AtomicBool, Display (+11 more)

### Community 24 - "transcript_proposals_test.rs"
Cohesion: 0.09
Nodes (31): detect_proposals(), filler_word_set(), map_source_range_to_timeline(), merge_source_ranges(), norm(), Option, String, Vec (+23 more)

### Community 25 - "Built-in Font Catalog"
Cohesion: 0.06
Nodes (33): Acquisition and update workflow, Built-in Font Catalog, Catalog manifest, Definition of done, Display, condensed, and gaming: 10 families, FONT-01A: Manifest and migration, FONT-01B: Vendoring and catalog expansion, FONT-01C: Variable weights and TEXT-01 integration (+25 more)

### Community 26 - "main.rs"
Cohesion: 0.48
Nodes (6): init_logging(), install_panic_hook(), main(), platform_log_dir(), PathBuf, Result

### Community 32 - "poll_for_descendants"
Cohesion: 0.13
Nodes (25): oca_window(), poll_for_descendants(), Repeatedly re-queries `window`'s accessibility tree for a descendant control…, Launches a fresh ui.exe, waits for its main window, yields it, then tears it…, _wait_for_window(), _set_reframe_model_path(), test_auto_reframe_button_runs_detection_without_crashing(), test_importing_a_file_adds_it_to_the_library_quickly() (+17 more)

### Community 34 - "generate_waveform"
Cohesion: 0.19
Nodes (14): generate_waveform(), Display, Error, Formatter, Path, Result, Vec, WaveformError (+6 more)

### Community 36 - "transcribe"
Cohesion: 0.25
Nodes (13): AtomicBool, FnMut, Option, Path, transcribe(), cancelling_before_the_call_skips_inference_entirely(), fails_on_a_missing_model_file(), fails_on_a_source_with_no_audio_stream() (+5 more)

### Community 37 - "run"
Cohesion: 0.28
Nodes (13): Bound, Args, emit(), Event, extract_final_path(), main(), Option, PathBuf (+5 more)

### Community 38 - "Project"
Cohesion: 0.09
Nodes (17): ExportAspectRatio, default_target_lufs(), PanelLayout, Project, Default, Option, PathBuf, Self (+9 more)

### Community 39 - "Security Fix: FFmpeg Download Verification"
Cohesion: 0.12
Nodes (16): 1. `.github/workflows/ci.yml`, 2. `.github/FFMPEG_UPDATE.md` (new file), After, Attack Surface Reduction, Before, Changes Made, Environment Variables (lines 9-16), Implementation Notes (+8 more)

### Community 40 - "test_project"
Cohesion: 0.11
Nodes (18): add_and_open_project_appends_and_opens_it(), add_asset_to_timeline_appends_after_whatever_is_already_on_the_matching_track(), confirm_apply_layer_template_puts_each_layer_on_its_own_new_track(), confirm_apply_layer_template_skips_layers_left_without_an_asset(), load_panel_layout_applies_the_active_projects_saved_layout(), load_panel_layout_is_a_no_op_under_per_user_scope(), ripple_trim_clip_end_is_bounded_by_the_source_assets_own_duration(), ripple_trim_clip_start_shifts_only_later_clips() (+10 more)

### Community 41 - "avbridge_encode_timeline_export_multi"
Cohesion: 0.17
Nodes (25): AVFilterInOut, add_graph_input(), advance_overlay_decoder(), avbridge_encode_timeline_export_multi(), build_overlay_vfilter(), build_setpts_str(), build_vfilter_descr(), AVCodecContext (+17 more)

### Community 42 - "ClipInstance"
Cohesion: 0.11
Nodes (3): ClipInstance, clip(), clip()

### Community 43 - "Preview"
Cohesion: 0.17
Nodes (10): AppSink, AudioLevel, build_audio_mix_output(), build_metering_audio_sink(), Preview, Arc, HashMap, Mutex (+2 more)

### Community 44 - "timeline_export_multi_test.rs"
Cohesion: 0.27
Nodes (29): audio_asset(), cancelling_mid_multi_track_export_reports_cancelled(), clip(), fixture(), layer_scale_at_native_size_adds_no_filter_stage(), layer_scale_is_appended_to_the_overlay_tracks_video_filter(), overlay_track_animated_opacity_keyframes_composites_without_error(), overlay_track_animated_position_keyframes_composites_without_error() (+21 more)

### Community 45 - "track_region"
Cohesion: 0.09
Nodes (31): extract_patch(), GrayFrame, a_single_frame_produces_one_clamped_position(), frame_with_block(), initial_center_near_the_edge_is_clamped_so_the_template_fits(), keyframes_ride_the_delta_from_the_first_tracked_frame(), stays_put_when_the_block_does_not_move(), template_width_and_height_clamp_independently_to_each_frame_dimension() (+23 more)

### Community 46 - "timeline_with"
Cohesion: 0.13
Nodes (22): add_marker_assigns_ids_starting_at_one_and_clamps_a_negative_position(), add_marker_reuses_the_max_plus_one_id_even_after_a_removal(), add_multicam_group_hides_every_member_except_the_program_track(), add_multicam_group_rejects_fewer_than_two_members_or_an_unknown_program_track(), marker_mut_edits_the_right_marker_and_none_for_an_unknown_id(), markers_sorted_orders_by_position_regardless_of_insertion_order(), move_clip_to_track_is_a_no_op_across_mismatched_kinds(), move_clip_to_track_is_a_no_op_for_a_negative_position() (+14 more)

### Community 47 - "src/background_removal.rs"
Cohesion: 0.13
Nodes (26): approx(), model_input_size_leaves_frames_already_spanning_target_alone_then_rounds(), model_input_size_rounds_down_to_multiple_of_32(), model_input_size_scales_constrained_side_toward_target(), model_input_size_upscales_tiny_frames_toward_target(), preprocess_normalizes_black_pixel_to_negative_one(), preprocess_normalizes_white_pixel_to_one(), resize_matte_downsamples_without_out_of_bounds() (+18 more)

### Community 48 - "text_metrics_test.rs"
Cohesion: 0.13
Nodes (21): bundled_font(), parse_font(), Font, Option, Vec, every_bundled_font_face_parses_and_is_cached(), selecting_another_family_changes_text_metrics(), text_width_px_grows_with_more_characters() (+13 more)

### Community 49 - "Keyframe"
Cohesion: 0.09
Nodes (4): Keyframe, App, String, Vec

### Community 50 - "PHASE 1 — Application UI Inventory"
Cohesion: 0.04
Nodes (46): Acceptance Criteria, Application chrome, Borders, Color, Component consistency, Component system — enforcement rules, not new components, Controls inventory, Corner radius — new, second-highest-leverage fix (+38 more)

### Community 51 - "Locale"
Cohesion: 0.30
Nodes (13): Recency, Screen, delete_sequence_prompt(), job_detail_line(), job_status_label(), Locale, nav_label(), recency_label() (+5 more)

### Community 52 - "Update Process"
Cohesion: 0.13
Nodes (14): 1. Choose a Release, 2. Download Assets and Compute Checksums, 3. Update Workflow Environment Variables, 4. Update Cache Keys, 5. Test the Changes, Checksum Mismatch Error, Download Failures, FFmpeg Update Guide (+6 more)

### Community 53 - "src/auto_reframe.rs"
Cohesion: 0.12
Nodes (23): approx(), main_subject_picks_highest_score(), nms_collapses_overlapping_boxes(), no_subject_falls_back_to_center_crop(), same_aspect_ratio_keeps_full_frame(), subject_near_edge_clamps_without_overflow(), subject_off_center_shifts_crop_toward_it(), wider_target_keeps_full_width() (+15 more)

### Community 54 - "test_canvas"
Cohesion: 0.29
Nodes (7): ocqueue_file_atomically_replaces_the_previous_snapshot(), pending_export_conflict_overwrite_queues_with_the_original_path(), queue_export_appends_a_queued_job_with_the_next_id(), queue_export_starts_at_one_when_no_jobs_exist(), queued_job_keeps_the_sequence_export_snapshot_after_settings_change(), test_canvas(), test_job()

### Community 55 - "keyframe.rs"
Cohesion: 0.25
Nodes (23): clamp_scale(), color_balance_filter_expr(), crop_filter_expr(), eq_axis_expr(), gain_filter_db_expr(), scale_filter_expr_builds_a_geq_expression_for_an_animated_ramp(), scale_filter_expr_is_static_crop_scale_for_a_single_non_unity_keyframe(), text_scale_sample_exprs_builds_a_piecewise_ramp_for_an_animated_scale() (+15 more)

### Community 56 - "app/mod.rs"
Cohesion: 0.14
Nodes (19): apply_bundled_model_defaults(), BindableAction, default_add_opacity_marker_binding(), default_lib_panel_width(), default_props_panel_width(), default_redo_binding(), default_timeline_height(), default_undo_binding() (+11 more)

### Community 57 - "bridge_internal.h"
Cohesion: 0.16
Nodes (12): AVCodec, loudness_log_callback(), avbridge_encode_matte_video(), AVFrame, scale_video_frame(), append_stage(), avbridge_apply_text_overlays(), TextOverlayStatus (+4 more)

### Community 58 - "App"
Cohesion: 0.07
Nodes (9): App, frozen_playhead(), Context, IntoIterator, Item, Option, PathBuf, String (+1 more)

### Community 59 - "pcm_extract.c"
Cohesion: 0.29
Nodes (10): append_pcm_frame(), avbridge_extract_pcm_16k_mono(), AudioFilterChain, AVCodecContext, AVFrame, drain_pcm_frame(), init_pcm_filter_chain(), pcm_buffer_reserve() (+2 more)

### Community 60 - "overlay_render_test.rs"
Cohesion: 0.16
Nodes (22): a_highlighted_word_follows_the_base_layout_onto_the_next_line(), a_local_time_covered_by_no_word_leaves_only_the_base_color(), circle_mask_center_is_visible_and_corners_are_masked(), empty_text_produces_a_fully_transparent_buffer(), highlight_disabled_never_draws_the_highlight_color_even_within_a_words_window(), long_text_wraps_onto_multiple_lines_once_narrower_than_the_canvas(), non_empty_text_draws_at_least_one_opaque_pixel(), none_mask_shape_is_fully_masked_out() (+14 more)

### Community 61 - "probe_media"
Cohesion: 0.20
Nodes (27): probe_media(), render_timeline_export(), converts_container_bitrate_from_bps_to_mbps(), falls_back_to_the_audio_stream_when_there_is_no_video(), fixture(), into_media_asset_carries_the_probed_fields_through(), parses_frame_rate(), probes_a_video_stream_as_the_primary_track() (+19 more)

### Community 62 - "parse_loudnorm_stderr"
Cohesion: 0.14
Nodes (20): extract_first_json_object(), LoudnessError, LoudnormReport, measure_loudness(), parse_loudnorm_stderr(), Display, Error, Formatter (+12 more)

### Community 63 - "waveform.c"
Cohesion: 0.33
Nodes (9): accumulate_waveform_frame(), avbridge_generate_waveform(), AudioFilterChain, AVCodecContext, AVFrame, drain_waveform_frame(), init_waveform_filter_chain(), WaveformAccumulator (+1 more)

### Community 64 - "avbridge_measure_loudness"
Cohesion: 0.33
Nodes (7): avbridge_measure_loudness(), AudioFilterChain, AVCodecContext, AVFrame, drain_measure_frame(), init_measure_filter_chain(), LoudnessStatus

### Community 65 - "gpu_encoder.c"
Cohesion: 0.61
Nodes (7): AVRational, AVCodecContext, open_video_encoder(), pix_fmt_for_encoder_name(), try_open_encoder(), try_open_vaapi_device(), try_open_vaapi_encoder()

### Community 66 - "render.rs"
Cohesion: 0.17
Nodes (36): AudioSegment, Canvas, ClipSegment, PathBuf, String, ShapeSegment, TextOverlaySegment, apply_audio_mix_pass() (+28 more)

### Community 67 - "transcript.rs"
Cohesion: 0.14
Nodes (22): cache_dir_for_project(), load_transcript_document(), word(), Display, Error, Formatter, Option, Path (+14 more)

### Community 68 - "App"
Cohesion: 0.15
Nodes (3): TrackKind, App, Pos2

### Community 69 - "audio_mix.c"
Cohesion: 0.24
Nodes (14): AudioMixStatus, avbridge_mix_audio_timeline(), avbridge_mux_video_audio(), build_mix_graph(), AVFilterContext, AVFilterGraph, AVFormatContext, AVPacket (+6 more)

### Community 70 - "text_to_speech.rs"
Cohesion: 0.11
Nodes (32): default_length_scale(), default_noise_scale(), default_noise_w(), io_from_hound(), load_voice_config(), phonemes_to_ids(), PiperAudioConfig, PiperEspeakConfig (+24 more)

### Community 71 - "motion_track_one"
Cohesion: 0.25
Nodes (4): App, motion_track_one(), Path, Vec

### Community 73 - "nested_sequence_test.rs"
Cohesion: 0.36
Nodes (14): clip(), detects_a_direct_self_nesting_cycle(), detects_an_indirect_nesting_cycle(), errors_on_a_missing_nested_sequence(), fixture(), is_a_noop_when_no_clip_is_nested(), project(), renders_a_real_nested_sequence_to_a_probeable_synthetic_asset() (+6 more)

### Community 74 - "preview_test.rs"
Cohesion: 0.11
Nodes (44): a_cropped_clip_shrinks_the_decoded_frame(), a_fade_transition_clip_still_opens_and_decodes(), a_flipped_clip_still_opens_and_decodes(), a_gained_clip_opens_with_a_real_audio_sink_and_still_decodes_video(), a_neutral_clip_applies_no_filter_bin_and_frame_size_is_unchanged(), a_pixelized_clip_still_opens_and_decodes_at_full_size(), a_shaken_clip_still_opens_and_decodes_at_full_size(), a_slide_transition_clip_still_opens_and_decodes_at_full_size() (+36 more)

### Community 75 - "render_test.rs"
Cohesion: 0.21
Nodes (19): resolve_shape_segments(), resolve_text_segments(), cancelling_mid_render_reports_cancelled(), fixture(), rejects_a_missing_source(), renders_and_normalizes_loudness_toward_target(), resolve_shape_segments_returns_empty_when_no_shape_track_exists(), resolve_shape_segments_returns_one_segment_for_a_shape_clip() (+11 more)

### Community 76 - "core/build.rs"
Cohesion: 0.44
Nodes (8): copy_dir_recursive(), find_espeak_data_source(), find_profile_dir(), main(), Option, Path, PathBuf, Result

### Community 77 - "split_keyframes_at"
Cohesion: 0.38
Nodes (7): evaluate_keyframes(), split_keyframes_at_drops_keyframes_that_land_on_the_other_side(), split_keyframes_at_is_empty_for_an_empty_input(), split_keyframes_at_rescales_and_inserts_a_matching_boundary_point(), T, Vec, split_keyframes_at()

### Community 78 - "draw.rs"
Cohesion: 0.07
Nodes (43): AudioRole, thumbnail_frame_index(), color_filter_tint(), draw_filmstrip(), draw_frozen_poster(), draw_keyframe_markers(), draw_marker_ticks(), draw_playhead() (+35 more)

### Community 79 - "INDEX.md"
Cohesion: 0.15
Nodes (6): Bundled fonts, Plano de execução, Known gaps, Robustness, Definition of done (short form), Mandatory Rules (read before implementing anything from ROADMAP.md)

### Community 80 - "src/update_check.rs"
Cohesion: 0.16
Nodes (24): apply_update(), ApplyUpdateError, ApplyUpdateOutcome, auto_update_supported(), expected_update_asset_name(), fetch_latest_release(), GithubAsset, GithubRelease (+16 more)

### Community 81 - "ExportJob"
Cohesion: 0.06
Nodes (34): Box, Condvar, every_preset_has_a_distinct_non_empty_label(), every_preset_targets_the_vertical_aspect_ratio(), ExportJob, ExportJobStatus, PlatformExportPreset, String (+26 more)

### Community 82 - "generate_tts_one"
Cohesion: 0.22
Nodes (6): App, generate_tts_one(), Path, PathBuf, Result, String

### Community 84 - "enum_combo"
Cohesion: 0.33
Nodes (5): enum_combo(), Fn, String, T, Ui

### Community 85 - "Watch-Gameplay.ps1"
Cohesion: 0.47
Nodes (3): Format-Arg(), Get-AudioAnalysis(), Invoke-FfmpegWithProgress()

### Community 86 - "Duration"
Cohesion: 0.19
Nodes (8): FrameSampler, Option, Path, Result, Self, Vec, VideoFrame, Duration

### Community 87 - "silence_detection.rs"
Cohesion: 0.15
Nodes (19): amplitude_exactly_at_threshold_counts_as_silent(), clip_silence_gaps(), clips_a_gap_that_straddles_the_clips_trim_boundary(), detect_silence_gaps(), detects_a_single_silent_run_in_the_middle(), detects_silence_running_to_the_very_end(), ignores_runs_shorter_than_the_minimum_duration(), loud() (+11 more)

### Community 88 - "Auto-update release contract"
Cohesion: 0.40
Nodes (4): Application flow, Asset names, Auto-update release contract, Publishing checklist

### Community 90 - "shape_render_test.rs"
Cohesion: 0.14
Nodes (18): build_shape_filter_desc(), rgb_to_ycbcr(), build_shape_filter_desc_animates_position_when_center_keyframes_are_present(), build_shape_filter_desc_animates_rotation_via_geq_sin_cos_when_rotation_keyframes_are_present(), build_shape_filter_desc_animates_size_when_width_or_height_keyframes_are_present(), build_shape_filter_desc_bakes_in_the_visible_time_window(), build_shape_filter_desc_ellipse_uses_lte_not_mod(), build_shape_filter_desc_starts_with_geq_and_has_all_channels() (+10 more)

### Community 95 - "preview_effects.rs"
Cohesion: 0.09
Nodes (30): a_lut_that_zeroes_red_darkens_only_the_red_channel_of_the_frame(), apply_glitch_to_rgba(), apply_lut_to_rgba(), apply_vignette_to_rgba(), empty_buffer_glitch_is_a_no_op(), glitch_actually_perturbs_pixels_at_full_intensity(), glitch_differs_across_seeds(), glitch_is_deterministic_for_the_same_seed() (+22 more)

### Community 96 - "String"
Cohesion: 0.18
Nodes (14): CropRect, AutoReframeEvent, AvailableUpdate, MatteGenerationEvent, RenderEvent, Arc, AtomicBool, String (+6 more)

### Community 97 - "Complex Text Shaping and Bidirectional Layout"
Cohesion: 0.08
Nodes (24): Acceptance spike, Architecture decision, Caching and concurrency, Candidate stack, Cluster-safe timed highlights, Complex Text Shaping and Bidirectional Layout, Definition of done, Deterministic fallback and bundled coverage (+16 more)

### Community 98 - "ytbridge/build.rs"
Cohesion: 0.90
Nodes (4): copy_dir_all(), copy_file(), main(), Path

### Community 100 - "App"
Cohesion: 0.06
Nodes (19): MarkerKind, App, format_color_hex(), parse_alpha(), parse_byte(), parse_color_value(), parse_hex_color(), Result (+11 more)

### Community 101 - "bundle_macos_dylibs.py"
Cohesion: 0.38
Nodes (14): build_index(), dependencies(), digest(), expand_special(), install_ids(), is_macho(), loader_reference(), main() (+6 more)

### Community 102 - "auto_reframe_one"
Cohesion: 0.27
Nodes (7): App, auto_reframe_one(), extract_frame(), Option, Path, UnboundedSender, Vec

### Community 103 - "test_asset"
Cohesion: 0.18
Nodes (12): apply_silence_review_ripple_deletes_only_accepted_gaps_and_closes_the_modal(), detach_audio_is_a_no_op_when_the_asset_has_no_audio(), ensure_preview_loaded_clears_state_once_the_playhead_moves_past_every_clip(), ensure_preview_loaded_resolves_the_clip_covering_the_playhead(), pump_import_queue_adds_the_asset_to_its_target_project_with_a_fresh_id(), pump_import_queue_applies_enrichment_to_the_asset_it_was_assigned(), pump_import_queue_targets_the_project_by_id_not_the_active_index(), selecting_a_sequence_clears_undo_history_from_the_previous_one() (+4 more)

### Community 104 - "main"
Cohesion: 0.80
Nodes (4): main(), Path, sha256(), write()

### Community 105 - "dpkg_field"
Cohesion: 0.83
Nodes (3): dpkg_field(), main(), Path

### Community 106 - "Differentiator Features (beyond `features/request.md`)"
Cohesion: 0.20
Nodes (9): D1 — Automatic silence/dead-air cut, D2 — Highlight detection from audio spikes, D3 — Series-level loudness consistency, D4 — Automatic chapter markers from scene cuts, D5 — Beat-aligned cut snapping, D6 — One-click shorts pack, D7 — Lightweight collaboration package, Differentiator Features (beyond `features/request.md`) (+1 more)

### Community 108 - "Client Error Reporting"
Cohesion: 0.09
Nodes (23): Architecture decision, Client Error Reporting, Component boundary, Consent and user controls, Data minimization and sanitization, Definition of done, ER-01A: Contract, sanitization, and local adapter, ER-01B: Consent and Rust reporting (+15 more)

### Community 109 - "test_track"
Cohesion: 0.06
Nodes (50): add_shape_clip_ids_stay_unique_past_an_existing_high_shape_clip_id(), apply_detected_scene_cuts_adds_numbered_chapter_markers_at_timeline_coordinates(), apply_detected_scene_cuts_numbering_continues_from_existing_chapters(), apply_silence_review_with_nothing_accepted_is_a_no_op(), apply_speed_ramp_clamps_to_speed_factor_range(), apply_speed_ramp_is_a_no_op_with_fewer_than_two_steps(), apply_speed_ramp_splits_into_contiguous_steps_with_interpolated_speed(), apply_transcript_proposals_ripple_deletes_only_accepted_and_closes() (+42 more)

### Community 110 - "PathBuf"
Cohesion: 0.16
Nodes (8): prefs_path(), Context, PathBuf, sentinel_path(), tts_output_dir(), TtsEvent, youtube_downloads_dir(), App

### Community 111 - "Performance & Caching Patterns (ported from nimble)"
Cohesion: 0.29
Nodes (7): 1. Dirty flags, not blanket invalidation, 2. Versioned cache, recomputed on demand, 3. No allocation/parsing in the hot path, 4. No permanent fake APIs, 5. Reuse an existing primitive before building a new one, 6. Central mutation pipeline, Performance & Caching Patterns (ported from nimble)

### Community 112 - "AGENTS.md"
Cohesion: 0.25
Nodes (7): Approach, Architecture, Commands, graphify, On-demand code quality references, Status, What this is

### Community 113 - "CLAUDE.md"
Cohesion: 0.29
Nodes (6): Approach, Architecture, Commands, graphify, Status, What this is

### Community 114 - "Effects, Keyframes, Color"
Cohesion: 0.33
Nodes (6): Confirmed hard wall — no matching GStreamer element exists on the dev machine at all, Effects, Keyframes, Color, Known gaps, Text, shapes, Wired to export, Wired to preview

### Community 115 - "Roadmap — Actionable Queue"
Cohesion: 0.22
Nodes (9): P0 — Editing Foundations, P1 — Performance Infrastructure, P2 — High-Impact Parity, P3 — Differentiators, P4 — Hardware-Dependent / Confirmed Hard Walls / Lower-Priority Parity, P5 — Competitive Product Growth, P6 — Explicitly Deferred, P7 — Design System Consolidation (+1 more)

### Community 116 - "EGUI.md"
Cohesion: 0.13
Nodes (15): card_frame(), Frame, open_in_finder(), App, PathBuf, Ui, show(), App (+7 more)

### Community 117 - "Runtime dependency bundle"
Cohesion: 0.40
Nodes (4): Deliberate system contract, Included components, Release process, Runtime dependency bundle

### Community 118 - "linux-launcher.sh"
Cohesion: 0.40
Nodes (4): LD_LIBRARY_PATH, OCA_RESOURCE_DIR, PATH, linux-launcher.sh script

### Community 119 - "main"
Cohesion: 0.80
Nodes (4): any_file(), main(), Path, sha256()

### Community 120 - "digest"
Cohesion: 0.83
Nodes (3): digest(), main(), Path

### Community 121 - "main"
Cohesion: 0.83
Nodes (3): main(), Path, sha256()

### Community 128 - "oca"
Cohesion: 0.40
Nodes (5): Common commands, oca, Prerequisites, Status, Workspace layout

### Community 129 - "Undo/Redo (`ROADMAP.md` P0 item 1)"
Cohesion: 0.40
Nodes (5): Call sites (item 3) — done, Design note: why per-sequence, not per-project, Done, Undo/Redo (`ROADMAP.md` P0 item 1), Wired

### Community 130 - "Competitor Parity Survey"
Cohesion: 0.33
Nodes (6): Competitor Parity Survey, Deliberately not adopted, New gaps found (2026-08-27 update — lower cost than the P5 tier), New gaps found (2026-08-27 update — reusing the existing `Keyframe<T>` system), New gaps found (not previously tracked anywhere in `spec/`), Validates existing plans (found independently, matches what's already queued)

### Community 132 - "Mandatory Rules & Definition of Done"
Cohesion: 0.50
Nodes (4): Before writing code, Definition of done, Keeping the spec in sync, Mandatory Rules & Definition of Done

### Community 133 - "spec/ — Start Here"
Cohesion: 0.50
Nodes (4): Directory map, Keeping this in sync, Reading order, spec/ — Start Here

### Community 134 - "bundle.rs"
Cohesion: 0.14
Nodes (20): bundled_resource_path(), bundled_resources_dir(), BundledResource, configure_bundled_runtime(), MissingBundleResources, resource_path_in(), resources_dir_for_executable(), Display (+12 more)

### Community 135 - "encode_timeline_export"
Cohesion: 0.21
Nodes (21): encode_export(), encode_timeline_export(), encode_timeline_export_multi(), EncodeOutcome, GpuEncoderPreference, AtomicBool, cancelling_mid_render_leaves_no_valid_file(), cancelling_mid_timeline_export_leaves_no_valid_file() (+13 more)

### Community 136 - "subtitles.rs"
Cohesion: 0.26
Nodes (11): export_srt(), exports_two_clips_as_sequential_numbered_entries(), format_srt_timestamp(), ignores_non_text_tracks(), orders_entries_by_start_time_regardless_of_input_order(), returns_an_empty_string_for_a_timeline_with_no_text_clips(), String, Vec (+3 more)

### Community 137 - "properties_panel"
Cohesion: 0.20
Nodes (16): ColorFilter, color_filter_label(), f32_keyframe_editor(), mask_shape_label(), polygon_vertex_editor(), position_keyframe_editor(), Option, String (+8 more)

### Community 141 - "editor/mod.rs"
Cohesion: 0.27
Nodes (21): audio_level_meter(), draw_custom_shape_surface(), draw_motion_track_region_picker(), export_srt_for_active_sequence(), fullscreen_preview_overlay(), layer_transform_preview(), media_library_panel(), preview_panel() (+13 more)

### Community 143 - "LibraryTrack"
Cohesion: 0.14
Nodes (13): LibraryTrack, Path, PathBuf, String, Vec, scan_library_dir(), SoundCategory, fixture() (+5 more)

### Community 144 - "save_transcript_document"
Cohesion: 0.26
Nodes (16): save_transcript_document(), ProjectTranscriptHit, Path, String, Vec, search_transcripts_in_project(), asset(), case_insensitive_and_partial_word_match() (+8 more)

### Community 145 - "src/highlight_detection.rs"
Cohesion: 0.23
Nodes (13): clip_amplitude_samples(), clip_amplitude_samples_excludes_buckets_outside_the_clips_trim_range(), clip_amplitude_samples_is_empty_for_non_positive_speed_factor(), clip_amplitude_samples_maps_every_bucket_into_timeline_coordinates(), detect_highlight_candidates(), detects_a_candidate_where_both_channels_spike_together(), does_not_flag_a_spike_on_only_one_channel(), drops_a_candidate_shorter_than_the_minimum_duration() (+5 more)

### Community 146 - "timeline_window_test.rs"
Cohesion: 0.25
Nodes (15): extract_timeline_window(), drops_a_clip_entirely_outside_the_window(), drops_a_text_clip_straddling_a_boundary_rather_than_trimming_word_timing(), drops_markers_and_resets_the_playhead(), empty_or_inverted_window_yields_no_tracks(), extracts_a_clip_fully_inside_the_window_and_rebases_it(), extracts_a_shape_clip_fully_inside_the_window_and_rebases_it(), Vec (+7 more)

### Community 147 - "transcript_test.rs"
Cohesion: 0.22
Nodes (12): from_transcribe_segments_flattens_words_with_sequential_ids(), Vec, save_and_load_round_trips_a_document(), segments(), validate_accepts_a_well_formed_document(), validate_rejects_a_duplicate_word_id(), validate_rejects_a_negative_start(), validate_rejects_a_newer_schema_version() (+4 more)

### Community 148 - "package.json"
Cohesion: 0.13
Nodes (14): husky, lint-staged, description, devDependencies, husky, lint-staged, lint-staged, *.{c,h} (+6 more)

### Community 149 - "project_test.rs"
Cohesion: 0.27
Nodes (13): duplicate_sequence_copies_the_complete_tab_with_a_fresh_id_and_name(), duplicate_sequence_is_a_no_op_for_an_out_of_range_index(), move_sequence_rejects_invalid_or_unchanged_positions(), move_sequence_reorders_tabs_without_changing_the_active_identity(), new_sequence_appends_and_switches_to_it(), new_sequence_ids_keep_increasing_after_multiple_calls(), new_sequence_inherits_the_active_sequences_export_settings(), remove_sequence_never_removes_the_projects_last_tab() (+5 more)

### Community 151 - "ProbeError"
Cohesion: 0.14
Nodes (12): ProbedMedia, ProbeError, Display, Error, Formatter, From, Option, Path (+4 more)

### Community 152 - "CURRENTLY IMPLEMENTED"
Cohesion: 0.11
Nodes (17): ClipInstance (`timeline.rs`), CURRENTLY IMPLEMENTED, Effects / Keyframes (`keyframe.rs`), Markers (`timeline.rs`), MediaAsset (`media.rs`), NOT YET DESIGNED, Persistence (`persistence.rs`), PLANNED / REQUIRED (per `spec/ROADMAP.md`, not yet built as of this writing) (+9 more)

### Community 153 - "SmartBin"
Cohesion: 0.26
Nodes (9): an_empty_bin_matches_everything(), kind_filter_excludes_the_other_kind(), multiple_criteria_are_anded_together(), name_contains_is_case_insensitive(), requires_audio_filters_either_way(), Option, Self, String (+1 more)

### Community 154 - "TranscribeSegment"
Cohesion: 0.24
Nodes (13): abort_trampoline(), collect_segments(), collect_words(), Arc, c_void, String, Vec, TranscribeOutcome (+5 more)

### Community 155 - "apply_text_overlays"
Cohesion: 0.45
Nodes (10): apply_text_overlays(), applies_a_keyframed_opacity_fade_to_a_text_overlay(), applies_a_keyframed_position_offset_to_a_text_overlay(), applies_a_keyframed_rotation_to_a_text_overlay(), applies_a_keyframed_scale_to_a_text_overlay(), applies_a_text_overlay_with_no_opacity_expr_unchanged(), fixture(), Path (+2 more)

### Community 156 - "Option"
Cohesion: 0.24
Nodes (17): AutoReframeState, ImportEvent, ImportState, MatteGenerationState, MotionTrackingState, HashMap, HashSet, Option (+9 more)

### Community 157 - "mix_audio_timeline"
Cohesion: 0.40
Nodes (9): AudioMixOutcome, mix_audio_timeline(), a_target_branch_without_any_trigger_falls_back_to_a_plain_mix(), fixture(), mixes_a_segment_with_a_keyframed_gain_expression(), mixes_overlapping_audio_segments_and_muxes_them_without_reencoding_video(), mixes_with_sidechain_ducking_across_multiple_branches_per_role(), mixes_with_sidechain_ducking_when_both_roles_are_present() (+1 more)

### Community 158 - "open_input"
Cohesion: 0.22
Nodes (8): AVFormatContext, open_input(), avbridge_probe(), avbridge_remux_copy(), avbridge_apply_shape_overlays(), TextOverlayStatus, ProbeStatus, RemuxStatus

### Community 159 - "extract_pcm_16k_mono"
Cohesion: 0.50
Nodes (7): extract_pcm_16k_mono(), extracts_16k_mono_samples_from_a_real_audio_file(), fails_on_missing_input(), fixture(), PathBuf, sample_count_matches_16khz_for_the_fixtures_known_duration(), works_on_a_video_files_embedded_audio_track_too()

### Community 160 - "overlay_render.rs"
Cohesion: 0.27
Nodes (13): active_highlight_word_index(), blend_pixel(), draw_laid_out_text(), draw_rounded_background(), draw_text_segment_onto(), ellipse_inside(), render_text_segment_rgba(), Font (+5 more)

### Community 161 - "shorts_pack_scratch_dir"
Cohesion: 0.22
Nodes (9): collab_bundle_scratch_dir(), export_collab_bundle_writes_a_zip_and_toasts_success(), import_collab_bundle_opens_the_project_with_its_new_file_path(), PathBuf, shorts_pack_scratch_dir(), spawn_shorts_pack_queues_one_job_per_highlight_marker(), spawn_shorts_pack_skips_a_window_landing_entirely_in_a_gap(), spawn_shorts_pack_toasts_when_there_are_no_highlight_markers() (+1 more)

### Community 162 - "Ordered implementation backlog"
Cohesion: 0.18
Nodes (11): CF-01: Transcript-based editing and speech cleanup, CF-02: Gameplay event ingestion and watched-folder import, CF-03: Integrated gameplay-voice cleanup, CF-04: Dynamic auto-reframe for vertical outputs, CF-05: OpenTimelineIO interchange, CF-06: Live multicam monitor, CF-07: Parameterized motion-graphics templates, CF-08: Semantic transcript and visual media search (+3 more)

### Community 163 - "shape_clip_properties"
Cohesion: 0.43
Nodes (7): App, String, Ui, shape_clip_properties(), shape_kind_to_preset(), shape_preset_label(), ShapePreset

### Community 164 - "text_clip_properties"
Cohesion: 0.43
Nodes (7): App, Response, Ui, text_clip_properties(), text_color_button(), text_font_family_label(), text_font_style_label()

### Community 165 - "code-quality/README.md"
Cohesion: 0.29
Nodes (4): Evidence standard, On-demand code quality references, Precedence, Routing

### Community 166 - "Rust code quality"
Cohesion: 0.25
Nodes (8): Concurrency, Dependencies and maintainability, Errors and control flow, Ownership and allocation, Required validation, Rust code quality, Types and APIs, Unsafe and FFI

### Community 167 - "Mobile Support (Android + iOS) — ADR"
Cohesion: 0.25
Nodes (7): Decision 1 — `avbridge`: cross-compile FFmpeg per target, keep the FFI shape, Decision 2 — `core::preview`: replace GStreamer with native players per platform, Decision 3 — `ui`: egui stays, but touch-first interaction is a redesign, not reflow, Decision 4 — packaging has no desktop equivalent, Mobile Support (Android + iOS) — ADR, Proposed phasing (if approved), Why this is hard: the three-crate split doesn't just recompile

### Community 168 - "generate_waveform"
Cohesion: 0.52
Nodes (6): generate_waveform(), computes_peaks_for_a_real_audio_file(), fails_on_missing_input(), fails_on_zero_bucket_count(), fixture(), PathBuf

### Community 169 - "Path"
Cohesion: 0.24
Nodes (12): apply_shape_overlays(), encode_matte_video(), generate_proxy(), measure_loudness_json(), mux_video_audio(), remux_copy(), Path, Result (+4 more)

### Community 170 - "prefs.rs"
Cohesion: 0.57
Nodes (6): model_path_row(), App, String, Ui, shortcut_binding_editor(), show()

### Community 171 - "queue.rs"
Cohesion: 0.43
Nodes (6): format_file_size(), open_containing_folder(), App, String, Ui, show()

### Community 172 - "Performance quality"
Cohesion: 0.29
Nodes (6): Caches and resource bounds, Evidence before optimization, Findings that block approval, Hot-path rules, Media and UI specifics, Performance quality

### Community 173 - "Code review checklist"
Cohesion: 0.29
Nodes (6): Code review checklist, Completion evidence, Finding format, Required gates, Review order, Severity

### Community 174 - "Security and trust boundaries"
Cohesion: 0.29
Nodes (7): Approval blockers, Dependencies, Files and paths, Native and external boundaries, Secrets, logs, and diagnostics, Security and trust boundaries, Untrusted input

### Community 175 - "TranscribeError"
Cohesion: 0.20
Nodes (8): Display, Error, Formatter, From, PathBuf, Result, Self, TranscribeError

### Community 177 - "tests/frame_sampler_test.rs"
Cohesion: 0.47
Nodes (4): fixture(), PathBuf, sample_can_be_called_repeatedly_against_the_same_open_pipeline(), sample_decodes_a_frame_at_the_requested_time()

### Community 180 - "nav_rail.rs"
Cohesion: 0.80
Nodes (5): prefs_button(), rail_button(), App, Ui, show()

### Community 181 - "SOLID adapted to Rust"
Cohesion: 0.18
Nodes (10): Core rules, Dependency inversion, Design principles, Design smells worth reporting, Interface segregation, Liskov substitution, Open/closed, Review questions (+2 more)

### Community 182 - "Testing quality"
Cohesion: 0.29
Nodes (6): Determinism and isolation, Quality checks, Review questions, Test level, Test the contract, Testing quality

### Community 184 - "Architecture"
Cohesion: 0.22
Nodes (8): Architecture, Async work — threads + mpsc, not tokio tasks, Caching convention — value-equality, not version counters, Crates and dependency direction, Dependency direction in practice, Layers, as actually implemented, Persistence framing, Undo/redo — full-snapshot, not command-pattern

### Community 185 - "workflow/README.md"
Cohesion: 0.22
Nodes (5): Agents (in pipeline order), Hard rules across the pipeline, How to run it, On-demand quality guidance, Workflow — multi-agent feature pipeline

### Community 187 - "probe"
Cohesion: 0.31
Nodes (10): probe(), ProbeInfo, Option, StreamKind, falls_back_to_audio_when_there_is_no_video_stream(), fixture(), missing_file_returns_open_error(), non_media_file_returns_open_error() (+2 more)

### Community 192 - "property.rs"
Cohesion: 0.15
Nodes (16): icon_button(), IconButtonOpts, Color32, Option, Response, Ui, property_block(), property_row() (+8 more)

### Community 194 - "breadcrumb.rs"
Cohesion: 0.39
Nodes (7): handle_resize_borders(), App, Color32, Response, Ui, show(), window_button()

### Community 195 - "ShapeKind"
Cohesion: 0.42
Nodes (3): Default, Self, ShapeKind

### Community 196 - "generate_matte_one"
Cohesion: 0.25
Nodes (6): App, generate_matte_one(), Path, PathBuf, Result, String

### Community 197 - "Competitive Feature Implementation Plan"
Cohesion: 0.25
Nodes (8): Competitive Feature Implementation Plan, Competitive findings, Current product baseline, Deliberate non-goals, Delivery order, Product strategy, Quick wins before or alongside CF-01, Security requirements shared by the backlog

### Community 198 - "Timeline"
Cohesion: 0.09
Nodes (10): ClipFormatting, LayerTemplate, Marker, MaskShape, MulticamGroup, HashMap, String, Vec (+2 more)

### Community 199 - "Design System"
Cohesion: 0.20
Nodes (9): Borders and shadows, Color — real tokens, `crates/ui/src/theme.rs`, Design System, Hierarchy, Icons, Personality, Spacing, States (+1 more)

### Community 200 - "tag.rs"
Cohesion: 0.54
Nodes (7): Color32, Ui, tag(), tag_accent(), tag_error(), tag_outline(), tag_warning()

### Community 201 - "UX Principles"
Cohesion: 0.20
Nodes (9): Context over permanence, Direct manipulation, Fast feedback, Long-session ergonomics, Preserve focus, Progressive disclosure, Spatial consistency, State clarity (+1 more)

### Community 206 - "UI Implementation Plan — oca"
Cohesion: 0.10
Nodes (19): 10. Risk Assessment, 11. Acceptance Criteria, 1. Current State Summary, 2. Product UX Vision, 3. Workspace Architecture, 4. Component Architecture, 5. Timeline Plan, 6. Design System Plan (+11 more)

### Community 207 - "solid_rgba"
Cohesion: 0.25
Nodes (13): chroma_cb_cr(), luma(), luma_waveform_rgba(), Vec, luma_waveform_is_empty_for_a_zero_sized_input_or_output(), luma_waveform_puts_a_solid_black_field_at_the_bottom_row(), luma_waveform_puts_a_solid_white_field_at_the_top_row(), Vec (+5 more)

### Community 208 - "build_video_filter_bin"
Cohesion: 0.18
Nodes (9): AtomicI32, build_video_filter_bin(), live_balance_element_name(), live_blur_element_name(), live_crop_element_name(), live_pixelize_caps_name(), String, shake_margin_pixels() (+1 more)

### Community 209 - "App"
Cohesion: 0.29
Nodes (3): App, Context, Option

### Community 210 - "multicam_sync.rs"
Cohesion: 0.26
Nodes (10): amplitude_envelope(), amplitude_envelope_reports_rms_per_window(), best_lag_windows(), best_lag_windows_finds_a_burst_shifted_earlier(), best_lag_windows_finds_a_burst_shifted_later(), burst_signal(), compute_sync_offset_secs(), compute_sync_offset_secs_matches_a_known_delay() (+2 more)

### Community 211 - "Project Instructions"
Cohesion: 0.22
Nodes (8): Before changing UI, Design before code, Done criteria, Non-negotiable UI principles, Project Instructions, Real build/test/quality commands, Real workspace structure, Rust principles

### Community 212 - "Timeline — Domain and Interaction Rules"
Cohesion: 0.20
Nodes (9): Coordinate systems, Core interactions — real implementation shape, How it's actually built, Markers, Strengths (preserve these), Thumbnails and waveforms, Timeline — Domain and Interaction Rules, Weaknesses / risks — see `engineering/TIMELINE_PERFORMANCE.md` (+1 more)

### Community 213 - "Timeline Performance"
Cohesion: 0.25
Nodes (7): Caching — already consistent, follow the convention, Hot path discipline — confirmed good, one exception worth checking, Interaction, Known gap: no track/clip-level viewport culling, Level of detail, Timeline Performance, When adding a new timeline feature

### Community 214 - "TextClip"
Cohesion: 0.41
Nodes (6): CompositeBranch, Option, Path, Self, ShapeClip, TextClip

### Community 215 - "egui Implementation Guide"
Cohesion: 0.22
Nodes (9): Custom widgets, egui Implementation Guide, i18n (`crates/ui/src/i18n.rs`, ~750 lines), Input, Layout, Repaint, State — real categories, with an important warning, Structure — real shape (+1 more)

### Community 216 - "Core Workflows"
Cohesion: 0.25
Nodes (7): Clip editing, Core Workflows, Export, Import to timeline, Playback, Property editing, Reviewable batch suggestions (silence review, transcript-proposal review)

### Community 217 - "Design System Consolidation Review"
Cohesion: 0.15
Nodes (12): 0. What already exists (read in full), 1. Sources of Truth, 2. Enforcement Strategy — why bypass is currently possible, 3. Frozen Design Foundation, 4. Component Inventory, Design System Consolidation Review, Interaction states — already canonical, no new definition needed, Radius — 3 named constants, not 4 preserved (+4 more)

### Community 218 - "UI Review Checklist"
Cohesion: 0.33
Nodes (5): egui, UI Review Checklist, UX, Video editor, Visual

### Community 219 - "render_shape_clip_rgba"
Cohesion: 0.47
Nodes (6): ellipse_center_pixel_is_opaque_and_corners_are_transparent(), rectangle_fills_its_whole_local_square_not_just_the_center(), sample_shape(), stroke_only_shape_leaves_its_own_center_transparent(), put_pixel(), render_shape_clip_rgba()

### Community 220 - "Claude Code Configuration for Rust + egui Video Editor"
Cohesion: 0.40
Nodes (4): Claude Code Configuration for Rust + egui Video Editor, Important, Install, Recommended workflow

### Community 221 - "PreviewState"
Cohesion: 0.20
Nodes (10): PreviewState, TextureHandle, Vec, SceneCutEvent, ThumbnailReady, TranscribeEvent, Path, UnboundedSender (+2 more)

### Community 222 - "PreviewError"
Cohesion: 0.20
Nodes (9): BoolError, PreviewError, Display, Error, Formatter, FlowError, PadLinkError, ProbeError (+1 more)

### Community 223 - "Position"
Cohesion: 0.22
Nodes (7): f32, position_overlay_xy_expr_builds_a_t_based_ramp_for_an_animated_axis(), position_overlay_xy_expr_scales_a_single_keyframe_by_canvas_dimensions(), Lerp, Position, position_overlay_xy_expr(), MotionTrackEvent

### Community 230 - "Quality Gates"
Cohesion: 0.33
Nodes (5): Commit discipline (repo-wide convention, applies to UI work too), "Done" gate for a UI change specifically, Git hooks, Quality Gates, Test placement — real convention

### Community 234 - "shape_render.rs"
Cohesion: 0.48
Nodes (6): ellipse_inside_expr(), inside_expr(), polygon_inside_expr(), String, ellipse_inside_expr_keyframed_width_produces_a_valid_multiply_through_test(), inside_expr_outline_wraps_outer_and_inner_tests()

### Community 235 - "smooth_speed_ramp_duration_secs"
Cohesion: 0.25
Nodes (8): smooth_speed_ramp_duration_secs_is_shorter_for_a_speed_up_ramp(), smooth_speed_ramp_duration_secs_matches_numerical_integration(), smooth_speed_ramp_duration_secs_matches_plain_division_for_a_constant_speed(), smooth_speed_ramp_source_secs_at_is_the_true_inverse_of_the_duration_formula(), smooth_speed_ramp_source_secs_at_matches_plain_multiply_for_a_constant_speed(), smooth_speed_ramp_duration_secs(), smooth_speed_ramp_elapsed_timeline_secs(), smooth_speed_ramp_source_secs_at()

## Knowledge Gaps
- **492 isolated node(s):** `App`, `App`, `SequenceTabDrag`, `TrimEdge`, `name` (+487 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **31 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `ClipInstance` connect `ClipInstance` to `timeline_test.rs`, `src/preview.rs`, `properties_panel`, `track_with`, `next_clip_id`, `Track`, `src/highlight_detection.rs`, `timeline_window_test.rs`, `transcript_proposals_test.rs`, `timeline_export_multi_test.rs`, `timeline_with`, `Keyframe`, `App`, `probe_media`, `render.rs`, `App`, `Timeline`, `nested_sequence_test.rs`, `preview_test.rs`, `draw.rs`, `build_video_filter_bin`, `App`, `TextClip`, `silence_detection.rs`, `Position`, `test_track`?**
  _High betweenness centrality (0.149) - this node is a cross-community bridge._
- **Why does `App` connect `App` to `UndoStack`, `MediaAsset`, `LibraryTrack`, `transcript_proposals_test.rs`, `SmartBin`, `Option`, `Project`, `ClipInstance`, `Locale`, `app/mod.rs`, `render.rs`, `transcript.rs`, `Timeline`, `ExportJob`, `silence_detection.rs`, `PreviewState`, `String`, `App`, `.add_and_open_project`, `PathBuf`?**
  _High betweenness centrality (0.104) - this node is a cross-community bridge._
- **Why does `Project` connect `Project` to `persistence_test.rs`, `app_test.rs`, `transcript.rs`, `App`, `Timeline`, `test_asset`, `test_project`, `.add_and_open_project`, `ensure_proxy`, `MediaAsset`, `test_track`, `src/background_removal.rs`, `Locale`, `project_test.rs`, `SmartBin`?**
  _High betweenness centrality (0.079) - this node is a cross-community bridge._
- **What connects `App`, `App`, `SequenceTabDrag` to the rest of the system?**
  _492 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `keyframe_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.04 - nodes in this community are weakly interconnected._
- **Should `persistence_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06882882882882883 - nodes in this community are weakly interconnected._
- **Should `app_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.017872523686477175 - nodes in this community are weakly interconnected._