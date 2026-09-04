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

/// The "Primary" button variant from `CINECUT_Design_System_v1.0.md`'s Section 9 / `CINECUT_
/// UI_UX_SPEC_v1.0.md`'s Section 18 (Export Button) — accent-filled, reserved for the one
/// dominant action on a screen (Export, New Project, Confirm, Apply), not every button. Every
/// other button stays a plain `ui.button(...)`, which already inherits the correct secondary/
/// ghost look from `theme::apply`'s global `Visuals`.
///
/// Hover/press use the spec's own `accent-hover`/`accent-active`, scoped to just this button
/// (`ui.scope`, same pattern `icon_button`'s `hover_color` option already uses) rather than
/// through `theme::apply`'s global hovered/active `Visuals` — those drive every widget in the
/// app, not just the one dominant accent-filled action this component is reserved for. Same
/// scoping for the spec's own Primary Button geometry (height 32px, padding 12px/8px) — the
/// app-wide default is a tighter 8px/4px, correct for the Secondary/Ghost buttons that make up
/// the rest of the app.
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    ui.scope(|ui| {
        let style = ui.style_mut();
        let widgets = &mut style.visuals.widgets;
        widgets.hovered.weak_bg_fill = theme::ACCENT_HOVER;
        widgets.hovered.bg_fill = theme::ACCENT_HOVER;
        widgets.active.weak_bg_fill = theme::ACCENT_ACTIVE;
        widgets.active.bg_fill = theme::ACCENT_ACTIVE;
        style.spacing.button_padding = egui::vec2(12.0, 8.0);
        ui.add(
            egui::Button::new(RichText::new(text).color(theme::TEXT_PRIMARY))
                .fill(theme::ACCENT)
                .min_size(egui::vec2(0.0, 32.0)),
        )
    })
    .inner
}
