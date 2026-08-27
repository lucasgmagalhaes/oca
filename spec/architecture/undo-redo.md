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

## Not done — next steps

1. **Wire `App::undo_stack: UndoStack`** (`crates/ui/src/app/mod.rs`, next to `PrefsState`).
2. **`App::push_undo_snapshot()`** — clones `self.active_project().active_sequence()`'s
   `Sequence` (need a `Project::active_sequence()` accessor if one doesn't already exist
   alongside `active_project()`/`active_project_mut()`) and calls `self.undo_stack.push(...)`.
3. **Call it before every timeline-mutating operation** in `crates/ui/src/app/timeline_ops.rs`
   — `move_clip`, `trim_start`/`trim_end`, `split_clip_at`, `apply_formatting`, track add/
   remove, keyframe add/remove, effect property changes. This is the bulk of the remaining
   work: audit every call site that calls `active_project_mut()` for a timeline edit and add
   the snapshot push immediately before it. Not every `active_project_mut()` call needs one —
   only ones that mutate the *active sequence's timeline*, not e.g. panel-layout writes.
4. **`App::undo()`/`App::redo()`** — read the active sequence, call `undo_stack.undo(current)`/
   `.redo(current)`, replace the active sequence's `Sequence` with the result if `Some`.
   Must also invalidate whatever cached preview state (`App::current_preview_*`,
   `App::ensure_preview_loaded`'s reopen-detection ids) assumes the timeline hasn't changed
   underneath it — same reopen path an external edit would trigger.
5. **`App::undo_stack.clear()`** on every sequence switch (`reset_sequence_context`, per
   `matrix/timeline-and-editing.md`'s existing sequence-tab-management entry) and on
   project switch/open. History from one sequence context is meaningless applied to another.
6. **Key bindings** — add `Undo`/`Redo` to `BindableAction` (`crates/ui/src/app/mod.rs`) and
   `KeyBindings` (default `Ctrl+Z`/`Ctrl+Y` or `Ctrl+Shift+Z`), following the exact pattern
   `AddOpacityMarker` already established (`#[serde(default = "...")]` on the new
   `KeyBindings` field so an older saved `prefs.oc` still loads).
7. **UI affordance** — toolbar undo/redo buttons reflecting `can_undo()`/`can_redo()` (disabled
   state when empty), not just the key binding.
8. **Update `spec/matrix/timeline-and-editing.md`** — flip the undo/redo gap entry to `[~]`
   once wired, `[x]` once tested end to end through the UI.

## Design note: why per-sequence, not per-project

Snapshotting the whole `Project` (media library, all sequences, display metadata) on every
timeline edit would work but is heavier than necessary and conflates two different kinds of
"undo" a user would expect to be separate — undoing a clip move on sequence tab A shouldn't be
affected by having imported a new asset while working on tab B in between. Scoping to the
active `Sequence` keeps undo history meaningful per timeline, matches how every surveyed editor
(`matrix/competitor-parity.md`) scopes undo to the active edit context, not the whole project.

---

[← back to spec/INDEX.md](../INDEX.md)
