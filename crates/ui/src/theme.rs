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

//! The dark/violet color palette and the egui `Visuals` it's applied through, per
//! `OCA_Design_System_egui.md` (repo root) — this is Implementation Priority #1 from that doc
//! ("theme tokens"), the highest-leverage single-file change every screen inherits for free
//! through the existing `theme::TOKEN` references. Screens reference these constants directly
//! rather than going through egui's default palette, so the whole app reads as one consistent
//! theme. See that doc's own sections 2/3/5/6 for the exact source values; every constant below
//! carries the doc's own token name in its doc comment for traceability.
//!
//! Deliberately **not** done in this pass (see the doc's own Implementation Priority list and
//! this repo's plan notes): typography (Inter/IBM Plex Mono aren't bundled yet), the `cine_*`
//! component-catalog rename (this crate's existing `components::` module already covers the same
//! ground under different names), and per-screen geometry (tool rail/track-header/timeline-
//! header sizes) — each is its own separate, scoped follow-up.

use egui::{Color32, CornerRadius, Shadow, Stroke, Visuals};

/// `bg_canvas` — application background.
pub const BG: Color32 = Color32::from_rgb(0x09, 0x0a, 0x0d);
/// `bg_workspace` — main workspace. Not yet wired to a distinct call site (every screen today
/// reads `BG` or `SURFACE`); kept available as its own token since the doc's palette treats it
/// as a real, separate step between `BG`/`SURFACE`.
pub const BG_WORKSPACE: Color32 = Color32::from_rgb(0x0d, 0x0e, 0x12);
/// `bg_panel` — side panels.
pub const SURFACE: Color32 = Color32::from_rgb(0x11, 0x12, 0x18);
/// `bg_elevated` — menus, popovers, dialogs. Not yet wired to a distinct call site (menus/
/// dialogs currently read `SURFACE`/`window_fill`); kept available for a future pass that gives
/// elevated surfaces their own fill.
pub const BG_ELEVATED: Color32 = Color32::from_rgb(0x15, 0x16, 0x1d);
/// `bg_control` — inputs and controls.
pub const SURFACE_2: Color32 = Color32::from_rgb(0x19, 0x1a, 0x21);
/// `bg_hover` — hovered controls. Not yet wired to a distinct call site (hover currently reuses
/// `SURFACE_2`); kept available for a future pass that separates control-fill from hover-fill.
pub const BG_HOVER: Color32 = Color32::from_rgb(0x1d, 0x1e, 0x26);
/// `border_subtle` — very subtle separators. Not yet wired to a distinct call site.
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x1b, 0x1c, 0x23);
/// `border_default` — standard structure.
pub const BORDER: Color32 = Color32::from_rgb(0x24, 0x26, 0x30);
/// `border_strong` — active structure/dialogs. Not yet wired to a distinct call site.
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x30, 0x32, 0x3d);
/// `text_primary` — main labels.
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xe8, 0xe9, 0xed);
/// `text_secondary` — secondary information.
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x9a, 0x9d, 0xa8);
/// `text_tertiary` — metadata / hints.
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x62, 0x65, 0x71);
/// `text_disabled` — disabled controls. Not yet wired to a distinct call site (disabled widgets
/// currently fall back to egui's own default disabled styling).
pub const TEXT_DISABLED: Color32 = Color32::from_rgb(0x3e, 0x40, 0x4a);
/// `accent_primary` — primary interaction. Global, high-blast-radius constant: selection
/// highlighting, hovered/active widget strokes, the active tool-button fill, tags, etc. all
/// derive from it.
pub const ACCENT: Color32 = Color32::from_rgb(0x92, 0x70, 0xff);
/// `accent_hover`. Not yet wired to a distinct call site (hovered widgets currently derive their
/// stroke from `ACCENT` directly via `apply()`'s `Visuals` setup).
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0xa1, 0x84, 0xff);
/// `accent_active` — pressed. Not yet wired to a distinct call site (active/pressed widgets
/// currently derive their fill from `ACCENT.gamma_multiply(..)` via `apply()`).
pub const ACCENT_ACTIVE: Color32 = Color32::from_rgb(0x7c, 0x5c, 0xe0);
/// `accent_muted` — selected background. Opaque per the doc's own hex (previously a
/// semi-transparent premultiplied tint derived from the old teal/violet accent) — every existing
/// call site uses this as a flat `.fill(...)` on a selected/active chip, so an opaque fill reads
/// the same or cleaner, not a regression.
pub const ACCENT_TINT: Color32 = Color32::from_rgb(0x21, 0x1c, 0x31);
/// Not covered by the design doc's own token table (no `accent_secondary`/equivalent listed) —
/// kept at its prior value. Used for the timeline's transition-wedge fill and similar secondary
/// accents that shouldn't compete with `ACCENT` itself.
pub const ACCENT_2: Color32 = Color32::from_rgb(0x4f, 0x7c, 0xe0);
/// `media_video`. Not yet wired to a distinct call site (video clips currently render via
/// `SURFACE_2`); kept available for a future timeline-color pass.
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
/// `state_success`. Not yet wired to a distinct call site.
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
/// `space_xl` — major groups. Not yet wired to a distinct call site.
pub const SPACE_XL: f32 = 20.0;
/// `space_2xl` — section separation. Not yet wired to a distinct call site.
pub const SPACE_2XL: f32 = 24.0;
/// `space_3xl` — major layout separation. Not yet wired to a distinct call site.
pub const SPACE_3XL: f32 = 32.0;

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

/// Applies the dark/violet palette to the egui context, per `OCA_Design_System_egui.md`'s own
/// Section 7 theme baseline.
pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);

    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE;
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
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(RADIUS_SM as u8);

    visuals.widgets.hovered.bg_fill = SURFACE_2;
    visuals.widgets.hovered.weak_bg_fill = SURFACE_2;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(RADIUS_SM as u8);

    visuals.widgets.active.bg_fill = ACCENT.gamma_multiply(0.3);
    visuals.widgets.active.weak_bg_fill = ACCENT.gamma_multiply(0.3);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.active.corner_radius = CornerRadius::same(RADIUS_SM as u8);

    visuals.window_corner_radius = CornerRadius::same(RADIUS_MD as u8);
    visuals.window_stroke = Stroke::new(1.0, BORDER);
    visuals.menu_corner_radius = CornerRadius::same(RADIUS_MD as u8);
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
