# Mandatory Rules & Definition of Done

Ported from nimble's `spec/architecture/rules-and-dod.md` / `spec/RULES.md`, adapted to oca's
own conventions (which already mostly follow this in practice — see `matrix/changelog.md`).
Read before picking up a `ROADMAP.md` item.

## Before writing code

1. **Reuse before building.** Check `performance-and-caching.md` §5 pattern first — an existing
   primitive (e.g. the `Preview` seek-and-poll sampling loop) may already do most of what a new
   feature needs.
2. **No new work in a hot path without a cache/dirty-flag plan.** Anything touched every frame
   (preview tick, scrub) needs a versioned-cache or dirty-flag story before landing — not
   "optimize later." See `performance-and-caching.md` §1–3.
3. **Mutation rule.** Timeline/project mutations should go through the project's own mutation
   methods (`Project`/`Timeline`/`ClipInstance` — `core/src/timeline.rs`, `project.rs`), never
   bypass them to poke preview/export state directly from `ui`.
4. **Resource rule.** Anything that decodes a frame off the timeline (auto-reframe, motion
   tracking, matte generation, highlight detection) should go through one shared sampling
   primitive once it exists (`performance-and-caching.md` §5), not a fourth reimplementation.
5. **No permanent fake APIs.** A stub that silently pretends to work is worse than an explicit
   error or a documented gap. If something is genuinely not implemented yet: say so in the
   matrix file, mark it `[ ]`/`[~]`, and don't claim it in UI copy or commit messages.

## Definition of done

A feature isn't done just because the UI button exists and doesn't crash. In order:

```text
API/UI implemented
    ├── Semantics defined (what does it actually do, not just its shape)
    ├── Unit tested (crate's own `tests/` dir per this repo's placement convention)
    ├── Integration tested where it crosses avbridge/core/ui
    ├── Verified against a real pipeline where possible (GStreamer/FFmpeg — not just
    │     "compiles and doesn't panic"; this repo's own strong convention already)
    ├── Error behavior verified (not just the happy path)
    ├── Performance measured, if it's a hot path (preview tick, scrub, playback)
    └── Matrix updated: flip `- [ ]` to `- [x]`/`- [~]` in the relevant spec/matrix/*.md
```

"Confirmed working end-to-end on this dev machine" vs. "implemented but unverified — no GPU/no
network/no display in this sandbox" is a real, load-bearing distinction in this codebase's own
history (`matrix/changelog.md` is full of both) — keep making that distinction explicit rather
than letting "implemented" quietly mean "verified."

## Keeping the spec in sync

Same convention as nimble:

1. Flip the checkbox in the relevant `matrix/*.md` file, one-line note + file reference.
2. Update its line in `ROADMAP.md` — matrix files are the detailed source, ROADMAP is the
   reconciled summary; they can drift, ROADMAP wins on "what's next."
3. Commit the doc update separately from the code commit (`docs: mark <item> done`), consistent
   with this repo's own "one commit per crate/layer" convention in `CLAUDE.md`.

Never overwrite a spec file wholesale — edit the specific line/section touched.

---

[← back to spec/INDEX.md](../INDEX.md)
