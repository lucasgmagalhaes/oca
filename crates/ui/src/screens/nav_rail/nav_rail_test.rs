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

//! Headless UI tests for custom-painted navigation controls. They exercise the same AccessKit
//! role/name contract consumed by screen readers and the Windows UI Automation E2E suite.

use super::*;
use eframe::egui::accesskit::Role;
use egui_kittest::{kittest::Queryable as _, Harness};
use std::cell::Cell;

#[test]
fn custom_rail_button_is_discoverable_and_clickable_by_its_accessible_name() {
    let clicked = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        if rail_button(ui, false, "⌂", egui::FontFamily::Proportional, "Home").clicked() {
            clicked.set(true);
        }
    });

    harness.get_by_role_and_label(Role::Button, "Home").click();
    harness.run();

    assert!(
        clicked.get(),
        "the accessible rail button must receive an activation"
    );
}
