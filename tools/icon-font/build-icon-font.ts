// Converts the vendored Lucide SVGs into an icon font for egui, per
// spec/architecture/editor-ui-visual-redesign.md's "Icon set" section (route (b): a
// Unicode-private-use-area icon font, rendered via egui's normal RichText/FontFamily path --
// not SVG-to-texture rasterization).
//
// Not part of the Rust build. Run manually (`npm run build` in this directory, or
// `make icon-font` from the repo root) whenever spec/architecture/assets/icons/*.svg changes.
// Output (the .ttf and its codepoint mapping) is committed like any other generated asset --
// re-running this script and committing the result is the whole update workflow; no Cargo
// dependency is touched.
//
// Codepoint stability: an icon's codepoint, once assigned, never changes across reruns -- the
// existing mapping (if present at OUTPUT_JSON) is loaded first and reused verbatim; only icons
// not yet in it get a new codepoint, assigned as the next free slot in the Private Use Area
// (U+E000-U+F8FF) above every codepoint already in use. This keeps a previously-fetched icon's
// codepoint stable even as new icons are added in between alphabetically, so Rust-side
// constants referencing a codepoint by value don't silently shift.

import { readdir, readFile, mkdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { generateFonts, FontAssetType, OtherAssetType } from 'fantasticon';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(HERE, '..', '..');
const INPUT_DIR = path.join(REPO_ROOT, 'spec', 'architecture', 'assets', 'icons');
const OUTPUT_DIR = path.join(REPO_ROOT, 'crates', 'ui', 'assets', 'fonts');
const FONT_NAME = 'lucide-oca';
const OUTPUT_JSON = path.join(OUTPUT_DIR, `${FONT_NAME}.json`);

const PUA_START = 0xe000;
const PUA_END = 0xf8ff;

type CodepointsMap = Record<string, number>;

async function loadExistingCodepoints(): Promise<CodepointsMap> {
  if (!existsSync(OUTPUT_JSON)) {
    return {};
  }
  const raw = await readFile(OUTPUT_JSON, 'utf-8');
  const parsed = JSON.parse(raw) as CodepointsMap;
  return parsed;
}

async function listIconNames(): Promise<string[]> {
  const entries = await readdir(INPUT_DIR);
  return entries
    .filter((entry) => entry.endsWith('.svg'))
    .map((entry) => path.basename(entry, '.svg'))
    .sort();
}

function assignCodepoints(iconNames: string[], existing: CodepointsMap): CodepointsMap {
  const assigned: CodepointsMap = { ...existing };

  let nextFree = PUA_START;
  const usedValues = new Set(Object.values(existing));
  const nextFreeSlot = (): number => {
    while (usedValues.has(nextFree)) {
      nextFree += 1;
    }
    if (nextFree > PUA_END) {
      throw new Error(
        `Ran out of Private Use Area codepoints (>${PUA_END - PUA_START + 1} icons) -- ` +
          'pick a second PUA plane instead of extending this range.',
      );
    }
    usedValues.add(nextFree);
    return nextFree;
  };

  // Drop mappings for icons that no longer exist on disk, so a removed .svg's codepoint frees
  // up for reuse instead of lingering in the committed mapping forever.
  for (const name of Object.keys(assigned)) {
    if (!iconNames.includes(name)) {
      delete assigned[name];
    }
  }

  for (const name of iconNames) {
    if (!(name in assigned)) {
      assigned[name] = nextFreeSlot();
    }
  }

  return assigned;
}

async function main() {
  if (!existsSync(INPUT_DIR)) {
    throw new Error(`Icon source directory not found: ${INPUT_DIR}`);
  }

  await mkdir(OUTPUT_DIR, { recursive: true });

  const iconNames = await listIconNames();
  if (iconNames.length === 0) {
    throw new Error(`No .svg files found in ${INPUT_DIR}`);
  }

  const existing = await loadExistingCodepoints();
  const codepoints = assignCodepoints(iconNames, existing);

  const result = await generateFonts({
    inputDir: INPUT_DIR,
    outputDir: OUTPUT_DIR,
    name: FONT_NAME,
    fontTypes: [FontAssetType.TTF],
    assetTypes: [OtherAssetType.JSON],
    codepoints,
    // Every vendored icon is Lucide's 24x24/2px-stroke grid -- normalize+round keep glyph
    // metrics consistent across icons fetched at slightly different times/tool versions.
    normalize: true,
    round: 1e3,
  });

  const finalCodepoints = result.codepoints;
  const sortedEntries = Object.entries(finalCodepoints).sort(([a], [b]) => a.localeCompare(b));

  console.log(`Wrote ${FONT_NAME}.ttf and ${FONT_NAME}.json to ${OUTPUT_DIR}`);
  console.log(`${sortedEntries.length} icons:`);
  for (const [name, codepoint] of sortedEntries) {
    console.log(`  U+${codepoint.toString(16).toUpperCase().padStart(4, '0')}  ${name}`);
  }
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
