# AI Features

Background removal, auto-reframe, motion tracking, text-to-speech, subtitles/Whisper. All
models ship under `resources/models/` — no runtime downloader anywhere in this group. Detail:
`matrix/changelog.md` Fase 4 (background removal / auto-reframe / motion tracking / TTS
sub-sections).

- [x] **AI background removal** — MODNet (ONNX, Apache-2.0) via `ort`, per-frame alpha matte
      (`avcore::background_removal::segment_person`). Matte generation samples across a clip's
      trim range (`SAMPLES_PER_SEC = 2.0`, coarser than clip fps — export compositor tolerates
      by holding last known frame), encodes to H.264 (`matte_encode.c`), export- and
      preview-wired (`alphacombine`/`alphamerge`). Confirmed working end-to-end (real inference).
- [x] **Auto-reframe** (static crop only, not animated) — UltraFace RFB-320 (ONNX, MIT) face
      detection, falls back to centered crop with a toast if no face found.
      `compute_reframe_crop()` pure geometry, fully unit tested. Confirmed working end-to-end.
- [x] **Motion tracking** — plain fixed-template block matching (SAD against first frame,
      no ML), converts tracked path into `position_keyframes` delta. Region now user-editable:
      numeric entry (center/width/height/search-radius) *and* drag-on-preview picker. Fully
      unit tested including off-center starting region.
- [x] **Text-to-speech** — espeak-rs (phonemizer) + Piper VITS ONNX (pt-BR `faber` voice,
      MIT). Confirmed real synthesis, non-silent output.
- [x] **Whisper subtitles** — Base model bundled, live word-highlight timing shared between
      preview/export layout.
- [x] Standalone `.srt` export (`avcore::export_srt`) — separate from the embedded-overlay
      subtitle path, which happens on every export regardless.

## Known gaps

- [ ] Highlight detection from audio spikes (D2, `architecture/differentiators.md`) — no ML,
      reuses waveform + loudness measurement already built.
- [ ] Scene-cut chapter-marker detection (D4) — piggybacks on frames already decoded during
      import/proxy.
- [ ] Voice-clone TTS beyond the single bundled voice — explicitly out of scope for now, see
      `architecture/differentiators.md`.

---

[← back to spec/INDEX.md](../INDEX.md)
