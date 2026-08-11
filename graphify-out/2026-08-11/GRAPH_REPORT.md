# Graph Report - oca  (2026-08-11)

## Corpus Check
- 71 files · ~58,052 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 758 nodes · 1365 edges · 37 communities (35 shown, 2 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 55 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `7331dea2`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- OcaApp
- Project
- app_test.rs
- avbridge/src/lib.rs
- timeline_test.rs
- Preview
- widgets.rs
- bridge.c
- i18n.rs
- Plano de Execução — oca (PacoPaçoca)
- MediaAsset
- ensure_proxy
- parse_loudnorm_stderr
- render_export
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
- core/src/lib.rs
- editor.rs
- ClipInstance

## God Nodes (most connected - your core abstractions)
1. `OcaApp` - 84 edges
2. `test_app()` - 66 edges
3. `Project` - 30 edges
4. `MediaAsset` - 20 edges
5. `Preview` - 15 edges
6. `Timeline` - 15 edges
7. `track_with()` - 15 edges
8. `Task Breakdown — Fase 1 (Motor central)` - 15 edges
9. `ClipInstance` - 14 edges
10. `Track` - 14 edges

## Surprising Connections (you probably didn't know these)
- `bench_parse_loudnorm_stderr()` --calls--> `parse_loudnorm_stderr()`  [INFERRED]
  crates/core/benches/parsing.rs → crates/core/src/loudness.rs
- `renders_and_normalizes_loudness_toward_target()` --calls--> `measure_loudness()`  [INFERRED]
  crates/core/tests/render_test.rs → crates/core/src/loudness.rs
- `errors_when_the_json_block_is_not_a_loudnorm_report()` --calls--> `parse_loudnorm_stderr()`  [INFERRED]
  crates/core/tests/loudness_test.rs → crates/core/src/loudness.rs
- `errors_when_there_is_no_json_block()` --calls--> `parse_loudnorm_stderr()`  [INFERRED]
  crates/core/tests/loudness_test.rs → crates/core/src/loudness.rs
- `extracts_the_measurement_fields_from_a_realistic_stderr_dump()` --calls--> `parse_loudnorm_stderr()`  [INFERRED]
  crates/core/tests/loudness_test.rs → crates/core/src/loudness.rs

## Import Cycles
- 2-file cycle: `crates/ui/src/app.rs -> crates/ui/src/i18n.rs -> crates/ui/src/app.rs`

## Communities (37 total, 2 thin omitted)

### Community 0 - "OcaApp"
Cohesion: 0.06
Nodes (29): Arc, downscale_rgba(), EditorTool, extract_thumbnail(), import_one(), ImportEvent, next_clip_id(), OcaApp (+21 more)

### Community 1 - "Project"
Cohesion: 0.08
Nodes (36): bench_parse_loudnorm_stderr(), bench_project_json_round_trip(), bench_timeline_duration(), large_project(), from_json(), load_project_from_file(), PersistError, Display (+28 more)

### Community 2 - "app_test.rs"
Cohesion: 0.06
Nodes (68): add_and_open_project_appends_and_opens_it(), add_asset_to_timeline_appends_after_whatever_is_already_on_the_matching_track(), add_asset_to_timeline_creates_a_track_and_appends_a_clip_when_none_exists(), add_asset_to_timeline_creates_an_audio_track_for_an_audio_asset(), add_asset_to_timeline_is_a_no_op_for_an_unknown_asset_id(), add_sequence_appends_a_named_tab_and_switches_to_it(), add_sequence_clears_a_stale_clip_selection(), cancel_export_job_flags_an_active_render_instead_of_removing_it() (+60 more)

### Community 3 - "avbridge/src/lib.rs"
Cohesion: 0.07
Nodes (50): c_char, c_int, c_longlong, c_void, encode_export(), EncodeError, EncodeOutcome, generate_proxy() (+42 more)

### Community 4 - "timeline_test.rs"
Cohesion: 0.11
Nodes (25): clip(), clip_mut_finds_a_clip_by_id(), clip_mut_returns_none_for_an_unknown_id(), move_clip_is_a_no_op_for_a_negative_position(), move_clip_is_a_no_op_for_an_unknown_clip_id(), move_clip_repositions_start_secs_and_leaves_the_source_range_untouched(), move_clip_to_track_is_a_no_op_across_mismatched_kinds(), move_clip_to_track_is_a_no_op_for_a_negative_position() (+17 more)

### Community 5 - "Preview"
Cohesion: 0.09
Nodes (23): AppSink, BoolError, Preview, PreviewError, Display, Error, Formatter, Option (+15 more)

### Community 6 - "widgets.rs"
Cohesion: 0.10
Nodes (23): App, Ui, show(), Ui, show(), Ui, show(), Ui (+15 more)

### Community 7 - "bridge.c"
Cohesion: 0.11
Nodes (34): AudioFilterChain, AVCodec, AVCodecContext, AVFormatContext, AVFrame, AVPacket, AVStream, accumulate_waveform_frame() (+26 more)

### Community 8 - "i18n.rs"
Cohesion: 0.10
Nodes (22): ExportJob, ExportJobStatus, PathBuf, String, Recency, test_job(), Screen, job() (+14 more)

### Community 9 - "Plano de Execução — oca (PacoPaçoca)"
Cohesion: 0.08
Nodes (22): Approach, Architecture, Commands, graphify, What this is, Decisões já tomadas, Fase 0 — Escopo do MVP, Fase 1 — Motor central (+14 more)

### Community 10 - "MediaAsset"
Cohesion: 0.07
Nodes (29): format_timecode(), MediaAsset, MediaKind, Option, PathBuf, String, Vec, probe_media() (+21 more)

### Community 11 - "ensure_proxy"
Cohesion: 0.14
Nodes (19): cache_dir_for_project(), ensure_proxy(), is_up_to_date(), proxy_path_for(), a_proxy_newer_than_its_source_is_up_to_date(), a_proxy_older_than_its_source_is_not_up_to_date(), missing_proxy_is_not_up_to_date(), ProxyError (+11 more)

### Community 12 - "parse_loudnorm_stderr"
Cohesion: 0.14
Nodes (19): extract_first_json_object(), LoudnessError, LoudnormReport, measure_loudness(), parse_loudnorm_stderr(), Display, Error, Formatter (+11 more)

### Community 13 - "render_export"
Cohesion: 0.15
Nodes (17): render_export(), RenderError, RenderOutcome, AtomicBool, Display, Error, Formatter, From (+9 more)

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
Cohesion: 0.29
Nodes (7): CHANGELOG.md — expected format, Constraints, Docs Specialist, Expected inputs, Output, Required workflow, Role

### Community 20 - "Git Agent"
Cohesion: 0.22
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

### Community 34 - "core/src/lib.rs"
Cohesion: 0.18
Nodes (12): generate_waveform(), Display, Error, Formatter, Path, Result, Vec, WaveformError (+4 more)

### Community 35 - "editor.rs"
Cohesion: 0.19
Nodes (23): draw_filmstrip(), draw_playhead(), draw_waveform(), media_library_panel(), preview_panel(), prop_row(), properties_panel(), resizable_divider() (+15 more)

### Community 36 - "ClipInstance"
Cohesion: 0.12
Nodes (14): ClipInstance, Option, String, Vec, Track, TrackKind, new_sequence_appends_and_switches_to_it(), new_sequence_ids_keep_increasing_after_multiple_calls() (+6 more)

## Knowledge Gaps
- **101 isolated node(s):** `TrimEdge`, `What this is`, `Commands`, `Architecture`, `Approach` (+96 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **2 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `OcaApp` connect `OcaApp` to `Project`, `app_test.rs`, `editor.rs`, `ClipInstance`, `Preview`, `widgets.rs`, `i18n.rs`?**
  _High betweenness centrality (0.206) - this node is a cross-community bridge._
- **Why does `RenderError` connect `render_export` to `avbridge/src/lib.rs`?**
  _High betweenness centrality (0.118) - this node is a cross-community bridge._
- **Why does `EncodeError` connect `avbridge/src/lib.rs` to `render_export`?**
  _High betweenness centrality (0.105) - this node is a cross-community bridge._
- **What connects `TrimEdge`, `What this is`, `Commands` to the rest of the system?**
  _101 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `OcaApp` be split into smaller, more focused modules?**
  _Cohesion score 0.05985915492957746 - nodes in this community are weakly interconnected._
- **Should `Project` be split into smaller, more focused modules?**
  _Cohesion score 0.08081632653061224 - nodes in this community are weakly interconnected._
- **Should `app_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.06010230179028133 - nodes in this community are weakly interconnected._