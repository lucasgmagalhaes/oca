# Robustness

Autosave, crash recovery, logging, preferences, key bindings, export queue reliability.
Detail: `matrix/changelog.md` Fase 5/6.

- [x] Debounced autosave (2s debounce, 30s hard ceiling), background thread, separate
      recovery file from manual save — restore-on-reopen modal after an abnormal close.
- [x] Crash detection + panic hook — panics captured with stack trace, saved locally (also
      feeds telemetry, see `matrix/performance.md`).
- [x] Configurable key bindings (Preferences), including `Ctrl+O` for opacity marker
      (`BindableAction`/`KeyBindings` — fifth entry, rebindable like the other four).
- [x] Structured logging (`tracing`, rotating file, enough context to reproduce an issue).
- [x] Central error handling — `Result<T, E>` propagated to one handler that decides
      warn-user / auto-recover / log-and-continue, not silent failure at any risk point.
- [x] Preferences modal (centered, in-window, not an OS window) covering audio profiles,
      export worker count, default output folder, default panel layout.
- [x] Export queue: reordering, cooperative pause/resume (`Condvar`-based checkpoint, not
      instant — native post-passes like text/shape/audio don't emit progress callbacks, so a
      pause requested late may show briefly before the running post-pass finishes), persists
      across sessions (`queue.ocqueue`, gzip MessagePack, atomic write + migration from old
      `queue.json`).
- [x] Output-folder overwrite/rename/cancel prompt on a filename collision.

## Known gaps

- [x] Undo/redo — `core::undo::UndoStack` and its UI integration cover timeline and effect
      mutations with coalesced drag snapshots; see `matrix/timeline-and-editing.md` and ROADMAP
      P0 item 1 for the exact coverage and verification.

---

[← back to spec/INDEX.md](../INDEX.md)
