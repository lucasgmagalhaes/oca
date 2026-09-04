# Engine — avbridge (probe/export/loudness/proxy/GPU encode)

Native FFI bridge over libavformat/libavcodec/libavfilter/libavutil (`crates/avbridge`,
`csrc/*.c`) — no subprocess, no `ffprobe`/`ffmpeg` on PATH. Detail/verification caveats:
`matrix/changelog.md` Fase 1-3, Fase 7's GPU encode note, Fase 8's YouTube-download runtime note.

- [x] Metadata probe (codec, bitrate, LUFS, fps, resolution) — `probe.c`/`avcore::probe`.
- [x] Timeline export render (single-track) — `export.c`/`avcore::render`.
- [x] Multi-track/multi-layer timeline export — `timeline_export.c`, `timeline_export_multi.c`
      (dynamic N-layer compositor, sequential `overlay` nodes in track order).
- [x] Loudness measurement (LUFS) — `loudness.c`/`avcore::loudness`.
- [x] Two-pass loudness normalization + noise reduction (`afftdn`) + true-peak limiter
      (`alimiter`) — applied automatically before export, all three unnamed-filter-stage in the
      audio chain (`export.c`, `timeline_export.c`, `timeline_export_multi.c`).
- [x] Editing proxy generation — `proxy.c`/`avcore::proxy`, resolution now user-selectable
      (`PreviewQuality` Low/Medium/High = 360p/480p/720p).
- [x] Waveform extraction — `waveform.c`/`avcore::waveform`.
- [x] Matte video encoding (background-removal alpha) — `matte_encode.c`.
- [x] Text/shape overlay compositing (native mux pass) — `text_overlay.c`, `shape_overlay.c`.
- [x] Native audio mix/mux (multi-branch `amix`, stream-copy final mux) — `audio_mix.c`.
- [~] GPU-accelerated encode (NVENC/Quick Sync/AMF/VAAPI/VideoToolbox, CPU fallback) —
      `gpu_encoder.c`.
      **NVENC hardware success now confirmed** on a real NVIDIA RTX 4070 — see `ROADMAP.md` P4
      item 19 for the full verification writeup (direct `ffmpeg` CLI ground-truth + a new
      additive `av_log` line in `open_video_encoder` proving oca's own code path opened
      `h264_nvenc`, not just that a file happened to come out). Quick Sync/AMF still unverified
      on the positive path (no such hardware on any dev machine checked so far); VAAPI unverified
      at all (Linux-only, `#ifdef __linux__`, no Linux+VAAPI dev machine available yet).
      **VideoToolbox hardware success is confirmed** on an Apple M4 MacBook Pro with macOS 26.6.2
      and Homebrew FFmpeg 9.0: the direct CLI encoded H.264 and oca's focused native timeline
      test logged `h264_videotoolbox (hardware)`. The UI preference persists as
      `video_toolbox`; `Auto` tries it on macOS after the existing cross-platform candidates.
- [x] Python runtime + yt-dlp bridge (`crates/ytbridge`) for YouTube download — fully bundled
      (Windows confirmed working end-to-end with `PATH` stripped; Linux branch written,
      unverified on a real Linux build).
- [x] Series-level/batch loudness matching (D3, `architecture/differentiators.md`) —
      `App::match_loudness_across_queued_jobs` (`ui/src/app/export.rs`) sets one target LUFS
      across every `Queued` export-queue job at once, from a button row on the Fila screen
      (`ROADMAP.md` P2 item 12). Orchestration over the existing per-job `target_lufs` field,
      no new normalization DSP.
- [x] Lightweight collaboration bundle (D7, `architecture/differentiators.md`, `ROADMAP.md` P3
      item 14) — `avcore::collab_bundle::{export_collab_bundle, import_collab_bundle}` package a
      project's `.ocproj` plus its already-generated editing proxies into one portable `.zip`,
      never the source media. Pure packaging over the existing proxy cache
      (`proxy::cache_dir_for_project`) and `.ocproj` framing (`persistence`) — no new transcode
      or serialization format. `ui`: Editor toolbar export button, Início import button.

---

[← back to spec/INDEX.md](../INDEX.md)
