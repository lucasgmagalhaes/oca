# Graph Report - oca  (2026-08-12)

## Corpus Check
- 77 files · ~81,269 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1003 nodes · 1988 edges · 57 communities (56 shown, 1 thin omitted)
- Extraction: 97% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 69 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `d71d0239`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- OcaApp
- Project
- app_test.rs
- avbridge/src/lib.rs
- timeline_test.rs
- Preview
- i18n.rs
- bridge.c
- MediaAsset
- Plano de Execução — oca (PacoPaçoca)
- track_with
- ensure_proxy
- parse_loudnorm_stderr
- timeline_export_test.rs
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
- core/src/lib.rs
- editor.rs
- app.rs
- probe_media
- .ui
- .active_project
- test_project
- ClipInstance
- i18n_test.rs
- property.rs
- App
- timeline_with
- theme.rs
- enum_combo
- tag.rs
- ExportJob
- render_test.rs
- queue.rs
- test_canvas
- nav_rail.rs
- render.rs

## God Nodes (most connected - your core abstractions)
1. `OcaApp` - 127 edges
2. `test_app()` - 119 edges
3. `clip()` - 50 edges
4. `ClipInstance` - 38 edges
5. `track_with()` - 33 edges
6. `Project` - 30 edges
7. `MediaAsset` - 24 edges
8. `render_timeline_export()` - 16 edges
9. `Track` - 16 edges
10. `Preview` - 15 edges

## Surprising Connections (you probably didn't know these)
- `rejects_an_empty_timeline()` --calls--> `encode_timeline_export()`  [INFERRED]
  crates/avbridge/tests/encode_test.rs → crates/avbridge/src/lib.rs
- `bench_parse_loudnorm_stderr()` --calls--> `parse_loudnorm_stderr()`  [INFERRED]
  crates/core/benches/parsing.rs → crates/core/src/loudness.rs
- `renders_and_normalizes_loudness_toward_target()` --calls--> `measure_loudness()`  [INFERRED]
  crates/core/tests/render_test.rs → crates/core/src/loudness.rs
- `errors_when_the_json_block_is_not_a_loudnorm_report()` --calls--> `parse_loudnorm_stderr()`  [INFERRED]
  crates/core/tests/loudness_test.rs → crates/core/src/loudness.rs
- `errors_when_there_is_no_json_block()` --calls--> `parse_loudnorm_stderr()`  [INFERRED]
  crates/core/tests/loudness_test.rs → crates/core/src/loudness.rs

## Import Cycles
- 2-file cycle: `crates/ui/src/app.rs -> crates/ui/src/i18n.rs -> crates/ui/src/app.rs`

## Communities (57 total, 1 thin omitted)

### Community 0 - "OcaApp"
Cohesion: 0.07
Nodes (13): Arc, OcaApp, AtomicBool, FnOnce, HashMap, TextureHandle, Ui, show() (+5 more)

### Community 1 - "Project"
Cohesion: 0.10
Nodes (33): bench_parse_loudnorm_stderr(), bench_project_json_round_trip(), bench_timeline_duration(), large_project(), from_json(), load_project_from_file(), PersistError, Display (+25 more)

### Community 2 - "app_test.rs"
Cohesion: 0.04
Nodes (103): add_asset_to_timeline_creates_a_track_and_appends_a_clip_when_none_exists(), add_asset_to_timeline_creates_an_audio_track_for_an_audio_asset(), add_asset_to_timeline_is_a_no_op_for_an_unknown_asset_id(), add_sequence_appends_a_named_tab_and_switches_to_it(), add_sequence_clears_a_stale_clip_selection(), cancel_export_job_flags_an_active_render_instead_of_removing_it(), cancel_export_job_removes_a_job_that_has_not_started_rendering(), copy_selected_clip_formatting_is_a_no_op_when_nothing_is_selected() (+95 more)

### Community 3 - "avbridge/src/lib.rs"
Cohesion: 0.07
Nodes (58): c_char, c_int, c_longlong, c_void, ClipSegment, encode_export(), encode_timeline_export(), EncodeError (+50 more)

### Community 4 - "timeline_test.rs"
Cohesion: 0.08
Nodes (34): apply_formatting_overwrites_all_effect_fields(), clip(), formatting_roundtrip_preserves_all_fields(), has_vignette_is_true_above_zero(), is_color_filtered_is_true_for_any_filter_but_none(), is_cropped_is_true_when_any_crop_field_differs_from_the_full_frame(), is_masked_is_true_for_any_shape_but_none(), negative_gain_db_scales_linear_gain_below_unity() (+26 more)

### Community 5 - "Preview"
Cohesion: 0.12
Nodes (18): AppSink, BoolError, build_video_filter_bin(), Preview, PreviewError, Display, Error, Formatter (+10 more)

### Community 6 - "i18n.rs"
Cohesion: 0.32
Nodes (11): Recency, Screen, job_detail_line(), job_status_label(), Locale, nav_label(), recency_label(), String (+3 more)

### Community 7 - "bridge.c"
Cohesion: 0.12
Nodes (41): AudioFilterChain, AVCodec, AVCodecContext, AVFormatContext, AVFrame, AVPacket, AVStream, accumulate_waveform_frame() (+33 more)

### Community 8 - "MediaAsset"
Cohesion: 0.21
Nodes (12): format_timecode(), MediaAsset, MediaKind, Option, PathBuf, String, Vec, ProbedMedia (+4 more)

### Community 9 - "Plano de Execução — oca (PacoPaçoca)"
Cohesion: 0.06
Nodes (28): Approach, Architecture, Commands, graphify, What this is, Approach, Architecture, Commands (+20 more)

### Community 10 - "track_with"
Cohesion: 0.07
Nodes (30): clip_at_finds_the_clip_covering_a_position(), clip_at_is_exclusive_of_a_clips_end(), clip_at_is_inclusive_of_a_clips_start(), clip_at_is_none_for_a_gap_between_clips(), clip_mut_finds_a_clip_by_id(), clip_mut_returns_none_for_an_unknown_id(), move_clip_is_a_no_op_for_a_negative_position(), move_clip_is_a_no_op_for_an_unknown_clip_id() (+22 more)

### Community 11 - "ensure_proxy"
Cohesion: 0.14
Nodes (19): cache_dir_for_project(), ensure_proxy(), is_up_to_date(), proxy_path_for(), a_proxy_newer_than_its_source_is_up_to_date(), a_proxy_older_than_its_source_is_not_up_to_date(), missing_proxy_is_not_up_to_date(), ProxyError (+11 more)

### Community 12 - "parse_loudnorm_stderr"
Cohesion: 0.14
Nodes (19): extract_first_json_object(), LoudnessError, LoudnormReport, measure_loudness(), parse_loudnorm_stderr(), Display, Error, Formatter (+11 more)

### Community 13 - "timeline_export_test.rs"
Cohesion: 0.33
Nodes (11): cancelling_mid_timeline_export_reports_cancelled(), clip(), fixture(), frozen_clip_still_exports_its_full_timeline_duration(), rejects_a_clip_with_a_missing_asset(), rejects_a_sequence_with_no_video_track(), renders_two_clips_with_different_effects_as_one_concatenated_export(), PathBuf (+3 more)

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

### Community 34 - "core/src/lib.rs"
Cohesion: 0.18
Nodes (14): generate_waveform(), Display, Error, Formatter, Path, Result, Vec, WaveformError (+6 more)

### Community 35 - "editor.rs"
Cohesion: 0.14
Nodes (33): ColorFilter, EditorTool, color_filter_label(), color_filter_tint(), draw_filmstrip(), draw_frozen_poster(), draw_playhead(), draw_waveform() (+25 more)

### Community 36 - "app.rs"
Cohesion: 0.19
Nodes (19): autosave_is_newer(), downscale_rgba(), extract_thumbnail(), import_one(), ImportEvent, load_prefs(), load_queue(), prefs_path() (+11 more)

### Community 37 - "probe_media"
Cohesion: 0.16
Nodes (16): probe_media(), ProbeError, Display, Error, Formatter, From, Path, Result (+8 more)

### Community 38 - ".ui"
Cohesion: 0.15
Nodes (5): Context, Frame, Self, Ui, CreationContext

### Community 39 - ".active_project"
Cohesion: 0.15
Nodes (5): TrackKind, next_clip_id(), resolve_or_create_track(), Option, ClipDrag

### Community 40 - "test_project"
Cohesion: 0.15
Nodes (15): add_and_open_project_appends_and_opens_it(), add_asset_to_timeline_appends_after_whatever_is_already_on_the_matching_track(), ensure_preview_loaded_clears_state_once_the_playhead_moves_past_every_clip(), ensure_preview_loaded_resolves_the_clip_covering_the_playhead(), pump_import_queue_adds_the_asset_to_its_target_project_with_a_fresh_id(), pump_import_queue_applies_enrichment_to_the_asset_it_was_assigned(), pump_import_queue_targets_the_project_by_id_not_the_active_index(), Vec (+7 more)

### Community 42 - "ClipInstance"
Cohesion: 0.06
Nodes (23): ClipFormatting, ClipInstance, MaskShape, Option, String, Vec, Timeline, Track (+15 more)

### Community 43 - "i18n_test.rs"
Cohesion: 0.20
Nodes (3): job(), job_detail_line_flips_the_negative_lufs_sign_for_display(), job_detail_line_shows_translated_error_prefix_on_failure()

### Community 44 - "property.rs"
Cohesion: 0.31
Nodes (7): property_block(), property_section(), property_toggle(), FnOnce, Ui, Ui, section_label()

### Community 45 - "App"
Cohesion: 0.29
Nodes (5): App, Ui, show(), Ui, show()

### Community 46 - "timeline_with"
Cohesion: 0.25
Nodes (8): move_clip_to_track_is_a_no_op_across_mismatched_kinds(), move_clip_to_track_is_a_no_op_for_a_negative_position(), move_clip_to_track_is_a_no_op_for_an_unknown_target_track(), move_clip_to_track_relocates_the_clip_to_a_same_kind_track(), Vec, timeline_clip_mut_finds_a_clip_across_tracks(), timeline_clip_mut_returns_none_for_an_unknown_id(), timeline_with()

### Community 47 - "theme.rs"
Cohesion: 0.20
Nodes (8): card_frame(), Frame, open_in_finder(), PathBuf, Ui, show(), apply(), Context

### Community 48 - "enum_combo"
Cohesion: 0.29
Nodes (5): enum_combo(), String, Ui, Fn, T

### Community 50 - "tag.rs"
Cohesion: 0.57
Nodes (6): Color32, Ui, tag(), tag_accent(), tag_error(), tag_outline()

### Community 51 - "ExportJob"
Cohesion: 0.53
Nodes (5): ExportJob, ExportJobStatus, String, Vec, test_job()

### Community 52 - "render_test.rs"
Cohesion: 0.53
Nodes (5): cancelling_mid_render_reports_cancelled(), fixture(), rejects_a_missing_source(), renders_and_normalizes_loudness_toward_target(), PathBuf

### Community 53 - "queue.rs"
Cohesion: 0.47
Nodes (5): format_file_size(), open_containing_folder(), String, Ui, show()

### Community 54 - "test_canvas"
Cohesion: 0.67
Nodes (3): queue_export_appends_a_queued_job_with_the_next_id(), queue_export_starts_at_one_when_no_jobs_exist(), test_canvas()

### Community 55 - "nav_rail.rs"
Cohesion: 0.80
Nodes (4): prefs_button(), rail_button(), Ui, show()

### Community 56 - "render.rs"
Cohesion: 0.21
Nodes (20): Canvas, Sequence, apply_export_aspect_ratio(), ExportAspectRatio, fps_to_rational(), render_export(), render_export_job(), render_timeline_export() (+12 more)

## Knowledge Gaps
- **107 isolated node(s):** `TrimEdge`, `What this is`, `Commands`, `Architecture`, `Approach` (+102 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `OcaApp` connect `OcaApp` to `Project`, `app_test.rs`, `editor.rs`, `app.rs`, `Preview`, `.ui`, `.active_project`, `i18n.rs`, `.active_project_mut`, `ClipInstance`, `App`, `theme.rs`, `ExportJob`, `queue.rs`, `nav_rail.rs`, `render.rs`?**
  _High betweenness centrality (0.242) - this node is a cross-community bridge._
- **Why does `test_app()` connect `app_test.rs` to `OcaApp`, `Project`, `test_project`, `ExportJob`, `test_canvas`?**
  _High betweenness centrality (0.103) - this node is a cross-community bridge._
- **Why does `Project` connect `Project` to `OcaApp`, `app_test.rs`, `i18n.rs`, `.active_project`, `MediaAsset`, `test_project`, `ClipInstance`, `ensure_proxy`, `.active_project_mut`, `.ui`, `render.rs`?**
  _High betweenness centrality (0.099) - this node is a cross-community bridge._
- **What connects `TrimEdge`, `What this is`, `Commands` to the rest of the system?**
  _107 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `OcaApp` be split into smaller, more focused modules?**
  _Cohesion score 0.07435897435897436 - nodes in this community are weakly interconnected._
- **Should `Project` be split into smaller, more focused modules?**
  _Cohesion score 0.10220673635307782 - nodes in this community are weakly interconnected._
- **Should `app_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.03827483196415235 - nodes in this community are weakly interconnected._