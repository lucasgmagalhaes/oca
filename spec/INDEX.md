# spec/ — Start Here

Split out of `CLAUDE.md`'s old single giant "Status" section (2026-08-27), same pattern used
in nimble (sibling project): a task only needs the 2-3 small files relevant to it, not one
huge file.

## Reading order

1. **[ROADMAP.md](ROADMAP.md)** — the actionable queue. Pick your next item here first.
2. **[RULES.md](RULES.md)** — mandatory rules + definition of done. Short, read every time.
3. The specific `matrix/*.md` (current status + file references) and/or `architecture/*.md`
   (the *how*/*why*) file(s) ROADMAP.md points you to.

Do not read every file here for one task. Read `matrix/changelog.md` only when you need the
historical "why was it built this way" detail (verification caveats, real bugs found, empirical
discoveries) — it's a narrative log of already-completed work, not a task list.

## Directory map

```text
spec/
├── INDEX.md                          ← you are here
├── ROADMAP.md                        ← actionable P0-P5 queue, START HERE
├── RULES.md                          ← mandatory rules + definition of done
├── matrix/                           ← WHAT is done/needed, per area
│   ├── engine.md                     (avbridge: probe/export/loudness/proxy/GPU encode)
│   ├── timeline-and-editing.md       (timeline model, editing ops, sequences, undo/snap gaps)
│   ├── effects-and-color.md          (visual effects, keyframes, masks, transitions, LUTs)
│   ├── ai-features.md                (bg removal, auto-reframe, motion tracking, TTS, Whisper)
│   ├── preview-pipeline.md           (GStreamer real-time playback/compositing)
│   ├── robustness.md                 (autosave, crash recovery, logs, prefs, export queue)
│   ├── performance.md                (proxy, filmstrip, hw decode, telemetry)
│   ├── packaging.md                  (installers, bundling, auto-update)
│   ├── competitor-parity.md          (real feature research: CapCut/DaVinci/Premiere/FCP gaps)
│   └── changelog.md                  (large historical log — read only for "why", not "what's next")
└── architecture/                     ← HOW to build it (principles, not per-feature status)
    ├── differentiators.md            (D1-D7: the proposed differentiator feature specs)
    ├── performance-and-caching.md    (dirty flags, versioned cache, hot-path rules — from nimble)
    ├── undo-redo.md                  (P0 item 1: what's done, what's left, why per-sequence)
    └── rules-and-dod.md              (full text behind RULES.md's summary)
```

## Keeping this in sync

When you finish a `ROADMAP.md` item:

1. Flip its `- [ ]` to `- [x]` (or `[~]`) in the relevant `matrix/*.md` file, one-line note +
   file reference.
2. Update its line in `ROADMAP.md` too — matrix files are the detailed source, ROADMAP is the
   reconciled summary; they can drift, ROADMAP wins on "what's next."
3. Doc-update commit, separate from the code commit (`docs: mark <item> done`).

Never overwrite a spec file wholesale — edit the specific line/section touched.

---

Also relevant, outside `spec/`: `CLAUDE.md` (repo-wide conventions, crate architecture, build
gotchas), `AGENTS.md` (same content, Codex-facing mirror), `features/request.md` (the original
phased plan this queue picks up after — kept as-is, historical).
