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

use eframe::egui;

use crate::theme;

/// The standard bordered/rounded card background used for project cards, media cards, and
/// export queue rows.
pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::SURFACE)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(theme::RADIUS_MD)
        .inner_margin(egui::Margin::same(12))
}

/// A structural panel background — `CINECUT_Design_System_v1.0.md`'s Section 23 ("Panels:
/// `bg-panel`, 1px `border-default`, 0px radius, no shadow... workspace panels should visually
/// read as parts of one workstation"). Distinct from [`card_frame`]: same fill/border color, but
/// square-cornered —
/// for the Editor's three real structural panels (media library, properties, timeline), not for
/// individual list-item cards. `card_frame`'s `RADIUS_MD` rounding reads correctly on a project
/// card or a queue row; it read as an anti-pattern on `timeline_panel`, which used to reuse
/// `card_frame` for exactly this and is this function's original motivating fix.
pub fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::SURFACE)
        .stroke(egui::Stroke::new(1.0, theme::BORDER))
        .corner_radius(theme::RADIUS_NONE)
        .inner_margin(egui::Margin::same(theme::SPACE_MD as i8))
}
