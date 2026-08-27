# Undo/Redo (`ROADMAP.md` P0 item 1)

## Done

`crates/core/src/undo.rs` — `UndoStack`: two bounded `Vec<Sequence>` stacks (undo/redo),
snapshot-based (not command-pattern — see the module's own doc for why `Sequence: Clone` makes
this the correct reuse-before-build choice per `RULES.md`). `push`/`undo`/`redo`/`can_undo`/
`can_redo`/`clear`. Unit tested (`crates/core/tests/undo_test.rs`, 7 tests): fresh-stack state,
undo restores the pushed snapshot, redo restores what undo moved away from, a new push after
undo clears the redo branch (real editor semantics), undo-beyond-history returns `None` without
touching redo, capacity bounds the stack by dropping the oldest entry, `clear` drops both.

**Not yet verified against a real build** — this dev sandbox has no GStreamer install
(`PKG_CONFIG_PATH` unset, matches `CLAUDE.md`'s own documented pre-existing gap for this
platform), so `cargo test -p core` can't run here. Only `cargo fmt --check` ran clean. The code
is plain safe Rust (a `Vec`-backed stack, no unsafe/FFI) — low risk, but **run
`cargo test -p core --test undo_test` on a machine with GStreamer configured before trusting
this beyond code review.**

## Wired (this pass)

Items 1-2 and 4-7 below are done. `Project::active_sequence()`/`active_sequence_mut()`
accessors added (`crates/core/src/project.rs`, next to `timeline()`/`timeline_mut()`).
`App::undo_stack: avcore::undo::UndoStack` (`crates/ui/src/app/mod.rs`, next to
`selected_clip_id`). `App::push_undo_snapshot()`/`undo()`/`redo()`/`can_undo()`/`can_redo()`
added next to `reset_sequence_context`. `Undo`/`Redo` added to `BindableAction`/`KeyBindings`
(default `Ctrl+Z`/`Ctrl+Y`), configurable in Preferences same as the other five. Toolbar's
pre-existing dead `↺`/`↻` buttons (`screens/editor/mod.rs`) now call `undo()`/`redo()`,
enabled state from `can_undo()`/`can_redo()`. `undo_stack.clear()` called from
`reset_sequence_context` (covers `select_sequence`/`add_sequence`/`duplicate_sequence`/
`delete_sequence`) and from `App::open_project`. Unit tests in `app_test.rs`
(`undo_after_split_at_playhead_restores_the_unsplit_clip`, `undo_is_a_no_op_with_empty_history`,
`selecting_a_sequence_clears_undo_history_from_the_previous_one`) plus the 7 `core` tests —
**all verified passing** on a machine with GStreamer configured (`cargo test -p core --test
undo_test`, `cargo test -p ui undo`), closing the "not verified" gap from the primitive-only
pass.

## Not done — remaining call sites (item 3)

`push_undo_snapshot()` is wired into `timeline_ops.rs`'s single-shot mutations
(`add_asset_to_timeline[_at]`, `split_at_playhead`, `delete_selected_clip`,
`paste_clip_at_playhead`, `merge_into_composite`, `paste_selected_clip_formatting`,
`add_video_track`, `add_text_track`/`add_text_clip`, `add_shape_track`/`add_shape_clip`/
`finish_drawing_custom_shape`) and into the timeline strip's trim/move drags
(`screens/editor/timeline_panel.rs`, pushed once on `drag_started()` — not inside
`trim_clip_start`/`trim_clip_end`/`move_clip`/`move_clip_with_group`/`move_clip_to_track`
themselves, since those are called every frame of a drag and a naive per-call push would
record one undo step per frame instead of one per drag).

**Not yet covered: effect-property setters in `crates/ui/src/app/clip_props.rs`** (gain, crop,
mask, color/vignette/blur/etc., keyframe add/remove) and their sliders in
`screens/editor/properties_panel.rs`. These all funnel through `App::with_selected_clip_mut`,
but that dispatch point is called every frame while a slider is being dragged — pushing a
snapshot inside it would spam one undo entry per frame, the same problem the trim/move drags
avoid via `drag_started()`. Fixing this properly means the same drag-start-vs-continuous-drag
split egui's `Response` gives `timeline_panel.rs` for free, but threaded through ~20+ individual
slider/checkbox call sites in `properties_panel.rs` rather than one shared loop — a
materially bigger, more error-prone change than the rest of this wiring pass, left for a
follow-up rather than rushed. `toggle_track_visibility` also stays unwired — a display toggle,
not timeline content, same category `panel_layout` writes are excluded for.

## Design note: why per-sequence, not per-project

Snapshotting the whole `Project` (media library, all sequences, display metadata) on every
timeline edit would work but is heavier than necessary and conflates two different kinds of
"undo" a user would expect to be separate — undoing a clip move on sequence tab A shouldn't be
affected by having imported a new asset while working on tab B in between. Scoping to the
active `Sequence` keeps undo history meaningful per timeline, matches how every surveyed editor
(`matrix/competitor-parity.md`) scopes undo to the active edit context, not the whole project.

---

[← back to spec/INDEX.md](../INDEX.md)
