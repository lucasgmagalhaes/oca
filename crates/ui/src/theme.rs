// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! The CINECUT dark/Petroleum-Blue color palette and the egui `Visuals` it's applied through,
//! per `CINECUT_UI_UX_SPEC_v1.0.md` (repo root, superseding `CINECUT_Design_System_v1.0.md`,
//! which itself superseded the original `OCA_Design_System_egui.md`) — Section 76's own
//! Definition of Done requires "Petroleum Blue is the only primary accent" and "purple is absent
//! from the product UI", non-negotiable per Section 1's "explicitly avoid: purple as an accent".
//! Screens reference these constants directly rather than going through egui's default palette,
//! so the whole app reads as one consistent theme. See the spec's own Section 2 for the exact
//! source values; every constant below carries the spec's own token name in its doc comment for
//! traceability.
//!
//! Deliberately **not** done in this pass: typography (Inter/IBM Plex Mono aren't bundled yet —
//! Section 3), and per-screen geometry (tool rail/track-header/timeline-header sizes, most of
//! Sections 7-58's actual pixel layout) — each is its own separate, scoped follow-up. This pass
//! covers colors/spacing/radius tokens (Sections 2/4/5) only.

use egui::{Color32, CornerRadius, Shadow, Stroke, Visuals};

/// `bg_canvas` — application background.
pub const BG: Color32 = Color32::from_rgb(0x09, 0x0a, 0x0d);
/// `bg_workspace` — main workspace, one step lighter than `BG`/`bg_canvas`. Genuinely not
/// wirable as a distinct fill without a layout change: `apply()`'s `panel_fill` plays both
/// `bg_canvas`'s role (eframe reads it as the window's own clear color, with no separate
/// call site of its own) and the one `CentralPanel` that fills the whole Editor's role at once —
/// `screens::editor::mod`'s `show()` never nests a second, inset "workspace" panel/frame that
/// could carry its own distinct fill (confirmed by reading it: media library, timeline, and
/// properties panel all share that single outer `CentralPanel`, not their own sub-panels).
/// Giving `bg_workspace` a real call site needs that restructuring first, which is a layout
/// decision, not a token-wiring one — kept reserved rather than forced onto a case that isn't
/// actually a distinct visual region yet.
#[allow(dead_code)]
pub const BG_WORKSPACE: Color32 = Color32::from_rgb(0x0d, 0x0e, 0x12);
/// `bg-panel` — major panels.
pub const SURFACE: Color32 = Color32::from_rgb(0x10, 0x12, 0x16);
/// `bg-elevated` — elevated controls, menus, popovers, dialogs. Wired via `apply()`'s
/// `window_fill`: every `egui::Window`/`Modal`/popup (menu dropdowns, `ComboBox` popups) in this
/// app renders through that one shared fill. Side panels/`card_frame()` set their own fill
/// explicitly (`SURFACE`/`bg-panel`) and aren't affected.
pub const BG_ELEVATED: Color32 = Color32::from_rgb(0x15, 0x18, 0x20);
/// `bg-control` — hover/control surfaces.
pub const SURFACE_2: Color32 = Color32::from_rgb(0x19, 0x1c, 0x23);
/// Not a spec token (the spec's own `bg-control` already covers "hover/control surfaces" as one
/// role) — kept as this codebase's own finer-grained addition, wired via `apply()`'s
/// `widgets.hovered.bg_fill`/`weak_bg_fill` so a hovered widget still reads distinctly from a
/// merely-idle control surface.
pub const BG_HOVER: Color32 = Color32::from_rgb(0x1d, 0x20, 0x27);
/// Not a spec token — this codebase's own addition for a divider even quieter than
/// `border-default`. Wired into `components::property_block`'s between-block divider
/// (`ui.separator()`, scoped via `ui.scope`) — a properties panel stacks a few dozen of these.
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x1b, 0x1c, 0x23);
/// `border-default` — normal separation.
pub const BORDER: Color32 = Color32::from_rgb(0x25, 0x29, 0x32);
/// `border-strong` — strong separation. Wired via `apply()`'s `window_stroke`, alongside
/// `BG_ELEVATED`'s `window_fill`.
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x34, 0x39, 0x45);
/// `border-accent` — focus/open/active, 40% alpha over whatever sits behind it (`#159EAD66`).
pub const BORDER_ACCENT: Color32 = Color32::from_rgba_premultiplied(0x08, 0x3f, 0x45, 0x66);
/// `text-primary` — main text.
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xe5, 0xe7, 0xeb);
/// `text-secondary` — secondary labels.
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x96, 0x9b, 0xa6);
/// `text-tertiary` — metadata.
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x62, 0x68, 0x74);
/// `text_disabled` — disabled controls. **Not wirable** through `apply()`'s global `Visuals`:
/// `ui.add_enabled(false, ...)` doesn't switch to a distinct `Widgets::disabled` visuals set (no
/// such field exists on `egui::style::Widgets` in 0.36 — only `noninteractive`/`inactive`/
/// `hovered`/`active`/`open`); a disabled widget is painted with the normal `inactive` visuals
/// and then the whole painted output's *opacity* is multiplied by `Style::disabled_alpha`
/// (`Painter::multiply_opacity`, see `Ui::disable()`) — an alpha fade, not a color swap, so
/// setting this color *inside* an `add_enabled(false)` block would double-dim it. Wired instead
/// into two hand-painted labels that sit *outside* any `add_enabled` scope but still describe a
/// genuinely unavailable state: the Editor preview pane's "Pré-visualização indisponível"
/// placeholder (`screens::editor::mod`) and the TTS modal's "no model configured" notice
/// (`app::modals`) — both previously read `TEXT_MUTED`/`text_tertiary`, which is really "less
/// important info", a different role from "this is inactive."
pub const TEXT_DISABLED: Color32 = Color32::from_rgb(0x41, 0x45, 0x4e);
/// `accent-primary` — Petroleum Blue, the official CINECUT accent (Section 2.1). Global,
/// high-blast-radius constant: selection highlighting, hovered/active widget strokes, the
/// active tool-button fill, tags, etc. all derive from it. Communicates selection/focus/active-
/// state/primary-action/current-tool — Section 74's own rule: "the interface should look mostly
/// neutral at first glance... Petroleum Blue should become noticeable only after interaction."
/// Must never be used decoratively (Section 1's "explicitly avoid: purple as an accent" — this
/// was violet before this pass; Section 76's Definition of Done requires "purple is absent from
/// the product UI").
pub const ACCENT: Color32 = Color32::from_rgb(0x15, 0x9e, 0xad);
/// `accent-hover`. Wired into `components::primary_button`'s hover state — Section 8's Primary
/// button spec pairs it with `accent-active` below, scoped to just that one dominant-action
/// button rather than every widget's global hover state (which stays `bg_hover`/`ACCENT`, see
/// `apply()`).
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x1b, 0xae, 0xbd);
/// `accent-active` — pressed. Wired into `components::primary_button`'s pressed state, same
/// scoping as `ACCENT_HOVER` above.
pub const ACCENT_ACTIVE: Color32 = Color32::from_rgb(0x12, 0x8b, 0x99);
/// Section 9's Icon Buttons color table — deliberately its own scale, not `TEXT_SECONDARY`/
/// `TEXT_PRIMARY`: an icon-only action (the vast majority of buttons in this app, via
/// `components::icon_button`) should read quieter at rest than a labeled Secondary button's
/// `text_primary` text. Wired into `icon_button`'s own scoped `Visuals` override.
pub const ICON_DEFAULT: Color32 = Color32::from_rgb(0x77, 0x7a, 0x86);
/// Icon Buttons "Hover" — see `ICON_DEFAULT`.
pub const ICON_HOVER: Color32 = Color32::from_rgb(0xc5, 0xc7, 0xcf);
/// Icon Buttons "Active" (pressed) — see `ICON_DEFAULT`.
pub const ICON_ACTIVE: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);
/// `accent-subtle` — 10% alpha over whatever sits behind it (`#159EAD1A`). Selected/active-chip
/// backgrounds (e.g. `nav_rail`'s active rail button) and Section 71/15's own "selected"
/// dropdown/menu-item background both use exactly this value, so this stays a translucent
/// overlay rather than a flattened opaque color the way an earlier pass (pre-Petroleum-Blue)
/// had it.
pub const ACCENT_TINT: Color32 = Color32::from_rgba_premultiplied(0x02, 0x10, 0x12, 0x1a);
/// `accent-strong` — 20% alpha over whatever sits behind it (`#159EAD33`). Not yet wired to a
/// distinct call site (every existing "stronger selected" case so far reaches for `ACCENT_TINT`
/// or a `gamma_multiply`'d `ACCENT` instead) — kept available since the spec names it as its own
/// token.
#[allow(dead_code)]
pub const ACCENT_STRONG: Color32 = Color32::from_rgba_premultiplied(0x04, 0x20, 0x23, 0x33);
/// Not covered by the spec's own token table (no `accent-secondary`/equivalent listed) — kept at
/// its prior value. Used for the timeline's transition-wedge fill and similar secondary accents
/// that shouldn't compete with `ACCENT` itself — Section 73's own state-hierarchy rule ("Petroleum
/// Blue should not appear simultaneously on dozens of unrelated elements") is the reason this
/// stays a separate color rather than a dimmed `ACCENT`.
pub const ACCENT_2: Color32 = Color32::from_rgb(0x4f, 0x7c, 0xe0);
/// `media_video` — default (non-color-labeled) video-clip fill on the timeline, mirroring
/// `AUDIO_TINT`'s own role for audio clips. Wired into `timeline_panel::mod`'s clip-color match
/// (`TrackKind::Video` arm), replacing a reused `SURFACE_2` that gave video clips no color
/// identity of their own.
pub const MEDIA_VIDEO: Color32 = Color32::from_rgb(0x3d, 0x5f, 0x91);
/// `media_audio` — default (non-color-labeled) audio-clip fill on the timeline. Deliberately its
/// own token rather than derived from `ACCENT` — a dimmed accent would silently follow `ACCENT`'s
/// own hue, which this doc's audio color explicitly isn't.
pub const AUDIO_TINT: Color32 = Color32::from_rgb(0x3d, 0x8a, 0x62);
/// `state_error`.
pub const ERROR: Color32 = Color32::from_rgb(0xd9, 0x5c, 0x5c);
pub const ERROR_TINT: Color32 = Color32::from_rgba_premultiplied(0x2a, 0x11, 0x10, 0x80);
/// `state_warning` — non-error caution states (e.g. a paused job) — distinct from `ERROR`
/// (failure) and `ACCENT` (selection/brand), so a warning doesn't have to borrow either's
/// meaning.
pub const WARNING: Color32 = Color32::from_rgb(0xd5, 0xa8, 0x4a);
pub const WARNING_TINT: Color32 = Color32::from_rgba_premultiplied(0x2a, 0x20, 0x0a, 0x80);
/// `state_success`. Wired into `components::tag_success` — the Fila queue's "Concluído" job
/// status, previously reusing `tag_accent`/`ACCENT` (the same color as "Rendering", conflating a
/// terminal positive outcome with an in-progress one).
pub const SUCCESS: Color32 = Color32::from_rgb(0x62, 0xb9, 0x82);
/// `state_playhead` — the timeline's playhead line. A dedicated token rather than reusing
/// `ERROR`: the doc explicitly lists them as separate roles (failure vs. a scrub/edit indicator)
/// even though they happened to share a value in this app's own prior teal/violet palette.
pub const PLAYHEAD: Color32 = Color32::from_rgb(0xff, 0x5b, 0x67);
/// Neutral informational callout background (e.g. Queue's tech-note banner) — named so a
/// second consumer doesn't reinvent `ACCENT.gamma_multiply(0.10)`. Not part of the design doc's
/// own token table; kept as this codebase's own addition.
pub const INFO_TINT: Color32 = Color32::from_rgba_premultiplied(0x0a, 0x20, 0x1f, 0x40);

/// `space_xs` — icon/text gap.
pub const SPACE_XS: f32 = 4.0;
/// `space_sm` — control padding. Matches `apply()`'s global `item_spacing`.
pub const SPACE_SM: f32 = 8.0;
/// `space_md` — component spacing.
pub const SPACE_MD: f32 = 12.0;
/// `space_lg` — panel content.
pub const SPACE_LG: f32 = 16.0;
/// `space_xl` — major groups. Genuinely reserved, not forced: no existing gap in the codebase
/// happens to already be 20px the way `SPACE_2XL`'s 24px matched `home.rs`'s literal — every
/// candidate site checked (page-title-to-content gaps) currently uses `SPACE_MD`/`SPACE_LG`
/// instead, so wiring this in would change real, currently-intentional visible spacing rather
/// than just naming an existing value. That's a per-screen geometry decision this pass's own
/// module doc explicitly scopes out, not a token-wiring one — `#[allow(dead_code)]` rather than
/// silently dropping the token or forcing an unjustified visual change to clear the warning.
#[allow(dead_code)]
pub const SPACE_XL: f32 = 20.0;
/// `space_2xl` — section separation. Wired into `home.rs`'s page-top gap (replacing a bare
/// `24.0` literal that already happened to match this value).
pub const SPACE_2XL: f32 = 24.0;
/// `space_3xl` — major layout separation. Same "genuinely reserved" status as `SPACE_XL` above —
/// no existing gap in the codebase is already 32px, and this pass's module doc explicitly scopes
/// out per-screen geometry changes.
#[allow(dead_code)]
pub const SPACE_3XL: f32 = 32.0;

/// Technical micro elements (Section 5) — meter/level-bar fills, small badges. Wired into
/// `screens::editor::mod`'s `draw_meter_bar`/`stereo_db_meter`, replacing their own hardcoded
/// `2.0`/`2` literals.
pub const RADIUS_XS: u8 = 2;
/// Standard controls/inputs/buttons/tooltips/timeline clips, per the doc's Geometry table.
pub const RADIUS_SM: u8 = 3;
/// Menus/dialogs, per the doc's Geometry table.
pub const RADIUS_MD: u8 = 4;
/// Panels — explicitly square-cornered per the doc ("Panels | 0px").
pub const RADIUS_NONE: u8 = 0;
/// Full pill shape — status tags, circular avatars. Not covered by the doc's radius table (no
/// tag/badge geometry listed); kept as this codebase's own addition rather than forced into a
/// 0/3/4px box that would visibly break every existing pill-shaped tag.
pub const RADIUS_PILL: u8 = 200;

/// Applies the CINECUT dark/Petroleum-Blue palette to the egui context, per the spec's own
/// Section 60 dialog-system baseline plus the widget states Section 6 describes generically.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);

    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.panel_fill = BG;
    visuals.faint_bg_color = SURFACE_2;
    visuals.extreme_bg_color = BG;
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = ACCENT;

    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.weak_bg_fill = SURFACE_2;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);

    visuals.widgets.inactive.bg_fill = SURFACE_2;
    visuals.widgets.inactive.weak_bg_fill = SURFACE_2;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    // Section 8's "Secondary" button spec: `bg_control`/`border_default`/`text_primary` — this
    // `inactive` slot already matched the first two; text was `TEXT_SECONDARY` (a dimmer, more
    // "metadata"-reading gray) until this fix, which every plain `ui.button(...)` (the vast
    // majority of buttons in this app — Secondary is the unstated default, `primary_button` the
    // rare exception) inherits.
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(RADIUS_SM);

    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(RADIUS_SM);

    visuals.widgets.active.bg_fill = ACCENT.gamma_multiply(0.3);
    visuals.widgets.active.weak_bg_fill = ACCENT.gamma_multiply(0.3);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.active.corner_radius = CornerRadius::same(RADIUS_SM);

    // Section 72's Menu Behavior "Open" state (`#1B2028`, subtle accent border) — egui's
    // `open` widget visuals are exactly this: a top-level `menu_button`'s own fill/stroke while
    // its dropdown is showing, distinct from a plain `hovered` menu item.
    visuals.widgets.open.bg_fill = Color32::from_rgb(0x1b, 0x20, 0x28);
    visuals.widgets.open.weak_bg_fill = Color32::from_rgb(0x1b, 0x20, 0x28);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, BORDER_ACCENT);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.open.corner_radius = CornerRadius::same(RADIUS_SM);

    // `bg_elevated`/`border_strong` — Section 8's File-menu spec ("Background: bg_elevated,
    // Border: border_strong"), which in egui terms is every `Window`/`Modal`/popup surface
    // (menu_button dropdowns, ComboBox popups) — distinct from `SURFACE`/`bg_panel`, which
    // `card_frame()` and the side panels already set explicitly and aren't affected by this.
    visuals.window_fill = BG_ELEVATED;
    visuals.window_corner_radius = CornerRadius::same(RADIUS_MD);
    visuals.window_stroke = Stroke::new(1.0, BORDER_STRONG);
    visuals.menu_corner_radius = CornerRadius::same(RADIUS_MD);
    // Doc's Section 6 "Shadows: default none" — menus/dialogs may keep a very subtle one, but
    // this app's existing borderless/flat look already reads as "none", so this just makes it
    // explicit rather than relying on egui's own default shadow.
    visuals.window_shadow = Shadow::NONE;
    visuals.popup_shadow = Shadow::NONE;

    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 4.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
    });
}
