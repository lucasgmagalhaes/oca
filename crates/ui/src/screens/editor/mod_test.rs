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

use super::preview_canvas_size;
use crate::app::PreviewZoom;
use eframe::egui;

const EPS: f32 = 0.01;

fn assert_close(actual: egui::Vec2, expected: egui::Vec2) {
    assert!(
        (actual.x - expected.x).abs() < EPS && (actual.y - expected.y).abs() < EPS,
        "expected {expected:?}, got {actual:?}"
    );
}

#[test]
fn fit_fills_the_available_space_exactly_when_aspect_already_matches() {
    let avail = egui::vec2(1000.0, 562.5); // exactly 16:9
    let size = preview_canvas_size(PreviewZoom::Fit, avail, 1920, 1080, 1920.0 / 1080.0);
    assert_close(size, avail);
}

#[test]
fn fit_letterboxes_when_available_space_is_wider_than_the_canvas_aspect() {
    // A very wide panel, 16:9 canvas -- height is the limiting dimension.
    let avail = egui::vec2(2000.0, 500.0);
    let size = preview_canvas_size(PreviewZoom::Fit, avail, 1920, 1080, 1920.0 / 1080.0);
    assert_close(size, egui::vec2(500.0 * 16.0 / 9.0, 500.0));
}

#[test]
fn fit_pillarboxes_when_available_space_is_narrower_than_the_canvas_aspect() {
    let avail = egui::vec2(400.0, 800.0);
    let size = preview_canvas_size(PreviewZoom::Fit, avail, 1920, 1080, 1920.0 / 1080.0);
    assert_close(size, egui::vec2(400.0, 400.0 * 9.0 / 16.0));
}

#[test]
fn percent_50_uses_half_the_real_canvas_resolution_when_it_fits() {
    let avail = egui::vec2(2000.0, 2000.0);
    let size = preview_canvas_size(PreviewZoom::Percent50, avail, 1920, 1080, 1920.0 / 1080.0);
    assert_close(size, egui::vec2(960.0, 540.0));
}

#[test]
fn percent_100_uses_the_real_canvas_resolution_when_it_fits() {
    let avail = egui::vec2(2000.0, 2000.0);
    let size = preview_canvas_size(PreviewZoom::Percent100, avail, 1920, 1080, 1920.0 / 1080.0);
    assert_close(size, egui::vec2(1920.0, 1080.0));
}

#[test]
fn percent_100_caps_back_to_fit_when_the_panel_is_smaller_than_the_canvas() {
    // A 1920x1080 canvas requested at 100% inside a much smaller panel -- must never exceed
    // `avail`, and must still preserve the canvas's own aspect ratio (no scrollable viewport).
    let avail = egui::vec2(300.0, 300.0);
    let size = preview_canvas_size(PreviewZoom::Percent100, avail, 1920, 1080, 1920.0 / 1080.0);
    assert!(size.x <= avail.x + EPS);
    assert!(size.y <= avail.y + EPS);
    assert_close(size, egui::vec2(300.0, 300.0 * 9.0 / 16.0));
}

#[test]
fn percent_50_caps_back_when_half_resolution_still_exceeds_the_panel() {
    let avail = egui::vec2(500.0, 200.0);
    let size = preview_canvas_size(PreviewZoom::Percent50, avail, 1920, 1080, 1920.0 / 1080.0);
    assert!(size.x <= avail.x + EPS);
    assert!(size.y <= avail.y + EPS);
    assert_close(size, egui::vec2(200.0 * 16.0 / 9.0, 200.0));
}

#[test]
fn portrait_canvas_aspect_is_preserved_at_every_zoom_level() {
    let avail = egui::vec2(2000.0, 2000.0);
    let aspect = 1080.0 / 1920.0;
    for zoom in [
        PreviewZoom::Fit,
        PreviewZoom::Percent50,
        PreviewZoom::Percent100,
    ] {
        let size = preview_canvas_size(zoom, avail, 1080, 1920, aspect);
        assert!(
            ((size.x / size.y) - aspect).abs() < EPS,
            "{zoom:?}: expected aspect {aspect}, got {}",
            size.x / size.y
        );
    }
}
