# egui Implementation Guide

Ground truth against `crates/ui/src/app/mod.rs`, `crates/ui/src/screens/`, `theme.rs`, `i18n.rs`.

## Structure — real shape

`impl eframe::App for App` (`app/mod.rs`) is a thin dispatcher, not a monolithic `update()`.
`ui()` does, in order: drain ~12 `pump_*` background-job queues, handle a couple of one-off
frame-level concerns (prefs-modal-closed save, crash sentinel), set repaint cadence, then
(if not fullscreen preview) `nav_rail::show` → `ensure_active_project()` → `breadcrumb::show` →
`match self.screen { ... }` inside `egui::CentralPanel` dispatching to `screens::{home, editor,
library, sound_library, queue}::show` → ~18 `self.show_*_modal(...)` calls → `show_toasts`.

Every screen module exposes `pub fn show(app: &mut App, ui: &mut egui::Ui)` — a uniform,
non-trait convention (no `impl Screen for X`). Follow it for a new screen rather than introducing
a trait-based abstraction.

`app/` (~30 files) holds state + `impl App` business logic, one file per feature area
(`export.rs`, `timeline_ops.rs`, `preview.rs`, `transcribe.rs`, `markers.rs`, ...) — **no
drawing** happens here. `screens/` holds drawing and delegates mutation to `app/` methods. Keep
new features split the same way: state/logic in `app/<feature>.rs`, drawing in `screens/` (or
`screens/editor/timeline_panel/draw.rs` if it's a timeline visual).

## State — real categories, with an important warning

`App` (`app/mod.rs`) had grown to **122 flat fields with no substructure** before a documented
cleanup introduced per-feature sub-structs (`PreviewState`, `TranscribeState`,
`AutoReframeState`, `MotionTrackingState`, `SceneCutDetectionState`, `MatteGenerationState`,
`TtsState`, `YoutubeDownloadState`, `ImportState`, `TelemetryState`, `ThumbnailState`, and more).
**Add new feature state to a new or existing sub-struct, not as another flat field on `App`** —
the flat-field growth is a real, previously-fixed problem, not a hypothetical one.

Real categories:
- **Persistent project state**: `projects: Vec<Project>`, `active_project`, `undo_stack` — owned
  by `avcore` types, cloned into undo snapshots.
- **Editor session state**: `screen`, `tool`, `selected_clip_id`/`multi_selected_clip_ids`,
  `timeline_px_per_sec`, panel widths, `clipboard_clip` — lives on `App` directly or `PrefsState`,
  never serialized into `.ocproj`.
- **Transient interaction state**: `pending_asset_drop`, `drawing_shape_points`,
  `undo_drag_active` — short-lived, frame-to-frame or gesture-to-gesture.
- **Async-channel state**: each feature sub-struct holds its own
  `UnboundedSender`/`UnboundedReceiver` pair plus in-flight job ids (`render_tx/rx`,
  `active_renders`, etc).

Do not serialize hover, drag preview, or transient interaction state as project data — confirmed
true throughout; `Project`'s `#[serde(skip)]` fields (`file_path`, derived caches) are the only
persisted-struct fields excluded from serialization, and none of them are UI interaction state.

## Custom widgets

No custom `egui::Widget` trait impls exist in this codebase — painting is done imperatively
against `ui.painter()`/an allocated `Painter`, most visibly in `timeline_panel/draw.rs`. Reusable
non-timeline UI pieces instead live in `crates/ui/src/components/` (`frame.rs`, `section.rs`,
`property.rs`, `combo.rs`, `tag.rs`) as plain functions taking `&mut egui::Ui`, not `Widget`
impls. Match this convention — a plain drawing function, not a `Widget` — unless there's a
concrete reason (e.g. needing `Widget::ui`'s return-`Response` composability) to differ.

Prefer custom painting for the timeline specifically (already the case — see
`domain/TIMELINE.md`); standard widgets/layout are fine and used everywhere else (`screens/home`,
`prefs`, `library`, properties panel).

## Layout

Panels/areas are used intentionally: `screens/editor/mod.rs` lays out a toolbar, a three-column
row (media library / preview / properties), then a timeline strip below — not deeply nested
groups for spacing. Centralized theme/spacing lives in `theme.rs` (see below); reuse its tokens
rather than hardcoding colors or spacing values in a new screen.

## Theme (`crates/ui/src/theme.rs`, ~80 lines)

`Color32` constants matching a dark/teal palette: `BG`, `SURFACE`, `SURFACE_2`, `BORDER`,
`TEXT_PRIMARY`/`TEXT_SECONDARY`/`TEXT_MUTED`, `ACCENT`, `ACCENT_TINT`, `ACCENT_2`, `ERROR`,
`ERROR_TINT`. `theme::apply(ctx)` builds an egui `Visuals` (panel/window fill, widget states for
noninteractive/inactive/hovered/active, corner radii, stroke) and sets global spacing
(`item_spacing`, `button_padding`) via `ctx.all_styles_mut`. Any new color/spacing need should be
added as a named token here, not an inline `Color32::from_rgb(...)` at the call site.

## i18n (`crates/ui/src/i18n.rs`, ~750 lines)

The `text_catalog!` macro generates a `Text` enum, one variant per string, from
`Variant: pt_br = "...", en = "...";` entries, plus `Text::tr(self, locale: Locale) -> &'static
str`. The module's own doc comment states the rule explicitly: **all translatable UI text lives
here, not in screen modules; `core`/`avcore` stays locale-neutral.** A few functions
(`recency_label`, `track_summary`, `job_status_label`, `job_detail_line`) handle
pluralized/interpolated strings outside the flat catalog — follow that pattern for anything
needing runtime interpolation rather than string-formatting inline at the call site. `Locale` is
`PtBr`/`En`, switchable at runtime, persisted in `PrefsState`. Never hardcode a display string in
a `screens/` or `app/` file.

## Input

Most keyboard shortcuts (Ctrl+S, split, delete, copy/cut/paste, undo/redo) are handled inline near
the top of `screens/editor/mod.rs::show`, before delegating to submodules — check there first
before adding a new shortcut handler elsewhere, to avoid two competing key-handling sites.
Pointer capture follows standard egui `Sense`/`Response` semantics; the timeline's
"collect-during-loop, apply-after" pattern (see `domain/TIMELINE.md`) is specifically how this
codebase avoids parent/child interaction conflicts during multi-clip drag/trim.

## Repaint

`app/mod.rs`'s `ui()`: `request_repaint()` (every frame, uncapped) only while
`preview_state.preview_playing`; otherwise `request_repaint_after(200ms)`. One confirmed
exception: `screens/editor/mod.rs`'s `fullscreen_preview_overlay` requests continuous repaint
unconditionally while the overlay is showing (justified in its own comment by the idle-fade timer
needing a steady cadence even when paused) — a known, accepted minor cost, not a bug to "fix" by
reflex. Match the playing-gated pattern for any new continuous-repaint need; don't request
continuous repaint for static workspace state.
