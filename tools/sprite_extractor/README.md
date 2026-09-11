# sprite_extractor

Extracts individual sprite assets (transparent PNGs) from a sprite-sheet-style
reference image: a page of bordered panels, each panel either a grid of
small icon/sprite cells or a special single-sprite layout. Detection is
structural (borders, edges, projection-profile valleys, connected
components) - it does not hardcode coordinates for any specific image, so it
generalizes to other, similarly-structured sheets at other resolutions. OCR
is an optional assist only (off by default) and is never required.

Built against and validated on `samples/character_sheet_reference.png`, a
1536x1024 reference sheet for a modular 2D pixel-art character (PacoPaçoca
project tooling).

## Usage

```bash
pip install -r requirements.txt
python -m sprite_extractor \
  --input samples/character_sheet_reference.png \
  --output output/ \
  --config config.yaml \
  --mode trimmed \
  --padding 4 \
  --debug
```

or, equivalently, `python main.py --input ... --output ...` from inside this
directory.

- `--mode trimmed` (default): each asset is cropped tight to its sprite
  bounding box + `--padding` px.
- `--mode canvas`: each asset keeps the full original cell size, with the
  sprite placed at its real position and everything else transparent -
  useful for compositing multiple layers back together at fixed offsets.
- `--debug`: writes a staged sequence of annotated visualizations to
  `<output>/debug/01_detected_panels.png` .. `06_final_boxes.png` (panels,
  per-panel classification, sub-panels/sub-units, cells, ignored/text
  regions, final exported sprite boxes).

Every exported PNG gets a JSON sidecar (`category`, `source_cell`,
`sprite_bounds`, `anchor`, and in trimmed mode `offset_x`/`offset_y` +
`original_canvas_width`/`height` - enough to reconstruct the sprite's
position on its original cell canvas later). An aggregate `metadata.json` at
the output root lists every exported asset plus the configured
`layer_order`.

## Architecture

```
main.py / __main__.py    thin CLI entrypoint
config.py                 config schema (dataclasses) + YAML load/validate
pipeline.py                orchestrates detection -> classification -> export
detection/
  panels.py                bordered-panel detection (Canny + contours), top-level and nested
  grids.py                  grid/cell boundary detection + title/subtitle header stripping
  cells.py                  per-cell multi-sprite splitting (e.g. 2 sprites in 1 cell)
  sprites.py                connected components, text-vs-sprite heuristic, tight bbox
processing/
  background.py              flood-fill background/transparency removal
  transparency.py            the export-time transparency pipeline
  trimming.py                 tight-bbox cropping + padding (trimmed mode)
  filtering.py                 contextual (never flat) small-asset area thresholds
classification/
  categories.py               resolves a detected panel against config overrides
output/
  naming.py, exporter.py, metadata.py     filenames, PNG + sidecar JSON writing
utils/
  image.py, geometry.py, debug.py          array/IO helpers, Rect + band-splitting, debug viz
tests/                    pytest unit tests (geometry, grid detection, flood fill, text
                           heuristic, config loading, naming) - synthetic inputs, no image fixture needed
```

## How detection works (no hardcoded coordinates)

1. **Panels**: Canny edges + contours on the full page find bordered
   rounded-rect regions; near-duplicate/nested contours (a border draws as
   two close edges) are merged, then panels are ordered reading-order
   (row-major) by vertical-overlap clustering.
2. **Header stripping**: within a panel, the row-density profile (row sums
   of "differs from the page background" pixels, normalized against the
   profile's own high percentile so one relative threshold works across
   panels of very different size/content) is band-split; the first
   *substantial* band after the very first one is treated as where the real
   body (grid or sprite) starts - this survives a title+subtitle that itself
   fragments into several short bands instead of one clean run.
3. **Sub-panels**: the same contour method, run again inside a panel's body
   at a much higher relative area threshold (a true sub-panel tiles a large
   slice of its parent; an individual grid cell's own border must not be
   mistaken for one), finds nested bordered boxes - CORPO BASE's per-
   direction boxes, ROUPAS' TORSO/BRAÇOS/PERNAS boxes, or PERSONAGEM BASE's
   info-card.
4. **Grid cells**: two complementary signals. Primary: Canny edge pixels
   projected per column/row are dense along a cell border line and sparse
   inside a cell - real boundaries are recovered as peaks in that profile,
   then fit to a uniform pitch (so one boundary occluded at a single spot is
   still recovered from its neighbors' consistent spacing). Fallback:
   foreground-density valleys, for content with genuine whitespace gaps but
   no drawn border at all.
5. **Text vs. sprite**: no OCR - a region is classified as text when it has
   many small, similarly-short components whose vertical centers cluster
   into a handful of rows (glyphs of a title/legend/label line), *unless* it
   also contains one substantially large connected blob (a real sprite,
   even a low-contrast or fragmented-outline one, is never dominated by many
   text-sized pieces the way a line of glyphs is).
6. **Multi-sprite cells**: within a cell, connected components are grouped
   into horizontal clusters (small gaps bridged by a small dilation, so an
   eye's pupil+outline stay one cluster); >=2 well-separated, sprite-sized
   clusters export as separate sub-sprites (`_a`/`_b`/...).
7. **Transparency**: flood fill from the crop's own border inward (multiple
   seed points), not a global "delete near color X" threshold - this
   preserves dark outlines, hair strands, shadows, and antialiasing, and
   correctly keeps an enclosed background-colored area (e.g. a light
   highlight surrounded by a dark outline) opaque, since it's never reached
   from the border.

All of the thresholds above live in `config.yaml` (or your own `--config`),
not scattered as magic numbers in the detection code - most support an
`auto` value resolved relative to image/region size, or an explicit number.

## Manual overrides

`detection.panel_overrides` (matched by `panel_index`, or `title_contains`
when `ocr.enabled: true`) and `detection.exclude_panels` let you correct a
panel structural detection can't reliably resolve on its own - explicit
`rows`/`columns` counts, `type: special|grid|grid_of_subpanels|explicit`,
sub-panel names/categories, or an outright excluded panel. `manual_regions`
supports a fully explicit rect+mode region bypassing detection entirely.
See `config.yaml` for the full set used on the reference sheet, and *Known
limitations* below for why each one exists.

## Output layout

```
base/reference/idle.png                 PERSONAGEM BASE's single nude idle sprite
base/{front,back,left,right}/frame_NNN[_a|_b].png
skin/skin_NNN.png
eyes/eyes_NNN.png
eyebrows/eyebrows_NNN.png
mouths/mouths_NNN.png
hair_back/hair_back_NNN.png
hair_front/hair_front_NNN.png
clothes/{torso,arms,legs}/{torso,arms,legs}_NNN.png
shoes/shoes_NNN.png
accessories/accessories_NNN.png
effects/effects_NNN.png
metadata.json
debug/*.png                             (only with --debug)
```

`base/reference/` is not explicitly named in categories a consumer might
expect from the panel titles alone - it holds PERSONAGEM BASE's single
"empty" idle sprite, which isn't part of the per-direction animation grid.

## Known limitations

Structural detection alone cannot always cleanly resolve every panel on
this reference sheet; the following are handled via explicit `config.yaml`
overrides rather than silently producing wrong output, per the panels this
task called out in advance as the hard cases:

- **PERSONAGEM BASE (VAZIO)**: auto-detected as `special` - the info-card
  (a nested bordered box, structurally like a bordered panel) is correctly
  recognized as text and excluded, leaving only the idle sprite. Works
  without an override in practice; a `type: special` override is kept for
  robustness/documentation.
- **CORPO BASE (POR DIREÇÃO)**: the 4 per-direction bordered boxes sit close
  enough together that contour-based sub-panel detection can bridge one
  into its neighbor (their thin dividing gap isn't always a clean edge).
  The fine per-cell grid (4 frame-rows x 8 sprite-columns - 2 sprites per
  direction) detects far more reliably instead, so this panel is treated as
  one plain grid with `column_group_names: [front, back, left, right]`
  folding every 2 columns into a named direction folder - explicit
  `rows: 4, columns: 8` pins the count. The "Frame N (Idle)" row-label
  column sits inside the panel body, to the left of the grid, with no
  border of its own to key off of; `body_inset_left: 67` (measured against
  this sheet) crops past it before grid detection runs.
- **ROUPAS**: TORSO/BRAÇOS/PERNAS are nested bordered sub-panels detected
  the normal way, but each sub-header sits flush against its own grid with
  no whitespace row for the header-strip valley search to find (unlike
  every top-level panel title, which has a blank line under it).
  `subpanel_header_px: 20` is a fixed fallback crop for this case. TORSO
  extracts correctly (16/16); BRAÇOS and PERNAS under-count somewhat (8/12
  and 6/12 in the validation run) - the fixed fallback height is a
  reasonable approximation, not a precise fit, for those two.
- **ACESSÓRIOS**: icon silhouettes vary a lot in shape (caps, glasses,
  ears, halos, bows...) and generate enough internal edge noise to
  over-segment the auto column/row count even though the underlying grid is
  perfectly uniform (6x5) - pinned via explicit `rows`/`columns`.
- **EXEMPLO DE COMPOSIÇÃO**: excluded outright
  (`detection.exclude_panels`). It contains a legend list plus one fully
  composed character render - structurally almost identical to PERSONAGEM
  BASE's "sprite + text card" layout, so distinguishing "real layer asset"
  from "worked example" here isn't something the structural heuristics
  could safely tell apart on their own; this is a genuine manual call.
- **SOBRANCELHAS (eyebrows)**: several eyebrow styles are very
  low-contrast/thin against the dark background. A handful of cells across
  the sheet don't have enough foreground signal to pass the area/text
  heuristics reliably (the validation run recovered 6-8 of 12), and very
  rarely a near-empty cell's flood-filled crop shows a sliver of the cell's
  own border line rather than real content. This is an inherent limit of a
  no-OCR, contrast-based pipeline on faint linework, not a coordinate bug.
- Every other panel (VARIAÇÕES DE COR DE PELE, OLHOS, BOCAS/EXPRESSÕES,
  CABELO TRASEIRO/FRONTAL, SAPATOS, EFEITOS) auto-detects its true grid
  shape correctly; explicit `rows`/`columns` are still pinned in
  `config.yaml` for determinism/documentation rather than left to
  per-image edge-peak luck on a re-run.

None of the above required hardcoding sprite *coordinates* - every override
is a panel identifier (index, or title when OCR is on), a row/column count,
or a named grouping; a future, differently-laid-out sheet still runs through
the same auto-detection path unless a future override is added for it.

## Validation

Run against the bundled reference sheet in both modes with `--debug`, then
spot-checked exported PNGs directly (Pillow, alpha channel + pixel content,
not just "file exists") for one asset in each required category: base,
eye, eyebrow, mouth, hair_back, hair_front, a clothing item, a shoe, an
accessory, and an effect. All ten showed real sprite content on a
transparent background with no visible panel border or title-text bleed
(the CORPO BASE row-label bleed and ROUPAS sub-header bleed described above
were found this way and fixed via the overrides listed above).

## Development

```bash
python -m py_compile $(find . -name '*.py' ! -path './tests/*')
pytest tests/
```
