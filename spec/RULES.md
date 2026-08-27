# Mandatory Rules (read before implementing anything from ROADMAP.md)

Full detail: [architecture/rules-and-dod.md](architecture/rules-and-dod.md). This file is the
short version.

1. **Reuse before building.** Check whether an existing primitive already does the job (the
   `Preview` seek-and-poll sampler is the clearest current example) before adding a new one.
2. **No hot-path work without a cache/dirty-flag story.** Anything that runs every preview
   frame/scrub needs one before it lands, not after.
3. **Go through the project's own mutation methods.** Never bypass `Project`/`Timeline`/
   `ClipInstance` to poke preview/export state directly.
4. **No permanent fake APIs.** An unimplemented feature says so — in the matrix file, not
   silently pretended in the UI or commit message.
5. **Verify against a real pipeline where possible.** This codebase's own strong existing
   convention — "confirmed working end-to-end" vs. "implemented, unverified" is a real,
   load-bearing distinction here (`matrix/changelog.md`). Keep it.

## Definition of done (short form)

API/UI implemented → semantics defined → unit tested → integration tested across
avbridge/core/ui where relevant → verified against a real pipeline where possible → error
behavior verified → performance measured if hot-path → matrix updated.

Commits: one per crate/layer, in dependency order (avbridge → core → ui → tests), per
`CLAUDE.md`'s existing convention — this didn't change.

---

[← back to spec/INDEX.md](INDEX.md)
