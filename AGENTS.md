# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## What this is

oca — a native Rust video editor (`egui`/`eframe`) for cutting gameplay
footage for the PacoPaçoca YouTube channel. Its two headline features are automatic loudness
normalization and export-that-matches-the-source-bitrate. Full phased execution plan:
[`features/request.md`](features/request.md) (a near-duplicate lives at `docs/plano.md` —
not yet reconciled, treat `features/request.md` as canonical).

Current status: the GUI shell (all five screens, navigable) and JSON project save/load are
wired end-to-end from the UI (`home.rs` open dialog, `editor.rs` save). Probing
(`avcore::probe`), export rendering (`avcore::render`), loudness measurement
(`avcore::loudness`), and proxy generation (`avcore::proxy`) all go through
`avbridge`, a native FFI bridge over libavformat/libavcodec/libavfilter/libswscale — no
subprocess, no ffprobe/ffmpeg on PATH required for any of them (proxy uses `libopenh264` — BSD
— since this LGPL FFmpeg build has no `libx264`/GPL). The GStreamer preview pipeline
(`avcore::preview::Preview`) is wired into the Editor screen's preview panel (`ui/src/
app.rs::OcaApp::{select_asset,ensure_preview_loaded,pump_preview_frame}` +
`ui/src/screens/editor.rs::preview_panel`): clicking an asset in the media library selects it,
but the pipeline itself opens lazily — `ensure_preview_loaded` runs once per frame from the top
of `preview_panel` and is a no-op once a pipeline is open or has already been tried for the
current selection, so switching projects or launching the app doesn't pay GStreamer's open cost
until the Editor screen's preview panel is actually painted. Decoded frames are uploaded to an
egui texture every frame, and play/pause/seek (including a click-to-seek position slider) drive
the pipeline — playback of the selected clip only, not
yet full multi-clip timeline playback. Double-clicking an asset in the Editor's media library
panel adds it to the timeline (`OcaApp::add_asset_to_timeline`) — appended, untrimmed, onto
the first track of matching kind (auto-creating `"V1"`/`"A1"` if none exists yet). Dragging an
asset out of the library and dropping it on the timeline strip does the same insert but at the
drop position, on whichever track row the pointer landed on
(`OcaApp::add_asset_to_timeline_at`, `editor.rs::media_library_panel`/`timeline_panel` relaying
the drop through `OcaApp::pending_asset_drop`) — both entry points share track
resolution/creation via `resolve_or_create_track`. The timeline
(`editor.rs::timeline_panel`) draws clips at their real `start_secs` position, video clips draw
a filmstrip of distinct per-position poster frames (`editor.rs::draw_filmstrip`, one tile per
on-screen column; each tile's frame is extracted lazily on a background thread once its
source-time bucket — fixed-width, quantized by `OcaApp::THUMBNAIL_BUCKET_SECS`, shared across
every clip on that asset rather than per-clip — scrolls into view, so it gets visibly denser as
the timeline zooms in, per `request.md`'s Fase 3 spec; a known simplification, not zoom-adaptive,
so a filmstrip zoomed in past roughly one tile per bucket repeats a tile a few times in a row),
audio clips draw a min/max peak waveform (`avcore::waveform::generate_waveform`, a fixed
`WAVEFORM_BUCKET_COUNT`-bucket table computed once per asset during import enrichment and
resampled per pixel column at draw time — see `editor.rs::draw_waveform`), has a click/drag
ruler that moves the playhead, and `Ctrl` + scroll zooms it
(`OcaApp::timeline_px_per_sec`). Real editing, with no ripple (a cut/delete/move just leaves
or closes a gap at the point of the edit, nothing downstream shifts) and no overlap checking
(`avcore::timeline::Track`'s long-standing documented policy): clip select
(`OcaApp::selected_clip_id`, separate from `selected_asset_id` which drives the preview
panel), `Ctrl+B`/toolbar split-at-playhead across every track (`Track::split_clip_at` +
`OcaApp::split_at_playhead`), `Delete` (`OcaApp::delete_selected_clip`), drag-trim either edge
bounded by a minimum duration and (right edge) the source asset's own length
(`ClipInstance::trim_start`/`trim_end`), and drag-move a clip's body — same-track reposition
or onto a different same-`TrackKind` track, resolved by which row's Y-range the drag lands on
(`Timeline::move_clip_to_track`/`Track::move_clip`), and `Ctrl+C`/`Ctrl+X`/`Ctrl+V`
(`OcaApp::copy_selected_clip`/`cut_selected_clip`/`paste_clip_at_playhead`) copy, cut and paste
a clip via a one-slot `OcaApp::clipboard_clip` — paste always lands at the playhead on a
matching-kind track (auto-created if none exists) rather than wherever the clip was cut from, a
known simplification. Not scoped to a project or sequence, so pasting into a different tab (or
even a different project) "just works" — request.md's Fase 3 "copiar e colar entre abas" for
free. Right-click a clip for a context menu with the same actions (per `request.md`'s Fase 3
spec) plus delete; apply-effect is a follow-up once effects themselves exist. `Ctrl`+click 2+
clips (`OcaApp::toggle_multi_select`) then the toolbar's "Mesclar em bloco" button
(`OcaApp::merge_into_composite`) groups them into a composite block — `ClipInstance::composite_id`,
shared by every member, not a distinct clip type — per `request.md`'s Fase 3 "blocos compostos"
spec. A composite block then behaves as one clip for the operations that matter most: dragging
any member moves the whole group by the same delta (`OcaApp::move_clip_with_group`), splitting
one at the playhead keeps both halves in the group (`Track::split_clip_at`), and deleting one
deletes all of them (`OcaApp::delete_selected_clip`). Two scope limits, both enforced rather
than silently broken: a group can't span tracks (merging across tracks, or dragging a member
onto a different track, is a no-op/falls back to a same-track move), and copy/paste doesn't
replicate group membership yet (a pasted clip is always standalone). The Editor screen's three
columns (media
library / preview / properties) and the timeline strip are all resizable by dragging the
divider between them (`editor.rs::resizable_divider`/`resizable_divider_horizontal`,
`OcaApp::lib_panel_width`/`props_panel_width`/`timeline_height`) — sizes clamp to the window's
current size every frame but aren't persisted across restarts yet, short of `request.md`'s
"layout salvo por projeto ou por usuário". A project can hold multiple sequences (tabs) —
`avcore::project::Sequence`, each with its own `Timeline` — shown as a tab bar above the
three-column body (`editor.rs::sequence_tab_bar`); every project always has at least one, and
every clip-editing `OcaApp` method reads/writes through `Project::timeline`/`timeline_mut`
(the active tab), never a `timeline` field directly. Export settings staying per-job rather
than gaining a per-sequence default, and no rename/delete/reorder for tabs yet, are the two
gaps short of `request.md`'s full "abas de projeto" spec. Importing files
(`library.rs`/`OcaApp::spawn_import`)
runs each file on its own background thread instead of blocking the UI — large source files
used to freeze the app. Each file becomes usable in the media library as soon as its (cheap,
metadata-only) probe returns; loudness measurement, proxy generation, and waveform computation,
all full decode passes that can take minutes, keep running afterward and patch the
already-visible asset in place once done (`ImportEvent::AssetReady` then
`ImportEvent::Enriched`, correlated by an `import_token` in `OcaApp::pending_enrichment`) —
matching how other NLEs show an import instantly and refine it in the background, rather than
blocking "imported" on every decode pass finishing first. A background export queue worker
already runs (`OcaApp::pump_export_queue`
dispatches `avcore::render_export` on a spawned thread, progress/done/failed/cancelled
reported back over `tokio::mpsc`) — the queue panel doesn't yet support reordering/pausing
jobs or persisting the queue across sessions. Check the plan doc for which phase a task
belongs to before assuming a feature is live.

## Commands

Requires Rust (stable) via rustup. Both the MSVC target (`stable-x86_64-pc-windows-msvc`, needs
Visual Studio Build Tools with the "Desktop development with C++" workload) and the GNU target
(`x86_64-pc-windows-gnu` + a MinGW-w64 toolchain e.g. WinLibs, `bin` dir on `PATH`) work — this
workspace currently defaults to MSVC, but nothing in it requires MSVC specifically.

`avbridge` links against libavformat/libavcodec/libavfilter/libswscale/libavutil and needs
`FFMPEG_DIR` set to an FFmpeg dev build with `include/` and `lib/` subdirectories (e.g. a BtbN
shared build, with `avfilter`/`swscale`/`libopenh264` among the linked libs/enabled encoders)
to compile at all — the workspace won't build without it. At runtime the FFmpeg DLLs
(`FFMPEG_DIR/bin`) need to be next to the built binary or on `PATH`.

`core` also depends on the `gstreamer` crate (`preview` module, see Architecture below),
needing `pkg-config` plus GStreamer's dev build (MSVC or mingw,
matching whichever Rust target you're using) discoverable via
`PKG_CONFIG_PATH=<gstreamer_root>/lib/pkgconfig`, with `<gstreamer_root>/bin` on `PATH` too
(build-time lookup and runtime DLL resolution both need it).

**Do not remove `avbridge/build.rs`'s import-lib-renaming step.** GStreamer's SDK bundles
its own FFmpeg build (`gst-libav`) whose import libs share filenames with (but are a different,
incompatible version from) the ones under `FFMPEG_DIR` — confirmed on both MSVC and GNU/mingw,
and confirmed to be a filename collision, not a toolchain-ABI issue (ruled out by testing both).
Left as bare `-l<name>` + `-L<path>`, a bare-name search can silently resolve to GStreamer's
bundled copy instead, compiling fine but corrupting `avbridge`'s FFmpeg calls at runtime
(`avformat_open_input` starts failing on every call, no error at build time). `build.rs` copies
its FFmpeg import libs into its own `OUT_DIR` under unique names before linking specifically to
make that impossible — full diagnosis in `features/fase1/commit_plan.md` (chore-002).

```bash
make build     # debug build, whole workspace
make run       # run the GUI app (debug)
make test      # run every crate's test suite
make test-core # cargo test -p core only
make test-e2e  # pytest + pywinauto against a built target/debug/ui.exe (see below)
make bench     # criterion benchmarks (core parsing/serialization) -> target/criterion/report/index.html
make fmt       # cargo fmt --all
make lint      # cargo clippy --workspace --all-targets
make check     # cargo check --workspace --all-targets (fast compile-only loop)
```

No `make` on PATH → run the underlying `cargo`/`rustup` command directly; every Makefile target
is a one-liner (see [`Makefile`](Makefile)).

Single test: `cargo test -p core --test probe_test measure_loudness` (integration tests, one
`<module>_test.rs` file per module under `crates/core/tests/`) or `cargo test -p ui i18n::tests`
for the `src/<module>/<module>_test.rs` unit tests in `ui`.

**End-to-end tests** (`e2e/`) drive the real built `ui.exe` through Windows UI Automation via
`pytest` + `pywinauto` (`backend="uia"`) — a different layer from the Rust unit/integration
tests above: those exercise `OcaApp`'s methods directly, these click on the actual rendered
window the way a user would, so they're what actually proves a change *works*, not just that
it compiles and its logic is correct in isolation. Works because `ui/Cargo.toml` builds
`eframe` with the `accesskit` feature — egui's own widgets (`ui.button`/`ui.label`/...)
publish an accessible name/role for free. Hand-painted controls that skip egui's widget API
don't get this automatically — `nav_rail.rs`'s `rail_button` was patched to call
`response.widget_info(...)` explicitly so the nav rail is reachable by name at all; other
custom-painted controls (media library items, timeline clips) aren't yet, so e2e coverage for
those still needs coordinate-based pywinauto calls, or the same `widget_info` treatment first.
One-time setup: `python -m pip install -r e2e/requirements.txt`. Needs a debug build first
(`make build`) and the FFmpeg/GStreamer runtime DLL dirs on `PATH` for the *launched process*
(same requirement as running `ui.exe` by hand) — `make test-e2e` exports both automatically,
running `pytest` directly under `e2e/` needs them exported first. The `oca_window` fixture
(`e2e/conftest.py`) launches its own `ui.exe`, waits for the main window, and calls
`set_focus()` on it before yielding — without that, synthetic clicks land on whatever window
actually has focus (often the terminal `pytest` ran from) and silently do nothing, no
exception raised, the kind of failure that looks like a UI bug but isn't one.

## Architecture

Three-crate split, enforced by dependency direction: `avbridge` has no dependents within
the workspace besides `core`; `core` is UI-agnostic (no `egui` dependency at all)
and `ui` is its only consumer.

- **`avbridge`** — thin C bridge (`csrc/bridge.c`) over libavformat/libavcodec/libavfilter/
  libavutil, called from Rust via FFI (`src/lib.rs`). Written for this project, not an
  auto-generated binding of the full FFmpeg API — keeps the C surface small and auditable.
  `build.rs` locates FFmpeg via `FFMPEG_DIR` (see Commands above). Exposes probing
  (`avbridge_probe`), export rendering (`avbridge_encode_export`: video
  passthrough-copied, audio decoded → loudnorm+limiter filter graph → AAC re-encoded, with
  progress callback and cooperative cancellation), loudness-only measurement
  (`avbridge_measure_loudness`, same decode→loudnorm shape without the encoder — its
  report has no queryable struct API, only `av_log` output during filter-graph teardown, so
  this installs a process-global log callback for the call's duration; **not thread-safe**,
  documented on the function), proxy generation (`avbridge_generate_proxy`: video
  decoded → libswscale downscale (aspect-preserving) → `libopenh264` re-encode, audio decoded
  → format-matched → AAC re-encode), and waveform peak extraction (`avbridge_generate_waveform`:
  audio decoded → downmixed to mono float → min/max amplitude accumulated into a fixed number
  of sample-index buckets, nothing written or re-encoded).
- **`core`** — project/timeline/media data model plus the media wrappers that populate
  it (`probe`, `render`, `loudness`, `proxy`, and `waveform` — all via `avbridge` FFI, no
  subprocess left in any of them), a `playbin`-based GStreamer playback pipeline (`preview`: open/play/pause/seek/
  query, `current_frame()` pulls packed RGBA via an appsink), JSON save/load (`persistence`),
  and mock sample data (`sample`) used to exercise the UI before real files
  are wired in. Locale-neutral by design: it stores data like `Recency` (an enum), never
  pre-formatted display strings — formatting is `ui`'s job.
- **`ui`** — the eframe/egui GUI (glow/OpenGL backend): `app.rs` holds all top-level
  state (`OcaApp`, which screen is active, loaded projects, export jobs) and mutation methods
  (`open_project`, `queue_export`, etc.); `screens/` has one module per of the five screens
  (home, library, editor, queue, prefs) plus shared `widgets`; `theme.rs` is the dark/teal
  palette; **all UI strings live in `i18n.rs`** (pt-BR and English) — never hardcode display
  text in a screen module, add a `Text` variant instead.

`docs/CODE_GRAPH.md` (module dependency graph + public-API index) used to be regenerated by an
`xtask` crate (`cargo run -p xtask -- graph` / `make graph`); that crate was dropped and the
Makefile/docs referencing it are stale (`make graph`, `make test-xtask` — not fixed as part of
this pass). Use `graphify` instead — see the `## graphify` section below.

Test placement follows what's reachable: `core` has a `[lib]` target, so its tests are
real integration tests in `crates/core/tests/` (one `<module>_test.rs` file per module)
exercising only the public API, plus `crates/core/benches/` for criterion benchmarks. A
handful of tests that need a private helper unreachable from `tests/` stay as
`src/<module>/<module>_test.rs` unit tests instead (wired in via `#[path = "..."] mod
tests;`, since the module is still named `tests`). `ui` is a bin-only crate (no `[lib]`), so
all its tests are `src/<module>/<module>_test.rs` unit tests. `avbridge` has a `[lib]`
target, so its FFI tests live in `crates/avbridge/tests/` (also `<module>_test.rs`) against
small checked-in media fixtures.

Planned architecture (not yet implemented, see the plan doc for phases): extracting `preview`'s
decoded frames into an egui texture and wiring that into the Editor screen's playhead, and a
background export queue where rendering runs on a worker communicating over `tokio::mpsc` so
the editing UI never blocks on an in-progress export — each queued job snapshots its render
config at enqueue time, so later edits to the active project don't affect jobs already in the
queue.

## Approach

- Read existing files before writing. Don't re-read unless changed.
- Thorough in reasoning, concise in output.
- Skip files over 100KB unless required.
- No sycophantic openers or closing fluff.
- No emojis or em-dashes.
- Do not guess APIs, versions, flags, commit SHAs, or package names. Verify by reading code or docs before asserting.
- Write all code and commit messages in English.
- Commit using Conventional Commits format (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`, etc.), always in English.

## graphify

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
