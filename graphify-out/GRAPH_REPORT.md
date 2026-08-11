# Graph Report - oca  (2026-08-11)

## Corpus Check
- 73 files · ~67,035 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 867 nodes · 1592 edges · 42 communities (40 shown, 2 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 58 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `36b58341`
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
- probe_media
- ensure_proxy
- parse_loudnorm_stderr
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
- main
- Commit Plan — Fase 1
- poll_for_descendants
- generate_waveform
- editor.rs
- app.rs
- .ui
- Option
- test_project
- test_asset
- ClipInstance

## God Nodes (most connected - your core abstractions)
1. `OcaApp` - 102 edges
2. `test_app()` - 101 edges
3. `clip()` - 32 edges
4. `Project` - 30 edges
5. `track_with()` - 26 edges
6. `ClipInstance` - 23 edges
7. `MediaAsset` - 21 edges
8. `Preview` - 15 edges
9. `Timeline` - 15 edges
10. `Task Breakdown — Fase 1 (Motor central)` - 15 edges

## Surprising Connections (you probably didn't know these)
- `every_timeline_clip_references_an_asset_in_the_same_project()` --calls--> `sample_projects()`  [INFERRED]
  crates/core/tests/sample_test.rs → crates/core/src/sample.rs
- `media_asset_ids_are_unique_within_each_project()` --calls--> `sample_projects()`  [INFERRED]
  crates/core/tests/sample_test.rs → crates/core/src/sample.rs
- `bench_parse_loudnorm_stderr()` --calls--> `parse_loudnorm_stderr()`  [INFERRED]
  crates/core/benches/parsing.rs → crates/core/src/loudness.rs
- `renders_and_normalizes_loudness_toward_target()` --calls--> `measure_loudness()`  [INFERRED]
  crates/core/tests/render_test.rs → crates/core/src/loudness.rs
- `errors_when_the_json_block_is_not_a_loudnorm_report()` --calls--> `parse_loudnorm_stderr()`  [INFERRED]
  crates/core/tests/loudness_test.rs → crates/core/src/loudness.rs

## Import Cycles
- 2-file cycle: `crates/ui/src/app.rs -> crates/ui/src/i18n.rs -> crates/ui/src/app.rs`

## Communities (42 total, 2 thin omitted)

### Community 0 - "OcaApp"
Cohesion: 0.09
Nodes (8): MaskShape, ClipFormatting, OcaApp, AtomicBool, HashMap, TextureHandle, Pos2, UnboundedReceiver

### Community 1 - "Project"
Cohesion: 0.10
Nodes (34): bench_parse_loudnorm_stderr(), bench_project_json_round_trip(), bench_timeline_duration(), large_project(), from_json(), load_project_from_file(), PersistError, Display (+26 more)

### Community 2 - "app_test.rs"
Cohesion: 0.04
Nodes (89): add_asset_to_timeline_creates_a_track_and_appends_a_clip_when_none_exists(), add_asset_to_timeline_creates_an_audio_track_for_an_audio_asset(), add_asset_to_timeline_is_a_no_op_for_an_unknown_asset_id(), add_sequence_appends_a_named_tab_and_switches_to_it(), add_sequence_clears_a_stale_clip_selection(), cancel_export_job_flags_an_active_render_instead_of_removing_it(), cancel_export_job_removes_a_job_that_has_not_started_rendering(), copy_selected_clip_formatting_is_a_no_op_when_nothing_is_selected() (+81 more)

### Community 3 - "avbridge/src/lib.rs"
Cohesion: 0.07
Nodes (50): c_char, c_int, c_longlong, c_void, encode_export(), EncodeError, EncodeOutcome, generate_proxy() (+42 more)

### Community 4 - "timeline_test.rs"
Cohesion: 0.07
Nodes (48): clip(), clip_mut_finds_a_clip_by_id(), clip_mut_returns_none_for_an_unknown_id(), has_vignette_is_true_above_zero(), is_color_filtered_is_true_for_any_filter_but_none(), is_cropped_is_true_when_any_crop_field_differs_from_the_full_frame(), is_masked_is_true_for_any_shape_but_none(), move_clip_is_a_no_op_for_a_negative_position() (+40 more)

### Community 5 - "Preview"
Cohesion: 0.09
Nodes (23): AppSink, BoolError, Preview, PreviewError, Display, Error, Formatter, Option (+15 more)

### Community 6 - "i18n.rs"
Cohesion: 0.08
Nodes (36): App, Screen, job_detail_line(), job_status_label(), Locale, nav_label(), recency_label(), String (+28 more)

### Community 7 - "bridge.c"
Cohesion: 0.11
Nodes (34): AudioFilterChain, AVCodec, AVCodecContext, AVFormatContext, AVFrame, AVPacket, AVStream, accumulate_waveform_frame() (+26 more)

### Community 8 - "MediaAsset"
Cohesion: 0.06
Nodes (25): ExportJob, ExportJobStatus, PathBuf, String, format_timecode(), MediaAsset, MediaKind, Option (+17 more)

### Community 9 - "Plano de Execução — oca (PacoPaçoca)"
Cohesion: 0.06
Nodes (28): Approach, Architecture, Commands, graphify, What this is, Approach, Architecture, Commands (+20 more)

### Community 10 - "probe_media"
Cohesion: 0.07
Nodes (37): probe_media(), ProbedMedia, ProbeError, Display, Error, Formatter, From, Option (+29 more)

### Community 11 - "ensure_proxy"
Cohesion: 0.14
Nodes (19): cache_dir_for_project(), ensure_proxy(), is_up_to_date(), proxy_path_for(), a_proxy_newer_than_its_source_is_up_to_date(), a_proxy_older_than_its_source_is_not_up_to_date(), missing_proxy_is_not_up_to_date(), ProxyError (+11 more)

### Community 12 - "parse_loudnorm_stderr"
Cohesion: 0.14
Nodes (19): extract_first_json_object(), LoudnessError, LoudnormReport, measure_loudness(), parse_loudnorm_stderr(), Display, Error, Formatter (+11 more)

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

### Community 32 - "poll_for_descendants"
Cohesion: 0.26
Nodes (9): oca_window(), poll_for_descendants(), Repeatedly re-queries `window`'s accessibility tree for a descendant control…, Launches a fresh ui.exe, waits for its main window, yields it, then tears it…, _wait_for_window(), test_importing_a_file_adds_it_to_the_library_quickly(), test_navigating_to_each_screen_updates_the_breadcrumb(), fixture (+1 more)

### Community 34 - "generate_waveform"
Cohesion: 0.20
Nodes (12): generate_waveform(), Display, Error, Formatter, Path, Result, Vec, WaveformError (+4 more)

### Community 35 - "editor.rs"
Cohesion: 0.14
Nodes (32): ColorFilter, ClipDrag, color_filter_label(), color_filter_tint(), draw_filmstrip(), draw_frozen_poster(), draw_playhead(), draw_waveform() (+24 more)

### Community 36 - "app.rs"
Cohesion: 0.19
Nodes (15): Arc, downscale_rgba(), EditorTool, extract_thumbnail(), import_one(), ImportEvent, PrefsState, RenderEvent (+7 more)

### Community 38 - ".ui"
Cohesion: 0.21
Nodes (5): Context, Frame, Self, Ui, CreationContext

### Community 39 - "Option"
Cohesion: 0.31
Nodes (4): TrackKind, next_clip_id(), resolve_or_create_track(), Option

### Community 40 - "test_project"
Cohesion: 0.25
Nodes (9): add_and_open_project_appends_and_opens_it(), add_asset_to_timeline_appends_after_whatever_is_already_on_the_matching_track(), Vec, select_timeline_clip_synchronizes_its_backing_asset(), test_project(), test_project_with_tracks(), test_track(), trim_clip_end_is_bounded_by_the_source_assets_own_duration() (+1 more)

### Community 41 - "test_asset"
Cohesion: 0.50
Nodes (4): pump_import_queue_adds_the_asset_to_its_target_project_with_a_fresh_id(), pump_import_queue_applies_enrichment_to_the_asset_it_was_assigned(), pump_import_queue_targets_the_project_by_id_not_the_active_index(), test_asset()

### Community 42 - "ClipInstance"
Cohesion: 0.08
Nodes (9): cuphead_timeline(), ClipInstance, Option, String, Vec, Timeline, Track, test_clip() (+1 more)

## Knowledge Gaps
- **107 isolated node(s):** `TrimEdge`, `What this is`, `Commands`, `Architecture`, `Approach` (+102 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **2 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `OcaApp` connect `OcaApp` to `Project`, `app_test.rs`, `editor.rs`, `app.rs`, `Preview`, `i18n.rs`, `Option`, `MediaAsset`, `.active_project`, `ClipInstance`, `.ui`?**
  _High betweenness centrality (0.224) - this node is a cross-community bridge._
- **Why does `RenderError` connect `probe_media` to `avbridge/src/lib.rs`?**
  _High betweenness centrality (0.108) - this node is a cross-community bridge._
- **Why does `EncodeError` connect `avbridge/src/lib.rs` to `probe_media`?**
  _High betweenness centrality (0.096) - this node is a cross-community bridge._
- **What connects `TrimEdge`, `What this is`, `Commands` to the rest of the system?**
  _107 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `OcaApp` be split into smaller, more focused modules?**
  _Cohesion score 0.08534850640113797 - nodes in this community are weakly interconnected._
- **Should `Project` be split into smaller, more focused modules?**
  _Cohesion score 0.09966777408637874 - nodes in this community are weakly interconnected._
- **Should `app_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.044194756554307116 - nodes in this community are weakly interconnected._