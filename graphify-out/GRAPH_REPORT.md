# Graph Report - oca  (2026-08-15)

## Corpus Check
- 126 files · ~155,759 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1572 nodes · 3080 edges · 76 communities (69 shown, 7 thin omitted)
- Extraction: 93% EXTRACTED · 7% INFERRED · 0% AMBIGUOUS · INFERRED: 203 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `ace25a5d`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- keyframe_test.rs
- Project
- app_test.rs
- avbridge/src/lib.rs
- timeline_test.rs
- preview_test.rs
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
- Plano de Execução — oca (PacoPaçoca)
- Manager Agent
- task_id: impl-004b
- Docs Specialist
- Git Agent
- Impl Specialist
- Test Specialist
- Watch-Gameplay.ps1
- workflow/README.md
- Required checklist
- main.rs
- Commit Plan — Fase 1
- test_layer_transform.py
- generate_waveform
- timeline_panel.rs
- transcribe
- TrackKind
- download_whisper_model
- Security Fix: FFmpeg Download Verification
- test_asset
- avbridge_encode_timeline_export_multi
- ClipInstance
- LibraryTrack
- MediaAsset
- track_region
- timeline_with
- background_removal_test.rs
- text_metrics_test.rs
- App
- Track
- Locale
- Update Process
- src/auto_reframe.rs
- test_canvas
- keyframe.rs
- render.rs
- bridge_internal.h
- App
- pcm_extract.c
- App
- open_input
- free_audio_filter_chain
- avbridge_generate_waveform
- avbridge_measure_loudness
- gpu_encoder.c
- theme.rs
- split_keyframes_at
- test_project
- extract_thumbnail
- motion_track_one
- auto_reframe_one
- Keyframe
- Position
- App

## God Nodes (most connected - your core abstractions)
1. `test_app()` - 133 edges
2. `clip()` - 62 edges
3. `App` - 54 edges
4. `ClipInstance` - 53 edges
5. `track_with()` - 36 edges
6. `Project` - 28 edges
7. `MediaAsset` - 27 edges
8. `render_timeline_export()` - 25 edges
9. `App` - 25 edges
10. `probe_media()` - 24 edges

## Surprising Connections (you probably didn't know these)
- `scale_filter_expr_builds_a_geq_expression_for_an_animated_ramp()` --calls--> `scale_filter_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `scale_filter_expr_is_static_crop_scale_for_a_single_non_unity_keyframe()` --calls--> `scale_filter_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `rotation_filter_angle_expr_converts_a_single_keyframe_to_radians()` --calls--> `rotation_filter_angle_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `rotation_filter_angle_expr_uses_t_for_an_animated_ramp()` --calls--> `rotation_filter_angle_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs
- `opacity_alpha_ramp_expr_clamps_a_single_keyframe()` --calls--> `opacity_alpha_ramp_expr()`  [INFERRED]
  crates/core/src/keyframe/keyframe_test.rs → crates/core/src/keyframe.rs

## Import Cycles
- 2-file cycle: `crates/ui/src/app/mod.rs -> crates/ui/src/i18n.rs -> crates/ui/src/app/mod.rs`

## Communities (76 total, 7 thin omitted)

### Community 0 - "keyframe_test.rs"
Cohesion: 0.09
Nodes (7): opacity_alpha_ramp_expr_clamps_a_single_keyframe(), position_overlay_xy_expr_builds_a_t_based_ramp_for_an_animated_axis(), position_overlay_xy_expr_scales_a_single_keyframe_by_canvas_dimensions(), rotation_filter_angle_expr_converts_a_single_keyframe_to_radians(), rotation_filter_angle_expr_uses_t_for_an_animated_ramp(), scale_filter_expr_builds_a_geq_expression_for_an_animated_ramp(), scale_filter_expr_is_static_crop_scale_for_a_single_non_unity_keyframe()

### Community 1 - "Project"
Cohesion: 0.08
Nodes (40): bench_parse_loudnorm_stderr(), bench_project_ocproj_round_trip(), bench_timeline_duration(), large_project(), from_ocproj_bytes(), load_project_from_file(), PersistError, Display (+32 more)

### Community 2 - "app_test.rs"
Cohesion: 0.03
Nodes (113): add_asset_to_timeline_creates_a_track_and_appends_a_clip_when_none_exists(), add_asset_to_timeline_creates_an_audio_track_for_an_audio_asset(), add_asset_to_timeline_is_a_no_op_for_an_unknown_asset_id(), add_sequence_appends_a_named_tab_and_switches_to_it(), add_sequence_clears_a_stale_clip_selection(), begin_apply_layer_template_stages_one_empty_slot_per_layer(), begin_save_layer_template_is_a_no_op_when_nothing_is_multi_selected(), cancel_export_job_flags_an_active_render_instead_of_removing_it() (+105 more)

### Community 3 - "avbridge/src/lib.rs"
Cohesion: 0.06
Nodes (69): c_char, c_int, c_longlong, apply_text_overlays(), encode_export(), encode_timeline_export(), encode_timeline_export_multi(), EncodeError (+61 more)

### Community 4 - "timeline_test.rs"
Cohesion: 0.06
Nodes (45): apply_formatting_overwrites_all_effect_fields(), clip(), formatting_roundtrip_preserves_all_fields(), has_vignette_is_true_above_zero(), is_color_filtered_is_true_for_any_filter_but_none(), is_cropped_is_true_when_any_crop_field_differs_from_the_full_frame(), is_masked_is_true_for_any_shape_but_none(), negative_gain_db_scales_linear_gain_below_unity() (+37 more)

### Community 5 - "preview_test.rs"
Cohesion: 0.08
Nodes (32): AppSink, BoolError, build_video_filter_bin(), Preview, PreviewError, Display, Error, Formatter (+24 more)

### Community 6 - "timeline.rs"
Cohesion: 0.11
Nodes (9): ClipFormatting, ColorFilter, LayerTemplate, MaskShape, String, Vec, TextClip, TransitionType (+1 more)

### Community 7 - "encode_write_packet"
Cohesion: 0.32
Nodes (14): AVStream, AVCodecContext, AVFormatContext, AVFrame, AVPacket, encode_write_packet(), filter_encode_write_frame(), filter_encode_write_video_frame() (+6 more)

### Community 8 - "App"
Cohesion: 0.07
Nodes (37): CropRect, App, AutoReframeEvent, BindableAction, EditorTool, ImportEvent, KeyBindings, KeyCombo (+29 more)

### Community 9 - "Plano de Execução — oca (PacoPaçoca)"
Cohesion: 0.06
Nodes (28): Approach, Architecture, Commands, graphify, What this is, Approach, Architecture, Commands (+20 more)

### Community 10 - "track_with"
Cohesion: 0.06
Nodes (33): clip_at_finds_the_clip_covering_a_position(), clip_at_is_exclusive_of_a_clips_end(), clip_at_is_inclusive_of_a_clips_start(), clip_at_is_none_for_a_gap_between_clips(), clip_mut_finds_a_clip_by_id(), clip_mut_returns_none_for_an_unknown_id(), move_clip_is_a_no_op_for_a_negative_position(), move_clip_is_a_no_op_for_an_unknown_clip_id() (+25 more)

### Community 11 - "ensure_proxy"
Cohesion: 0.14
Nodes (19): cache_dir_for_project(), ensure_proxy(), is_up_to_date(), proxy_path_for(), a_proxy_newer_than_its_source_is_up_to_date(), a_proxy_older_than_its_source_is_not_up_to_date(), missing_proxy_is_not_up_to_date(), ProxyError (+11 more)

### Community 12 - "parse_loudnorm_stderr"
Cohesion: 0.14
Nodes (20): extract_first_json_object(), LoudnessError, LoudnormReport, measure_loudness(), parse_loudnorm_stderr(), Display, Error, Formatter (+12 more)

### Community 14 - "Task Breakdown — Fase 1 (Motor central)"
Cohesion: 0.12
Nodes (15): Task Breakdown — Fase 1 (Motor central), task_id: chore-001, task_id: chore-002, task_id: docs-001, task_id: docs-002, task_id: impl-002a, task_id: impl-002b, task_id: impl-003 (+7 more)

### Community 15 - "Reviewer Agent"
Cohesion: 0.29
Nodes (6): Constraints, Decision rules, Expected inputs, Required output: `review_report.md`, Reviewer Agent, Role

### Community 16 - "Plano de Execução — oca (PacoPaçoca)"
Cohesion: 0.15
Nodes (12): Decisões já tomadas, Fase 0 — Escopo do MVP, Fase 1 — Motor central, Fase 2 — Ajuste automático de áudio, Fase 3 — Timeline, edição e organização do projeto, Fase 4 — Ferramentas e efeitos de edição, Fase 5 — Exportação e fila em background, Fase 6 — Robustez (+4 more)

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

### Community 23 - "Watch-Gameplay.ps1"
Cohesion: 0.47
Nodes (3): Format-Arg(), Get-AudioAnalysis(), Invoke-FfmpegWithProgress()

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

### Community 35 - "timeline_panel.rs"
Cohesion: 0.22
Nodes (18): color_filter_tint(), draw_filmstrip(), draw_frozen_poster(), draw_keyframe_markers(), draw_playhead(), draw_waveform(), App, Color32 (+10 more)

### Community 36 - "transcribe"
Cohesion: 0.07
Nodes (38): abort_trampoline(), collect_segments(), collect_words(), Arc, AtomicBool, c_void, Display, Error (+30 more)

### Community 37 - "TrackKind"
Cohesion: 0.22
Nodes (6): TrackKind, create_new_track(), next_clip_id(), resolve_or_create_track(), Option, ClipDrag

### Community 38 - "download_whisper_model"
Cohesion: 0.11
Nodes (25): BackgroundRemovalModel, download_background_removal_model(), download_file(), download_reframe_model(), download_whisper_model(), DownloadError, DownloadOutcome, ReframeModel (+17 more)

### Community 39 - "Security Fix: FFmpeg Download Verification"
Cohesion: 0.12
Nodes (16): 1. `.github/workflows/ci.yml`, 2. `.github/FFMPEG_UPDATE.md` (new file), After, Attack Surface Reduction, Before, Changes Made, Environment Variables (lines 9-16), Implementation Notes (+8 more)

### Community 40 - "test_asset"
Cohesion: 0.25
Nodes (9): ensure_preview_loaded_clears_state_once_the_playhead_moves_past_every_clip(), ensure_preview_loaded_resolves_the_clip_covering_the_playhead(), pump_import_queue_adds_the_asset_to_its_target_project_with_a_fresh_id(), pump_import_queue_applies_enrichment_to_the_asset_it_was_assigned(), pump_import_queue_targets_the_project_by_id_not_the_active_index(), Vec, test_asset(), test_project_with_tracks() (+1 more)

### Community 41 - "avbridge_encode_timeline_export_multi"
Cohesion: 0.20
Nodes (15): AVFilterContext, AVFilterGraph, advance_overlay_decoder(), avbridge_encode_timeline_export_multi(), build_overlay_vfilter(), build_vfilter_descr(), AVCodecContext, AVFrame (+7 more)

### Community 43 - "LibraryTrack"
Cohesion: 0.14
Nodes (13): LibraryTrack, Path, PathBuf, String, Vec, scan_library_dir(), SoundCategory, fixture() (+5 more)

### Community 44 - "MediaAsset"
Cohesion: 0.06
Nodes (50): format_timecode(), MediaAsset, MediaKind, Option, PathBuf, String, Vec, probe_media() (+42 more)

### Community 45 - "track_region"
Cohesion: 0.24
Nodes (14): extract_patch(), GrayFrame, a_single_frame_produces_one_clamped_position(), frame_with_block(), initial_center_near_the_edge_is_clamped_so_the_template_fits(), keyframes_ride_the_delta_from_the_first_tracked_frame(), stays_put_when_the_block_does_not_move(), tracks_a_block_moving_in_a_straight_line() (+6 more)

### Community 46 - "timeline_with"
Cohesion: 0.25
Nodes (8): move_clip_to_track_is_a_no_op_across_mismatched_kinds(), move_clip_to_track_is_a_no_op_for_a_negative_position(), move_clip_to_track_is_a_no_op_for_an_unknown_target_track(), move_clip_to_track_relocates_the_clip_to_a_same_kind_track(), Vec, timeline_clip_mut_finds_a_clip_across_tracks(), timeline_clip_mut_returns_none_for_an_unknown_id(), timeline_with()

### Community 47 - "background_removal_test.rs"
Cohesion: 0.14
Nodes (21): approx(), model_input_size_leaves_frames_already_spanning_target_alone_then_rounds(), model_input_size_rounds_down_to_multiple_of_32(), model_input_size_scales_constrained_side_toward_target(), model_input_size_upscales_tiny_frames_toward_target(), preprocess_normalizes_black_pixel_to_negative_one(), preprocess_normalizes_white_pixel_to_one(), resize_matte_downsamples_without_out_of_bounds() (+13 more)

### Community 48 - "text_metrics_test.rs"
Cohesion: 0.18
Nodes (12): default_font(), Option, Vec, text_width_px_grows_with_more_characters(), text_width_px_scales_up_with_font_size(), word_x_offsets_px_is_empty_for_no_words(), word_x_offsets_px_is_strictly_increasing(), word_x_offsets_px_matches_word_count() (+4 more)

### Community 51 - "Locale"
Cohesion: 0.07
Nodes (42): ExportJob, ExportJobStatus, String, Vec, Recency, test_job(), Screen, job() (+34 more)

### Community 52 - "Update Process"
Cohesion: 0.13
Nodes (14): 1. Choose a Release, 2. Download Assets and Compute Checksums, 3. Update Workflow Environment Variables, 4. Update Cache Keys, 5. Test the Changes, Checksum Mismatch Error, Download Failures, FFmpeg Update Guide (+6 more)

### Community 53 - "src/auto_reframe.rs"
Cohesion: 0.12
Nodes (23): approx(), main_subject_picks_highest_score(), nms_collapses_overlapping_boxes(), no_subject_falls_back_to_center_crop(), same_aspect_ratio_keeps_full_frame(), subject_near_edge_clamps_without_overflow(), subject_off_center_shifts_crop_toward_it(), wider_target_keeps_full_width() (+15 more)

### Community 54 - "test_canvas"
Cohesion: 0.50
Nodes (4): pending_export_conflict_overwrite_queues_with_the_original_path(), queue_export_appends_a_queued_job_with_the_next_id(), queue_export_starts_at_one_when_no_jobs_exist(), test_canvas()

### Community 55 - "keyframe.rs"
Cohesion: 0.47
Nodes (10): clamp_scale(), opacity_alpha_ramp_expr(), piecewise_expr(), position_overlay_xy_expr(), rotation_filter_angle_expr(), Fn, Option, String (+2 more)

### Community 56 - "render.rs"
Cohesion: 0.06
Nodes (77): Canvas, ClipSegment, PathBuf, String, TextSegment, Sequence, apply_export_aspect_ratio(), apply_text_overlay_pass() (+69 more)

### Community 57 - "bridge_internal.h"
Cohesion: 0.32
Nodes (3): AVCodec, loudness_log_callback(), va_list

### Community 58 - "App"
Cohesion: 0.22
Nodes (4): App, frozen_playhead(), Context, Option

### Community 59 - "pcm_extract.c"
Cohesion: 0.29
Nodes (10): append_pcm_frame(), avbridge_extract_pcm_16k_mono(), AudioFilterChain, AVCodecContext, AVFrame, drain_pcm_frame(), init_pcm_filter_chain(), pcm_buffer_reserve() (+2 more)

### Community 60 - "App"
Cohesion: 0.18
Nodes (6): App, autosave_is_newer(), media_kind_matches(), Context, Path, String

### Community 61 - "open_input"
Cohesion: 0.20
Nodes (9): AVFormatContext, open_input(), avbridge_probe(), avbridge_remux_copy(), avbridge_apply_text_overlays(), escape_drawtext_text(), ProbeStatus, RemuxStatus (+1 more)

### Community 62 - "free_audio_filter_chain"
Cohesion: 0.27
Nodes (10): avbridge_encode_export(), EncodeStatus, ProgressCallback, AudioFilterChain, free_audio_filter_chain(), init_audio_filter_chain(), avbridge_generate_proxy(), AVFrame (+2 more)

### Community 63 - "avbridge_generate_waveform"
Cohesion: 0.31
Nodes (9): accumulate_waveform_frame(), avbridge_generate_waveform(), AudioFilterChain, AVCodecContext, AVFrame, drain_waveform_frame(), init_waveform_filter_chain(), WaveformAccumulator (+1 more)

### Community 64 - "avbridge_measure_loudness"
Cohesion: 0.33
Nodes (7): avbridge_measure_loudness(), AudioFilterChain, AVCodecContext, AVFrame, drain_measure_frame(), init_measure_filter_chain(), LoudnessStatus

### Community 65 - "gpu_encoder.c"
Cohesion: 0.60
Nodes (5): AVRational, AVCodecContext, open_video_encoder(), pix_fmt_for_encoder_name(), try_open_encoder()

### Community 66 - "theme.rs"
Cohesion: 0.06
Nodes (46): card_frame(), Frame, property_block(), property_section(), property_toggle(), FnOnce, Ui, Ui (+38 more)

### Community 68 - "split_keyframes_at"
Cohesion: 0.38
Nodes (7): evaluate_keyframes(), split_keyframes_at_drops_keyframes_that_land_on_the_other_side(), split_keyframes_at_is_empty_for_an_empty_input(), split_keyframes_at_rescales_and_inserts_a_matching_boundary_point(), T, Vec, split_keyframes_at()

### Community 69 - "test_project"
Cohesion: 0.17
Nodes (12): add_and_open_project_appends_and_opens_it(), add_asset_to_timeline_appends_after_whatever_is_already_on_the_matching_track(), begin_save_layer_template_snapshots_the_multi_selection_ordered_by_track_then_start(), confirm_apply_layer_template_creates_one_clip_per_filled_layer_with_its_formatting(), confirm_apply_layer_template_puts_each_layer_on_its_own_new_track(), confirm_apply_layer_template_skips_layers_left_without_an_asset(), select_timeline_clip_synchronizes_its_backing_asset(), test_clip() (+4 more)

### Community 70 - "extract_thumbnail"
Cohesion: 0.20
Nodes (10): App, downscale_rgba(), extract_thumbnail(), import_one(), Context, Option, Path, PathBuf (+2 more)

### Community 71 - "motion_track_one"
Cohesion: 0.33
Nodes (4): App, motion_track_one(), Path, Vec

### Community 73 - "auto_reframe_one"
Cohesion: 0.27
Nodes (7): App, auto_reframe_one(), extract_frame(), Option, Path, UnboundedSender, Vec

### Community 75 - "Position"
Cohesion: 0.33
Nodes (4): f32, Lerp, Position, MotionTrackEvent

## Knowledge Gaps
- **129 isolated node(s):** `TrimEdge`, `Security Context`, `1. Choose a Release`, `Windows (PowerShell)`, `Linux (bash)` (+124 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **7 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `App` connect `App` to `Project`, `preview_test.rs`, `timeline.rs`, `TrackKind`, `ClipInstance`, `LibraryTrack`, `Position`, `Locale`, `render.rs`?**
  _High betweenness centrality (0.169) - this node is a cross-community bridge._
- **Why does `ClipInstance` connect `ClipInstance` to `Project`, `timeline_panel.rs`, `timeline_test.rs`, `preview_test.rs`, `timeline.rs`, `test_project`, `test_asset`, `App`, `Keyframe`, `Position`, `MediaAsset`, `track_with`, `App`, `Track`, `render.rs`, `App`?**
  _High betweenness centrality (0.137) - this node is a cross-community bridge._
- **Why does `Project` connect `Project` to `app_test.rs`, `test_project`, `test_asset`, `App`, `ensure_proxy`, `MediaAsset`, `Track`, `Locale`, `render.rs`?**
  _High betweenness centrality (0.076) - this node is a cross-community bridge._
- **What connects `TrimEdge`, `Security Context`, `1. Choose a Release` to the rest of the system?**
  _129 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `keyframe_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.08695652173913043 - nodes in this community are weakly interconnected._
- **Should `Project` be split into smaller, more focused modules?**
  _Cohesion score 0.08244897959183674 - nodes in this community are weakly interconnected._
- **Should `app_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.03302911777488049 - nodes in this community are weakly interconnected._