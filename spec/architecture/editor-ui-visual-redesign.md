# Editor UI Visual Redesign — OCA Mockup Mapping

Source: a single reference image (`OCA` mockup, generated externally, dropped into the
project folder as a design target — not a file checked into this repo) depicting a full NLE
layout: menu bar, tool rail, media bins, program monitor, inspector, and timeline. Per the
request that produced this doc: **the image wins wherever it disagrees with the current UI
code** — but it must still land on top of oca's real data model and existing widget
conventions (`components/*`, `theme.rs`), not invent a parallel one. This doc is the mapping
from image regions to real `App`/`avcore` state, with every color claim backed by actually
sampling the source PNG's pixels (see "Color system" below), not eyeballing it.

Read this alongside `.claude/CLAUDE.md`'s "Before changing UI"/"Non-negotiable UI principles"
sections before implementing any part of it.

**Reference image**: [`assets/editor-ui-visual-redesign-mockup.png`](assets/editor-ui-visual-redesign-mockup.png)
— the actual source PNG the "Color system" measurements below were sampled from, and the only
place to check any layout/icon/spacing detail this doc didn't call out in text.

## Icon set

The mockup's icon glyphs (tool rail, transport row, track headers, top bar) aren't custom art
— cropping and zooming into each one (see below) shows they match **Lucide**
(`lucide.dev`/`lucide-icons/lucide`, ISC-licensed overall, with a subset inherited from Feather
under MIT — both permissive) glyph-for-glyph, not an approximation. That means the right source
for these icons is Lucide's own vector files, not a hand-traced reconstruction from a ~40px
raster crop — tracing pixels would only recreate what already exists cleanly as a vector.

`assets/icons/*.svg` in this directory are the real, unmodified Lucide source files (24×24,
`stroke="currentColor"`, 2px stroke — trivially recolorable to any `theme.rs` token by whatever
renders them), fetched from `lucide-icons/lucide`'s `icons/` directory and matched to mockup
regions as follows:

| File | Mockup location | Note |
|---|---|---|
| `scissors.svg` | Tool rail — Cut | matches `EditorTool`'s cut action exactly |
| `lock.svg` | Timeline track header — lock icon | matches `track.locked`'s existing 🔒 glyph |
| `chevron-left.svg` / `chevron-right.svg` | Timeline track header — `< >` collapse | matches existing collapse-arrow affordance |
| `settings.svg` | Top bar — gear icon | matches `nav_rail.rs`'s existing ⚙ Prefs entry |
| `camera.svg` | Preview transport row — snapshot | new capability per "Program monitor" section above |
| `skip-forward.svg` | Preview transport row — seek-to-end | matches existing seek-to-end affordance |
| `mouse-pointer-2.svg` | Tool rail — Select | matches `EditorTool::Select` |
| `fold-horizontal.svg` | Tool rail — Trim/Ripple | best-guess slot resolved by pixel-comparing the mockup's ~20px glyph against several Lucide candidates (`dumbbell`, `infinity`, `link-2`, `chevrons-left-right`, `move-horizontal`) cropped and upscaled with nearest-neighbor scaling — `fold-horizontal`'s two inward-pointing chevrons plus center gap was the closest match; `move-horizontal`'s arrows point outward (expand), which is the opposite direction from what the mockup shows |
| `type.svg` | Tool rail — Text tool | matches `EditorTool` text entry |
| `wand-sparkles.svg` | Tool rail — Effects | Lucide renamed `wand-2` → `wand-sparkles` upstream; confirmed via directory listing (`wand-2.svg` 404s, `wand-sparkles.svg` 200s) |
| `hand.svg` | Tool rail — Pan | no current `EditorTool` equivalent, see "Left icon rail" section above |
| `lock-open.svg` | Timeline track header — unlocked state | pairs with `lock.svg` for `track.locked`'s two states |
| `eye.svg` / `eye-off.svg` | Timeline track header — visibility toggle | pairs for `track.visible`'s two states |
| `ellipsis-vertical.svg` | Timeline track header — ⋮ menu | Lucide renamed `more-vertical` → `ellipsis-vertical` upstream; confirmed via directory listing the same way as `wand-sparkles` above |
| `chevron-down.svg` | Top bar — "Sequence: Interview ▾" breadcrumb dropdown | |
| `upload.svg` | Top bar — Export button | |
| `skip-back.svg` / `rewind.svg` / `pause.svg` / `play.svg` / `fast-forward.svg` / `repeat.svg` | Preview transport row | rest of the transport row alongside the existing `skip-forward.svg` |
| `music.svg` | Preview transport row — rightmost small icon | resolved the doc's earlier `share-2` guess: pixel-cropping this icon at native resolution shows two note-heads joined by a beam, i.e. Lucide's `music` (two circles + one connecting path), not a share icon — likely an "add/detach audio" affordance, not yet mapped to a real `App` action |

All 24 mockup icon glyphs identified so far are now vendored. Fetch method used throughout:
`api.github.com/repos/lucide-icons/lucide/contents/icons/<name>.svg` (JSON envelope, base64
`content` field, works but has a low unauthenticated rate limit — hit it once this session
after ~20 requests spent partly on identifying `fold-horizontal`/`music` above) and, once that
limit was hit, `raw.githubusercontent.com/lucide-icons/lucide/main/icons/<name>.svg` directly
via `curl` — which, contrary to this doc's earlier note, works fine from a shell `curl` call and
isn't subject to the same rate limit; the earlier "direct .svg URLs don't work in this
environment" finding was specific to whatever fetch tool produced that empty-response behavior,
not a property of the URLs themselves.

**Decided, not yet implemented**: getting these into the actual running app. `egui` here has
zero SVG rendering capability today (checked `crates/ui/Cargo.toml` — no `resvg`/`usvg`/
`egui_extras` `svg` feature), so the vendored `.svg` files above are source assets, not yet
wired into any screen. Two routes were considered — (a) add an SVG-rasterization dependency and
convert each icon to a texture at startup, or (b) an **icon font**: bundle the Lucide glyphs as
a font with Unicode-private-use-area codepoints, rendered exactly like every existing
single-glyph icon already is via `RichText`/`painter.text`. **(b) is the chosen route** — same
runtime cost as (a) (`epaint`'s glyph atlas caches a rasterized icon exactly like it caches any
other font glyph, so cost is paid once per codepoint, not per frame), but zero new rendering
pipeline or dependency, and it reuses the exact mechanism `theme.rs`/existing screens already use
for single-glyph icons (⚙, 🔒, 👁, etc.) instead of introducing a second, image-based one
alongside it.

Mechanics of the chosen route, for whoever implements it:

1. **Build a font from the vendored SVGs** — this does not need a new `Cargo.toml` dependency:
   the conversion (SVG paths → TTF glyphs with PUA codepoints) is a one-off/regenerate-on-demand
   step, not something the running app or its build (`cargo build`) needs to do. A small script
   outside the Rust toolchain (e.g. `fantasticon`/`fonttools` via Node or Python, invoked
   manually or from a `make` target like the existing `make fmt`/`make lint`) reads
   `assets/icons/*.svg`, regenerates `lucide.ttf` + the name→codepoint mapping, and both get
   checked in like any other asset — the same "generate once, commit the output" shape as, say,
   a schema-generated file. Adding a new icon later means: fetch its SVG into `assets/icons/`
   the same way this doc's fetch process did, add its name to the script's icon list, rerun the
   script, commit the regenerated `.ttf` and the updated mapping — no Rust dependency added or
   touched.
2. **Register it with egui** — `egui::FontDefinitions::font_data` + a dedicated
   `FontFamily::Name("icons")`, set once via `ctx.set_fonts(...)` at app startup, alongside
   whatever `theme.rs`/font setup already runs there.
3. **Render each icon** — `egui::RichText::new('\u{E0xx}').family(FontFamily::Name("icons".into())).size(..).color(theme::TOKEN)`,
   the same call shape every other themed label in this codebase already uses. Color, size, and
   opacity come from the existing text-rendering pipeline for free — no separate tint/blend code
   path the way an image-texture icon would need.
4. **Caveat carried over from the SVGs themselves**: this only works because every vendored
   icon is a single-color `stroke="currentColor"` glyph (true for all 24 fetched above) — a
   route that needs a two-color icon later would need texture-based rendering instead.

**Font-build step: done.** `tools/icon-font/` is a small standalone Node/TypeScript project
(not a Cargo dependency — `make icon-font` or `npm run build` in that directory) whose
`build-icon-font.ts` reads `assets/icons/*.svg`, assigns each icon a stable PUA codepoint
(existing codepoints are read back from the previous output and never reassigned — only a
newly-added icon gets a new one, the next free PUA slot), and calls `fantasticon`'s Node API
to emit `crates/ui/assets/fonts/lucide-oca.ttf` + `lucide-oca.json` (the name→codepoint
mapping, both checked in). All 26 vendored icons are in the font as of this commit.

**`ctx.set_fonts` wiring: done too.** `crates/ui/src/icons.rs` embeds the generated `.ttf`
(`include_bytes!`) and JSON mapping (`include_str!`), registers the font under a dedicated
`FontFamily::Name("lucide-oca")` via [`icons::install`], called once from `App::new` right
after `theme::apply`, and exposes one `char` constant per icon (`icons::LOCK`, `icons::EYE`,
etc.) for call sites to use with `RichText::new(icons::LOCK).family(icons::family())`. A unit
test in that module cross-checks every constant against the embedded JSON mapping, so a
`make icon-font` rerun that reassigns a codepoint fails a test instead of silently drawing the
wrong glyph somewhere.

**First per-screen call sites: done.** `components::icon_button` gained a third
`family: Option<FontFamily>` option (applied to the `RichText` when set, `None` keeps today's
default proportional font — every existing call site is unaffected) so an icon-font glyph
renders through the same shared button component instead of a parallel one. Wired so far, only
where a vendored icon has a confirmed mockup mapping *and* a real existing `App` action:
timeline track header's visibility toggle (`eye`/`eye-off`, video tracks only — audio tracks
keep the 🔊/🔇 emoji, no Lucide speaker icon is vendored) and lock toggle (`lock`/`lock-open`),
and the nav rail's Prefs gear (`settings`). `icons.rs` also gained `_STR` twins of its `char`
constants (egui's text APIs take `&str`, not `char`), kept in sync by a test.

**Toolbar icon+label buttons: done.** `components::icon_label_job` builds a two-section
`egui::text::LayoutJob` (icon-font glyph, then a space, then the plain-text label — each with
its own `TextFormat`, since `RichText`/`icon_button` can only carry one font per string) and a
new `tool_button_icon_font` (same active/inactive chip styling as `tool_button`) renders it.
Wired: Select (`mouse-pointer-2`), Cut (`scissors`), Trim (`fold-horizontal` — the mockup's
resolved best-guess for the shared Trim/Ripple slot, applied to Trim only; Ripple keeps its own
distinct button). Ripple/Roll/Slip/Slide have no vendored icon and keep their unicode glyphs
via the unchanged `tool_button`.

**Preview transport row: done.** Seek-to-start/play-pause/seek-to-end in both the normal and
fullscreen preview transport rows now render `skip-back`/`play`/`pause`/`skip-forward` through
the icon font instead of their unicode glyphs — all map to existing `App` actions
(`seek_preview`, `toggle_preview_playback`) unchanged, no new behavior. The icon-only seek
buttons use `icon_button`'s `family` option; the hand-built play/pause `RichText` (not routed
through `icon_button`) gets `.family(icons::family())` directly. Decorative, non-interactive
"▶" placeholders elsewhere (media-library thumbnail kind glyph, empty-preview state) are left
as-is — not transport controls, no confirmed mockup mapping of their own.

**Media library favorite toggle: done.** `star` (Lucide's actual `star.svg`, fetched the same
way as the other 25) is the 26th vendored icon, added after the original mockup survey
specifically to back the Favorites filter's per-asset toggle button (`App::toggle_asset_favorite`)
— not present in the mockup's own icon set, since the mockup never showed a per-asset favorite
affordance up close. `IconButtonOpts` gained a fourth `color: Option<Color32>` field for this:
the star's *resting* color (not just its hover color, which `hover_color` already covered)
carries the favorited/unfavorited state — `theme::ACCENT` when on, `theme::TEXT_MUTED` when
off — replacing what was originally shipped as a plain `"★"`/`"☆"` unicode-glyph toggle.

Still not wired: `chevron-left`/`chevron-right` (no real "collapse a timeline track row"
feature exists today to attach them to — the mockup mapping here may be aspirational, worth
re-checking against the reference image before building), `ellipsis-vertical` (no track
⋮-menu exists), `chevron-down`/`upload` (top bar breadcrumb/Export — no top bar redesign done
yet), the rest of the transport row (`camera`/`rewind`/`fast-forward`/`repeat` — camera has no
snapshot-capture action yet, rewind/fast-forward/repeat have no scrub-speed/loop actions yet),
`type`/`wand-sparkles`/`hand` (Text/Effects/Pan tool-rail slots — `hand` also has no
`EditorTool` equivalent yet, see "Left icon rail" above), and `music` (unmapped to any real
action).

## Headline finding: the structure is already ~80% there

`screens/editor/mod.rs` already implements the mockup's macro-layout almost exactly:
toolbar row → sequence tabs → three-column body (media library / preview / properties) →
resizable timeline strip. This isn't a coincidence — comments in `nav_rail.rs`, `mod.rs`
(`asset_thumb`, `preview_panel`) reference an `oca-editor-mock.html` design comp this layout
was already built against, and `theme.rs`'s palette is that same comp's dark/teal system.
The OCA image is a _second_, more elaborate mockup of the same idea (closer to a
Premiere/Resolve reference), not a from-scratch redesign. So most of this doc is about
**restyling and extending** existing panels, not replacing them.

## Color system

`theme.rs`'s current background/surface/border values (`BG #12161c`, `SURFACE #1a2029`,
`BORDER #2b3340`) are already almost pixel-identical to the mockup's own background
(measured: `#11141a` dominant, i.e. `rgb(17,20,26)`) — no change needed there.

The one real, measured difference is the accent. Sampled directly from the source PNG
(Export button fill, timecode readout, and the single largest saturated-color cluster in the
whole image — 3849 matching pixels, by far the dominant chrome accent):

```
rgb(112, 88, 228) → #7058E4   (indigo/violet)
```

This replaces `theme::ACCENT`'s current teal (`#2ea39e`). This is a global, high-blast-radius
change — `ACCENT` drives selection highlighting, hovered/active widget strokes, the active
tool-button fill, tags (`tag_accent`), the playhead-draggable-layer border, hyperlinks, etc.
Recommend: change the constant, then do a visual pass (screenshot comparison, not just
"compiles") across Home/Library/Queue/Editor before calling this done — a single-constant
change with this much fan-out is exactly the kind of thing that looks fine in isolation and
wrong somewhere unexpected.

**Real risk found by measuring, not guessing**: `timeline_panel/mod.rs`'s default (non-
color-labeled) clip fill for audio clips is `theme::ACCENT.gamma_multiply(0.5)` / `0.6`
(`mod.rs:417-422`) — i.e. it's currently _derived from_ `ACCENT`, on purpose, so audio clips
render as a dim teal. But the mockup's own audio-track fill, sampled cleanly from the
`Ambient_Score.wav` lane (a large, photo-free, uniform region — 15k+ matching pixels):

```
rgb(45, 74, 65) → #2D4A41   (dark teal-green)
```

That's a _different_ hue family from the new violet accent, not a dimmed version of it. If
`ACCENT` is simply repointed to `#7058E4`, every audio clip silently goes violet instead of
staying teal-green — a real regression the mockup itself doesn't call for. **Action**:
decouple audio-clip fill from `ACCENT` into its own token (e.g. `theme::AUDIO_TINT =
Color32::from_rgb(0x2d, 0x4a, 0x41)`) before repointing `ACCENT`, and update the two call
sites at `timeline_panel/mod.rs:417-422` to use it instead of `theme::ACCENT.gamma_multiply(..)`.

Other sampled regions, for reference (lower confidence — these areas mix in photographic
thumbnail content, so the numbers are the _cleanest_ pixel cluster found in each region, not
a guaranteed single flat fill):

- Video clip body (the `B-Roll_City.mp4` clip's label strip, away from its thumbnail):
  `rgb(38, 52, 89) → #263459` — a dark indigo, richer than the current plain
  `theme::SURFACE_2` (`#212836`) video clips render with today. Minor, low-priority tweak;
  not worth a new token, `SURFACE_2` could just be nudged, or left as-is — the difference is
  subtle enough that it's a genuine judgment call, not a measured requirement the way the
  audio-tint finding above is.
- The "FX" lane blocks (`Color Grade`/`Vignette`): `rgb(62, 42, 26) → #3E2A1A`, dark
  amber-brown. No existing token — only relevant if the "represent effects as timeline blocks"
  question below is resolved in favor of adding one. Do not invent an `TrackKind::Fx` to use it
  — see that section.
- REC badge / playhead red: not cleanly measurable (small, anti-aliased, sits over a video
  thumbnail). Recommend reusing `theme::ERROR` (`#e0574f`) rather than sampling harder for a
  near-identical second red — the visual role (attention/recording/active) already matches
  `ERROR`'s existing meaning closely enough that a second token would be redundant.

## Region-by-region mapping

### Top bar

Mockup: logo mark + "OCA" wordmark, breadcrumb ("Project: Documentary / Sequence:
Interview ▾"), a full desktop menu bar (File Edit View Sequence Clip Markers Graphics Window
Help), then timecode / FPS / resolution chips, an Export button, a settings gear.

Current (`breadcrumb.rs`): app name, screen title, active project name + unsaved-changes dot,
window controls (min/max/close), locale code.

**Menu bar — done, for the seven menus with a real feature behind them.**
`screens/editor/menu_bar.rs` adds a real `egui::MenuBar` above the toolbar (confirmed with the
user first: it **coexists** with the toolbar, nothing removed from it — the open question below
is resolved, not left silently unaddressed). Pure UI wiring, no new `App`/`avcore` work — every
item calls straight into a method the toolbar, a context menu, or a keyboard shortcut already
exercised:

- **File** → Save (`Ctrl+S`), Export (`Screen::Queue`), Export SRT, Export Collaboration
  Bundle, Import Gameplay Events.
- **Edit** → Undo/Redo (`app.undo()`/`redo()`, hover text from the configurable `KeyCombo`),
  Copy/Cut/Paste, Copy/Paste Formatting.
- **View** → checkboxes for the Timeline Index panel, Transcript panel, and preview scopes,
  plus a fullscreen-preview button.
- **Sequence** → add tab, rename/duplicate/move-left/move-right/delete (same enablement rules
  as `sequence_tab_bar()`'s own context menu — delete needs 2+ sequences, move needs room to
  move), plus Add Video Track.
- **Clip** → split, an edit-mode submenu (Select/Trim/Ripple/Roll/Slip/Slide, mirroring the
  toolbar's tool buttons), merge-into-composite, save-as-template, the Templates browser,
  detach audio, speed ramp (slow→fast/fast→slow presets — the custom-ramp modal isn't wired
  here, only the two toolbar/context-menu presets are), and color label (reusing
  `timeline_panel`'s own `CLIP_COLOR_LABEL_PALETTE`, now `pub(super)`, instead of a second
  copy that could drift). Everything past split is gated on a clip actually being selected.
- **Markers** → add Standard/To Do/Chapter (the same three kinds the Timeline Index panel's
  own add buttons offer — no Highlight button exists there either, so none was invented here)
  plus a Timeline Index panel checkbox. "Search/seek" stays inside that panel, same as before —
  a live text filter isn't a menu command.
- **Graphics** → Add Text Track/Clip, Add Shape Track/Clip, Draw Custom Shape.

**Window/Help — still not built**, exactly as this doc originally found: `PanelLayout`/
`LayoutScope` exists as data but has no UI to expose as a "Window" menu yet, and there's no
About/docs dialog anywhere in the app for "Help" to open. Neither is faked with an empty menu.

Timecode/FPS/resolution chips: real data already exists (`format_timecode`, the preview's
decoded texture size, the active sequence's frame rate) — currently displayed in the preview
panel's transport row, not the top bar. Moving/duplicating into the top bar is a layout change,
not a new-data problem.

### Left icon rail — important disambiguation, not a straight mapping

The mockup's far-left vertical strip (select arrow, scissors, ripple icon, T, wand, hand) is
**not** the same thing as this app's actual left rail. oca already has two, different, things
that both plausibly map onto that one mockup column:

1. `nav_rail.rs` — the real app's global screen switcher (Home/Editor/Library/SoundLibrary/
   Queue/WatchFolder + Prefs), always visible, vertical, icon-only, at the far left. This has
   **no equivalent in the mockup at all** — the mockup only ever depicts one screen (a single
   NLE workspace), so it has nothing to say about cross-screen navigation.
2. The Editor toolbar's tool buttons (`EditorTool::Select/Trim/Ripple/Roll/Slip/Slide` +
   the inline Cut button) — currently a _horizontal_ row under the top bar
   (`toolbar()`/`tool_button()`), not a vertical rail.

The mockup's icon set (arrow=Select, scissors=Cut, a trim-like glyph=Trim/Ripple, T=text tool,
wand=effects, hand=pan) lines up much better with (2) than (1) — but drawn vertically, at the
window's left edge, in the position `nav_rail` currently occupies.

**Open decision, not silently resolved**: either (a) keep the toolbar tools horizontal (as
now) and leave `nav_rail` doing screen-switching in that left-edge slot — smaller change,
matches "reuse existing primitives" — or (b) move the Editor's tool buttons into a new
vertical rail in that position, and relocate `nav_rail`'s screen-switching elsewhere (a menu?
a slimmer strip above it?) for when the user isn't in the Editor screen. (b) is a materially
bigger layout change and displaces a working, real navigation affordance the mockup simply
doesn't depict — because it not once shows a non-Editor screen. Recommend (a) unless there's
a specific reason to want the vertical tool rail; this doc takes no side, per "confirm
forks with the user before a big item" (`spec/ROADMAP.md`'s own convention).

"Hand" (pan/pan-and-zoom tool) has no oca equivalent today — not a currently existing
`EditorTool` variant. Real gap, not just a restyle, if kept.

### Media panel

Mockup: MEDIA / Bins / Favorites / Recent tabs, search + filter + grid/list view toggle icons,
"Project Media" header with item count, a 2-column grid of asset cards with real-looking video
thumbnails (an actual decoded frame per asset, not a placeholder), duration/resolution/fps
caption per card, a bottom bar with count + view controls.

Current (`media_library_panel()` in `screens/editor/mod.rs`): a single-column list, search box
(`app.media_search`, already wired), a smart-bin/Favorites/Recent filter-chip row
(`app.media_filter`, a `MediaLibraryFilter` enum — see below), each row using a real per-asset
thumbnail once decoded (`resolve_thumbnail`, reusing the pre-existing `request_thumbnail`/
`pump_thumbnail_queue`/LRU pipeline also used by `screens::library`/`screens::home`; falls back
to the flat-color placeholder — video=grey block with "▶", audio=blue-tinted block with "♪" —
until the real frame arrives).

Mapping:

- **MEDIA tab** ↔ the existing list, as-is.
- **Bins tab** ↔ the existing smart-bin chip row (`SmartBin`, already fully implemented per
  `ROADMAP.md` P4 item 22) — could become its own tab instead of an inline chip row, a layout
  choice, not a new feature.
- **Favorites / Recent tabs — done.** `MediaAsset` gained a manual `#[serde(default)] favorited:
  bool` flag, toggled by a vendored-icon-font `star` button (accent-colored when on, muted when
  off — see the Icon set section's own bullet) on each list row and grid tile
  (`App::toggle_asset_favorite`) — purely user-driven, not derived from usage. "Recent" tracks
  assets added to the timeline: `Project` gained `#[serde(default)] recent_asset_ids: Vec<u64>`
  (an MRU list, most-recent-first, deduplicated, capped at `RECENT_ASSET_CAPACITY` = 20 — the
  same MRU-`Vec` shape as `Prefs::recent_project_paths`, not a timestamp scheme), updated via
  `Project::record_recent_asset` from both `App::add_asset_to_timeline`/`add_asset_to_timeline_at`
  every time a clip is dropped onto the timeline. Both scope decisions ("manual flag" over
  "auto-favorite on repeated use"; "added to timeline" over "imported" or "browsed") were made
  explicitly, not guessed.

  The panel's filter state is unified into one `App::media_filter: MediaLibraryFilter` enum
  (`All | SmartBin(u64) | Favorites | Recent`, replacing the old single-purpose
  `active_smart_bin_id: Option<u64>`) so the four filters stay mutually exclusive by
  construction. `Recent`'s asset list is additionally sorted by MRU position rather than the
  library's natural order. Like `MediaViewMode`/`PreviewZoom`, `media_filter` is transient
  `App` state, not persisted — resets to `All` on project switch/launch.
- **Grid/list view toggle — done.** `App::media_view_mode` (`MediaViewMode::{List, Grid}`, not
  persisted — resets on launch like `tool`/`properties_tab`) toggled via two `▦`/`☰`
  `selectable_label`s in the panel header. `List` is the existing one-column row layout
  unchanged; `Grid` wraps assets into tiles (`ui.horizontal_wrapped`) with a bigger thumbnail
  (`GRID_ASSET_THUMB_SIZE`, 120×72 vs. the list row's 48×28) and the filename beneath instead of
  alongside. Click/double-click/drag-to-timeline behavior is identical in both layouts (shared
  via one `handle_interaction` closure) — only the tile's own content differs.
- **Real per-asset thumbnails — done**, and turned out cheaper than this doc originally scoped
  it: `App::request_thumbnail`/`pump_thumbnail_queue` (the bounded-LRU poster-frame pipeline
  built for the timeline filmstrip, `avcore::FrameSampler`-backed) already explicitly supports
  "one media library/project card poster frame" at `frame_index: 0` — `screens::library`
  (Mídia screen) and `screens::home` (project cards) already reused it exactly this way. The
  Editor's own media library panel was the one caller still on the placeholder-only path; wiring
  it in was a matter of reusing the existing pattern, not building a second thumbnailing system.
  `asset_thumb`/`asset_thumb_sized` now take an `Option<&egui::TextureHandle>` and paint the
  real poster frame when cached (`painter.image`), falling back to the placeholder glyph
  otherwise — the duration badge still overlays either way. Audio assets never request a frame
  (nothing to decode), same "kind reads at a glance" placeholder as before. Works in both List
  and Grid layouts, sharing one `resolve_thumbnail` lookup closure.

### Program monitor (preview panel)

Mockup: "PROGRAM MONITOR" label + zoom-level controls (50%/Fit/100%/...), the video frame with
a "CAM 01" chip (top-left), a "REC ●" chip (top-right), a resolution+fps overlay (bottom-left),
a timecode+frame-number overlay (bottom-right), a scrub bar with a round drag handle, and a
transport row (skip-to-start, step-back, play/pause, step-forward, skip-to-end, loop, snapshot,
marker).

Current (`preview_panel()`): already renders the decoded texture, a resolution chip
(top-left — `{w}×{h}` only, no fps), transport buttons (seek-to-start/play-pause/seek-to-end,
fullscreen, scopes toggle, audio level meter), and a scrub `egui::Slider` — real functional
overlap is high.

Mapping:

- **Zoom controls (50%/Fit/100%) — done.** `PreviewState::zoom` (`PreviewZoom::{Fit, Percent50,
  Percent100}`, not persisted — resets to `Fit` on launch) toggled via three `selectable_label`s
  in a new header row above the preview frame. `Fit` is the original always-fill-available-space
  behavior, unchanged; `Percent50`/`Percent100` instead size the canvas off the sequence's real
  pixel dimensions. The actual size math (`preview_canvas_size`) is a pure function extracted out
  of `layer_transform_preview` specifically so it's unit-testable without an `egui::Ui` — 8 real
  tests (fit/letterbox/pillarbox, both zoom percentages, portrait aspect) executed for real via
  an isolated scratch crate (only needs the `egui` crate itself, no `avcore`/GStreamer). **Scope
  limit, not a bug**: this preview has no scrollable viewport, so a zoom level bigger than the
  panel silently caps back down to whatever `Fit` would have produced — genuine 1:1-pixel
  scrolling for a canvas resolution larger than the panel is a separate, not-yet-built follow-up.
- **"CAM 01" chip — done.** `App::current_preview_multicam_angle()` checks whether the video
  track behind the previewed clip is a `MulticamGroup`'s `program_track_id` (Multicam editing,
  `ROADMAP.md` P2 item 10, was already real) and returns its 1-based angle number if so —
  `None` (chip omitted, not faked) when no group applies. Drawn top-right (the mockup's "REC"
  slot, freed up per the next bullet).
- **"REC ●" chip** — no oca equivalent, and arguably shouldn't have one: this is a live-
  recording indicator, and oca edits already-captured footage; there is no "recording" state
  to reflect. Treat as a non-goal, not a gap — this is a mockup-generator artifact (it likely
  copied a generic NLE reference image's chrome verbatim) rather than a real requirement.
- **Bottom overlays (resolution+fps, timecode+frame) — done, with a correction.** fps is *not*
  available from "the active sequence's export settings" as this doc originally guessed — oca
  has no per-sequence fps at all, only per-asset (`MediaAsset::fps`, the same field the
  properties panel's own fps row reads); `App::current_preview_fps()` reads it off the
  previewed clip's asset instead. Folded onto the existing top-left resolution chip (`{w}×{h} ·
  {fps}fps`) rather than a second overlay, since the two numbers read as one unit; a new
  bottom-right chip shows timecode (already available via `format_timecode`) plus a
  frame-within-second suffix, omitted when fps is unknown rather than guessed.
- **Scrub bar with round handle** — functionally identical to the existing `egui::Slider`;
  the round-handle look is `egui::Slider`'s own default rendering already, so this is likely
  already close — verify visually rather than assume a custom-painted widget is needed.
- **Transport additions — done: step-frame, loop, snapshot, and marker.** New
  `App::step_preview_frame(delta_frames)` (one frame at the previewed clip's fps, 30.0
  fallback) backs two new buttons (no vendored icon for single-frame step, so thin outline
  triangles in the default font). New `PreviewState::loop_enabled` + a loop-toggle button (the
  vendored `repeat` icon) restart playback from 0 instead of stopping at the timeline's end —
  `ensure_preview_loaded`'s existing "nothing covers the new playhead" branch checks it,
  gated on playback having actually been running. **Snapshot** — the vendored `camera` icon
  (present since the icon-vendoring pass but unwired until now) opens a save-file dialog
  (`rfd`, matching every other export flow's own pattern) and writes the most recently
  displayed frame as a PNG via a new `avcore::preview::VideoFrame::save_png`; `PreviewState::
  last_frame` holds a clone of the same decoded, effects-applied frame `pump_preview_frame`
  just uploaded to the egui texture (the texture upload doesn't hand pixels back, so a plain
  reference isn't enough), reset alongside the texture whenever the pipeline reopens. Toasts
  instead of writing anything when no frame has decoded yet. **Marker** — reuses the existing,
  already-real `App::add_marker_at_playhead(MarkerKind::Standard)` (the Timeline Index panel's
  own "Add" action) behind a small button using `MarkerKind::Standard`'s own icon (🔹, matching
  `modals.rs`'s `marker_kind_icon`) rather than a newly vendored one — the doc's original "no
  vendored Lucide icon" note was about a dedicated glyph, not a blocker for wiring the button
  to real behavior.

### Inspector (properties panel)

Mockup: INSPECTOR / EFFECTS / AUDIO tabs; under Inspector: item name + metadata line, then
TRANSFORM (Position X/Y, Scale, Rotation, Anchor X/Y), CROP (Left/Right/Top/Bottom %),
COMPOSITE (Blend Mode dropdown, Opacity), SPEED (% + an "Add Effect" button); a vertical
stereo (L/R) audio meter with dB ticks on the far right.

Current (`properties_panel()`): one long scrolling column, no tabs — every property
(gain, freeze, deflicker, speed, crop, mask, color filter/LUT, layer scale, vignette,
brightness/contrast/saturation, sharpen, chroma key, background removal, voice cleanup, blur,
shake, glitch, pixelize, stabilization, transitions, plus every field's own keyframe editor)
rendered via the same `components::property_section`/`property_toggle` (a collapsible
header + slider/checkbox + a muted "applies at export" note), collapsed or expanded by default
based on whether the clip already has a non-default value there.

Mapping:

- **Three-tab split (Inspector/Effects/Audio)** is a real, low-risk reorganization: group the
  _existing_ `property_section` calls into three groups (Inspector: crop/mask/layer
  scale/keyframed position-scale-rotation-opacity; Effects: color filter/LUT/vignette/
  brightness-contrast-saturation/sharpen/chroma-key/background-removal/blur/shake/glitch/
  pixelize/stabilization/transitions; Audio: gain/voice cleanup/gain keyframes) behind a
  `PropertiesTab` enum + three `match` arms. No new `App` state beyond that one enum field, no
  new `avcore` work — every section keeps using the exact same `components::property_section`
  call it already does, just under a different tab. This is the single most "just do it"
  item in this whole doc.
- **TRANSFORM → Position/Scale/Rotation**: maps directly to the existing
  `position_keyframes`/`layer_scale_x,y`/`rotation_keyframes` — already real fields, currently
  edited via `keyframe_editors::position_keyframe_editor`/sliders.
- **TRANSFORM → Anchor X/Y — done, real pivot (per explicit request, not just a position
  offset).** `ClipInstance::anchor_x`/`anchor_y` (`0.0..=1.0`, default `0.5`/`0.5` = center —
  every clip before this field existed effectively used this). Neither FFmpeg's `rotate`
  filter nor GStreamer's `rotate` element has a native off-center pivot parameter — both always
  rotate around their own input frame's center — so a real anchor needs to move *what's at that
  center*, not add a pivot parameter that doesn't exist. Every segment already goes through a
  canvas-conforming `scale=...,pad=cw:ch:(ow-iw)/2:(oh-ih)/2` stage before any per-clip filter
  (rotation included) runs; replacing that fixed `(ow-iw)/2` centering offset with
  `ow/2-(anchor_x)*iw` (and the `y` equivalent) moves *which point of the content* lands at the
  padded buffer's center — the point rotation/scale then pivot around, since `rotate` pivots on
  its own input's center. At anchor `0.5/0.5` this reduces to the exact original expression
  (verified byte-identical against real `ffmpeg` output on synthetic test images), and the
  anchor point empirically stays fixed under rotation (also verified against real `ffmpeg`
  output) — not derived from documentation or memory. Applies uniformly to every clip (unlike
  `blend_mode`/position, not overlay-only) in both `avbridge::encode_timeline_export` and
  `encode_timeline_export_multi`. **Scope limit, not a bug**: live preview doesn't reflect a
  non-default anchor yet — GStreamer has no element to build the equivalent "shift what's at
  the rotate pivot" trick on (its `rotate` element has no generic dynamic-offset pad element
  comparable to avfilter's `pad`); a real preview implementation needs a `videobox`-based
  restructuring, a separate follow-up, same "editable but visually approximate" posture several
  other effects in this panel already have (LUT/vignette CPU-approximated, `RotationExportNote`
  itself already said "no live preview effect yet" before this field even existed).
- **CROP → Left/Right/Top/Bottom %**: same underlying rectangle as oca's existing
  `crop_x`/`crop_y`/`crop_w`/`crop_h` (position+size), just a different parameterization
  (edge-insets: `left=x`, `top=y`, `right=1-(x+w)`, `bottom=1-(y+h)`). Purely a units/label
  choice for the widget, not a data-model change — convert on display, store the same fields.
- **COMPOSITE → Blend Mode — done: data model, export, live preview, and UI wiring.**
  `avcore::timeline::BlendMode` (40 modes, matching FFmpeg's `blend` filter's `all_mode` option
  exactly — verified against a real `ffmpeg -h filter=blend` build) + `ClipInstance::
  blend_mode`/`ClipFormatting::blend_mode` exist now. Export wires it for real: `avbridge::
  ClipSegment::blend_mode` (a plain mode-name string, avbridge doesn't depend on `core`) crosses
  the FFI boundary into `timeline_export_multi.c`'s overlay filtergraph, where a non-`Normal`
  mode builds a `blend=all_mode=<name>` stage in place of the usual `overlay=x=...:y=...` stage
  for that layer — FFmpeg's own filter does the per-pixel math, so this is genuinely all 40
  modes for zero C code per mode, not a bounded subset. **Scope limit, not a bug**: `blend` has
  no x/y placement option, so `position_x_expr`/`position_y_expr` (PIP-style repositioning) are
  ignored for a layer with a non-`Normal` blend mode — it composites at full canvas size.
  Combining positioning with a blend mode is a separate, not-yet-built follow-up. `blend`'s two
  inputs are labeled "top" (#0, the layer whose mode is set) and "bottom" (#1, the accumulated
  stack beneath it) — verified against a real `ffmpeg -h filter=blend` build, not assumed; this
  matters because operand order changes the result for every asymmetric mode (subtract, divide,
  burn, dodge, overlay, ...).
  **Live preview done, via a different mechanism than export**: GStreamer's `compositor`
  element has no equivalent to FFmpeg's `blend` filter at all — only Porter-Duff
  `source`/`over`/`add` (verified via `gst-inspect-1.0 compositor`) — so a blend-mode overlay
  branch is instead diverted at pipeline-build time into its own dedicated `appsink` (forced to
  the canvas's own size), and `Preview::current_frame` composites that branch's frame onto the
  `compositor` output itself, on the CPU, via a new `avcore::blend_mode::blend_channel` — a
  term-for-term Rust transcription of FFmpeg's own `libavfilter/blend_modes.c` formulas (not
  reimplemented from documentation or memory), cross-checked against real `ffmpeg ...
  blend=all_mode=<mode>` output on gradient test images for all 40 modes before being trusted
  (max abs diff 0, except `interpolate` at 1 — float rounding), with those same reference values
  embedded as exact-match assertions in `blend_mode_test.rs` (executed for real in an isolated
  scratch crate — this sandbox's `core` test binary can't link, see CLAUDE.md's documented
  FFmpeg/ONNX-Runtime gaps). Same documented scope limit as export (position/PIP ignored), plus
  one preview-only limit: a blend-mode layer always composites on top of the *whole*
  `compositor` stack, not correctly interleaved with Normal-mode overlay layers above it in
  track order — combining the two needs a materially different pipeline shape, a separate
  follow-up.
  **UI wiring done**: the properties panel's Inspector tab has a Composite section (Blend Mode
  dropdown, grouped with the existing Opacity keyframe editor per the mockup) backed by a new
  `App::set_selected_clip_blend_mode`. Mode names in the dropdown are kept in English regardless
  of locale (`blend_mode_label`, not the `Text` i18n catalog `color_filter_label`/`mask_shape_
  label` go through) — deliberate: these are industry-standard names shown untranslated in a
  Portuguese UI by every mainstream NLE too, so a `Text` entry per mode would be pure `pt_br =
  en` duplication, not real localization.
  Real, executable regression coverage: `avbridge/tests/encode_test.rs`'s
  `multi_track_export_with_a_blend_mode_set_produces_a_valid_file` runs a real two-track export
  with `blend_mode: "multiply"` set and probes the result; `blend_mode_test.rs` exhaustively
  covers the CPU preview math (312 exact-value cases across all 40 modes, executed for real).
  GStreamer pipeline behavior itself (the `appsink` diversion, `current_frame`'s compositing
  loop) is only compile-checked in this sandbox — no display/audio device or test video files
  available to actually run the preview pipeline end-to-end.
- **SPEED + "Add Effect" button**: the speed slider maps directly to `speed_factor` (plus the
  richer stepped/smooth speed-ramp system oca already has, which the mockup doesn't even show
  — oca is ahead here, not behind). "Add Effect" itself doesn't map to anything: oca's model is
  "every possible effect is always a property section you toggle a value in," not
  Premiere/FCP's "start with an empty effect stack, add effects onto it." Per CLAUDE.md's "do
  not invent a new visual pattern when one already exists" — recommend dropping this button
  rather than bolting on a second, parallel effects-list mental model alongside the one that's
  already implemented everywhere else in this panel.
- **Stereo L/R audio meter with dB ticks — done, the real way.** `AudioLevel` (`avcore::
  preview`) gained `peak_l`/`rms_l`/`peak_r`/`rms_r` alongside its existing combined `peak`/
  `rms` — `build_metering_audio_sink`'s buffer probe now reads the negotiated channel count off
  the pad's own caps to de-interleave channel 0 ("L") from channel 1 ("R") instead of treating
  every buffer as one flat sequence (mono still reports the same value on both, matching a real
  hardware meter; a >2-channel source's extra channels still count toward the combined figures,
  just not split further). Two consumers: the preview transport row's compact
  `audio_level_meter()` is now two thin stacked bars (L/R) instead of one, and a new
  `stereo_db_meter()` in the properties panel's Audio tab is the mockup's actual vertical,
  dB-ticked (0/-6/-12/-24/-48, -60 dB floor) stereo meter. Neither renders the same mono value
  twice — the thing this doc originally flagged not to do.

### Timeline

Mockup: sequence tab row (matches oca's existing `sequence_tab_bar` almost exactly, down to
the "+" add button); a ruler with absolute timecodes; track headers (V3/V2/V1/A1/A2/FX) with
lock/eye icons and `<>` collapse arrows; V1/V2 as filmstrip-thumbnail clips with purple
transition wedges between adjacent clips; A1/A2 as green waveforms with white diamond
keyframe markers on a volume envelope line; a dedicated "FX" lane holding "Color Grade"/
"Vignette" blocks, each with its own keyframe diamonds; a red vertical playhead line with a
flag/tab at the top.

Current (`timeline_panel/mod.rs`): sequence tabs, ruler with click/drag-to-scrub + magnetic
snap (clip edges, markers, and — for trim drags — waveform low-energy points), per-track
header row with a real lock icon (🔒/🔓, `track.locked`, already wired to
`app.toggle_track_locked`) and a real eye/visibility icon (👁, `track.visible`) — **this part
is already a 1:1 match**, not a gap. Filmstrip thumbnails (`draw_filmstrip`) and waveforms
(`draw_waveform`) already render per clip. Keyframe markers already render
(`draw_keyframe_markers`) for any field with a `Vec<Keyframe<T>>`. The playhead already
renders (`draw_playhead`).

Mapping:

- **Track headers, lock/eye, collapse** — already real, just verify the visual styling
  (icon glyphs, spacing) against the mockup rather than re-wiring anything functional.
- **Filmstrip + transition wedges — done.** Filmstrips were already real (`draw_filmstrip`);
  the mockup's purple wedge between adjacent clips is now painted too, by a new
  `draw_transition_wedge` (a filled bowtie shape over the clip's incoming edge, sized by
  `transition_duration_secs * px_per_sec`, drawn whenever `ClipInstance::has_transition()` —
  `transition_in`, fade/slide/zoom — is set). `transition_in` itself already applied at
  export+preview per `matrix/effects-and-color.md`; this was purely the missing on-strip
  visual, not a new transition feature. No pixel-sampled color exists for this region in the
  source mockup, so it reuses `theme::ACCENT_2` rather than inventing an unsampled token.
- **A1/A2 green waveforms with diamond keyframe volume envelope — done.** `draw_waveform` was
  already real; `draw_keyframe_markers` (previously position/scale/rotation/opacity only, all
  video-transform fields) now also includes `gain_keyframes` — the one of those five lists
  that's ever populated on an audio clip — so its diamonds now show directly on the waveform's
  volume-envelope line, not only in the properties panel's keyframe editor.
- **A dedicated "FX" track/lane** — **no `TrackKind::Fx` exists, and shouldn't be added to
  match this mockup literally**. oca's effects (color grade fields, vignette, etc.) are
  per-clip properties on ordinary Video-track clips, not separate overlay clips on their own
  adjustment-layer-style track — a materially different editing model (Premiere/Resolve-style
  adjustment layers vs. oca's per-clip property model). Recommend **not** adding an FX track
  kind for this. If a "which effects are active on this clip, at a glance" affordance is
  wanted, a small badge/indicator drawn on the V1/V2 clip itself (which effects have non-
  default values) fits oca's existing model far better than a fake second timeline track that
  doesn't correspond to any real independently-movable object.
- **Playhead — done.** `draw_playhead` was switched from `theme::ACCENT` to `theme::ERROR`
  before the `ACCENT` repoint landed, so it stayed red-adjacent instead of following the accent
  from teal to violet.

## What NOT to change (explicit non-goals)

Per CLAUDE.md's "do not design a generic SaaS dashboard" / "keep contextual tools contextual"
principles, and this doc's own findings above:

- Don't add `TrackKind::Fx` / adjustment-layer clips — no real editing-model equivalent, see
  Timeline section above.
- Don't add "Add Effect" — see Inspector section above.
- Don't reproduce the "REC" chip — no live-recording concept in this app.

## Suggested implementation order

1. **Color — done.** `theme::AUDIO_TINT` added, `timeline_panel/mod.rs`'s two audio-clip-fill
   call sites decoupled onto it, `theme::ACCENT` repointed to `#7058E4`.
2. **Inspector tab split (Inspector/Effects/Audio) — done.** Pure reorganization of existing
   `property_section` calls behind a new `App::properties_tab` (`PropertiesTab`), zero new
   `avcore` work.
2.5. **Icon font — done.** `tools/icon-font/build-icon-font.ts` converts the vendored Lucide
   SVGs to `crates/ui/assets/fonts/lucide-oca.ttf` + a codepoint mapping; `icons.rs` registers
   it with egui and wires it into every call site that has both a confirmed icon *and* a real
   existing `App` action: timeline track header (lock/eye), nav rail (settings), toolbar
   (Select/Cut/Trim, via a new `icon_label_job`/`tool_button_icon_font` for the icon+label
   combo), and the preview transport row (seek-to-start/play-pause/seek-to-end). Not wired:
   anything needing a new feature (camera snapshot, loop toggle, scrub speed) or an unresolved
   layout decision (menu bar, track collapse, tool rail restructuring) — see the Icon set
   section's own "still not wired" list.
3. **Timeline visual polish — done.** Transition wedges (`draw_transition_wedge`) and
   gain-keyframe diamonds on waveforms (extended `draw_keyframe_markers`); playhead already
   verified red (`theme::ERROR`) post-accent-change.
4. **Preview panel — done.** Resolution+fps/timecode+frame HUD overlays, step-frame + loop +
   snapshot + marker transport buttons, zoom controls (50%/Fit/100%), and a CAM chip wired to
   real `MulticamGroup` data (omitted when none applies) — see the Program monitor section's
   own bullets for the full breakdown.
5. **Menu bar — done.** `screens/editor/menu_bar.rs`, coexisting with the toolbar (confirmed
   with the user first, per the open decision this doc originally left). See the Top bar
   section's own bullets for exactly which menu items are wired vs. which two menus (Window,
   Help) still have no real feature behind them.
6. Everything flagged as a genuine new feature above — its own scoped follow-up item, not part
   of "implement the mockup" in one pass. (Snapshot capture, the grid/list view toggle, zoom
   controls, real per-asset thumbnails, Anchor X/Y, and Favorites/Recent were all picked out of
   this list and shipped — see items 9-14 below.) The marker button was never in this list (it
   maps to an already-real feature, just unwired) but shipped alongside snapshot capture in the
   same slice.
7. **Per-channel audio metering — done**, picked out of item 6's list at the user's request —
   see the Inspector section's own bullet.
8. **Blend Mode — done.** Data model, export, live preview (the full FFmpeg 40-mode set, per
   the user's own request), and UI wiring all shipped, also picked out of item 6's list at the
   user's request — see the Inspector section's own bullet for what shipped and its documented
   scope limits (position/PIP ignored while a blend mode is active; a blend-mode layer always
   composites on top of the whole Normal-mode stack rather than interleaving into track order).
9. **Media library grid/list view toggle — done.** See the Media library section's own bullet.
10. **Preview transport snapshot capture + marker button — done.** See the Program monitor
    section's "Transport additions" bullet.
11. **Preview zoom controls (50%/Fit/100%) — done.** See the Program monitor section's own
    bullet.
12. **Media library real per-asset thumbnails — done.** See the Media library section's own
    bullet.
13. **Anchor X/Y — done, export only.** See the Inspector section's TRANSFORM bullet for what
    shipped and its documented live-preview scope limit.
14. **Media library Favorites/Recent filters — done.** See the Media library section's own
    bullet for the scope decisions (manual favorite flag; "recent" = added to timeline) and
    what shipped.

## Verification

- `make fmt` / `make lint` / `make check` after any change (per CLAUDE.md).
- Any `theme.rs` change needs a visual pass across every screen that reads `theme::ACCENT`
  (`Grep -r "theme::ACCENT\b"` first, to get the actual call-site count before assuming the
  blast radius is small).
- `ui`'s unit tests (`cargo test -p ui`) for anything with new `App` state (view-mode toggle,
  properties-tab enum, loop-enabled flag, etc.) — same bar every other `App`-level addition in
  this codebase already meets.
- No e2e/live-GUI verification is possible in a sandboxed session (per CLAUDE.md's own
  build-environment notes) — flag every visual claim in a PR description as "verified via
  `cargo check`/screenshot-diff," not "confirmed in the running app," unless it actually was.
