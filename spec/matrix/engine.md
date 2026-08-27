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
- [~] GPU-accelerated encode (NVENC/Quick Sync/AMF/VAAPI, CPU fallback) — `gpu_encoder.c`.
      **Hardware success unverified** — no dev machine has NVENC/AMF/VAAPI hardware; this dev
      machine's FFmpeg build also lacks `libopenh264`/`h264_qsv` entirely, so every test that
      reaches `avcodec_open2` fails here specifically (pre-existing build gap, not a regression).
- [x] Python runtime + yt-dlp bridge (`crates/ytbridge`) for YouTube download — fully bundled
      (Windows confirmed working end-to-end with `PATH` stripped; Linux branch written,
      unverified on a real Linux build).

## Known gap

- [ ] Series-level/batch loudness matching (D3, `architecture/differentiators.md`) — per-clip
      normalization exists, batch-level consistency across an export queue doesn't.

---

[← back to spec/INDEX.md](../INDEX.md)
