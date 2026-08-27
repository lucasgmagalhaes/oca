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
- [x] **Undo/redo.** `core::undo::UndoStack` (snapshot-based) wired into `ui` — `Ctrl+Z`/`Ctrl+Y`,
      toolbar buttons, covers clip add/split/delete/cut/copy/paste, trim/move drags, track/text/
      shape-track add, composite merge, paste-formatting, text-color-modal confirm, and every
      effect-property slider/keyframe editor (drag-coalesced into one undo step per drag, not
      one per frame). See `architecture/undo-redo.md`, `ROADMAP.md` P0.
- [x] **Magnetic snap** while dragging — playhead, clip trim (either edge), clip body move
      (single or composite group) all snap to the nearest other clip's edge or the playhead
      (`Alt` to disable). Timeline markers aren't a target — no markers feature exists yet
      (P2 item 9). See `ROADMAP.md` P0.
- [x] **Review/comment markers (P2 item 9).** `avcore::timeline::Marker`/`MarkerKind`
      (Standard/ToDo/Chapter — Final Cut Pro's typed-marker model, not just a plain note; `ToDo`
      tracks a `completed` flag) on `Timeline::markers`, `#[serde(default)]` so an older-saved
      project still loads. `Timeline::add_marker`/`remove_marker`/`marker_mut`/`markers_sorted`
      own the id-assignment/mutation invariants. `ui`: a searchable Timeline Index panel
      (`App::show_timeline_index_panel`, toolbar's "🏷 Marcadores" toggle) — text search over
      labels, per-marker kind picker, ToDo-complete checkbox, click-timestamp-to-seek, inline
      label editing, add/remove. **Not done**: magnetic snap (P0 item 2) doesn't treat markers
      as a snap target yet — that doc's own note said "revisit when it lands," this is the
      revisit-later follow-up, not silently included here. No ruler tick-mark rendering on the
      timeline strip itself either — the Timeline Index panel is the only way to see/navigate
      markers today.

---

[← back to spec/INDEX.md](../INDEX.md)
