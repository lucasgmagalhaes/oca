# Bundled fonts

The current directory contains the first six-family catalog. The reviewed expansion target,
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

Single-weight families intentionally map both runtime style requests to their one designed
face. The UI disables the Bold selector for those families.
