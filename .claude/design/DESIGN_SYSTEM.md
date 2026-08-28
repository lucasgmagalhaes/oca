# Design System

## Personality

Professional, precise, focused, calm, dense enough for creative software. Reference category:
mature desktop creative tools, not marketing websites or generic SaaS dashboards.

## Hierarchy

Visual priority:
1. active content and current editing context;
2. preview and timeline;
3. selected object and active controls;
4. navigation and persistent chrome;
5. secondary metadata.

Matches the real layout: `screens/editor/mod.rs` puts preview center, media library and
properties as side columns, timeline as a full-width strip below — chrome (`breadcrumb.rs`,
`nav_rail.rs`) stays thin and peripheral.

## Color — real tokens, `crates/ui/src/theme.rs`

The semantic roles below already exist as named `Color32` constants — **use these, don't
introduce a parallel ad-hoc palette**:

| Role | Token |
|---|---|
| Application/panel background | `BG` |
| Panel surface | `SURFACE` |
| Elevated surface | `SURFACE_2` |
| Subtle separator | `BORDER` |
| Primary text | `TEXT_PRIMARY` |
| Secondary text | `TEXT_SECONDARY` |
| Disabled/muted text | `TEXT_MUTED` |
| Accent/selection | `ACCENT`, `ACCENT_TINT`, `ACCENT_2` |
| Warning/error | `ERROR`, `ERROR_TINT` |

Dark/teal palette overall. `theme::apply(ctx)` builds the egui `Visuals` (panel/window fill,
per-state widget visuals for noninteractive/inactive/hovered/active, corner radii, stroke) and
sets global spacing (`item_spacing`, `button_padding`). A new component needing a color it can't
express with the table above is a signal to add a new named token to `theme.rs`, not to inline a
`Color32::from_rgb(...)`.

Timeline media categories (e.g. per-track `color_label: Option<[u8;3]>`) are user-assignable RGB,
not fixed semantic tokens — that's intentional (user-organized color coding), don't try to force
them into the semantic palette above.

## Spacing

Centralized via `theme::apply`'s `item_spacing`/`button_padding` on `ctx.all_styles_mut` — reuse
the global style, don't set per-widget spacing overrides unless a screen genuinely needs a
documented exception.

## Borders and shadows

Use separators (`BORDER` token) to establish workspace structure. Avoid putting every component
in a bordered rounded rectangle — `components/frame.rs`/`section.rs` are the existing grouping
primitives; reach for those before inventing a new bordered container.

## Typography

Optimize for dense scanning and extended use. Differentiate hierarchy through weight, size and
contrast (as seen in the transcript panel's current-word/low-confidence-word color+italics
treatment) rather than decorative styling.

## States

Every interactive component needs intentional idle/hover/active/selected/focused/disabled/error
states — `theme::apply`'s per-state `Visuals` config is the mechanism for the default egui
widgets; custom-painted elements (timeline clips, markers) must express these states manually
through the same token palette (e.g. `ACCENT` for selected, `TEXT_MUTED` for a
disabled/low-confidence item, as the transcript panel and clip selection already do).

## Icons

Icons must have consistent metaphor, sizing and alignment. Important or ambiguous actions should
include labels or tooltips — the toolbar buttons (`screens/editor/mod.rs`) mix emoji glyph +
short label text (e.g. "✂️ Edições de fala/Speech Edits") rather than icon-only controls; follow
that convention for new toolbar actions rather than going icon-only.
