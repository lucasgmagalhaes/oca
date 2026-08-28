# Built-in Font Catalog

**Status:** proposed

**Roadmap ID:** FONT-01

**Priority:** may ship alongside CF-01; required before CF-07 motion-graphics templates

**Target:** 43 built-in families, 51 upstream font binaries, available offline on every supported
platform

## Problem

oca already renders text deterministically with bundled fonts, but the catalog is too small for a
video editor. The current implementation has:

- 6 families and 9 TTF files under `crates/core/assets/fonts`;
- approximately 2.4 MiB of uncompressed font assets;
- one `TextFontFamily` enum variant and one `OnceLock` branch per face;
- only `Regular` and `Bold` as style choices;
- lazy parsing through `fontdue`, with the same rasterizer used by preview and export;
- simple Latin-oriented layout without full shaping, bidi, or variable-font axis selection.

Adding dozens of enum variants and `include_bytes!` match arms would preserve the immediate visual
result but would make catalog updates, licensing checks, migrations, search, and fallback behavior
increasingly fragile. FONT-01 therefore includes both the initial catalog and the registry needed
to maintain it.

## Goals

1. Ship a varied offline catalog appropriate for captions, titles, gaming videos, long-form
   editorial text, handwriting, and technical overlays.
2. Render the same font bytes in preview and export on Windows, macOS, and Linux.
3. Guarantee Portuguese, English, and Spanish coverage through GF Latin Core and ship deterministic
   fallbacks for Arabic, Hebrew, Devanagari, Bengali, Tamil, and Thai.
4. Keep application startup and the text-properties panel independent of parsing every font.
5. Make source, revision, checksum, license, family metadata, and supported styles auditable.
6. Preserve existing projects and stable font choices across catalog updates.
7. Prevent malformed or replaced font files from entering a release.

## Non-goals

- Downloading Google Fonts at application runtime.
- Exposing the entire Google Fonts collection as built-in content.
- Using whatever happens to be installed on the user's operating system for deterministic project
  rendering.
- Importing user-provided fonts in FONT-01. That is a separate feature with a different trust,
  licensing, portability, and project-sharing model.
- Bundling proprietary fonts, fonts with unclear redistribution rights, web-only WOFF/WOFF2 files,
  or font archives downloaded from aggregation websites.
- Shipping international fallback families before
  [TEXT-01](complex-text-shaping.md)'s shaping, bidi, and cluster tests pass.
- Bundling the large CJK and color-emoji files in the base 20-MiB catalog; those are separately
  locked optional packs.

## Source of truth

Use the official [google/fonts repository](https://github.com/google/fonts) as the only binary
source for the initial catalog. Google Fonts documents that:

- the top-level directory identifies the license;
- each family directory contains the served TTF files, `METADATA.pb`, and its license;
- the repository permits self-hosting subject to each family's license terms;
- the `ofl/` directory contains SIL Open Font License families.

Discovery may begin in the [Google Fonts catalog](https://fonts.google.com), but a family is not
approved until its exact files and metadata exist in the pinned Git repository revision. Do not use
the CSS API, generated webfont URLs, Fontsource, package-manager mirrors, release ZIPs, or arbitrary
upstream repositories as the release source.

The initial expansion deliberately uses only `ofl/` families. This keeps the license model uniform
and lets every unmodified binary be redistributed with its family-local `OFL.txt`. Apache-2.0 and
Ubuntu Font License families may be evaluated later, but must not be mixed into this slice without
updating the notices, policy, and tests.

## Initial catalog decision

The target is **43 families**: keep all 6 existing families, add 30 stylistic families, and add 7
international fallback families. This is large enough to cover common visual roles and the first
complex-script guarantee without turning the selector into an uncurated list or adding the 1+ GiB
full Google Fonts snapshot to the product.

The tables list paths relative to a pinned `google/fonts` revision. When two files share one family
directory, the directory prefix is shown on the first file only. An entry with bracketed axes is an
upstream variable TTF. FONT-01 must keep those source binaries unmodified and must not pretend that
`Bold` works until the renderer can select the `wght` axis correctly.

### General sans and rounded: 10 families

| Family | Purpose | Upstream files |
| --- | --- | --- |
| Lato | Existing general caption default | `ofl/lato/Lato-Regular.ttf`, `Lato-Bold.ttf` |
| Inter | Neutral UI/editorial text | `ofl/inter/Inter[opsz,wght].ttf` |
| Montserrat | Geometric titles and captions | `ofl/montserrat/Montserrat[wght].ttf` |
| Roboto | Dense informational overlays | `ofl/roboto/Roboto[wdth,wght].ttf` |
| Open Sans | Highly readable body/caption text | `ofl/opensans/OpenSans[wdth,wght].ttf` |
| Poppins | Geometric social-video captions | `ofl/poppins/Poppins-Regular.ttf`, `Poppins-Bold.ttf` |
| Nunito | Friendly rounded captions | `ofl/nunito/Nunito[wght].ttf` |
| Source Sans 3 | Editorial and tutorial text | `ofl/sourcesans3/SourceSans3[wght].ttf` |
| Barlow | Compact modern overlays | `ofl/barlow/Barlow-Regular.ttf`, `Barlow-Bold.ttf` |
| Fredoka | Rounded playful display text | `ofl/fredoka/Fredoka[wdth,wght].ttf` |

### Display, condensed, and gaming: 10 families

| Family | Purpose | Upstream files |
| --- | --- | --- |
| Bebas Neue | Existing condensed title face | `ofl/bebasneue/BebasNeue-Regular.ttf` |
| Archivo Black | Existing heavy caption face | `ofl/archivoblack/ArchivoBlack-Regular.ttf` |
| Oswald | Condensed subtitles and scoreboards | `ofl/oswald/Oswald[wght].ttf` |
| Anton | High-impact short titles | `ofl/anton/Anton-Regular.ttf` |
| Barlow Condensed | Compact HUD and list text | `ofl/barlowcondensed/BarlowCondensed-Regular.ttf`, `BarlowCondensed-Bold.ttf` |
| League Spartan | Geometric title cards | `ofl/leaguespartan/LeagueSpartan[wght].ttf` |
| Teko | Narrow esports/stat overlays | `ofl/teko/Teko[wght].ttf` |
| Black Ops One | Military/gameplay title treatment | `ofl/blackopsone/BlackOpsOne-Regular.ttf` |
| Russo One | Tech and gaming display text | `ofl/russoone/RussoOne-Regular.ttf` |
| Bangers | Comic/high-energy callouts | `ofl/bangers/Bangers-Regular.ttf` |

### Serif and editorial: 6 families

| Family | Purpose | Upstream files |
| --- | --- | --- |
| Playfair Display SC | Existing high-contrast serif | `ofl/playfairdisplaysc/PlayfairDisplaySC-Regular.ttf`, `PlayfairDisplaySC-Bold.ttf` |
| Merriweather | Readable long-form serif | `ofl/merriweather/Merriweather[opsz,wdth,wght].ttf` |
| Libre Baskerville | Documentary/editorial captions | `ofl/librebaskerville/LibreBaskerville[wght].ttf` |
| Lora | Contemporary narrative text | `ofl/lora/Lora[wght].ttf` |
| Cinzel | Cinematic/classical titles | `ofl/cinzel/Cinzel[wght].ttf` |
| Bitter | Slab-serif tutorial and callout text | `ofl/bitter/Bitter[wght].ttf` |

### Handwritten and script: 6 families

| Family | Purpose | Upstream files |
| --- | --- | --- |
| Patrick Hand | Existing casual handwritten face | `ofl/patrickhand/PatrickHand-Regular.ttf` |
| Caveat | Informal notes and annotations | `ofl/caveat/Caveat[wght].ttf` |
| Pacifico | Retro/script title accents | `ofl/pacifico/Pacifico-Regular.ttf` |
| Dancing Script | Friendly connected script | `ofl/dancingscript/DancingScript[wght].ttf` |
| Comic Neue | Casual readable dialog | `ofl/comicneue/ComicNeue-Regular.ttf`, `ComicNeue-Bold.ttf` |
| Gloria Hallelujah | Marker-like annotations | `ofl/gloriahallelujah/GloriaHallelujah.ttf` |

### Monospace: 4 families

| Family | Purpose | Upstream files |
| --- | --- | --- |
| Anonymous Pro | Existing classic monospace | `ofl/anonymouspro/AnonymousPro-Regular.ttf`, `AnonymousPro-Bold.ttf` |
| JetBrains Mono | Code/tutorial overlays | `ofl/jetbrainsmono/JetBrainsMono[wght].ttf` |
| Roboto Mono | Neutral counters and telemetry | `ofl/robotomono/RobotoMono[wght].ttf` |
| Space Mono | Distinctive retro/technical text | `ofl/spacemono/SpaceMono-Regular.ttf`, `SpaceMono-Bold.ttf` |

### International shaping and fallback: 7 families

These faces are visible under an International category but also form TEXT-01's deterministic
fallback chains. Exposing them is gated on the shaping tests in
[complex-text-shaping.md](complex-text-shaping.md).

| Family | Script/use | Upstream files |
| --- | --- | --- |
| Noto Sans Arabic | Arabic sans and fallback | `ofl/notosansarabic/NotoSansArabic[wdth,wght].ttf` |
| Noto Naskh Arabic | Arabic Naskh style/secondary fallback | `ofl/notonaskharabic/NotoNaskhArabic[wght].ttf` |
| Noto Sans Hebrew | Hebrew | `ofl/notosanshebrew/NotoSansHebrew[wdth,wght].ttf` |
| Noto Sans Devanagari | Hindi/Devanagari shaping | `ofl/notosansdevanagari/NotoSansDevanagari[wdth,wght].ttf` |
| Noto Sans Bengali | Bengali shaping | `ofl/notosansbengali/NotoSansBengali[wdth,wght].ttf` |
| Noto Sans Tamil | Tamil shaping | `ofl/notosanstamil/NotoSansTamil[wdth,wght].ttf` |
| Noto Sans Thai | Thai shaping and wrapping | `ofl/notosansthai/NotoSansThai[wdth,wght].ttf` |

## Measured package budget

The 51 files above were fetched successfully from candidate `google/fonts` revision
`ade3d1533e06b2b1462ffcde8e08b129627ca360` on 2026-08-28. Their combined uncompressed size is
17,902,692 bytes (17.07 MiB), including the 9 files already present in oca. This is a measurement,
not a promise about installer compression.

Release gates:

- at most **20 MiB** of bundled TTF data for the initial catalog;
- at most **5 MiB per binary**;
- exactly **43 visible families** and **51 approved source binaries** unless this document and the
  lock manifest are reviewed together;
- no full-repository ZIP, webfonts, italics, duplicate static instances, or unused language-specific
  families in the initial package.

If a future upstream revision exceeds a gate, the update must explain which family changed and
either keep the previous pinned binary or make an explicit product tradeoff. Do not silently subset
or recompress a font to make the check pass.

## License and attribution requirements

For every family:

1. Vendor the exact unmodified `OFL.txt` stored beside the selected binary.
2. Record the copyright string, license, upstream family path, pinned repository revision, and
   SHA-256 of every binary in the catalog lock.
3. Generate a human-readable `THIRD_PARTY_FONTS.md` for source distributions and package it with
   desktop legal notices.
4. Preserve the license inside the family asset directory; do not consolidate 36 copies into one
   assumed-equivalent text without verifying them.
5. Check the license for Reserved Font Names before any modification, subsetting, renaming, or
   static instancing. FONT-01 avoids that question by shipping upstream binaries unchanged.
6. Never imply that Google endorses oca or that the fonts are owned by the project.

Projects and exported videos do not need to inherit the OFL merely because they render text with an
OFL font. The font software and its notices remain part of the application distribution.

## Catalog manifest

Replace hard-coded catalog knowledge with one reviewed manifest. The exact file format is an
implementation decision, but it must contain typed fields equivalent to:

```text
catalog_version
google_fonts_revision
family_id                 # stable ASCII slug, never localized
display_name
category                  # sans, display, serif, handwritten, monospace
tags                      # caption, condensed, gaming, cinematic, tutorial, etc.
source_path
sha256
size_bytes
license_id                # OFL-1.1 in the initial catalog
license_path
source_kind               # static or variable
supported_weights
supported_axes
default_weight
fallback_family_id
glyphset_guarantees
```

The build generates Rust catalog data from this manifest. CI rejects duplicate IDs, duplicate
paths, unknown fields, non-ASCII IDs, missing licenses, out-of-range weights, mismatched checksums,
and files outside the expected asset root.

Stable examples are `lato`, `bebas-neue`, and `source-sans-3`. Display names may evolve or be
localized; serialized project identity must not depend on a translated label or enum ordinal.

## Acquisition and update workflow

Font acquisition is a maintainer-time vendoring operation, never an application runtime feature.

1. Choose one `google/fonts` commit SHA, not a branch name or tag resolved at build time.
2. Read each selected family's `METADATA.pb` and `OFL.txt` at that revision.
3. Download only manifest-declared paths from the fixed
   `raw.githubusercontent.com/google/fonts/<sha>/` origin, with an explicit timeout and no automatic
   redirect following.
4. Write to a newly created staging directory; never construct a destination from an unvalidated
   family or file name.
5. Enforce the count, per-file, and aggregate byte limits before copying into the repository.
6. Verify SHA-256, extension, sfnt/TrueType signature, family/style name records, declared axes,
   weight metadata, and GF Latin Core coverage.
7. Parse every binary with the production font parser in an isolated test process and run the
   Google Fonts/Universal FontBakery checks selected for shipped binaries.
8. Render the fixed visual and multilingual corpus used by preview/export parity tests.
9. Copy binaries, licenses, the lock, and generated notices atomically only after every check passes.
10. Review the resulting binary diff and attribution diff in the same change.

The production build is offline and uses only committed, locked assets. It must not fetch fonts,
resolve `main`, or accept a checksum override.

## Rendering architecture

### Stable selection model

Replace `TextFontFamily` as the persisted catalog identity with a forwards-compatible structure:

```text
FontSelection {
    family_id,
    weight,
    italic,
}
```

The exact Rust representation is deferred to implementation. Required semantics:

- `family_id` is a stable manifest slug;
- weight is numeric and validated against the selected face/axis;
- italic is exposed only when a bundled italic source exists;
- old enum values migrate losslessly to their matching IDs;
- an unknown future ID keeps its serialized value, displays a missing-font warning, and renders with
  the deterministic fallback rather than corrupting project load.

No family in the initial expansion may be removed or replaced under the same ID. An upstream design
change is a new catalog revision; project rendering must remain reproducible across application
updates, so replacing a font requires a compatibility decision and golden-image review.

### Static and variable fonts

The current `fontdue` integration does not expose selection of OpenType variable axes or complex
shaping. Final FONT-01 acceptance therefore depends on
[TEXT-01](complex-text-shaping.md), which selects and validates one shared shaping/layout/
rasterization path for preview, measurement, highlights, and export.

Rules:

- Never label a variable family as Bold while rendering its default instance.
- Prefer one unmodified variable TTF over generated static duplicates when the renderer supports
  the required axes.
- Use official static files when they exist in the pinned family directory.
- Do not generate static instances, subset, or rename OFL binaries in FONT-01. That would create a
  modified font and requires per-family Reserved Font Name review plus new checksums/notices.
- Unsupported axes remain at the upstream default and are not shown in the UI.

The candidate dependency, acceptance spike, Unicode standards, cluster model, bidi behavior,
resource limits, and fallback policy live in `complex-text-shaping.md`.

### Shaping and script support

All stylistic families must render GF Latin Core, which includes Portuguese. The international
families additionally satisfy TEXT-01's script-specific coverage matrix. The Latin acceptance
corpus must include at least:

```text
Ação, coração, órgão, bênção, você, pôr, consequência
Gameplay: vitória, missão, nível, crítico, sequência 01/10
Español: acción, corazón, niño, ¿qué pasó?
English: QUICK BROWN FOX — 0123456789
```

Complex-script support is delivered through TEXT-01. A codepoint existing in a TTF is not proof
that a script renders correctly; each language/script claim requires shaping and golden tests.

Missing glyphs are detected before export. The deterministic fallback order is:

1. selected family/face;
2. Lato Regular for GF Latin Core characters missing because of a corrupt/invalid selection;
3. visible replacement glyph plus a preflight warning containing codepoints, not private text.

Preview and export must resolve the same fallback decisions. Silent operating-system fallback is
prohibited.

## Loading and cache behavior

- Embed or package locked font bytes with the application; do not parse them during startup.
- Load the manifest eagerly because it is small; parse a face only when a preview, measurement, or
  export requests it.
- Replace one permanent `OnceLock` per face with a keyed cache that can scale beyond 44 binaries.
- Bound parsed-font cache memory, initially to 16 faces or 32 MiB, and evict least-recently-used
  entries that have no active render references.
- Cache selector preview textures separately by family ID, weight, sample, UI scale, theme, and
  locale. Generate them off the UI thread and cap both pending jobs and retained textures.
- A corrupt face produces a structured error and fallback; it must not panic, spin, or allocate an
  unbounded raster buffer.

Benchmarks must measure first-use parse latency, warm lookup, selector scrolling, a caption-heavy
preview, and export with many distinct families. The panel must not parse all fonts simply because
it opened.

## Font selector UX

Replace the current six-row combo box with a searchable, virtualized selector:

- search by family name and curated tags;
- sections for recent, favorites, sans, display/gaming, serif, handwritten, and monospace;
- each result rendered in its own face using a short localized sample;
- clear badges for single-weight and variable families;
- weight control shows only valid choices;
- a missing-glyph indicator based on the current text;
- keyboard navigation, accessible names, and no color-only category meaning;
- recent/favorite state stored in preferences, never in `.ocproj` rendering data.

The list is sorted by curated product order, not whatever order files happen to have on disk.
Search and selection remain responsive while previews are still being generated.

## Persistence and compatibility

- Migrate the six existing enum families to stable IDs without changing their selected files or
  rendered pixels.
- Keep Lato Regular as the default for old projects and malformed selections.
- Store family ID, numeric weight, and italic intent in `.ocproj`; do not store absolute font paths.
- Validate IDs/styles on load, but preserve unknown IDs so a project saved by a newer version can be
  reopened without destructive normalization.
- Collaboration bundles do not duplicate built-in font files. They record the catalog/release
  version needed to diagnose a mismatch.
- Golden images protect the six existing faces from accidental visual changes during migration.

## Security and supply-chain controls

Font files are complex untrusted binary inputs even when fetched from an official repository.
FONT-01 must:

- use a static destination allowlist and exact source paths;
- reject path traversal, symlinks, archives, collection files, unknown extensions, and extra files;
- enforce file count and byte limits before parsing;
- pin a repository commit and verify SHA-256 before build inclusion;
- parse in CI and isolated tests, not in the privileged acquisition process before validation;
- never execute font-provided programs or arbitrary scripts;
- keep runtime font loading limited to committed built-in assets;
- surface generic user errors while logging only family IDs/checksums, not project text;
- update fonts through reviewed pull requests with license and visual diffs, not automatic dependency
  bots that replace binaries without approval.

Relevant weakness classes: CWE-20 (input validation), CWE-22 (path traversal), CWE-434
(unrestricted file processing), and CWE-405 (unbounded resource consumption).

## Implementation slices

### FONT-01A: Manifest and migration

1. Add the locked catalog manifest and generated Rust metadata.
2. Introduce stable family IDs and migrate the six current enum variants.
3. Preserve existing preview/export pixels through golden tests.
4. Add strict catalog validation without changing the visible six-family selector yet.

### FONT-01B: Vendoring and catalog expansion

1. Add the 30 stylistic and 7 international families, family-local licenses, hashes, and notices.
2. Enforce 43-family/51-file/20-MiB release gates.
3. Validate GF Latin Core and the fixed pt-BR/es/en render corpus.
4. Keep unsupported variable weights hidden until FONT-01C.

### FONT-01C: Variable weights and TEXT-01 integration

1. Land TEXT-01's selected shaping/layout/rasterization path.
2. Support `wght` plus default `wdth`/`opsz` coordinates for cataloged variable fonts.
3. Keep preview, measurement, highlights, wrapping, bidi, and export on the same shaped glyph runs.
4. Expose international families only after their script and fallback corpora pass.

### FONT-01D: Searchable selector and cache

1. Add virtualized search/category/recent/favorite UI.
2. Add bounded asynchronous specimen generation and parsed-face caching.
3. Show accurate style and missing-glyph state.
4. Add keyboard/accessibility and responsiveness tests.

## Test strategy

- Manifest schema, duplicate ID, unknown field, invalid path, wrong hash, wrong license, wrong count,
  and aggregate/per-file limit tests.
- Parser tests for every shipped binary and every exposed face/weight.
- GF Latin Core coverage plus explicit Portuguese/Spanish/English corpus tests.
- Preview/export pixel parity for every family at one fixed size and a representative subset at
  multiple sizes/weights.
- Golden-image migration tests for the existing six families.
- Variable-axis tests proving weight 400 and 700 produce different glyph outlines/metrics where
  supported.
- Missing/corrupt/unknown-family fallback and export-preflight tests.
- Selector search, category, favorite, recent, keyboard, screen-reader label, and virtualization
  tests.
- Performance tests proving opening/scrolling the selector does not parse all 44 files or block the
  UI thread.
- Cross-platform packaged smoke tests to prove no system-installed font participates.

## Definition of done

- [ ] Exactly 43 families and their approved styles are visible offline on all supported platforms.
- [ ] All 51 source binaries, licenses, metadata, hashes, and revision are locked and reproducible.
- [ ] The uncompressed base font payload remains at or below 20 MiB.
- [ ] Existing projects retain identical rendering for the original six families.
- [ ] Variable-family style labels match the actual selected axes.
- [ ] Portuguese, English, and Spanish acceptance strings render without missing glyphs.
- [ ] Preview, highlights, wrapping, measurement, and export use the same resolved face and shaping.
- [ ] Search/preview stays responsive and does not eagerly parse the whole catalog.
- [ ] Unknown/corrupt fonts fail safely with deterministic fallback and export warning.
- [ ] Legal notices ship with every installer/package and source distribution.
- [ ] `ROADMAP.md`, `matrix/effects-and-color.md`, and the asset README reflect what actually ships.

## Future extensions

- User-installed font import with explicit license/portability warnings and project packaging.
- Locked optional CJK and color-emoji packs after TEXT-01 and pack installation support exist. At
  the measured candidate revision, four regional CJK fonts add 47.42 MiB and Noto Color Emoji adds
  23.15 MiB; the full base+CJK+emoji payload would be 87.64 MiB.
- Italic sources and additional weights based on observed usage rather than bundling every style.
- Curated font pairing and title/caption presets built on stable family IDs.
- An optional downloadable catalog only after signed manifests, rollback, retention, and offline
  project reproducibility have their own design.

## Primary references

- [Official google/fonts repository and licensing layout](https://github.com/google/fonts)
- [Google Fonts repository structure](https://googlefonts.github.io/gf-guide/googlefonts.html)
- [Google Fonts static and variable file guidance](https://googlefonts.github.io/gf-guide/statics.html)
- [Google Fonts glyph-set requirements](https://googlefonts.github.io/gf-guide/requirements.html)
- [GF Latin Core definition, including Portuguese](https://github.com/googlefonts/glyphsets/blob/main/Lib/glyphsets/definitions/GF_Latin_Core.yaml)
- [SIL Open Font License 1.1 official text](https://openfontlicense.org/open-font-license-official-text/)
- [FontBakery Google Fonts checks](https://fontbakery.readthedocs.io/en/latest/user/USAGE.html#fontbakery-check-googlefonts)
- [TEXT-01 complex shaping and bidi architecture](complex-text-shaping.md)

[<- back to spec/INDEX.md](../INDEX.md)
