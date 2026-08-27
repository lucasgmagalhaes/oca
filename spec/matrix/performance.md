# Performance

Detail: `matrix/changelog.md` Fase 7. Architecture-level caching/invalidation patterns (not
yet applied here — see the gaps below) live in `architecture/performance-and-caching.md`.

- [x] Editing proxy — user-selectable resolution (360p/480p/720p cap), baked into the proxy
      filename so a preference change can't collide with a stale-resolution proxy. No
      retroactive regeneration: an asset imported before a preference change keeps its old proxy.
- [x] Zoom-adaptive, bounded timeline filmstrip loading — samples visible tiles at the asset's
      own frame rate (not a fixed one-second bucket), ≤16 pending extraction pipelines, 512-entry
      LRU texture cache (~18 MiB worst case), independently bounded failed-key suppression.
- [x] Hardware-accelerated preview decode — see `matrix/preview-pipeline.md`.
- [x] Release build: `opt-level=3`, LTO, symbol stripping.
- [x] Runtime telemetry — import/export duration, preview frame time, CPU/RAM sampling
      (`sysinfo`-backed, 30s interval, own background thread), JSON-lines local file with
      size-based rotation (10 MiB), on-device only, toggleable in Preferences.

## Known gaps

- [ ] GPU usage telemetry — `sysinfo` has no cross-platform GPU reader; a vendor-specific one
      (NVML/etc.) is hardware-dependent, same "hard wall" class as the GPU encoder ladder.
- [ ] Versioned filter-graph cache (`architecture/performance-and-caching.md` §2) — the
      timeline→avfilter-graph resolution path rebuilds from scratch on every call today.
- [ ] Dirty-flag mutation classification (`architecture/performance-and-caching.md` §1, §6) —
      not applied to timeline mutations yet; worth confirming whether every edit currently
      forces a full preview-pipeline reopen regardless of what actually changed.

---

[← back to spec/INDEX.md](../INDEX.md)
