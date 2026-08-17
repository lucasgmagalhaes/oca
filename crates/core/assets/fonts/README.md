# Bundled fonts

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
