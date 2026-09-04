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

use egui::{Response, RichText, Ui};

use crate::theme;

/// The "Primary" button variant from `OCA_Design_System_egui.md`'s Section 8 — accent-filled,
/// reserved for the one dominant action on a screen (Export, New Project, Confirm, Apply), not
/// every button. Every other button stays a plain `ui.button(...)`, which already inherits the
/// correct secondary/ghost look from `theme::apply`'s global `Visuals`.
///
/// Hover/press use the doc's own `accent_hover`/`accent_active`, scoped to just this button
/// (`ui.scope`, same pattern `icon_button`'s `hover_color` option already uses) rather than
/// through `theme::apply`'s global hovered/active `Visuals` — those drive every widget in the
/// app, not just the one dominant accent-filled action this component is reserved for.
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    ui.scope(|ui| {
        let widgets = &mut ui.style_mut().visuals.widgets;
        widgets.hovered.weak_bg_fill = theme::ACCENT_HOVER;
        widgets.hovered.bg_fill = theme::ACCENT_HOVER;
        widgets.active.weak_bg_fill = theme::ACCENT_ACTIVE;
        widgets.active.bg_fill = theme::ACCENT_ACTIVE;
        ui.add(
            egui::Button::new(RichText::new(text).color(theme::TEXT_PRIMARY)).fill(theme::ACCENT),
        )
    })
    .inner
}
