# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

oca — a native Rust video editor (`egui`/`eframe`) for cutting gameplay footage for the
PacoPaçoca YouTube channel. Headline features: automatic loudness normalization and
export-that-matches-the-source-bitrate. Full phased plan: [`features/request.md`](features/request.md).

## Status

- **Fase 1-3 — done.** probe/export/loudness/proxy via avbridge FFI; GStreamer preview
  pipeline; full timeline editing (select/split/trim/drag/copy-paste/context menu/composite
  blocks/multi-sequence tabs/resizable panels/filmstrip/waveforms); `.ocproj` save/load;
  background import/export queue. The automatic export audio chain also runs `afftdn` noise
  reduction ahead of `loudnorm`/`alimiter` (`request.md`'s "redução de ruído... aplicada
  automaticamente antes da exportação", a CapCut-inspired addition beyond the original phased
  plan) — same unnamed-filter-stage convention as `alimiter`, in all three places the audio
  chain string is built (`export.c`, `timeline_export.c`, `timeline_export_multi.c`).

- **Fase 4 — in progress.** Export and preview are timeline-aware.
  `ClipInstance::video_filter_chain()` builds per-clip avfilter chains for export;
  `build_video_filter_bin` covers a narrower subset for GStreamer preview.

  **Wired to export:** gain_db, crop, flip, color_filter, vignette, brightness/contrast/
  saturation, sharpen, chroma_key, mask_shape, blur, pixelize, shake, glitch, zoom,
  speed_factor, freeze_frame, deflicker, transitions (fade/slide/zoom via `ClipSegment::
  transition_in`), 3D LUTs (`ClipInstance::lut_path`, no preview element exists for this),
  video stabilization (`stabilization_intensity` -> `deshake`, single-pass only — this
  build has `--disable-libvidstab`), a general keyframe system (position/scale/rotation/
  opacity via `Keyframe<T>` lists — see below), layer transform (position drag + resize),
  layer templates, auto-reframe, motion tracking.

  **Wired to preview:** scale/rotation/opacity keyframes, pixelize/shake/zoom/freeze_frame.
  **Not wired to preview:** transitions, vignette, chroma_key, mask_shape, gain_db, speed,
  glitch, deflicker, LUTs, stabilization, position keyframes (needs multi-track preview
  compositing that doesn't exist yet — single-clip `playbin` pipeline has no `overlay` stage).

  **Alpha/overlay-track caveat (applies to chroma_key, mask_shape, position/opacity
  keyframes, layer resize):** these only have a visible effect on a clip placed on an
  **overlay track** (`avbridge_encode_timeline_export_multi`'s track 1+) — a single/background
  track's final `format=yuv420p` conform drops the alpha plane they need, and has no
  pad/fit step for a resized footprint either.

  **Keyframe system:** `crate::keyframe` — position/scale/rotation/opacity each animate via
  a `Vec<Keyframe<T>>` on `ClipInstance`, piecewise-linearly interpolated
  (`evaluate_keyframes`). Since `ClipSegment` crosses the Rust→C FFI boundary as `#[repr(C)]`,
  each property's piecewise avfilter expression is built in Rust and passed across as a
  plain string (`keyframe_video_filter_chain`, `position_overlay_xy_expr`) — no new C
  functions. Splitting a keyframed clip rescales and inserts a continuity-preserving
  boundary point (`split_keyframes_at`). Visual markers (read-only diamonds) show on the
  timeline clip block; editing goes through the properties panel's list editor.
  **Verification caveat:** the filter-graph correctness of rotation/position's `t`/`if`/
  `between` expressions has not been empirically confirmed against a real render on any
  machine — this dev machine's FFmpeg build can't open any encoder at all right now (see
  GPU encode note below), so every encode-path test fails here regardless.

  **Auto-reframe (static crop only, not animated):** `avcore::auto_reframe` — reuses
  `ClipInstance`'s static `crop_x/y/w/h` (no new keyframe track), matching `request.md`'s own
  wording: a one-shot AI recenter on aspect-ratio change, then the existing crop controls are
  the "manual adjustment on top". `compute_reframe_crop()` is pure geometry (fully unit
  tested, no ONNX). `detect_faces()` runs UltraFace `version-RFB-320`
  (`Linzaer/Ultra-Light-Fast-Generic-Face-Detector-1MB`, MIT) via the `ort` crate
  (`download-binaries` feature — **statically links** `onnxruntime` into the binary, not a
  runtime `.dll`). Model isn't bundled with the installer yet —
  `avcore::model_download::download_reframe_model` fetches it on demand into
  `<prefs dir>/models/`, same shape as the Whisper model download (both share
  `model_download.rs`'s `download_file()` helper). Falls back to a centered crop (with a
  toast) when no face is found. **Confirmed working end-to-end** on this dev machine: a real
  inference pass against the downloaded model succeeded on the `core` test fixture (0 faces
  found, as expected for a fixture with no real face in it).

  **Motion tracking (region now user-editable):**
  `avcore::motion_tracking` — plain fixed-template block matching (full search within a
  radius each sampled frame, SAD against the *first* frame's template so it doesn't drift),
  no ML model. Reuses the keyframe system: `tracked_positions_to_keyframes()` converts the
  tracked path into a `position_keyframes` delta from the first tracked frame, added onto
  whatever fixed position was already set, so the layer rides the motion while keeping its
  initial placement. Fully unit tested against synthetic frames, including an off-center
  starting region (`tracks_a_block_starting_from_an_off_center_region`) so a future regression
  that silently re-hardcodes the center back to `(0.5, 0.5)` gets caught. `ui`'s "Rastrear
  movimento" button no longer always tracks a region centered on the frame — the properties
  panel now has region controls (center X/Y drag values, independent width/height/search-radius
  sliders, a "Centralizar região" reset button) right above it, defaulting to the old centered-
  square-region values but user-editable before clicking the button. `App::motion_track_center_x`/
  `_y`/`_width`/`_height`/`_search_radius` are plain, non-persisted session state (a one-shot
  tracking job's input, not a durable clip property like `crop_x/y` — only the job's *output*
  `position_keyframes` is saved), read once at click time by
  `App::spawn_motion_track_selected_clip` and threaded through to `avcore::track_region`, which
  now takes `template_width_frac`/`template_height_frac` independently (previously one
  `template_size_frac` scalar, forcing a square region) — each clamps to its own frame
  dimension (width to the frame's width, height to its height) rather than both to the shorter
  side. No drag-on-preview picker yet — numeric entry only.

  **AI background removal (inference, matte generation, and export wiring all done —
  preview still not):** `avcore::background_removal::segment_person` runs MODNet
  (`ZHKKKe/MODNet`, ONNX export by `yakhyo/modnet`, Apache-2.0) against one decoded frame,
  returning a per-pixel alpha matte. Model isn't bundled yet —
  `avcore::model_download::download_background_removal_model` fetches it on demand, same shape
  as the Whisper/reframe model downloads. **Confirmed working end-to-end** on this dev machine:
  real inference against the downloaded model succeeded on a synthetic frame.

  **Matte generation (`ui`):** `App::spawn_generate_matte_for_selected_clip`
  (`crates/ui/src/app/background_removal.rs`) — the properties panel's "Gerar máscara" button,
  next to the "Remoção de fundo (IA)" checkbox. Samples frames across the clip's own trimmed
  source range via `avcore::preview::Preview` (same seek+poll pattern motion-tracking/auto-
  reframe use, coarser — `SAMPLES_PER_SEC = 2.0`, `MAX_SAMPLES = 40` — since each sample re-runs
  a whole ONNX session, see `segment_person`'s doc comment on the lack of session reuse across
  calls), runs `segment_person` on each, and encodes the resulting alpha values (rounded to
  `0..=255`) into a small H.264 video via `avcore::encode_matte_video` — saved to
  `avcore::background_removal::mask_cache_dir_for_project` (a hidden sibling folder next to the
  project file, same convention as `crate::proxy`'s editing-proxy cache), keyed by clip id via
  `mask_path_for_clip` (per-clip, not per-asset, since the matte covers this clip's own
  trim range). The matte's own declared fps is however many frames actually got sampled per
  second of the clip's real duration — a "stepped" alpha update rate slower than the clip's own
  frame rate, not frame-perfect, but the export-side compositor already tolerates a
  shorter/coarser overlay input by holding the last known frame (the same mechanism a track-1
  clip whose own duration doesn't match track 0's already relies on).

  **Encoding (`avbridge`):** `avbridge_encode_matte_video` (new file `csrc/matte_encode.c`) —
  encodes a flat `luma_frames` buffer (frames concatenated, `width*height` bytes each) into
  YUV420P H.264 via a forced `libopenh264` open (no GPU-preference ladder — this is a small
  internal artifact, not user-facing output), luma plane = the supplied alpha bytes, both
  chroma planes filled with neutral 128 ("grayscale-as-luma", not a true single-plane GRAY8
  stream — sidesteps unverified GRAY8-support-in-libopenh264 risk). Closely mirrors
  `proxy.c`'s existing encoder-open/frame-write/flush pattern, since that already does almost
  everything needed here minus the decode/scale side (frames arrive pre-decoded from Rust,
  no demuxer/decoder needed at all — the first avbridge function shaped this way). Rust wrapper
  `avbridge::encode_matte_video`/`avcore::encode_matte_video`.

  **Export compositing (`avbridge`):** `ClipSegment` gained a `mask_video_path` field (C
  struct in `bridge.h`, the `#[repr(C)]` `RawClipSegment` mirror, and the public
  `avbridge::ClipSegment` — all three kept in lockstep, plus both `CString`-marshalling call
  sites in `avbridge/src/lib.rs`) — empty string = no matte, same "always non-null, empty
  means unset" convention every other optional string field on this struct already uses.
  `core::render::resolve_timeline_segments_multi` populates it from
  `ClipInstance::background_removal_mask_path`, gated the same way `resolve_clip_filters`
  already gates `layer_scale`'s resize stage: **only on an overlay track** (`is_overlay`) —
  a single/background track's clips never reach a compositing stage, so their alpha (from
  this or any other source, e.g. chroma_key/mask_shape) is always discarded by the final
  `format=yuv420p` conform regardless, same documented caveat those two already carry.
  `timeline_export_multi.c`'s `init_overlay_graph` (previously a fixed 2-input
  `[in0]<f0>[v0];[in1]<f1>[v1];[v0][v1]overlay=...` graph) now optionally takes a third
  `vdec2`/`f2` pair, producing `[in0]<f0>[v0];[in1]<f1>[v1];[in2]<f2>,format=gray[m2];
  [v1][m2]alphamerge[v1a];[v0][v1a]overlay=...` when a matte is present — a second
  `OverlayDecoder` (`ov2`) opened/advanced exactly like the existing overlay-track decoder
  (`ov1`), against a synthetic `ClipSegment` pointing at the matte file with its own 0-based
  `source_in_secs`/`source_out_secs` (the matte's internal timeline, distinct from the
  original clip's timeline/source coordinates — see `mask_video_path`'s doc comment in
  `bridge.h` for the exact mapping). Falls back to compositing without the matte (not a hard
  export failure) if the matte file can't be opened or the 3-input graph fails to build —
  matches this codebase's general "an optional post-effect degrades gracefully rather than
  aborting a multi-minute render" posture. `alphamerge` **replaces**, not combines with, any
  alpha `f1`'s own chain already produced (e.g. simultaneous chroma_key/mask_shape on the same
  clip) — combining multiple alpha sources on one clip isn't supported.

  **Verification caveat:** this specific C work (the `init_overlay_graph` 3-input extension
  and `matte_encode.c`) was written and reviewed carefully but could not be exercised against
  a real render on any machine during development — same "this dev machine's FFmpeg build
  can't open any encoder" gap noted below applies. It *was*, however, syntax/type-checked for
  real: `gcc -fsyntax-only -I <ffmpeg include dir>` against each modified/new `.c` file
  individually (works even when the full crate can't build, since it only needs the headers
  the *specific* file includes — `filters.c`'s separate FFmpeg-7.1-only API usage doesn't
  block syntax-checking files that don't call those functions) came back clean, and the
  before/after brace/paren-count delta across the whole file matched exactly, both useful
  fallback techniques when `cargo check`/`build` itself is blocked in a sandbox missing a
  new-enough FFmpeg. Not a substitute for an actual render — treat the export-compositing path
  here as unverified beyond static analysis until it's run for real once.

  **Not yet done:** preview compositing (no `alphamerge` stage in the GStreamer preview
  pipeline — same gap chroma_key/mask_shape/position keyframes already have there).

  **Text-to-speech (done, real end-to-end):** `avcore::text_to_speech` — `espeak-rs`
  (statically-linked espeak-ng, no runtime DLL; `core/build.rs` copies its `espeak-ng-data`
  next to the workspace's build output, since `espeak-rs` looks for that directory name next
  to the running executable) phonemizes typed text, then a Piper VITS ONNX model
  (`rhasspy/piper-voices`' pt_BR `faber` voice, MIT) synthesizes it — Piper's phoneme-ID scheme
  (BOS/EOS/interspersed-PAD) and ONNX I/O contract (`input`/`input_lengths`/`scales` ->
  `output`) confirmed against a real downloaded voice, not guessed. **Confirmed working
  end-to-end** on this dev machine: real synthesis of a Portuguese sentence produced a WAV with
  real (non-silent, non-garbage) audio content. Model isn't bundled yet —
  `avcore::download_tts_voice` fetches both the `.onnx` and its `.onnx.json` sidecar on demand.
  UI: Mídia screen's "Texto-pra-fala" button opens a text modal; "Gerar" synthesizes on a
  background thread and imports the resulting WAV through the exact same pipeline as a
  drag-and-drop import ([`App::spawn_import`]) — no separate asset-creation path needed.

  **Geometric shapes (data model, export rendering, and UI all wired now, including hand-drawn
  custom shapes):** `TrackKind::Shape`/`ShapeClip` — rectangle/square, ellipse/circle, triangle,
  trapezoid, arrow presets (`ShapeKind::rectangle()` etc.), plus a `Polygon(Vec<(f32,f32)>)`
  variant that also covers `request.md`'s "forma personalizada" (custom shape) ask. Beyond the
  numeric `polygon_vertex_editor` list editor (edits an existing polygon's vertices one value at
  a time), the toolbar's "✎ Desenhar forma" button now lets the user click points directly on
  the preview to draw a brand-new custom polygon from scratch: `App::start_drawing_custom_shape`
  sets `App::drawing_shape_points = Some(vec![])`; `screens::editor::draw_custom_shape_surface`
  (a `layer_transform_preview` branch, mutually exclusive with its usual layer drag/resize
  handling — active whenever `drawing_shape_points.is_some()`) takes over `canvas_rect`, drawing
  each placed point as a dot with connecting lines via `App::push_drawing_shape_point`; Escape
  cancels, Enter (needs >= 3 points) commits via `App::finish_drawing_custom_shape`, which
  derives a bounding box across the clicked canvas-fraction points, centers/sizes the new
  `ShapeClip` on it, and re-expresses each point relative to that box (`(x-center)/width`) since
  `ShapeKind::Polygon` stores vertices in the shape's own local unit square, not absolute canvas
  fractions — `rotation_deg` starts at `0.0`, same as a fresh preset. **Known limitation:**
  drawing requires a loaded preview frame (`layer_transform_preview`'s existing precondition —
  the toolbar button toasts `ShapeDrawNeedsPreview` instead of entering drawing mode otherwise),
  and since shape preview rendering itself doesn't exist yet (see below), the drawing surface is
  the canvas outline only, not a live composited image to trace over. Every fixed preset is
  placeable and editable too. `avcore::shape_render` builds a `geq` avfilter node per
  shape (rotation via a per-pixel coordinate rotation, ellipse via a quadratic test, every
  straight-edged shape via ray-casting point-in-polygon — correct for the arrow's concave
  notches), applied as a post-processing pass (`avbridge::apply_shape_overlays`) the same way
  text overlays already are. **Confirmed working for real**, not just parse-checked: rendered
  the generated filter against a synthetic frame and visually verified correct position/
  rotation/color, including catching two real bugs (RGB not converted to YCbCr; yuv420p
  chroma-subsampled coordinates not matching luma's) before they shipped.
  UI (`App::add_shape_track`/`add_shape_clip`, `timeline_ops.rs`): toolbar "S+ Forma"/"+
  Adicionar forma" buttons (`screens/editor/mod.rs`'s `toolbar()`); `add_shape_clip` auto-
  creates a shape track if none exists yet (unlike `add_text_clip`, which stays a no-op without
  one — there's no drag-and-drop path onto a shape track the way there is for media assets, so
  the button has to be able to place the first clip itself). Timeline strip renders each shape
  clip as a colored block with a preset glyph (`shape_kind_glyph` in `timeline_panel.rs`),
  click-to-select and a context-menu delete, same interaction shape as text clips. Properties
  panel (`shape_clip_properties` in `properties_panel.rs`) edits preset/color/center position/
  size/rotation/outline thickness/timing via the same clone-mutate-writeback pattern
  `text_clip_properties` uses. **Preview still not implemented** — same gap `TextClip` has
  (`ShapeClip`'s own doc comment flags it); the timeline block is a color/glyph stand-in only,
  the real render only happens on export.

- **Fase 5/6 — partially done.** Aspect ratio selection; prefs (`prefs.oc` — same
  gzip-compressed MessagePack framing `.ocproj` uses, via `avcore::to_ocproj_bytes`/
  `from_ocproj_bytes` generalized to any `Serialize`/`DeserializeOwned` value) + export queue
  (`queue.json`, still plain JSON) persisted to the platform config dir; recent project list;
  debounced autosave + restore modal; crash detection + panic hook; prefs modal; home screen
  context menu; file size estimate; structured logging; copy formatting (`Ctrl+Shift+C`/`V`);
  configurable key bindings; export job reordering; output-folder overwrite/rename/cancel
  prompt; word-highlight subtitles (single-line only — a caption that wraps in `drawtext`
  gets every highlight positioned as if still on one line); Whisper subtitles (model fetched
  on demand via `avcore::model_download`, not bundled); music/SFX library scanning (no
  bundled content, points at a user-configured folder); per-user Editor panel layout
  (`PrefsState::lib_panel_width`/`props_panel_width`/`timeline_height`, the per-user half of
  Fase 3's "layout salvo por projeto ou por usuário" — per-project persistence would need
  `.ocproj` schema changes and isn't wired up). `App::save_prefs`'s background-thread write has
  a synchronous twin, `App::save_prefs_sync`, called from `on_exit` specifically — a spawned
  thread has no guarantee of finishing before the process actually exits, so a live-session-only
  change like a dragged panel width (no save trigger short of opening Preferences otherwise)
  needs a save that's guaranteed to complete before shutdown, not just kicked off.

  **Copy/paste now preserves composite block membership.** `App::copy_selected_clip` used to
  capture only the one clicked clip even when it was a composite block member, so pasting always
  produced a standalone clip — short of Fase 3's "reutilizado ... como se fosse um clipe só"
  spec, since a pasted "block" wasn't actually a block anymore. It now captures every clip
  sharing the selected clip's `composite_id` too; `App::paste_clip_at_playhead` pastes the whole
  group back as one unit (each clip keeping its original offset relative to the earliest one,
  anchored at the playhead, all sharing one fresh `composite_id`) — a lone copied clip stays
  standalone exactly as before. `App::clipboard_clip`'s type changed from a single
  `ClipInstance` to `Vec<ClipInstance>` accordingly.

  **Timeline context menu now offers "Mesclar em bloco composto"** — Fase 3's context-menu spec
  lists it among the actions the right-click menu should mirror from the toolbar, but it was
  toolbar-only until now. Enabled under the same condition as the toolbar button
  (`App::multi_selected_clip_ids.len() >= 2`); calls the same `App::merge_into_composite`.

  **`Ctrl+O` now adds an opacity marker.** Fase 6's key binding spec lists this shortcut
  explicitly, but no such action existed anywhere — `KeyBindings` had no field for it and
  `BindableAction` had no variant. `App::add_opacity_marker_at_playhead` (fifth entry in both,
  rebindable in Preferences same as the other four) adds one opacity keyframe at the playhead's
  position within the selected clip's own span, with a value equal to the clip's current
  effective opacity there (via `avcore::keyframe::evaluate_keyframes`) so placing the marker
  never itself changes how the clip looks — only moving it afterward does.

  **GPU encode — hardware success unverified.** `gpu_encoder.c`'s `open_video_encoder()`
  tries NVENC/Quick Sync/AMF per `Prefs.gpu_encoder`, falling back to CPU (libopenh264) on
  failure; `h264_qsv` correctly requests NV12 (not the yuv420p every other encoder uses) via
  `pix_fmt_for_encoder_name()`. No machine this was developed on has NVENC/AMF hardware, and
  this dev machine's FFmpeg build has neither `libopenh264` nor `h264_qsv` compiled in at
  all — every test that reaches `avcodec_open2`/`avcodec_send_frame` fails with
  `EncodeError::Encoder` here specifically, a pre-existing build gap, not a regression.

- **Fase 7 — partially done.** Editing proxy generation already existed (Fase 1-3, downscaled
  transcode for scrubbing); its resolution is now user-selectable —
  `avcore::proxy::PreviewQuality` (Low/Medium/High = 360p/480p/720p, `request.md`'s explicit
  cap) replaces the old fixed `PROXY_HEIGHT` constant. `PrefsState::preview_quality` (Ajustes
  screen, next to the GPU encoder picker) flows through `App::spawn_import` into
  `avcore::ensure_proxy`, so newly imported clips generate their proxy at the chosen height.
  `proxy_path_for` bakes the height into the filename (`{stem}_proxy_{height}p.mp4`)
  specifically so switching the preference can't collide with or silently keep serving a
  stale-resolution proxy under the old path — the tradeoff is no retroactive regeneration: an
  asset imported before the change keeps its existing proxy until re-imported.

  **Runtime telemetry (event-shaped metrics only — CPU/RAM/GPU sampling not done).**
  `avcore::telemetry` — `TelemetryEvent` (`ImportCompleted`/`ExportCompleted`/
  `PreviewFrameTime`/`Error`) appended as JSON lines via `record_event`, no rotation yet, plain
  on-device file (matches `request.md`'s "fica no dispositivo por padrão" — never sent
  anywhere). `ui`'s `App::record_telemetry` sends to a dedicated background writer thread
  (`telemetry::spawn_telemetry_writer`, spawned once in `App::new`) that owns the only
  receiver and appends to `telemetry.jsonl` next to `platform_log_dir()` — the same directory
  Fase 6's rolling daily `tracing` log already writes into. Wired at three points: import
  duration (`import_one` times itself, `pump_import_queue` records `ImportCompleted` on
  `Enriched` and an `Error` event on `Failed`), export duration (the render-worker thread times
  itself around `render_export_job_multi`; `output_duration_secs` is the exported timeline's
  own footage length, re-derived from track 0's resolved `ClipSegment`s — a different quantity
  from the wall-clock encode time `duration_ms` measures), and preview frame time (sampled from
  `ctx.input(|i| i.unstable_dt)` while `preview_playing`, throttled to once per
  `PREVIEW_FRAME_TELEMETRY_INTERVAL` — recording every frame would flood the file). A
  `PrefsState::telemetry_enabled` toggle (Ajustes screen's Project card, on by default) gates
  `record_telemetry` itself, so disabling it stops collection outright rather than just hiding
  a report. `record_event` now rotates `telemetry.jsonl` to a `.1`-suffixed backup once it
  crosses 10 MiB (`avcore::telemetry::rotate_if_oversized`, size-based rather than Fase 6's
  daily rotation since this crate has no date/time dependency) — bounds long-lived-install disk
  usage to roughly the cap times two. **Not yet done:** CPU/RAM/GPU resource-usage sampling
  (`request.md`'s full wishlist) — no `sysinfo`-style dependency is wired in, only the
  event-shaped metrics above.

  **Not yet done (rest of Fase 7):** hardware-accelerated preview decode and an explicit
  lazy-frame-loading layer. Release binary stripping is done — `[profile.release]` has
  `strip = true` alongside `opt-level = 3`/`lto = true`.

- **Fase 8 — barely started (auto-update check-and-notify only).** `avcore::update_check` —
  `fetch_latest_release` hits GitHub's `repos/lucasgmagalhaes/oca/releases/latest` API
  (`ureq::get` + manual `serde_json::from_str`, not `ureq`'s own `into_json` — that's gated
  behind a `json` feature this crate doesn't enable); `is_newer` does a pure, panic-free
  dotted-version comparison, fully unit tested. `ui`'s `App::spawn_update_check` runs this once
  on startup on a background thread (silent on any failure — offline, rate-limited, no releases
  published yet — this is a best-effort courtesy notice, never something that should alarm the
  user), and `App::available_update` (surfaced via a small banner + GitHub link on the Home
  screen) is only ever set when a real newer version is found. **Scoped down from
  `request.md`'s full ask:** only the "consulta a última release... e avisa" half is done —
  actually downloading and applying the update ("baixar e aplicar") isn't implemented; the
  banner just links to the release's GitHub page for a manual download. **Not started at all:**
  installers/packaging for Windows/Linux, bundled engines (FFmpeg/GStreamer/Whisper/TTS/ONNX
  Runtime), VAAPI GPU encode on Linux, configurable install location. **Verification caveat:**
  this dev environment's outbound network proxy blocks direct calls to `api.github.com`
  (returns its own "GitHub access is not enabled for this session" error, not a real GitHub
  response), so `fetch_latest_release` itself could not be exercised against the real API here
  — every `ureq` call it makes mirrors `model_download.rs`'s already-working
  `download_whisper_model` pattern exactly (`ureq::get(url).set(...).call()`), and the JSON
  shape/User-Agent-header requirement match GitHub's documented API, but this is unverified
  beyond that, same "implemented carefully, not run for real" caveat this file already carries
  for a few other network/hardware-dependent features.

- **Custom title bar (ad hoc, not from `request.md`).** `main.rs`'s `NativeOptions` now sets
  `.with_decorations(false)` — no OS window chrome. `screens::breadcrumb::show` (still the
  topmost `Panel::top`, now doubling as oca's own title bar) draws the window's drag-to-move
  region (double-click toggles maximize/restore) and hand-painted minimize/maximize/close
  buttons in the app's own theme colors, alongside its existing app-name/screen/project
  breadcrumb content. `screens::breadcrumb::handle_resize_borders`, called first thing in
  `App::ui` before any panel narrows the root `Ui`, replaces the OS's own edge/corner resize
  handles — covers south/west/east and the two bottom corners only, not the top edge/corners
  (those overlap the title bar's own drag region horizontally, so a north-facing resize zone
  there would fight it for the same pointer input; the accepted tradeoff is no straight-up-edge
  resize, only via the other three edges/corners). **Verification caveat:** window dragging/
  resizing/minimize/maximize/close all go through `egui::ViewportCommand`s this sandbox has no
  way to exercise (no display, no windowing system) — every API used was cross-checked against
  the vendored `egui`/`eframe` 0.36.1 source (exact method/variant names, not guessed), but the
  actual OS-level behavior (especially `StartDrag`/`BeginResize` under Wayland, which winit
  itself only supports on a best-effort basis) is unverified beyond that. E2E tests that assume
  OS-native title bar buttons exist via UI Automation would also need updating — not checked
  here, no Windows build available in this sandbox.

Check `features/request.md` for what's still unbuilt before assuming a feature is live —
when in doubt, `graphify query`.

## Commands

Requires Rust (stable) via rustup. `avbridge` needs `FFMPEG_DIR` set to an FFmpeg dev
build (`include/`+`lib/`); its DLLs must be on `PATH` at runtime. `core`'s `preview`
module needs GStreamer discoverable via `PKG_CONFIG_PATH`. `core`'s `whisper-rs` dependency
needs `LIBCLANG_PATH` pointing at a libclang install (bindgen) and a build directory that
isn't under `%TEMP%` on Windows (MSVC's FileTracker fails there — `FTK1011`).

**Ubuntu 24.04's packaged FFmpeg (6.1.1, via `apt install libavformat-dev` etc.) is too old
to build `avbridge`.** `csrc/filters.c` calls `avcodec_get_supported_config`/
`AV_CODEC_CONFIG_SAMPLE_FORMAT`/`AV_CODEC_CONFIG_SAMPLE_RATE`, added in FFmpeg 7.1 — a plain
`apt`-installed FFmpeg dev build on noble fails with "undeclared identifier" on that file
specifically, before reaching any of this project's own logic. A sandboxed session without
network access to fetch a newer FFmpeg build (or without access to `cdn.pyke.io` for `ort`'s
prebuilt ONNX Runtime binaries — set `ORT_SKIP_DOWNLOAD=1` to defer *that* failure past `cargo
check`, same idea as the GPU-encoder note below) can't get a full `cargo check`/`build`/`test`
to pass — same category of pre-existing, machine-specific build gap as the GPU encoder note
below, not a code regression. `cargo fmt --check` still works (parses each file independently,
no dependency build needed) and is a reasonable sanity check when a full build isn't possible.
For `avbridge/csrc/*.c` changes specifically, `gcc -fsyntax-only -I <ffmpeg-include-dir>
-Wall -Wextra <file>.c` run per-file from `crates/avbridge/csrc/` is a real (if partial)
compiler check that still works even when the whole crate can't build — it only needs the
headers *that file* includes, so it stays clean for every file except `filters.c` itself even
on Ubuntu's too-old packaged FFmpeg. It won't catch link-time or runtime/filtergraph-semantic
issues, but it does catch real syntax/type errors no amount of manual review guarantees.

**Do not remove `avbridge/build.rs`'s import-lib-renaming step.** GStreamer bundles its
own FFmpeg (gst-libav) with identically named import libs — `build.rs` copies them into
`OUT_DIR` under unique names to prevent silent ABI-mismatch linking at runtime.

```bash
make build     # debug build, whole workspace
make run       # run the GUI app (debug)
make test      # run every crate's test suite
make test-core # cargo test -p core only
make test-e2e  # pytest + pywinauto against a built target/debug/ui.exe
make bench     # criterion benchmarks -> target/criterion/report/index.html
make fmt       # cargo fmt --all
make lint      # cargo clippy --workspace --all-targets
make check     # cargo check --workspace --all-targets (fast compile-only loop)
```

Single test: `cargo test -p core --test probe_test measure_loudness` or
`cargo test -p ui i18n::tests`.

**End-to-end tests** (`e2e/`) drive the real `ui.exe` via Windows UI Automation
(`pytest` + `pywinauto`, `backend="uia"`). One-time setup:
`python -m pip install -r e2e/requirements.txt`. **Use a real Python, not a Windows Store
alias** — a Store-packaged `python.exe` carries MSIX package identity that breaks DLL
loading for spawned child processes (`ui.exe` crashes with `STATUS_DLL_NOT_FOUND` even with
`PATH` set correctly), while the identical binary launched directly from PowerShell works
fine. Needs a debug build first and FFmpeg/GStreamer DLL dirs on `PATH` (`make test-e2e`
exports both). The `oca_window` fixture calls `set_focus()` before yielding — without it,
synthetic clicks land on whatever window has focus.

## Architecture

Three-crate split, enforced by dependency direction: `avbridge` → `core` → `ui`.

- **`avbridge`** — thin C bridge (`csrc/bridge.c`) over libavformat/libavcodec/libavfilter/
  libavutil, called via FFI. Exposes probing, timeline export rendering, loudness
  measurement, proxy generation, and waveform extraction.
- **`core`** — project/timeline/media data model plus wrappers (`probe`, `render`,
  `loudness`, `proxy`, `waveform`), GStreamer playback pipeline (`preview`), and `.ocproj`
  save/load (`persistence`) — gzip-compressed MessagePack (struct-map mode, so
  `#[serde(default)]` still lets an older-saved project load after a new field is added).
  Locale-neutral — stores enums, never pre-formatted strings. No mock/sample data anywhere.
- **`ui`** — eframe/egui GUI (glow/OpenGL): `app.rs` holds `App` and mutation methods;
  `screens/` has one module per screen; `theme.rs` is the dark/teal palette; **all UI
  strings live in `i18n.rs`** (pt-BR and English) — never hardcode display text.

Test placement: `core` and `avbridge` have `[lib]` targets → integration tests in
`crates/<crate>/tests/`. `ui` is bin-only → unit tests in `src/<module>/<module>_test.rs`.

## Approach

- Read existing files before writing. Don't re-read unless changed.
- Thorough in reasoning, concise in output.
- Skip files over 100KB unless required.
- No sycophantic openers or closing fluff. No emojis or em-dashes.
- Do not guess APIs, versions, flags, or package names. Verify by reading code or docs.
- Write all code and commit messages in English.
- Commit using Conventional Commits format (`feat:`, `fix:`, `refactor:`, etc.).
- Keep commits small and split by crate/layer — never bundle `avbridge` (C/FFI), `core`, `ui`,
  and test changes for one feature into a single commit. Commit each layer separately, in
  dependency order (`avbridge` → `core` → `ui` → tests), even when they land in the same
  session for the same feature. A commit that only adds/changes tests for already-committed
  code gets its own `test:`-prefixed commit rather than being folded into the `feat:` commit
  it covers.

## graphify

This project has a knowledge graph at `graphify-out/` with god nodes, community structure,
and cross-file relationships.

- For codebase questions, first run `graphify query "<question>"` when `graphify-out/graph.json`
  exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"`
  for focused concepts.
- If `graphify-out/wiki/index.md` exists, use it for broad navigation.
- Read `graphify-out/GRAPH_REPORT.md` only for broad architecture review or when
  query/path/explain don't surface enough context.
- After modifying code, run `graphify update .` to keep the graph current.
