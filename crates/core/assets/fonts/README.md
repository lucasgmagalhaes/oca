# Bundled fonts

The current directory contains 15 families: the original six plus FONT-01B slice 1 (the
architecture doc's Sans table). The reviewed expansion target,
official acquisition source, exact 43-family selection, package budget, license rules, manifest
design, and migration plan live in
[`spec/architecture/built-in-font-catalog.md`](../../../../spec/architecture/built-in-font-catalog.md).
Complex shaping, bidi, cluster-safe highlights, and international fallback are specified in
[`spec/architecture/complex-text-shaping.md`](../../../../spec/architecture/complex-text-shaping.md).
Do not add a family here without updating that specification and the future locked catalog
manifest together.

These font files are downloaded from the official
[Google Fonts repository](https://github.com/google/fonts) and embedded into `avcore` for
deterministic text rendering. Each family is distributed under the SIL Open Font License 1.1;
its unmodified `OFL.txt` is stored beside the corresponding font files.

| UI category | Family | Bundled files | Google Fonts source |
| --- | --- | --- | --- |
| Modern sans | Lato | Regular, Bold | `ofl/lato` |
| Display/title | Bebas Neue | Regular | `ofl/bebasneue` |
| Serif | Playfair Display SC | Regular, Bold | `ofl/playfairdisplaysc` |
| Handwritten | Patrick Hand | Regular | `ofl/patrickhand` |
| Monospace | Anonymous Pro | Regular, Bold | `ofl/anonymouspro` |
| Bold captions | Archivo Black | Regular (designed black weight) | `ofl/archivoblack` |
| Sans | Inter | Variable (`opsz,wght`) | `ofl/inter` |
| Sans | Montserrat | Variable (`wght`) | `ofl/montserrat` |
| Sans | Roboto | Variable (`wdth,wght`) | `ofl/roboto` |
| Sans | Open Sans | Variable (`wdth,wght`) | `ofl/opensans` |
| Sans | Poppins | Regular, Bold | `ofl/poppins` |
| Sans | Nunito | Variable (`wght`) | `ofl/nunito` |
| Sans | Source Sans 3 | Variable (`wght`) | `ofl/sourcesans3` |
| Sans | Barlow | Regular, Bold | `ofl/barlow` |
| Sans | Fredoka | Variable (`wdth,wght`) | `ofl/fredoka` |

Single-weight families intentionally map both runtime style requests to their one designed
face. The UI disables the Bold selector for those families. A variable family's single locked
binary carries every weight/width axis in one file — `crates/core/src/font_catalog.rs`'s
`FontFace::weight`/`FontFamilyEntry::default_weight` records only the Regular (400) instance
selected at load time (FONT-01C's own "official axis-selection UI" is still open; today's
Editor properties panel only offers the fixed default/bold weight either way).

Vendored from `google/fonts` commit `ade3d1533e06b2b1462ffcde8e08b129627ca360` (the pinned
`avcore::font_catalog::GOOGLE_FONTS_REVISION`), same as the original six.
