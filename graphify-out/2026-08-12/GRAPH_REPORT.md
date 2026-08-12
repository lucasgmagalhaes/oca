# Graph Report - oca  (2026-08-12)

## Corpus Check
- 144 files · ~88,581 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1475 nodes · 2532 edges · 128 communities (76 shown, 52 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 57 edges (avg confidence: 0.76)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `571a6267`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- OcaApp
- persistence_test.rs
- app_test.rs
- avbridge/src/lib.rs
- timeline_test.rs
- Preview
- i18n.rs
- bridge.c
- cn
- Plano de Execução — oca (PacoPaçoca)
- probe_media
- ensure_proxy
- parse_loudnorm_stderr
- types.ts
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
- devDependencies
- sidebar.tsx
- routeTree.gen.ts
- compilerOptions
- utils.ts
- pagination.tsx
- components.json
- command.tsx
- menubar.tsx
- form.tsx
- Project
- carousel.tsx
- dependencies
- render_export
- chart.tsx
- MediaAsset
- timeline.rs
- ProbeError
- RenderError
- table.tsx
- breadcrumb.tsx
- navigation-menu.tsx
- select.tsx
- parsing.rs
- project_test.rs
- ProbedMedia
- alert.tsx
- input-otp.tsx
- CapCut Canvas
- sonner.tsx
- Routes
- clsx
- cmdk
- date-fns
- embla-carousel-react
- @hookform/resolvers
- lucide-react
- @radix-ui/react-accordion
- @radix-ui/react-alert-dialog
- @radix-ui/react-aspect-ratio
- @radix-ui/react-avatar
- @radix-ui/react-checkbox
- @radix-ui/react-collapsible
- @radix-ui/react-dialog
- @radix-ui/react-dropdown-menu
- @radix-ui/react-hover-card
- @radix-ui/react-label
- @radix-ui/react-menubar
- @radix-ui/react-navigation-menu
- @radix-ui/react-popover
- @radix-ui/react-progress
- @radix-ui/react-radio-group
- @radix-ui/react-scroll-area
- @radix-ui/react-select
- @radix-ui/react-separator
- @radix-ui/react-slider
- @radix-ui/react-slot
- @radix-ui/react-switch
- @radix-ui/react-tabs
- @radix-ui/react-toggle
- @radix-ui/react-toggle-group
- @radix-ui/react-tooltip
- react
- react-day-picker
- react-dom
- react-hook-form
- react-resizable-panels
- sonner
- @supabase/supabase-js
- tailwind-merge
- tailwindcss
- @tailwindcss/vite
- @tanstack/react-query
- @tanstack/react-router
- @tanstack/router-plugin
- tw-animate-css
- vaul
- vite-tsconfig-paths
- zod

## God Nodes (most connected - your core abstractions)
1. `cn()` - 220 edges
2. `test_app()` - 115 edges
3. `OcaApp` - 108 edges
4. `clip()` - 38 edges
5. `Project` - 30 edges
6. `track_with()` - 29 edges
7. `ClipInstance` - 27 edges
8. `compilerOptions` - 22 edges
9. `MediaAsset` - 19 edges
10. `Preview` - 15 edges

## Surprising Connections (you probably didn't know these)
- `bench_parse_loudnorm_stderr()` --calls--> `parse_loudnorm_stderr()`  [INFERRED]
  crates/core/benches/parsing.rs → crates/core/src/loudness.rs
- `bench_project_json_round_trip()` --calls--> `from_json()`  [INFERRED]
  crates/core/benches/parsing.rs → crates/core/src/persistence.rs
- `bench_project_json_round_trip()` --calls--> `to_json()`  [INFERRED]
  crates/core/benches/parsing.rs → crates/core/src/persistence.rs
- `renders_and_normalizes_loudness_toward_target()` --calls--> `measure_loudness()`  [INFERRED]
  crates/core/tests/render_test.rs → crates/core/src/loudness.rs
- `errors_when_the_json_block_is_not_a_loudnorm_report()` --calls--> `parse_loudnorm_stderr()`  [INFERRED]
  crates/core/tests/loudness_test.rs → crates/core/src/loudness.rs

## Import Cycles
- 2-file cycle: `crates/ui/src/app.rs -> crates/ui/src/i18n.rs -> crates/ui/src/app.rs`

## Communities (128 total, 52 thin omitted)

### Community 0 - "OcaApp"
Cohesion: 0.08
Nodes (7): OcaApp, AtomicBool, HashMap, TextureHandle, HashSet, Pos2, UnboundedReceiver

### Community 1 - "persistence_test.rs"
Cohesion: 0.18
Nodes (19): from_json(), load_project_from_file(), PersistError, Display, Error, Formatter, Path, Result (+11 more)

### Community 2 - "app_test.rs"
Cohesion: 0.04
Nodes (103): add_asset_to_timeline_creates_a_track_and_appends_a_clip_when_none_exists(), add_asset_to_timeline_creates_an_audio_track_for_an_audio_asset(), add_asset_to_timeline_is_a_no_op_for_an_unknown_asset_id(), add_sequence_appends_a_named_tab_and_switches_to_it(), add_sequence_clears_a_stale_clip_selection(), cancel_export_job_flags_an_active_render_instead_of_removing_it(), cancel_export_job_removes_a_job_that_has_not_started_rendering(), copy_selected_clip_formatting_is_a_no_op_when_nothing_is_selected() (+95 more)

### Community 3 - "avbridge/src/lib.rs"
Cohesion: 0.07
Nodes (50): c_char, c_int, c_longlong, c_void, encode_export(), EncodeError, EncodeOutcome, generate_proxy() (+42 more)

### Community 4 - "timeline_test.rs"
Cohesion: 0.06
Nodes (54): clip(), clip_mut_finds_a_clip_by_id(), clip_mut_returns_none_for_an_unknown_id(), has_vignette_is_true_above_zero(), is_color_filtered_is_true_for_any_filter_but_none(), is_cropped_is_true_when_any_crop_field_differs_from_the_full_frame(), is_masked_is_true_for_any_shape_but_none(), move_clip_is_a_no_op_for_a_negative_position() (+46 more)

### Community 5 - "Preview"
Cohesion: 0.09
Nodes (23): AppSink, BoolError, Preview, PreviewError, Display, Error, Formatter, Option (+15 more)

### Community 6 - "i18n.rs"
Cohesion: 0.05
Nodes (44): App, ExportJob, ExportJobStatus, PathBuf, String, test_job(), Screen, job() (+36 more)

### Community 7 - "bridge.c"
Cohesion: 0.11
Nodes (34): AudioFilterChain, AVCodec, AVCodecContext, AVFormatContext, AVFrame, AVPacket, AVStream, accumulate_waveform_frame() (+26 more)

### Community 8 - "cn"
Cohesion: 0.06
Nodes (46): AccordionContent, AccordionItem, AccordionTrigger, Avatar, AvatarFallback, AvatarImage, Card, CardContent (+38 more)

### Community 9 - "Plano de Execução — oca (PacoPaçoca)"
Cohesion: 0.06
Nodes (28): Approach, Architecture, Commands, graphify, What this is, Approach, Architecture, Commands (+20 more)

### Community 10 - "probe_media"
Cohesion: 0.33
Nodes (9): probe_media(), Path, converts_container_bitrate_from_bps_to_mbps(), falls_back_to_the_audio_stream_when_there_is_no_video(), fixture(), into_media_asset_carries_the_probed_fields_through(), parses_frame_rate(), probes_a_video_stream_as_the_primary_track() (+1 more)

### Community 11 - "ensure_proxy"
Cohesion: 0.14
Nodes (19): cache_dir_for_project(), ensure_proxy(), is_up_to_date(), proxy_path_for(), a_proxy_newer_than_its_source_is_up_to_date(), a_proxy_older_than_its_source_is_not_up_to_date(), missing_proxy_is_not_up_to_date(), ProxyError (+11 more)

### Community 12 - "parse_loudnorm_stderr"
Cohesion: 0.14
Nodes (19): extract_first_json_object(), LoudnessError, LoudnormReport, measure_loudness(), parse_loudnorm_stderr(), Display, Error, Formatter (+11 more)

### Community 13 - "types.ts"
Cohesion: 0.06
Nodes (38): lovable, lovableAuth, SignInOptions, attachSupabaseAuth, createSupabaseFetch(), isNewSupabaseApiKey(), requireSupabaseAuth, createSupabaseClient() (+30 more)

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
Nodes (33): ColorFilter, ClipDrag, color_filter_label(), color_filter_tint(), draw_filmstrip(), draw_frozen_poster(), draw_playhead(), draw_waveform() (+25 more)

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
Cohesion: 0.33
Nodes (6): add_and_open_project_appends_and_opens_it(), add_asset_to_timeline_appends_after_whatever_is_already_on_the_matching_track(), select_timeline_clip_synchronizes_its_backing_asset(), test_project(), trim_clip_end_is_bounded_by_the_source_assets_own_duration(), trim_clip_end_moves_the_right_edge_and_keeps_the_start_fixed()

### Community 41 - "test_asset"
Cohesion: 0.50
Nodes (4): pump_import_queue_adds_the_asset_to_its_target_project_with_a_fresh_id(), pump_import_queue_applies_enrichment_to_the_asset_it_was_assigned(), pump_import_queue_targets_the_project_by_id_not_the_active_index(), test_asset()

### Community 42 - "ClipInstance"
Cohesion: 0.10
Nodes (9): ClipInstance, Option, String, Track, Vec, test_clip(), test_composite_clip(), test_project_with_tracks() (+1 more)

### Community 43 - "devDependencies"
Cohesion: 0.04
Nodes (46): eslint, eslint-config-prettier, @eslint/js, eslint-plugin-prettier, eslint-plugin-react-hooks, eslint-plugin-react-refresh, globals, @lovable.dev/vite-tanstack-config (+38 more)

### Community 44 - "sidebar.tsx"
Cohesion: 0.06
Nodes (40): Input, Separator, SheetContent, SheetContentProps, SheetDescription, SheetFooter(), SheetHeader(), SheetOverlay (+32 more)

### Community 45 - "routeTree.gen.ts"
Cohesion: 0.06
Nodes (31): LovableErrorOptions, LovableEvents, reportLovableError(), Window, getRouter(), audioTrack, Clip, clipStyles (+23 more)

### Community 46 - "compilerOptions"
Cohesion: 0.06
Nodes (31): compilerOptions, allowImportingTsExtensions, exactOptionalPropertyTypes, jsx, lib, module, moduleResolution, noEmit (+23 more)

### Community 47 - "utils.ts"
Cohesion: 0.09
Nodes (15): Badge(), BadgeProps, badgeVariants, Checkbox, HoverCardContent, PopoverContent, Progress, Slider (+7 more)

### Community 48 - "pagination.tsx"
Cohesion: 0.12
Nodes (21): AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter(), AlertDialogHeader(), AlertDialogOverlay, AlertDialogTitle (+13 more)

### Community 49 - "components.json"
Cohesion: 0.11
Nodes (18): aliases, components, hooks, lib, ui, utils, iconLibrary, registries (+10 more)

### Community 50 - "command.tsx"
Cohesion: 0.12
Nodes (14): Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList, CommandSeparator, CommandShortcut() (+6 more)

### Community 51 - "menubar.tsx"
Cohesion: 0.12
Nodes (11): Menubar, MenubarCheckboxItem, MenubarContent, MenubarItem, MenubarLabel, MenubarRadioItem, MenubarSeparator, MenubarShortcut() (+3 more)

### Community 52 - "form.tsx"
Cohesion: 0.19
Nodes (12): FormControl, FormDescription, FormFieldContext, FormFieldContextValue, FormItem, FormItemContext, FormItemContextValue, FormLabel (+4 more)

### Community 53 - "Project"
Cohesion: 0.23
Nodes (9): Project, Recency, Option, PathBuf, String, Vec, Sequence, Vec (+1 more)

### Community 54 - "carousel.tsx"
Cohesion: 0.19
Nodes (13): Carousel, CarouselApi, CarouselContent, CarouselContext, CarouselContextProps, CarouselItem, CarouselNext, CarouselOptions (+5 more)

### Community 55 - "dependencies"
Cohesion: 0.15
Nodes (13): class-variance-authority, input-otp, @lovable.dev/cloud-auth-js, dependencies, class-variance-authority, input-otp, @lovable.dev/cloud-auth-js, @radix-ui/react-context-menu (+5 more)

### Community 56 - "render_export"
Cohesion: 0.30
Nodes (10): render_export(), RenderOutcome, AtomicBool, Path, cancelling_mid_render_reports_cancelled(), fixture(), rejects_a_missing_source(), renders_and_normalizes_loudness_toward_target() (+2 more)

### Community 57 - "chart.tsx"
Cohesion: 0.25
Nodes (9): ChartConfig, ChartContainer, ChartContext, ChartContextProps, ChartLegendContent, ChartTooltipContent, getPayloadConfigFromPayload(), THEMES (+1 more)

### Community 58 - "MediaAsset"
Cohesion: 0.33
Nodes (8): format_timecode(), MediaAsset, MediaKind, Option, PathBuf, String, Vec, test_asset_with_kind()

### Community 59 - "timeline.rs"
Cohesion: 0.22
Nodes (3): MaskShape, TransitionType, ClipFormatting

### Community 60 - "ProbeError"
Cohesion: 0.22
Nodes (7): ProbeError, Display, Error, Formatter, From, Result, Self

### Community 61 - "RenderError"
Cohesion: 0.22
Nodes (7): RenderError, Display, Error, Formatter, From, Result, Self

### Community 62 - "table.tsx"
Cohesion: 0.22
Nodes (8): Table, TableBody, TableCaption, TableCell, TableFooter, TableHead, TableHeader, TableRow

### Community 63 - "breadcrumb.tsx"
Cohesion: 0.25
Nodes (7): Breadcrumb, BreadcrumbEllipsis(), BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbPage, BreadcrumbSeparator()

### Community 64 - "navigation-menu.tsx"
Cohesion: 0.29
Nodes (7): NavigationMenu, NavigationMenuContent, NavigationMenuIndicator, NavigationMenuList, NavigationMenuTrigger, navigationMenuTriggerStyle, NavigationMenuViewport

### Community 65 - "select.tsx"
Cohesion: 0.25
Nodes (7): SelectContent, SelectItem, SelectLabel, SelectScrollDownButton, SelectScrollUpButton, SelectSeparator, SelectTrigger

### Community 67 - "parsing.rs"
Cohesion: 0.67
Nodes (5): bench_parse_loudnorm_stderr(), bench_project_json_round_trip(), bench_timeline_duration(), large_project(), Criterion

### Community 68 - "project_test.rs"
Cohesion: 0.60
Nodes (5): new_sequence_appends_and_switches_to_it(), new_sequence_ids_keep_increasing_after_multiple_calls(), test_project(), timeline_mut_writes_the_active_sequence(), timeline_reads_the_active_sequence()

### Community 69 - "ProbedMedia"
Cohesion: 0.50
Nodes (4): ProbedMedia, Option, PathBuf, String

### Community 70 - "alert.tsx"
Cohesion: 0.50
Nodes (4): Alert, AlertDescription, AlertTitle, alertVariants

### Community 71 - "input-otp.tsx"
Cohesion: 0.40
Nodes (4): InputOTP, InputOTPGroup, InputOTPSeparator, InputOTPSlot

### Community 72 - "CapCut Canvas"
Cohesion: 0.50
Nodes (3): Build with Lovable, CapCut Canvas, Development

## Knowledge Gaps
- **296 isolated node(s):** `TrimEdge`, `$schema`, `style`, `rsc`, `tsx` (+291 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **52 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `OcaApp` connect `OcaApp` to `app_test.rs`, `editor.rs`, `app.rs`, `Preview`, `i18n.rs`, `Option`, `.active_project`, `.ui`, `ClipInstance`, `Project`, `timeline.rs`?**
  _High betweenness centrality (0.099) - this node is a cross-community bridge._
- **Why does `Preview` connect `Preview` to `OcaApp`, `i18n.rs`?**
  _High betweenness centrality (0.047) - this node is a cross-community bridge._
- **Why does `RenderError` connect `RenderError` to `render_export`, `avbridge/src/lib.rs`?**
  _High betweenness centrality (0.046) - this node is a cross-community bridge._
- **What connects `TrimEdge`, `$schema`, `style` to the rest of the system?**
  _296 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `OcaApp` be split into smaller, more focused modules?**
  _Cohesion score 0.07751937984496124 - nodes in this community are weakly interconnected._
- **Should `app_test.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.03827483196415235 - nodes in this community are weakly interconnected._
- **Should `avbridge/src/lib.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.07377049180327869 - nodes in this community are weakly interconnected._