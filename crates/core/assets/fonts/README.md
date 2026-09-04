# Bundled fonts

The current directory contains all 43 families FONT-01B targets: the original six plus the full
Sans, Display/condensed/gaming, Serif, Handwritten, Monospace, and International tables from
the architecture doc. The reviewed expansion target, official acquisition source, exact
43-family selection, package budget, license rules, manifest design, and migration plan live in
[`spec/architecture/built-in-font-catalog.md`](../../../../spec/architecture/built-in-font-catalog.md).
Complex shaping, bidi, cluster-safe highlights, and international fallback are specified in
[`spec/architecture/complex-text-shaping.md`](../../../../spec/architecture/complex-text-shaping.md).
Do not add a family here without updating that specification and the locked catalog manifest
(`crates/core/src/font_catalog.rs`) together.

These font files are downloaded from the official
[Google Fonts repository](https://github.com/google/fonts) and embedded into `avcore` for
deterministic text rendering. Each family is distributed under the SIL Open Font License 1.1;
its unmodified `OFL.txt` is stored beside the corresponding font files.

| UI category | Family | Bundled files | Google Fonts source |
| --- | --- | --- | --- |
| Modern sans | Lato | Regular, Bold | `ofl/lato` |
| Sans | Inter | Variable (`opsz,wght`) | `ofl/inter` |
| Sans | Montserrat | Variable (`wght`) | `ofl/montserrat` |
| Sans | Roboto | Variable (`wdth,wght`) | `ofl/roboto` |
| Sans | Open Sans | Variable (`wdth,wght`) | `ofl/opensans` |
| Sans | Poppins | Regular, Bold | `ofl/poppins` |
| Sans | Nunito | Variable (`wght`) | `ofl/nunito` |
| Sans | Source Sans 3 | Variable (`wght`) | `ofl/sourcesans3` |
| Sans | Barlow | Regular, Bold | `ofl/barlow` |
| Sans | Fredoka | Variable (`wdth,wght`) | `ofl/fredoka` |
| Display/title | Bebas Neue | Regular | `ofl/bebasneue` |
| Bold captions | Archivo Black | Regular (designed black weight) | `ofl/archivoblack` |
| Display | Oswald | Variable (`wght`) | `ofl/oswald` |
| Display | Anton | Regular | `ofl/anton` |
| Display | Barlow Condensed | Regular, Bold | `ofl/barlowcondensed` |
| Display | League Spartan | Variable (`wght`) | `ofl/leaguespartan` |
| Display | Teko | Variable (`wght`) | `ofl/teko` |
| Display | Black Ops One | Regular | `ofl/blackopsone` |
| Display | Russo One | Regular | `ofl/russoone` |
| Display | Bangers | Regular | `ofl/bangers` |
| Serif | Playfair Display SC | Regular, Bold | `ofl/playfairdisplaysc` |
| Serif | Merriweather | Variable (`opsz,wdth,wght`) | `ofl/merriweather` |
| Serif | Libre Baskerville | Variable (`wght`) | `ofl/librebaskerville` |
| Serif | Lora | Variable (`wght`) | `ofl/lora` |
| Serif | Cinzel | Variable (`wght`) | `ofl/cinzel` |
| Serif | Bitter | Variable (`wght`) | `ofl/bitter` |
| Handwritten | Patrick Hand | Regular | `ofl/patrickhand` |
| Handwritten | Caveat | Variable (`wght`) | `ofl/caveat` |
| Handwritten | Pacifico | Regular | `ofl/pacifico` |
| Handwritten | Dancing Script | Variable (`wght`) | `ofl/dancingscript` |
| Handwritten | Comic Neue | Regular, Bold | `ofl/comicneue` |
| Handwritten | Gloria Hallelujah | Regular | `ofl/gloriahallelujah` |
| Monospace | Anonymous Pro | Regular, Bold | `ofl/anonymouspro` |
| Monospace | JetBrains Mono | Variable (`wght`) | `ofl/jetbrainsmono` |
| Monospace | Roboto Mono | Variable (`wght`) | `ofl/robotomono` |
| Monospace | Space Mono | Regular, Bold | `ofl/spacemono` |
| International | Noto Sans Arabic | Variable (`wdth,wght`) | `ofl/notosansarabic` |
| International | Noto Naskh Arabic | Variable (`wght`) | `ofl/notonaskharabic` |
| International | Noto Sans Hebrew | Variable (`wdth,wght`) | `ofl/notosanshebrew` |
| International | Noto Sans Devanagari | Variable (`wdth,wght`) | `ofl/notosansdevanagari` |
| International | Noto Sans Bengali | Variable (`wdth,wght`) | `ofl/notosansbengali` |
| International | Noto Sans Tamil | Variable (`wdth,wght`) | `ofl/notosanstamil` |
| International | Noto Sans Thai | Variable (`wdth,wght`) | `ofl/notosansthai` |

Single-weight families intentionally map both runtime style requests to their one designed
face. The UI disables the Bold selector for those families. A variable family's single locked
binary carries every weight/width axis in one file — `crates/core/src/font_catalog.rs`'s
`FontFace::weight`/`FontFamilyEntry::default_weight` records only the Regular (400) instance
selected at load time (FONT-01C's own "official axis-selection UI" is still open; today's
Editor properties panel only offers the fixed default/bold weight either way).

Only the original six families and the Sans/Display/Serif/Handwritten/Monospace families have
a `TextFontFamily` enum variant and are selectable in the Editor's text-clip font picker today.
The 7 International (`Noto`) families are vendored and catalog-validated but not yet exposed —
the doc gates their visibility on `TEXT-01`'s own shaping tests, and separately, exposing *any*
of the 37 non-original families needs FONT-01A's still-deferred persisted-identity swap
(`.ocproj` storing a `family_id` slug instead of the enum variant name) and FONT-01D's selector
UI, neither of which this vendoring pass includes.

Vendored from `google/fonts` commit `ade3d1533e06b2b1462ffcde8e08b129627ca360` (the pinned
`avcore::font_catalog::GOOGLE_FONTS_REVISION`), same as the original six. Total bundled size:
18 MiB, inside the doc's 20 MiB release gate.
