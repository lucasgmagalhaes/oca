# Core Workflows

## Import to timeline
1. User imports media.
2. Asset appears in media browser.
3. User previews or selects asset.
4. User drags or inserts it into a compatible timeline location.
5. Timeline shows placement preview and snapping.
6. Operation commits as an undoable command.

## Clip editing
1. User selects a clip (`App::selected_clip_id`, transient UI state, not persisted).
2. Timeline highlights selection.
3. Inspector (`screens/editor/properties_panel/`) becomes context-aware — already branches by
   clip type (video/`ClipInstance`, `TextClip`, `ShapeClip`).
4. Relevant controls become available.
5. Dragging/trimming previews changes via transient state (`ClipDrag`, `trim_requests`) collected
   during the timeline's per-frame draw loop.
6. Final action commits as one `App::push_undo_snapshot()` call after the draw loop — see
   `domain/TIMELINE.md`.

## Playback
1. User changes playhead or starts playback.
2. Preview reflects temporal position.
3. Timeline remains understandable during movement.
4. Expensive decoding/rendering must not freeze interaction.

## Property editing
1. Select object.
2. Inspector identifies object and editable properties.
3. Values update through direct controls.
4. Keyframe behavior is explicit when animation is available.
5. Changes integrate with undo/redo.

## Export
Export is a separate task context (`screens/queue.rs`, backed by the `.ocqueue` persisted export
queue and `app/export.rs`'s thread+channel orchestration into `avbridge`'s FFmpeg pipeline). Do
not overwhelm the editing workspace with permanent export controls.

## Reviewable batch suggestions (silence review, transcript-proposal review)
A real, deliberate exception to "prefer direct manipulation over modals": automatic detection
features (silence gaps, speech-edit proposals) stage their results in an `App` field
(`silence_review`/`transcript_review`) and show a checkbox-per-item `egui::Modal`, defaulting
every item to accepted, applying only what's still checked as **one** undo step on "Apply". Use
this same shape for a new automatic-suggestion feature rather than either auto-applying silently
or inventing a different review UI.
