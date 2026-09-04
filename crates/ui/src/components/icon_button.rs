// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use eframe::egui::{self, Color32, FontFamily, Response, RichText, Ui};

/// Escape hatches for [`icon_button`] — deliberately kept small, not open-ended configurability,
/// so this stays one canonical component instead of becoming a fifth convention with extra
/// steps. `size` covers the transport controls' enlarged (48-64pt) glyphs; `hover_color` covers
/// window-chrome's close-button-red-on-hover convention; `color` covers a glyph whose resting
/// (not just hovered) color carries meaning — e.g. the Media library's favorite star, accent-
/// colored while on, muted while off; `family` selects a non-default font — today only the
/// vendored Lucide icon font (`crate::icons::family()`) for a glyph that has a real vendored
/// icon (see `spec/architecture/editor-ui-visual-redesign.md`'s Icon set section), left `None`
/// (the default proportional font) for every emoji/text glyph that doesn't. Leave all fields
/// `None` for the default, small-button-styled icon action (delete/utility buttons).
#[derive(Default, Clone)]
pub struct IconButtonOpts {
    pub size: Option<f32>,
    pub hover_color: Option<Color32>,
    pub color: Option<Color32>,
    pub family: Option<FontFamily>,
}

/// The one shared icon-button component — collapses what were four independent conventions
/// (toolbar inline `format!`, a window-chrome-only helper, transport's large colored `RichText`
/// labels, and bare `small_button` delete actions) into a single component every icon-only
/// affordance should route through. Always carries a tooltip — no icon-only button in this app
/// should be reachable without one, since a screen reader or a `.on_hover_text`-less button
/// otherwise has no accessible name.
pub fn icon_button(ui: &mut Ui, glyph: &str, tooltip: &str, opts: IconButtonOpts) -> Response {
    let text = match opts.size {
        Some(size) => RichText::new(glyph).size(size),
        None => RichText::new(glyph),
    };
    let text = match opts.family {
        Some(family) => text.family(family),
        None => text,
    };
    let text = match opts.color {
        Some(color) => text.color(color),
        None => text,
    };
    let button = if opts.size.is_none() {
        egui::Button::new(text).small()
    } else {
        egui::Button::new(text).frame(false)
    };

    let response = if let Some(color) = opts.hover_color {
        ui.scope(|ui| {
            let hovered = &mut ui.style_mut().visuals.widgets.hovered;
            hovered.fg_stroke.color = color;
            hovered.bg_stroke.color = color;
            ui.add(button)
        })
        .inner
    } else {
        ui.add(button)
    };
    response.on_hover_text(tooltip)
}
