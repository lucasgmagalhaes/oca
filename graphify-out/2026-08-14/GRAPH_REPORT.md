# Graph Report - oca  (2026-08-14)

## Corpus Check
- 111 files · ~140,866 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1386 nodes · 2699 edges · 68 communities (67 shown, 1 thin omitted)
- Extraction: 94% EXTRACTED · 6% INFERRED · 0% AMBIGUOUS · INFERRED: 173 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `0f7c78a3`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- keyframe_test.rs
- Project
- app_test.rs
- avbridge/src/lib.rs
- timeline_test.rs
- preview_test.rs
- Locale
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
- poll_for_descendants
- generate_waveform
- timeline_panel.rs
- transcribe
- render_timeline_export
- download_whisper_model
- Security Fix: FFmpeg Download Verification
- test_project
- avbridge_encode_timeline_export_multi
- ClipInstance
- ExportJob
- property.rs
- prefs.rs
- timeline_with
- theme.rs
- text_metrics_test.rs
- extract_thumbnail
- tag.rs
- properties_panel.rs
- Update Process
- queue.rs
- test_canvas
- nav_rail.rs
- render.rs
- bridge_internal.h
- App
- pcm_extract.c
- editor/mod.rs
- open_input
- free_audio_filter_chain
- avbridge_generate_waveform
- avbridge_measure_loudness
- gpu_encoder.c
- home.rs

## God Nodes (most connected - your core abstractions)
1. `test_app()` - 123 edges
2. `clip()` - 56 edges
3. `App` - 50 edges
4. `ClipInstance` - 48 edges
5. `track_with()` - 34 edges
6. `Project` - 30 edges
7. `MediaAsset` - 27 edges
8. `render_timeline_export()` - 25 edges
9. `App` - 24 edges
10. `probe_media()` - 23 edges

## Surprising Connections (you probably didn't know these)
- `avbridge_encode_export()` --calls--> `open_input()`  [INFERRED]
  crates/avbridge/csrc/export.c → crates/avbridge/csrc/common.c
- `avbridge_measure_loudness()` --calls--> `open_input()`  [INFERRED]
  crates/avbridge/csrc/loudness.c → crates/avbridge/csrc/common.c
- `avbridge_extract_pcm_16k_mono()` --calls--> `open_input()`  [INFERRED]
  crates/avbridge/csrc/pcm_extract.c → crates/avbridge/csrc/common.c
- `avbridge_generate_proxy()` --calls--> `open_input()`  [INFERRED]
  crates/avbridge/csrc/proxy.c → crates/avbridge/csrc/common.c
- `avbridge_encode_timeline_export()` --calls--> `open_input()`  [INFERRED]
  crates/avbridge/csrc/timeline_export.c → crates/avbridge/csrc/common.c

## Import Cycles
- 2-file cycle: `crates/ui/src/app/mod.rs -> crates/ui/src/i18n.rs -> crates/ui/src/app/mod.rs`

## Communities (68 total, 1 thin omitted)

### Community 0 - "keyframe_test.rs"
Cohesion: 0.05
Nodes (30): clamp_scale(), evaluate_keyframes(), f32, Keyframe, opacity_alpha_ramp_expr_clamps_a_single_keyframe(), position_overlay_xy_expr_builds_a_t_based_ramp_for_an_animated_axis(), position_overlay_xy_expr_scales_a_single_keyframe_by_canvas_dimensions(), rotation_filter_angle_expr_converts_a_single_keyframe_to_radians() (+22 more)

### Community 1 - "Project"
Cohesion: 0.09
Nodes (39): bench_parse_loudnorm_stderr(), bench_project_ocproj_round_trip(), bench_timeline_duration(), large_project(), from_ocproj_bytes(), load_project_from_file(), PersistError, Display (+31 more)

### Community 2 - "app_test.rs"
Cohesion: 0.03
Nodes (107): add_asset_to_timeline_creates_a_track_and_appends_a_clip_when_none_exists(), add_asset_to_timeline_creates_an_audio_track_for_an_audio_asset(), add_asset_to_timeline_is_a_no_op_for_an_unknown_asset_id(), add_sequence_appends_a_named_tab_and_switches_to_it(), add_sequence_clears_a_stale_clip_selection(), cancel_export_job_flags_an_active_render_instead_of_removing_it(), cancel_export_job_removes_a_job_that_has_not_started_rendering(), copy_selected_clip_formatting_is_a_no_op_when_nothing_is_selected() (+99 more)

### Community 3 - "avbridge/src/lib.rs"
Cohesion: 0.06
Nodes (72): c_char, c_int, c_longlong, apply_text_overlays(), ClipSegment, encode_export(), encode_timeline_export(), encode_timeline_export_multi() (+64 more)

### Community 4 - "timeline_test.rs"
Cohesion: 0.07
Nodes (39): apply_formatting_overwrites_all_effect_fields(), clip(), formatting_roundtrip_preserves_all_fields(), has_vignette_is_true_above_zero(), is_color_filtered_is_true_for_any_filter_but_none(), is_cropped_is_true_when_any_crop_field_differs_from_the_full_frame(), is_masked_is_true_for_any_shape_but_none(), negative_gain_db_scales_linear_gain_below_unity() (+31 more)

### Community 5 - "preview_test.rs"
Cohesion: 0.08
Nodes (32): AppSink, BoolError, build_video_filter_bin(), Preview, PreviewError, Display, Error, Formatter (+24 more)

### Community 6 - "Locale"
Cohesion: 0.32
Nodes (11): Recency, Screen, job_detail_line(), job_status_label(), Locale, nav_label(), recency_label(), String (+3 more)

### Community 7 - "encode_write_packet"
Cohesion: 0.32
Nodes (14): AVStream, AVCodecContext, AVFormatContext, AVFrame, AVPacket, encode_write_packet(), filter_encode_write_frame(), filter_encode_write_video_frame() (+6 more)

### Community 8 - "App"
Cohesion: 0.05
Nodes (46): format_timecode(), LoudnessMetrics, MediaAsset, MediaKind, Option, PathBuf, String, Vec (+38 more)

### Community 9 - "Plano de Execução — oca (PacoPaçoca)"
Cohesion: 0.06
Nodes (28): Approach, Architecture, Commands, graphify, What this is, Approach, Architecture, Commands (+20 more)

### Community 10 - "track_with"
Cohesion: 0.06
Nodes (31): clip_at_finds_the_clip_covering_a_position(), clip_at_is_exclusive_of_a_clips_end(), clip_at_is_inclusive_of_a_clips_start(), clip_at_is_none_for_a_gap_between_clips(), clip_mut_finds_a_clip_by_id(), clip_mut_returns_none_for_an_unknown_id(), move_clip_is_a_no_op_for_a_negative_position(), move_clip_is_a_no_op_for_an_unknown_clip_id() (+23 more)

### Community 11 - "ensure_proxy"
Cohesion: 0.14
Nodes (19): cache_dir_for_project(), ensure_proxy(), is_up_to_date(), proxy_path_for(), a_proxy_newer_than_its_source_is_up_to_date(), a_proxy_older_than_its_source_is_not_up_to_date(), missing_proxy_is_not_up_to_date(), ProxyError (+11 more)

### Community 12 - "parse_loudnorm_stderr"
Cohesion: 0.14
Nodes (19): extract_first_json_object(), LoudnessError, LoudnormReport, measure_loudness(), parse_loudnorm_stderr(), Display, Error, Formatter (+11 more)

### Community 13 - "App"
Cohesion: 0.07
Nodes (10): App, next_clip_id(), resolve_or_create_track(), FnOnce, Option, App, Path, UnboundedSender (+2 more)

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

### Community 32 - "poll_for_descendants"
Cohesion: 0.26
Nodes (9): oca_window(), poll_for_descendants(), Repeatedly re-queries `window`'s accessibility tree for a descendant control…, Launches a fresh ui.exe, waits for its main window, yields it, then tears it…, _wait_for_window(), test_importing_a_file_adds_it_to_the_library_quickly(), test_navigating_to_each_screen_updates_the_breadcrumb(), fixture (+1 more)

### Community 34 - "generate_waveform"
Cohesion: 0.19
Nodes (14): generate_waveform(), Display, Error, Formatter, Path, Result, Vec, WaveformError (+6 more)

### Community 35 - "timeline_panel.rs"
Cohesion: 0.22
Nodes (17): color_filter_tint(), draw_filmstrip(), draw_frozen_poster(), draw_playhead(), draw_waveform(), App, Color32, HashMap (+9 more)

### Community 36 - "transcribe"
Cohesion: 0.09
Nodes (33): abort_trampoline(), collect_segments(), collect_words(), Arc, AtomicBool, c_void, Display, Error (+25 more)

### Community 37 - "render_timeline_export"
Cohesion: 0.11
Nodes (39): probe_media(), ProbedMedia, ProbeError, Display, Error, Formatter, From, Option (+31 more)

### Community 38 - "download_whisper_model"
Cohesion: 0.10
Nodes (19): download_whisper_model(), DownloadError, DownloadOutcome, AtomicBool, Display, Error, FnMut, Formatter (+11 more)

### Community 39 - "Security Fix: FFmpeg Download Verification"
Cohesion: 0.12
Nodes (16): 1. `.github/workflows/ci.yml`, 2. `.github/FFMPEG_UPDATE.md` (new file), After, Attack Surface Reduction, Before, Changes Made, Environment Variables (lines 9-16), Implementation Notes (+8 more)

### Community 40 - "test_project"
Cohesion: 0.15
Nodes (15): add_and_open_project_appends_and_opens_it(), add_asset_to_timeline_appends_after_whatever_is_already_on_the_matching_track(), ensure_preview_loaded_clears_state_once_the_playhead_moves_past_every_clip(), ensure_preview_loaded_resolves_the_clip_covering_the_playhead(), pump_import_queue_adds_the_asset_to_its_target_project_with_a_fresh_id(), pump_import_queue_applies_enrichment_to_the_asset_it_was_assigned(), pump_import_queue_targets_the_project_by_id_not_the_active_index(), Vec (+7 more)

### Community 41 - "avbridge_encode_timeline_export_multi"
Cohesion: 0.20
Nodes (15): AVFilterContext, AVFilterGraph, advance_overlay_decoder(), avbridge_encode_timeline_export_multi(), build_overlay_vfilter(), build_vfilter_descr(), AVCodecContext, AVFrame (+7 more)

### Community 42 - "ClipInstance"
Cohesion: 0.06
Nodes (14): ClipFormatting, ClipInstance, ColorFilter, MaskShape, Option, String, Vec, TextClip (+6 more)

### Community 43 - "ExportJob"
Cohesion: 0.07
Nodes (23): ExportJob, ExportJobStatus, String, Vec, test_job(), App, load_queue(), next_available_path() (+15 more)

### Community 44 - "property.rs"
Cohesion: 0.31
Nodes (7): property_block(), property_section(), property_toggle(), FnOnce, Ui, Ui, section_label()

### Community 45 - "prefs.rs"
Cohesion: 0.80
Nodes (4): App, Ui, shortcut_binding_editor(), show()

### Community 46 - "timeline_with"
Cohesion: 0.25
Nodes (8): move_clip_to_track_is_a_no_op_across_mismatched_kinds(), move_clip_to_track_is_a_no_op_for_a_negative_position(), move_clip_to_track_is_a_no_op_for_an_unknown_target_track(), move_clip_to_track_relocates_the_clip_to_a_same_kind_track(), Vec, timeline_clip_mut_finds_a_clip_across_tracks(), timeline_clip_mut_returns_none_for_an_unknown_id(), timeline_with()

### Community 47 - "theme.rs"
Cohesion: 0.16
Nodes (10): card_frame(), Frame, App, Ui, show(), App, Ui, show() (+2 more)

### Community 48 - "text_metrics_test.rs"
Cohesion: 0.18
Nodes (12): default_font(), Option, Vec, text_width_px_grows_with_more_characters(), text_width_px_scales_up_with_font_size(), word_x_offsets_px_is_empty_for_no_words(), word_x_offsets_px_is_strictly_increasing(), word_x_offsets_px_matches_word_count() (+4 more)

### Community 49 - "extract_thumbnail"
Cohesion: 0.20
Nodes (10): App, downscale_rgba(), extract_thumbnail(), import_one(), Context, Option, Path, PathBuf (+2 more)

### Community 50 - "tag.rs"
Cohesion: 0.57
Nodes (6): Color32, Ui, tag(), tag_accent(), tag_error(), tag_outline()

### Community 51 - "properties_panel.rs"
Cohesion: 0.30
Nodes (14): color_filter_label(), f32_keyframe_editor(), mask_shape_label(), position_keyframe_editor(), prop_row(), properties_panel(), App, Option (+6 more)

### Community 52 - "Update Process"
Cohesion: 0.13
Nodes (14): 1. Choose a Release, 2. Download Assets and Compute Checksums, 3. Update Workflow Environment Variables, 4. Update Cache Keys, 5. Test the Changes, Checksum Mismatch Error, Download Failures, FFmpeg Update Guide (+6 more)

### Community 53 - "queue.rs"
Cohesion: 0.43
Nodes (6): format_file_size(), open_containing_folder(), App, String, Ui, show()

### Community 54 - "test_canvas"
Cohesion: 0.50
Nodes (4): pending_export_conflict_overwrite_queues_with_the_original_path(), queue_export_appends_a_queued_job_with_the_next_id(), queue_export_starts_at_one_when_no_jobs_exist(), test_canvas()

### Community 55 - "nav_rail.rs"
Cohesion: 0.80
Nodes (5): prefs_button(), rail_button(), App, Ui, show()

### Community 56 - "render.rs"
Cohesion: 0.08
Nodes (62): Canvas, TextSegment, Sequence, apply_export_aspect_ratio(), apply_text_overlay_pass(), ExportAspectRatio, fps_to_rational(), render_export() (+54 more)

### Community 57 - "bridge_internal.h"
Cohesion: 0.32
Nodes (3): AVCodec, loudness_log_callback(), va_list

### Community 58 - "App"
Cohesion: 0.22
Nodes (4): App, frozen_playhead(), Context, Option

### Community 59 - "pcm_extract.c"
Cohesion: 0.29
Nodes (10): append_pcm_frame(), avbridge_extract_pcm_16k_mono(), AudioFilterChain, AVCodecContext, AVFrame, drain_pcm_frame(), init_pcm_filter_chain(), pcm_buffer_reserve() (+2 more)

### Community 60 - "editor/mod.rs"
Cohesion: 0.52
Nodes (11): media_library_panel(), preview_panel(), resizable_divider(), resizable_divider_horizontal(), App, Ui, save_active_project(), sequence_tab_bar() (+3 more)

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

### Community 66 - "home.rs"
Cohesion: 0.47
Nodes (5): open_in_finder(), App, PathBuf, Ui, show()

## Knowledge Gaps
- **129 isolated node(s):** `TrimEdge`, `Security Context`, `1. Choose a Release`, `Windows (PowerShell)`, `Linux (bash)` (+124 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `ClipInstance` connect `ClipInstance` to `keyframe_test.rs`, `Project`, `timeline_test.rs`, `render_timeline_export`, `preview_test.rs`, `test_project`, `App`, `track_with`, `App`, `render.rs`, `App`?**
  _High betweenness centrality (0.120) - this node is a cross-community bridge._
- **Why does `App` connect `App` to `Project`, `preview_test.rs`, `Locale`, `ClipInstance`, `ExportJob`, `render.rs`?**
  _High betweenness centrality (0.097) - this node is a cross-community bridge._
- **Why does `MediaAsset` connect `App` to `Project`, `timeline_panel.rs`, `render_timeline_export`, `test_project`, `render.rs`, `App`?**
  _High betweenness centrality (0.053) - this node is a cross-community bridge._
- **What connects `TrimEdge`, `Security Context`, `1. Choose a Release` to the rest of the system?**
  _129 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `keyframe_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.051560379918588875 - nodes in this community are weakly interconnected._
- **Should `Project` be split into smaller, more focused modules?**
  _Cohesion score 0.08673469387755102 - nodes in this community are weakly interconnected._
- **Should `app_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.03474903474903475 - nodes in this community are weakly interconnected._