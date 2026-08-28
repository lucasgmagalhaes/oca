# Mobile Support (Android + iOS) — ADR

Proposed 2026-08-27, discussion-only — no code written yet. Not in `features/request.md`
(desktop-only original plan) and not yet in `ROADMAP.md`. This is a second platform, not a
feature: treat it as its own phased effort with its own spec once (if) approved, not folded
into the desktop P-queue.

Goal as stated: **full editing** (not a viewer/companion app) on both Android and iOS —
timeline, effects, export — reusing `core`'s data model and as much of `avbridge` as possible.

---

## Why this is hard: the three-crate split doesn't just recompile

`avbridge → core → ui` holds on mobile too, but two of the three layers have platform-specific
bodies that don't cross-compile for free, and the desktop assumption "GStreamer + FFmpeg dev
build are on this machine" (`CLAUDE.md`) has no mobile equivalent — there's no apt/Homebrew
package, no dev build to point `FFMPEG_DIR`/`PKG_CONFIG_PATH` at.

## Decision 1 — `avbridge`: cross-compile FFmpeg per target, keep the FFI shape

FFmpeg itself cross-compiles for `aarch64-linux-android` (NDK toolchain) and
`aarch64-apple-ios`/`aarch64-apple-ios-sim` (Xcode toolchain) — this part is well-trodden
(ffmpeg-kit's build scripts, pre-2023 abandonment, are a reference even though the project is
dead). `csrc/bridge.c` and `csrc/filters.c` themselves are portable C; the FFI surface in
`avbridge/src/lib.rs` doesn't need to change shape.

What does change:
- `build.rs` needs a per-target branch: today it assumes one desktop `FFMPEG_DIR`
  (include/lib) plus the GStreamer-bundled-FFmpeg import-lib-renaming step (`CLAUDE.md` —
  **do not remove that step**, it prevents silent ABI-mismatch linking against gst-libav's
  copy). Mobile targets need their own `FFMPEG_DIR` per triple, built once and cached (CI
  artifact or vendored, not built on every dev machine).
- Static vs dynamic linking: iOS effectively requires static libs (`.a`) or `.xcframework`
  bundles inside the app bundle — no arbitrary dynamic loading. Android permits `.so` but
  bundling many FFmpeg `.so`s inside an APK/AAB is the same shape as desktop's DLL-next-to-exe
  problem, just inside `jniLibs/`.
- **Hardware encode is the real gap.** Desktop's `GpuEncoderPreference`
  ([lib.rs:593](crates/avbridge/src/lib.rs:593)) targets desktop GPU encoders reachable through
  FFmpeg's normal hwaccel path (nvenc/qsv/videotoolbox-desktop). Mobile hardware encode goes
  through MediaCodec (Android) or VideoToolbox (iOS, same API family as desktop macOS but a
  different profile/surface) — FFmpeg *can* wrap both, but only if built with those hwaccels
  enabled, and the calling convention (surface-based encode) differs enough from desktop GPU
  encode that `render.rs`'s encoder-selection logic will need a mobile-specific arm, not just a
  new enum variant. Software-only fallback works but costs battery/thermal — acceptable for v1,
  not for a shipped feature.

## Decision 2 — `core::preview`: replace GStreamer with native players per platform

This is the one **architectural fork**, not a portability shim, and worth deciding explicitly
rather than discovering mid-implementation.

**Option A — GStreamer everywhere.** Official Android and iOS GStreamer distributions exist
(`.aar`/`.xcframework`). Keeps one preview implementation, matching desktop's current design
(`preview-pipeline.md`). Real cost: mobile GStreamer builds ship a much narrower plugin set,
bundle size is heavy (tens of MB per ABI), and neither platform's build is as battle-tested as
desktop's — debugging a broken mobile-specific plugin gap has no fallback.

**Option B — native players (`ExoPlayer`/`MediaPlayer` on Android, `AVPlayer` on iOS) behind
the same `Preview` trait `core` already exposes.** Two real implementations instead of one,
callable only from their own platform (JNI bindings for ExoPlayer, Objective-C/Swift bridge for
AVPlayer) — meaningfully more code, but each is the platform's own blessed, battery-tuned path,
with better hardware-decode integration and none of GStreamer's mobile packaging tax.

**Recommendation: Option B.** `core`'s `preview` module already exists as an abstraction
consumed by `ui` — the fix here is discipline (keep the trait boundary real, don't leak
GStreamer-specific types through it) rather than new design. Cost is duplicated player logic,
not duplicated architecture. Revisit only if Android/iOS native integration proves harder in
practice than the mobile GStreamer packaging tax.

## Decision 3 — `ui`: egui stays, but touch-first interaction is a redesign, not reflow

- **Android**: `cargo-apk`/`cargo-ndk` + `winit` egui backends are the most mature mobile path
  available today — treat this as the platform to prototype against first.
- **iOS**: no equivalent turnkey backend. Needs a thin Objective-C/Swift host app owning the
  `UIApplication`/Metal surface lifecycle, calling into a Rust staticlib
  (`cargo build --target aarch64-apple-ios`) for everything above that. More host-side
  plumbing than Android, same egui core.
- Timeline trim handles, keyframe drag, multi-select — all currently mouse+keyboard-shaped in
  `screens::editor` — need touch-target sizing and gesture review per-widget, not just a
  responsive layout pass. Budget this as real UX work, not a CSS-equivalent tweak.

## Decision 4 — packaging has no desktop equivalent

Fase 8's installer/auto-update work (`matrix/packaging.md`) doesn't carry over: distribution is
Google Play (AAB) and App Store/TestFlight, with their own signing, review, and update
mechanisms. Treat as net-new scope when this phase is actually planned, not a port.

---

## Proposed phasing (if approved)

1. **Android spike, playback + trim only, no export.** Validates FFmpeg cross-compile for
   `aarch64-linux-android`, `cargo-apk` egui integration, and ExoPlayer behind the `Preview`
   trait — the three riskiest unknowns — before committing to iOS or full editing parity.
2. **Android export**, reusing `core::render`/`avbridge::encode_*` as-is once FFmpeg-android
   linking is proven; MediaCodec hwaccel as a stretch goal, software encode as the v1 fallback.
3. **iOS**, once the Android spike has de-risked the shared `core`/`avbridge` mobile path — iOS
   adds host-app plumbing and AVPlayer, but shouldn't re-litigate decisions 1-2.
4. **Feature parity pass** (effects/keyframes/AI features) — only after 1-3 prove the
   foundation; premature to scope in detail now.

Each phase gets its own `spec/matrix/mobile.md` entry once phase 1 actually starts — this file
stays the "why", not a task tracker.
