# Timeline & Editing

Project/timeline data model (`core/src/timeline.rs`, `project.rs`) plus the editing
interactions in `ui`. Detail: `features/request.md` Fase 3, `matrix/changelog.md` Fase 5/6's
sequence-management/copy-paste entries.

## Done

- [x] Custom timeline widget: per-frame filmstrip (zoom-adaptive, bounded — see
      `matrix/performance.md`) + synced waveform track.
- [x] Select / split / trim / drag between tracks.
- [x] Preview scrubbing synced to playhead.
- [x] Zoom (`Ctrl`+scroll).
- [x] Multiple sequence tabs per project, each own timeline + export defaults — full lifecycle:
      add/select/rename/duplicate/delete (confirmation modal)/reorder (drag or context menu).
      `Project::duplicate_sequence`/`remove_sequence`/`move_sequence` own the invariants (always
      ≥1 sequence, deletion selects nearest neighbor, reorder tracks by stable id).
- [x] Resizable UI panels (library/preview/properties/timeline), layout savable per-user or
      per-project (`PanelLayout`, `LayoutScope`).
- [x] Composite blocks (merge selected clips into one movable/cuttable unit) — toolbar action
      + context-menu action (`"Mesclar em bloco composto"`).
- [x] Copy/paste within and across sequence tabs — preserves composite-block membership
      (`App::copy_selected_clip` captures the whole `composite_id` group, not just the clicked
      clip).
- [x] Context menu mirrors toolbar shortcut actions.
- [x] Copy/paste of just formatting (`Ctrl+Shift+C`/`V`) — effects/config without duplicating
      the clip.
- [x] Multiple overlapping video/audio layers (not just sequential tracks) — base for overlays,
      PiP, layer templates.
- [x] Layer transform (position/size) — drag + resize handles on preview, or numeric entry;
      both stay in sync.
- [x] Layer templates — save a configured camera/webcam/background layer group, reapply to a
      new short asking only for source clips per layer.

## Known gaps (not found anywhere in the codebase — confirm before assuming, but no evidence of
## either in `graphify query "undo redo history stack snapping magnetic snap"`)

- [ ] **Undo/redo.** No command-history/undo-stack type found anywhere. Table stakes for any
      timeline editor — see `ROADMAP.md` P0.
- [ ] **Magnetic snap** while dragging (to playhead, other clip edges, markers). See
      `ROADMAP.md` P0 — also a dependency of D5 (beat-aligned cut snapping).
- [ ] Review/comment markers on the timeline (plain note at a point — not to be confused with
      opacity-keyframe markers).

---

[← back to spec/INDEX.md](../INDEX.md)
