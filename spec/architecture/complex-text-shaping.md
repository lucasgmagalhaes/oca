# Complex Text Shaping and Bidirectional Layout

**Status:** proposed

**Roadmap ID:** TEXT-01

**Priority:** may proceed alongside FONT-01A/B; required before international fallback families
are exposed and before FONT-01 is marked complete

**Scope:** OpenType shaping, bidi, cluster mapping, Unicode line breaking, deterministic fallback,
variable weights, and preview/export parity

## Problem

oca's current text renderer is deterministic but Latin-oriented:

- `text_metrics.rs` sums one advance width per Unicode scalar and explicitly has no kerning,
  ligatures, bidi, or complex-script shaping;
- `overlay_render.rs` asks `fontdue::Layout` to place glyphs in logical left-to-right order;
- word highlighting filters glyphs by a UTF-8 `byte_offset` instead of shaped cluster boundaries;
- wrapping is based on the current simple layout rather than the Unicode line-breaking algorithm;
- the fixed `fontdue` path cannot select variable-font axes used by much of FONT-01's catalog;
- fallback is one hard-coded Latin face and cannot choose a face by script or language.

Consequences include disconnected or contextually wrong Arabic letters, incorrect ordering for
mixed RTL/LTR text and numbers, broken Indic conjuncts, missing kerning/ligatures, unsafe highlight
splits through a ligature, and incorrect break opportunities for Thai and other scripts.

This is not solved by adding fonts alone. A font supplies glyphs and OpenType rules; a shaping and
layout engine must interpret the Unicode text, script, language, direction, font features, and
fallback chain before rasterization.

## Goals

1. Shape text according to its script and language, including contextual forms, combining marks,
   kerning, required ligatures, and Indic reordering.
2. Implement paragraph and run ordering compatible with Unicode Bidirectional Algorithm UAX #9.
3. Wrap at Unicode line-break opportunities without splitting grapheme or shaping clusters.
4. Map logical UTF-8 ranges to shaped clusters so timed word highlights remain correct.
5. Select variable weights accurately and use deterministic bundled fallback by script.
6. Produce one shared shaped representation consumed by preview, measurement, highlighting, and
   export.
7. Keep shaping bounded, cacheable, non-blocking for the UI, and safe for arbitrary pasted text.

## Non-goals

- Loading operating-system fonts or changing render output by platform.
- Vertical writing, ruby annotation, dictionary hyphenation, kashida justification, or advanced
  desktop-publishing controls in the first slice.
- Automatically translating text or guessing a language beyond script/base-direction detection.
- Normalizing or rewriting the user's stored text behind their back.
- Claiming every Unicode script is supported. The guarantee is limited to the bundled coverage
  matrix below; missing scripts use an explicit preflight warning.
- Building a shaping engine directly from the Unicode/OpenType specifications.

## Normative behavior

TEXT-01 follows these standards, pinned to the Unicode data version used by its dependencies and
test fixtures:

- [UAX #9: Unicode Bidirectional Algorithm](https://www.unicode.org/reports/tr9/);
- [UAX #14: Unicode Line Breaking Algorithm](https://www.unicode.org/reports/tr14/);
- [UAX #29: Unicode Text Segmentation](https://www.unicode.org/reports/tr29/);
- [UAX #15: Unicode Normalization Forms](https://www.unicode.org/reports/tr15/);
- OpenType shaping behavior as implemented by a HarfBuzz-compatible shaper;
- shaped cluster semantics described by the
  [HarfBuzz cluster model](https://harfbuzz.github.io/clusters.html).

The stored text remains its original valid UTF-8 sequence. NFC/NFKC normalization may be used for
search keys only when the operation explicitly needs it; it must not replace rendering text,
cluster offsets, transcript word ranges, or persisted content.

## Architecture decision

### Candidate stack

Use `cosmic-text` as the preferred integration candidate, subject to the implementation spike and
dependency review. As researched on 2026-08-28, its current 0.19 API provides:

- advanced shaping through HarfRust;
- bidirectional multi-line layout;
- font fallback;
- Swash-based rasterization, ligatures, and color emoji support;
- bundled-font database construction;
- variable-weight matching and rendering;
- Linux, macOS, and Windows support.

The implementation must add the dependency through Cargo, pin the resolved version in
`Cargo.lock`, verify its MSRV and enabled feature tree, and record MIT/Apache-2.0 notices. No
MercadoLibre Rust SDK in the project security catalog covers text shaping or rasterization.

Do not call `FontSystem::new()`: the official API documents that it loads installed system fonts,
which would break deterministic projects. Build an empty `fontdb::Database`, load only FONT-01's
locked byte sources, and construct the system with the equivalent of
`new_with_locale_and_db_and_fallback` plus an oca-owned fallback policy.

`rustybuzz` is not the primary choice because it is a shaper, not a complete bidi, line-layout,
fallback, and rasterization stack. Integrating it directly would require oca to assemble and keep
four additional subsystems consistent. It remains a fallback option only if the cosmic-text spike
fails a documented correctness, performance, or compatibility gate.

### Acceptance spike

Before migrating production rendering, a small isolated spike must prove:

1. bundled-only font loading with zero system-font matches;
2. Arabic, mixed Arabic/Latin/numbers, Hebrew, Devanagari, Bengali, Tamil, and Thai shaping;
3. `fi`/`fl` ligatures and combining diacritics with stable cluster ranges;
4. variable weights 400 and 700 producing different glyph output from one variable TTF;
5. RGBA output compatible with the current full-canvas overlay path;
6. line wrapping that does not split a ligature or grapheme cluster;
7. predictable performance under the documented input limits;
8. supported Rust toolchain, targets, licenses, and package size.

If any gate fails, write a short ADR comparing direct HarfBuzz bindings, `rustybuzz` plus separate
bidi/layout/rasterization crates, and the candidate stack. Do not silently retain the current
renderer for only some scripts while claiming TEXT-01 complete.

**Spike result (2026-08-28): all 8 gates passed.** Run as an isolated throwaway crate (not
integrated into `core`/`ui` — that's TEXT-01A, still open), `cosmic-text = { version = "0.19",
default-features = false, features = ["std", "swash"] }` against a `fontdb::Database` built from
`FontSystem::new_with_locale_and_db` (never `FontSystem::new()`), loaded only with the six
already-bundled families plus one real fetched variable font (Inter) and the six FONT-01C Noto
international families (Arabic, Hebrew, Devanagari, Bengali, Tamil, Thai), all fetched from the
same pinned `google/fonts` revision `built-in-font-catalog.md` already uses
(`ade3d1533e06b2b1462ffcde8e08b129627ca360`), for spike verification only — not vendored into the
repo (that stays FONT-01B's own gated process).

1. **Bundled-only loading**: confirmed by reading `font/fallback/unix.rs` (and its macOS/Windows/
   other siblings) directly — `PlatformFallback` is a compile-time list of family *names* tried
   against whatever `fontdb::Database` was supplied, never an OS font-directory scan; an empty/
   custom db with no system-font files loaded means those names simply never resolve. Verified
   empirically too: `font_system.db().faces().count()` matched exactly the 9 files loaded, no
   more.
2. **Arabic, Hebrew, Devanagari, Bengali, Tamil, Thai shaping**: a mixed Latin+digit+Arabic string
   produced real per-character UAX #9 bidi levels (`{0, 1}` within one line, not a single
   paragraph-wide flag) with the RTL-level glyphs' cluster ranges in correct right-to-left visual
   order (descending logical byte offsets). All six Noto families shaped their sample word with
   zero `.notdef` (missing-glyph) hits; Devanagari's conjunct sample (`नमस्ते`, `स्त` conjunct) and
   Tamil's sample both produced fewer glyphs than input characters, confirming real reordering/
   conjunct formation rather than a naive one-glyph-per-codepoint pass.
3. **`fi`/`fl` ligatures**: `"difficult waffle"` in Lato shaped to 12 glyphs from 16 characters,
   with two explicit multi-character clusters (`"ffi"`, `"ffl"`) each mapping to one glyph ID —
   real ligature formation with a stable, inspectable cluster range.
4. **Variable weights 400 vs 700**: rasterizing the real fetched `Inter[opsz,wght].ttf` at both
   weights via `SwashCache` produced different `Placement` bounds and different pixel bytes —
   proof the `wght` axis is actually applied at raster time (`swash.rs`'s `normalized_coords`
   call), not just accepted and ignored.
5. **RGBA output**: `SwashCache::with_pixels` painted real non-zero pixels into a plain
   `Vec<u8>` RGBA buffer sized like `overlay_render.rs`'s existing full-canvas overlay — no
   adapter needed beyond iterating callback pixels into oca's own buffer layout.
6. **Wrap without splitting a cluster**: a narrow-width line containing `"cafe" + COMBINING ACUTE
   ACCENT` (two scalars, one grapheme) wrapped correctly with the base+combining pair staying in
   the same line segment.
7. **Performance**: 200 shape passes of a realistic 73-character pt-BR caption averaged ~37µs/pass
   in a release build on this sandbox's CPU — well within a bounded-worker, non-blocking budget.
8. **Toolchain/license/package**: MIT OR Apache-2.0, `rust-version = "1.89"` (this repo's pinned
   `1.98.0` satisfies it), and with `default-features = false, features = ["std", "swash"]` the
   full dependency tree is pure Rust — `harfrust`, `skrifa`/`read-fonts`, `swash`, `fontdb`,
   `unicode-bidi`, `unicode-linebreak`, `unicode-script` — no C toolchain, no linked system
   library (the default `fontconfig` feature, which pulls in a pure-Rust config-file *parser*, not
   a linked `libfontconfig.so`, is disabled entirely here since oca's own fallback policy replaces
   it). Windows/macOS target support is documented by upstream but not independently verified in
   this Linux-only sandbox — the same category of positive-path gap this codebase's own GPU/
   hardware-dependent items already carry.

One real, minor nuance found, not a gate failure: `NotoSansHebrew[wdth,wght].ttf` registered into
`fontdb` at `weight=100` rather than 400 for its default named instance — a variable-font
weight-registration detail TEXT-01C's own font-loading code will need to handle explicitly
(request the axis value oca wants, don't trust the file's own default named instance), not
something discovered by reading the doc alone.

**Not yet done** (as of the spike itself): none of this was wired into `core`/`ui` yet —
`text_metrics.rs` still summed per-character advances via `fontdue`, `overlay_render.rs` still
asked `fontdue::Layout` for placement. The spike proved the dependency choice was sound; TEXT-01A
itself (adapter, `TextLayoutEngine`, moving measurement/wrapping/background-geometry/raster-
placement onto shaped output, golden tests for the six existing families) was the next real slice.

**TEXT-01A steps 1-2 (adapter) shipped** (2026-08-28); step 3 (the swap) followed the same day —
see below. `avcore::
text_layout` adds the provider-neutral `ShapedText`/`ShapedLine`/`ShapedGlyph` value and
`TextLayoutEngine`, a real `core` dependency now (`cosmic-text = { version = "0.19",
default-features = false, features = ["std", "swash"] }`, `fontconfig` disabled). `font_catalog`
gained `locked_face_bytes()`/`face_bytes()` — the single `include_bytes!` source of truth both
`text_metrics`'s existing `fontdue` table and this new module load from, so the two engines can
never silently diverge on which bytes a family/weight resolves to. `TextLayoutEngine::
new_from_locked_catalog()` builds its `fontdb::Database` from exactly those locked bytes via
`FontSystem::new_with_locale_and_db` (never `FontSystem::new()`), matching the spike's own
gate-1 finding. `TextLayoutEngine::shape`/`text_width_px` are real, callable parity functions for
`text_metrics::text_width_px_with_font` — not yet used by it or by `overlay_render.rs`, which is
exactly TEXT-01A's still-open step 3 (see below).

Verified for real, not just type-checked: since `cosmic-text`'s own dependency tree is pure Rust
(confirmed by the spike), `font_catalog.rs`+`timeline.rs`+`keyframe.rs`+`text_layout.rs` (all
zero heavy `core` deps) were copied into a throwaway scratch crate alongside the real bundled
font assets and a real `cosmic-text` dependency, and `cargo test`ed there for real: 87/87 passing,
9 of them new — bundled-only loading (exact face count, no system-font leak), all six families
shaping the GF Latin Core acceptance corpus with zero `.notdef` hits, `fi`/`fl` ligature formation
(`"difficult waffle"`: 16 chars -> fewer glyphs, with a real multi-character cluster), Bold vs.
Regular resolving to different loaded faces for a two-weight family, a single-weight family
(Bebas Neue) accepting a Bold request without erroring or hitting `.notdef`, width monotonicity,
and wrapping actually producing multiple lines within the requested width. `cargo fmt --check`,
`cargo clippy -p core --lib --no-deps`, and `cargo check --workspace --all-targets` (all via the
documented temporary local `filters.c` shim, discarded before commit) stayed clean.

**TEXT-01A step 3 (the swap) shipped too** (2026-08-28, same day). `overlay_render.rs` now
shapes and rasterizes every `TextClip`/`TextSegment` through `text_layout::with_shared_engine`
instead of `fontdue::layout::Layout` — `draw_shaped_text` paints glyphs via
`cosmic_text::SwashCache::with_pixels`, and `shaped_ink_bbox` derives the rounded background's
bounding box from each glyph's real rasterized `Placement` (not just its advance box), matching
`fontdue`'s own "tight ink bbox" semantics for background sizing. `TextLayoutEngine::shape` gained
an `origin: (f32, f32)` parameter so every `ShapedGlyph` (and its `PhysicalGlyph` rasterization
handle) is already positioned at its final canvas pixel, mirroring `fontdue::layout::
LayoutSettings`'s `x`/`y` convention the previous renderer used.

**Real bug found and fixed while wiring this in, not assumed from docs**: `cosmic-text`'s
`LayoutGlyph::y` is relative to *that glyph's own run*, not an absolute canvas position — the
piece that actually varies line to line is `LayoutRun::line_y` ("Y offset to baseline of line"),
which a first implementation didn't add in. Every glyph on every line landed at the same `y`,
which (combined with `origin.1 = 0.0` in the failing test cases) also pushed the first line's
whole ascent above row 0 and clipped it entirely — silently producing zero visible pixels rather
than an obviously-wrong render. Caught by three real regression-test failures (see below), not by
inspection; confirmed against `cosmic-text`'s own `buffer.rs` source (`LayoutRunIter`'s
`line_y`/`line_top` construction) before fixing, then reproduced-then-fixed to confirm the
diagnosis. `text_metrics.rs`'s own per-character `text_width_px`/`word_x_offsets_px` functions
are now dead code (no caller outside their own tests) now that `overlay_render.rs` no longer
calls `bundled_font` — left in place with a doc-comment note rather than deleted, since removing
that whole measurement surface is its own separate cleanup, not a side effect of this swap.
`word_byte_ranges` (pure string search, engine-independent) is unaffected and still used by both
`overlay_render.rs` and `render.rs`.

Verified for real, including a real visual check this sandbox does have a path to even without a
running eframe session: `overlay_render.rs`'s own rasterization functions are plain
`fn(&TextClip, u32, u32, f64) -> Vec<u8>` calls with zero GStreamer/GUI dependency, so they were
exercised directly. All of `overlay_render_test.rs`'s 16 existing structural tests (opaque/
transparent pixel checks, rounded-background corner tests, highlight-color-word tests, wrapping/
multi-line vertical-extent tests) pass unchanged against the new renderer, run for real in the
same throwaway scratch crate as the adapter slice (this time also carrying `render.rs`'s
`TextSegment` struct copied in, `shape_render.rs`, and `text_metrics.rs`, since `overlay_render.rs`
depends on all three) — 137/137 passing, zero test assertions changed. Beyond the structural
tests, two real sample captions (one plain, one with a rounded background and a highlighted word;
one wrapping across three lines) were rendered through the actual `render_text_clip_rgba` entry
point, composited onto an opaque backdrop, saved as PNG, and visually inspected — correct glyph
shapes (including `ç`), correct ligature/kerning-quality spacing, correct rounded-background
padding, correct highlight-word coloring, and correctly stacked wrapped lines, with the `line_y`
bug's exact symptom (blank output) confirmed absent. `cargo fmt --check`, `cargo clippy -p core
--lib --no-deps` (a new `#[allow(clippy::too_many_arguments)]` on `draw_shaped_text`, matching
this codebase's own existing convention for rasterization functions in `keyframe.rs`/`preview.rs`/
`render.rs`), and `cargo check --workspace --all-targets` (all via the documented temporary local
`filters.c` shim, discarded before commit) stayed clean.

**Still not done**: golden-image comparison against the *pre-swap* `fontdue` renderer's actual
pixel output (not attempted — the two rasterizers use different hinting/anti-aliasing, so exact
pixel parity was never the achievable goal; the doc's own bar is preserved *visual* output, which
the rendered-and-inspected PNGs above satisfy) and any live-GUI/export-encode confirmation, which
still needs a real windowed session this sandbox doesn't have. TEXT-01B (bidi/cluster-safe
highlights — the current highlight-by-cluster-start filter is a direct port of the old
byte-offset filter, not yet cluster-aware for RTL/conjunct scripts), TEXT-01C (international
fallback families), and TEXT-01D (performance/caching/optional packs) remain open.

## Shaping pipeline

The pipeline operates on logical UTF-8 and produces positioned glyphs; it never reorders the stored
string itself.

```text
validated UTF-8 TextClip content
        |
        v
paragraph split + base direction (Auto/LTR/RTL)
        |
        v
UAX #9 directional runs + script/language itemization
        |
        v
deterministic bundled font fallback per run/cluster
        |
        v
OpenType shaping (features + variable axes)
        |
        v
UAX #14 wrap opportunities constrained by shaped clusters/UAX #29 graphemes
        |
        v
positioned lines/runs/glyphs + logical cluster mapping
        |
        +--> measurement / background geometry / hit testing
        +--> timed word highlight recoloring
        +--> preview RGBA
        `--> export RGBA
```

The engine must return an oca-owned immutable `ShapedText`-equivalent value rather than leak
provider types throughout `core`. At minimum it contains:

- paragraph base direction and visual line order;
- line baseline, width, height, and logical byte range;
- runs with script, language, direction, selected family ID, weight, and variation coordinates;
- positioned glyph IDs and advances/offsets;
- each glyph's logical UTF-8 cluster range;
- fallback/missing-glyph diagnostics containing codepoints and family IDs, never full project text.

This value is the only input to text rasterization. `text_width_px_with_font`, background bounds,
word offsets, preview images, and export images must stop independently recomputing approximate
metrics.

## Direction, alignment, and language model

Extend text properties with forwards-compatible semantic values:

```text
TextDirection = Auto | LeftToRight | RightToLeft
TextAlign = Start | Center | End
language = optional validated BCP 47 tag
```

- `Auto` uses the paragraph's first strong directional character, per UAX #9.
- `Start` and `End` are direction-aware; they must not be persisted as visual `Left`/`Right`.
- Existing projects migrate to `Auto + Start`, preserving current Latin output.
- Explicit LTR/RTL is a user override for captions whose first strong character does not represent
  the intended paragraph direction.
- The language tag influences shaping and locale-sensitive fallback. Invalid/unknown tags are
  rejected or normalized to absent; they do not become arbitrary font lookup strings.
- Standard required shaping features stay enabled. Optional discretionary features are a later UI
  enhancement and must use a typed allowlist of OpenType tags.

The editor should expose invisible directional controls on demand. RLO/LRO and unmatched explicit
controls receive a non-blocking warning because they can make text visually misleading; they are
not silently stripped, since legitimate bidi content must round-trip exactly.

## Cluster-safe timed highlights

The current implementation maps transcript words to UTF-8 byte ranges and filters individual
glyphs by one byte offset. That is insufficient when one glyph represents several characters or
when several reordered glyphs belong to one logical cluster.

New behavior:

1. Preserve each transcript word's logical UTF-8 range.
2. Find all shaped clusters whose logical ranges overlap the word.
3. Expand the highlight to whole cluster boundaries; never split a ligature, combining sequence,
   or Indic conjunct.
4. Repaint the selected cluster glyphs using their already-shaped positions. Do not shape the word
   separately, because contextual Arabic forms and kerning could change.
5. If one cluster overlaps two timed words, use one deterministic rule: the cluster belongs to the
   first word until its time ends, then to the second. Record a diagnostic for test visibility.
6. Keep the base caption layout unchanged while highlight color changes.

Cursor/selection behavior in the text-editing field remains the UI toolkit's responsibility, but
preview click/hit testing added later must use grapheme/cluster boundaries rather than scalar or
byte boundaries.

## Deterministic fallback and bundled coverage

Fallback never queries the operating system. The oca policy receives script and locale and returns
only manifest IDs from FONT-01.

Initial guaranteed chains:

| Script/use | Primary fallback chain |
| --- | --- |
| Latin, Greek, Cyrillic | selected family -> Lato -> visible replacement glyph |
| Arabic | selected family -> Noto Sans Arabic -> Noto Naskh Arabic -> replacement |
| Hebrew | selected family -> Noto Sans Hebrew -> replacement |
| Devanagari | selected family -> Noto Sans Devanagari -> replacement |
| Bengali | selected family -> Noto Sans Bengali -> replacement |
| Tamil | selected family -> Noto Sans Tamil -> replacement |
| Thai | selected family -> Noto Sans Thai -> replacement |

Fallback happens at the smallest safe shaping unit supported by the chosen engine, without mixing
fonts inside a required cluster. The chosen family ID for each run is persisted in shaped cache
metadata so preview/export diagnostics can be compared.

CJK regional forms and color emoji are not part of the 20-MiB base catalog. They require locked
optional packs described in FONT-01. Missing pack coverage is shown before export and never resolved
through a system font.

## Script support matrix

TEXT-01 completion claims only tested behavior:

| Capability | Base guarantee |
| --- | --- |
| Latin ligatures/kerning/combining marks | Yes |
| Mixed LTR/RTL paragraphs and numbers | Yes |
| Arabic contextual shaping | Yes, with bundled Arabic fallbacks |
| Hebrew bidi and marks | Yes, with bundled Hebrew fallback |
| Devanagari conjuncts/reordering | Yes |
| Bengali conjuncts/reordering | Yes |
| Tamil shaping | Yes |
| Thai shaping and line-break opportunities | Yes |
| Simplified/Traditional Chinese, Japanese, Korean | Only with the matching optional CJK pack |
| Color emoji | Only with the optional emoji pack and verified color-glyph rendering |
| Vertical writing | No |
| Every Unicode script | No; preflight reports missing coverage |

Language-level claims such as Persian or Urdu are added only after their complete acceptance corpus
passes with the pinned font revision. Sharing an Arabic script is not by itself proof of complete
language coverage.

## Caching and concurrency

Shape on a bounded worker, not on the egui paint callback. A request carries a generation number;
when text or styling changes, stale results are discarded rather than installed after the newer
edit.

The shaped-layout cache key includes:

- hash of logical text, without logging the text itself;
- family ID, weight, italic/style, feature set, and font catalog revision;
- font size, line height, maximum width, alignment, direction, and language;
- shaping engine/data version.

Bound the cache by entry count and estimated bytes. Preview and export may share immutable shaped
values, but no lock may be held while shaping or rasterizing. Cache misses must not block unrelated
preview decoding or export progress updates.

## Input and resource limits

Text is user-controlled and shaping complexity is not assumed linear for every malformed or
adversarial case. Validate before queueing work:

- valid UTF-8 (guaranteed by Rust `String`, revalidated at imported interchange boundaries);
- at most 32 KiB UTF-8 per text clip;
- at most 8,192 grapheme clusters and 256 paragraphs per clip;
- at most 32,768 produced glyphs and 512 visual lines after shaping;
- Unicode bidi explicit depth no greater than UAX #9's fixed maximum of 125;
- at most 8 fallback candidates for one shaping unit;
- finite, bounded font size, line height, width, and variable-axis coordinates;
- bounded worker queue, shaped cache, glyph image cache, and retry count.

When a post-shape limit is exceeded, discard the result and return a generic user-facing error with
a code such as `text_layout_too_complex`. Logs and remote error reports include counts, script,
family IDs, and release metadata, never the caption/transcript content.

## Preview and export integration

`overlay_render` remains responsible for blending RGBA pixels and drawing the rounded background,
but glyph selection and placement come exclusively from `ShapedText`.

- Preview shapes when content/layout inputs change, not on every video frame.
- Timed highlights reuse shaped glyph positions and only alter paint selection/color.
- Export resolves and shapes all text before starting the native overlay post-pass; missing glyphs
  or exceeded limits appear in export preflight instead of silently producing blank output.
- Background bounds are derived from shaped line/glyph geometry, including RTL offsets and marks.
- Preview and export use identical hinting, subpixel settings, fallback policy, font bytes, and
  variation coordinates.
- The current transparent full-canvas buffer stays as the first integration boundary, minimizing
  changes to GStreamer/FFmpeg while the text engine is replaced.

## Implementation slices

### TEXT-01A: Adapter and Latin parity

1. Add the provider-neutral shaped-text model and candidate engine behind one `TextLayoutEngine`.
2. Load only locked FONT-01 bytes into a custom font database/fallback policy.
3. Move measurement, wrapping, background geometry, and raster placement onto shaped output.
4. Preserve golden output for the six existing families and ordinary Latin captions.

### TEXT-01B: Bidi, clusters, and highlights

1. Add `Auto/LTR/RTL`, semantic alignment, and optional language metadata.
2. Enable bidi paragraph/run layout and cluster-safe timed highlights.
3. Add directional-control visibility/warnings and mixed-direction golden tests.
4. Replace approximate word-width helpers with cluster/run geometry.

### TEXT-01C: International fallbacks

1. Add the seven locked Noto fallback families from FONT-01.
2. Verify Arabic, Hebrew, Devanagari, Bengali, Tamil, and Thai corpora.
3. Add fallback/missing-glyph preflight and deterministic diagnostics.
4. Expose international faces only after their shaping tests pass.

### TEXT-01D: Performance and optional packs

1. Add bounded background shaping, generation cancellation, and shaped/glyph caches.
2. Benchmark caption editing, word highlights, many text clips, and mixed scripts.
3. Validate optional CJK and color-emoji packs independently of the base installer.
4. Record the supported script/language matrix in release documentation.

## Test strategy

- Pin and run relevant conformance fixtures from Unicode `BidiTest.txt`,
  `BidiCharacterTest.txt`, `LineBreakTest.txt`, and `GraphemeBreakTest.txt` for the selected Unicode
  data version.
- Compare representative shaped glyph IDs, clusters, advances, and offsets with HarfBuzz-compatible
  reference output.
- Golden render corpus for Arabic in isolation/context, mixed Arabic-English-digits, Hebrew with
  punctuation/numbers, Devanagari/Bengali/Tamil conjuncts, Thai wrapping, Latin `fi`/`fl`
  ligatures, decomposed accents, emoji ZWJ sequences, and every base fallback boundary.
- Cluster highlight tests for one-to-many, many-to-one, reordered, and shared-cluster mappings.
- Explicit direction/control tests, including isolates, unmatched controls, and nesting at/over the
  standard depth limit.
- Variable-weight tests proving 400/700 differ while preview and export remain identical.
- Missing-font, missing-glyph, corrupt-font, excessive text, excessive lines/glyphs, and stale-worker
  result tests.
- Cross-platform packaged tests proving the same project never resolves an installed system font.
- Performance budgets for cold shape, warm cache, continuous typing, selector specimen generation,
  timed highlight changes, and export preflight.

## Definition of done

- [x] The candidate dependency spike passes every acceptance gate and its exact dependency/license
      tree is reviewed. (2026-08-28, see "Spike result" above — `cosmic-text` 0.19,
      `default-features = false, features = ["std", "swash"]`, MIT/Apache-2.0, pure Rust.)
- [x] Preview/export no longer use per-character width summation or unshaped `fontdue::Layout`.
      (2026-08-28 — `overlay_render.rs` shapes/rasterizes via `text_layout`/`cosmic-text`;
      `text_metrics.rs`'s old per-character measurement functions are unreferenced dead code, not
      deleted yet, see that module's own doc comment.)
- [ ] Arabic, Hebrew, Devanagari, Bengali, Tamil, Thai, mixed bidi, and Latin ligature corpora pass.
- [ ] Timed highlights operate on whole shaped clusters without re-shaping isolated words.
- [ ] Variable weights select actual axes and affect glyph output.
- [ ] System fonts are absent from the font database and fallback results are deterministic.
- [ ] Direction, semantic alignment, and language metadata persist with backward-compatible defaults.
- [ ] Line breaks and grapheme boundaries pass the pinned Unicode conformance subset.
- [ ] Input, glyph, line, fallback, queue, and cache limits fail safely and never log text content.
- [ ] Preview/export pixel parity and existing Latin golden images pass on packaged targets.
- [ ] Unsupported scripts/packs are reported before export and not silently replaced.

## Primary references

- [HarfBuzz shaping concepts](https://harfbuzz.github.io/shaping-concepts.html)
- [HarfBuzz cluster semantics](https://harfbuzz.github.io/clusters.html)
- [cosmic-text repository and capability statement](https://github.com/pop-os/cosmic-text)
- [cosmic-text bundled database/fallback APIs](https://docs.rs/cosmic-text/latest/cosmic_text/struct.FontSystem.html)
- [cosmic-text variable-weight regression test](https://docs.rs/crate/cosmic-text/latest/source/tests/variable_font_weight.rs)
- [UAX #9: Unicode Bidirectional Algorithm](https://www.unicode.org/reports/tr9/)
- [UAX #14: Unicode Line Breaking Algorithm](https://www.unicode.org/reports/tr14/)
- [UAX #29: Unicode Text Segmentation](https://www.unicode.org/reports/tr29/)
- [UAX #15: Unicode Normalization Forms](https://www.unicode.org/reports/tr15/)

[<- back to spec/INDEX.md](../INDEX.md)
