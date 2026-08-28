# Performance quality

> Load when: changing playback, preview, timeline painting, media processing, import/export,
> background work, caches, allocation, I/O, or when diagnosing a slowdown.
> Skip when: the change cannot affect runtime work or resource retention.

## Evidence before optimization

1. Name the workload and user-visible budget: frame time, startup, seek latency, throughput, memory,
   output size, or responsiveness.
2. Establish a baseline using a profile, benchmark, trace, telemetry, or reproducible timing.
3. Identify the dominant cost. Do not optimize code merely because it looks expensive.
4. Change one cost model at a time and compare the same workload after the change.
5. Record hardware, media characteristics, sample size, and whether the result is debug or release.

A correctness fix for an unbounded queue, cache, loop, or allocation does not need a benchmark to be
valid, but the unbounded path and imposed limit must be demonstrated.

## Hot-path rules

- No blocking I/O, decoding, probing, model loading, or long locks on the UI frame path.
- Avoid repeated allocation, parsing, formatting, regex compilation, filter-graph construction, and
  texture creation inside per-frame or per-pixel loops.
- Iterate only the visible timeline range. Quantize and share derived media work by source identity
  when the result is independent of a clip instance.
- Use the right complexity and data structure for repeated lookups. Watch for nested scans that turn
  frame or clip processing into quadratic work.
- Batch work across FFI, filesystem, decoder, and channel boundaries when batching preserves latency
  and cancellation requirements.
- Check cancellation between bounded units of expensive work, not only after the entire job.

## Caches and resource bounds

- Every cache needs a key containing all inputs that affect the result, an invalidation rule, a size
  or memory bound, and observable hit/miss behavior when the cache is important.
- Prefer versioned caches or classified dirty flags over blanket invalidation.
- Do not cache cheap work or data whose invalidation is harder to prove than recomputation.
- Bound worker pools, channels, retries, decoded frames, textures, waveforms, and queued jobs.
- Define backpressure: drop obsolete preview work, coalesce equivalent requests, or reject excess work
  explicitly. Never allow silent growth.
- Release native handles, frames, sessions, textures, and temporary files deterministically.

## Media and UI specifics

- Open reusable pipelines or model sessions once per stable workload rather than once per sample.
- Preserve source-frame and aspect-ratio correctness when reducing work; faster wrong output is a
  regression.
- Keep preview-oriented lossy shortcuts isolated from export-quality paths.
- Avoid invalidating preview for mutations that only require reseek or repaint.
- Prefer incremental or visible-range recomputation for waveforms, filmstrips, overlays, and timeline
  geometry.
- Measure release builds for CPU/GPU throughput. Debug timings are useful only for relative developer
  feedback and must be labeled as such.

## Findings that block approval

- Work proportional to total media or timeline size on every frame without an explicit small bound.
- Unbounded memory, cache, queue, worker, retry, or retained-resource growth.
- Reopening decoders, pipelines, or model sessions inside a sampling loop without evidence it is
  necessary.
- A claimed optimization without comparable evidence, or one that removes correctness checks.
- Locking or blocking work that can freeze the UI or prevent cancellation.

See `spec/architecture/performance-and-caching.md` for current project-specific cache and dirty-flag
patterns. Use Criterion benchmarks for stable computation and scenario timings or traces for media
pipelines and UI interaction.
