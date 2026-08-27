# Effects, Keyframes, Color

Video effects, keyframe system, masks/transitions, color/LUTs, stabilization. Detail:
`features/request.md` Fase 4, `matrix/changelog.md` Fase 4 (the largest section — read there
for exact export-vs-preview wiring per effect).

## Wired to export

- [x] gain_db, crop, flip, color_filter, vignette, brightness/contrast/saturation, sharpen,
      chroma_key, mask_shape, blur, pixelize, shake, glitch, zoom, speed_factor, freeze_frame,
      deflicker.
- [x] Transitions (fade/slide/zoom) via `ClipSegment::transition_in`.
- [x] 3D LUTs (`ClipInstance::lut_path`) — no preview element exists for this (see below).
- [x] Video stabilization (`stabilization_intensity` → `deshake`, single-pass — this build has
      `--disable-libvidstab`).
- [x] General keyframe system (position/scale/rotation/opacity via `Keyframe<T>`,
      piecewise-linear interpolation) — `core/src/keyframe.rs`. Splitting a keyframed clip
      rescales + inserts a continuity-preserving boundary point.
- [x] Layer transform (position drag + resize), auto-reframe, motion tracking (see
      `matrix/ai-features.md`).

## Wired to preview

- [x] Scale/rotation/opacity keyframes, pixelize/shake/zoom/freeze_frame, speed, transitions
      (fade/zoom/slide), mask_shape, gain_db (real audio route, not silent `fakesink`).
- [x] Multi-track compositing (`compositor` element, position/opacity/scale/chroma-key per
      overlay branch) — `Preview::open_composited`.
- [x] Per-branch seeking + per-branch playback rate (`seek_composited`).

## Confirmed hard wall — no matching GStreamer element exists on the dev machine at all

- [ ] vignette, glitch, deflicker, LUTs (3D `.cube` specifically), stabilization preview
      (`deshake`/`opencvvideostab` all checked via real `gst-inspect-1.0`). Each needs a
      custom-coded GStreamer element or CPU-side frame processing — materially bigger lift than
      every other preview gap closed so far (those all reused stock elements).

## Text, shapes

- [x] Manual text insertion, full HSV+preset+hex/rgb color editor (transactional modal).
- [x] Word-highlight subtitle style (shorts/MrBeast-style), live timing in both preview+export,
      auto line-wrap identical in both.
- [x] Bundled fonts (6 faces, SIL OFL, lazy-loaded).
- [x] Geometric shapes: presets (rectangle/ellipse/triangle/trapezoid/arrow) + hand-drawn
      custom polygon (click-to-place on preview).
- [x] Text/shape preview compositing (`appsrc ! imagefreeze` branches).

## Known gaps

- [x] Color scopes (waveform/vectorscope) for calibrated grading. `avcore::scopes`
      (`luma_waveform_rgba`/`vectorscope_rgba`, `crates/core/src/scopes.rs`) — pure pixel
      analysis against the already-decoded preview `VideoFrame`, no new avfilter/GStreamer
      element needed. Toggled via the Editor preview panel's "📊" button
      (`App::scopes_enabled`), computed opt-in inside `App::pump_preview_frame` alongside the
      main preview texture upload (never on the hot path when off). **Simplification, not a
      hard wall**: renders as a grayscale intensity image (BT.709 luma waveform, BT.601 Cb/Cr
      vectorscope), not a green-phosphor trace with a calibrated IRE/hue graticule — accurate
      for judging exposure/saturation spread at a glance, not a substitute for a calibrated
      broadcast monitor. Core math verified against real `cargo test` execution in an isolated
      scratch crate this session — this sandbox's own `core` crate still can't build end to end
      (Ubuntu 24.04's packaged FFmpeg is too old for `avbridge/csrc/filters.c`, same documented
      gap `CLAUDE.md` already calls out; rustc itself was upgraded 1.94→1.98 this session via
      `rustup update`, and GStreamer dev headers installed via `apt`, which got a real
      `cargo check -p core` all the way to that one pre-existing C failure — closer than any
      prior session, still not a full build). The Editor panel wiring itself is implemented but
      not visually verified (no way to drive the actual GUI in this environment).
- [ ] Audio ducking (auto-lower music under speech) — `audio_mix.c` already has the multi-branch
      mixing infra this would build on. Needs new `avfilter` wiring in C this sandbox can't even
      syntax-check right now (no FFmpeg dev headers reachable at all) — see `ROADMAP.md` P2 item 6.

---

[← back to spec/INDEX.md](../INDEX.md)
