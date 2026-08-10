# Commit Plan — Fase 1

order | task_id     | semantic_type | scope      | description                                              | status
------|-------------|---------------|------------|-----------------------------------------------------------|--------
1     | chore-001   | chore         | avbridge   | scaffold oca-avbridge C bridge crate                       | done (1c34a99)
2     | impl-002a   | feat          | avbridge   | probe FFI, C side (bridge.c/bridge.h)                       | done (fa6f75d)
3     | impl-002b   | feat          | avbridge   | probe FFI, Rust wrapper (safe API + error type)             | done (0898fc5)
4     | impl-003    | refactor      | probe      | swap probe.rs from ffprobe subprocess to FFI                | done (38e9432)
5     | test-001    | test          | probe      | tests for probe FFI + swapped probe.rs                      | done (64328b6)
6     | docs-001    | docs          | probe      | document avbridge probe path, fix stale status notes        | done (c59de60)
7     | impl-004a   | feat          | avbridge   | encode FFI: remux passthrough (proves mux path)              | done (4cb506c)
8     | impl-004b   | feat          | avbridge   | encode FFI: audio filter graph (loudnorm+limiter)             | done (7e99907)
9     | impl-004c   | feat          | avbridge   | encode FFI: progress callback + cancellation                  | done (1567b23)
10    | impl-004d   | refactor      | render     | swap render.rs from ffmpeg subprocess to encode FFI           | done (e541b27)
11    | test-002    | test          | render     | tests for encode FFI + swapped render.rs                      | done (2c43a4a)
12    | docs-002    | docs          | render     | document encode FFI path                                      | done (0c0ef2d)
13    | chore-002   | chore         | preview    | gstreamer-rs dependency + link verification                   | done (see below)
14    | impl-005    | feat          | preview    | minimal GStreamer preview pipeline (open/play/pause/seek/query, fakesink) | done (4b3e234)
15    | test-003    | test          | preview    | cover open/play/pause/seek against a real fixture              | done (8a98345)
16    | docs-003    | docs          | preview    | document the preview pipeline                                  | done (4518d84)

**impl-005 scope note:** deliberately stopped short of frame extraction/display (appsink ->
egui texture -> Editor screen playhead) — that's a materially different, bigger task (needs its
own design pass: which pixel format, texture upload cadence tied to the UI's repaint loop vs.
GStreamer's own clock, how scrubbing interacts with a `Paused` pipeline's preroll). Not sized
or broken down yet.

### chore-002 — root cause found and fixed (2026-08-10)

**Symptom:** adding `gstreamer` as a dependency to `nivela-core` — not even calling any of its
code from `probe.rs` — broke `oca_avbridge::probe()` at runtime: `avformat_open_input` started
returning `Open` on every call, 5/6 probe tests failing. 100% reproducible by toggling only the
dependency, everything else identical.

**Ruled out:** DLL *runtime* filename collisions (`comm -12` on sorted `bin/` listings —
zero overlap) and GCC toolchain-ABI variant mismatch (installed VS Build Tools 2022, switched
to the MSVC target, same symptom persisted — see below for why we're on MSVC now regardless).

**Root cause, confirmed:** GStreamer's SDK bundles its own FFmpeg build (`gst-libav`) with
generically-named import libs (`avformat.lib`, `libavformat.dll.a`, etc.) for a different, older
FFmpeg version (`avformat-61.dll`) than the one under `FFMPEG_DIR` (`avformat-62.dll`) —
confirmed via `comm -12` on sorted `lib/` listings, real overlap this time. When both crates'
`-L` search paths are on the same link line, `oca-avbridge`'s bare `-lavformat` can resolve to
GStreamer's bundled (incompatible) copy instead — compiles fine, corrupts behavior at runtime.
Confirmed on **both** MSVC and GNU/mingw toolchains once isolated to this cause, which is why
the earlier "GCC runtime variant" theory was wrong: it's a filename collision, not an ABI
mismatch between toolchains.

**Fix:** `crates/oca-avbridge/build.rs` now copies its FFmpeg import libs into its own
`OUT_DIR` under unique names (`oca_avbridge_ffmpeg_avformat`, etc.) before linking, so
bare-name search can't accidentally match GStreamer's bundled copy regardless of `-L` order.
(`cargo:rustc-link-arg` with a full path was tried first — doesn't work, that directive doesn't
propagate from a library crate's build script to a downstream binary's link step, only
`cargo:rustc-link-lib`/`cargo:rustc-link-search` do.)

**Toolchain note:** mid-diagnosis, switched the default Rust toolchain to
`stable-x86_64-pc-windows-msvc` (installed VS Build Tools 2022 + C++ workload to
`E:\VSBuildTools`, avoiding a `C:` disk-full blocker on the Windows SDK component along the
way). Confirmed after the real fix landed that GNU/mingw works too — MSVC was not actually
required, but it's what's set up and verified end-to-end now, so it stays the default.

impl-002 was split into impl-002a/impl-002b (see task_breakdown.md observations) after the
combined diff came in over both size gates.
