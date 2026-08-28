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

use eframe::egui;

use crate::components;
use crate::i18n::Text;

/// Renders an editable list of `(time_fraction, value)` keyframe rows plus an "add at 1.0"
/// button and a per-row delete button — the shared UI shape for scale/rotation/opacity
/// keyframes (position needs its own two-value-per-row variant, see
/// [`position_keyframe_editor`]). Returns `Some(new_list)` if the user added, removed, or
/// edited a row this frame, `None` otherwise — the caller only calls the corresponding
/// `app.set_selected_clip_*_keyframes` setter when this is `Some`, matching every other
/// property section's "only write back on change" convention.
pub(super) fn f32_keyframe_editor(
    ui: &mut egui::Ui,
    keyframes: &[avcore::Keyframe<f32>],
    value_range: std::ops::RangeInclusive<f32>,
    default_value: f32,
    locale: crate::i18n::Locale,
) -> Option<Vec<avcore::Keyframe<f32>>> {
    let mut list = keyframes.to_vec();
    let mut changed = false;
    let mut remove_index = None;
    for (i, kf) in list.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui
                .add(
                    egui::Slider::new(&mut kf.time_fraction, 0.0..=1.0)
                        .text(Text::KeyframeTime.tr(locale)),
                )
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut kf.value, value_range.clone()))
                .changed();
            if components::icon_button(
                ui,
                "🗑",
                Text::RemoveKeyframe.tr(locale),
                components::IconButtonOpts::default(),
            )
            .clicked()
            {
                remove_index = Some(i);
            }
        });
    }
    if let Some(i) = remove_index {
        list.remove(i);
        changed = true;
    }
    if ui.button(Text::AddKeyframe.tr(locale)).clicked() {
        list.push(avcore::Keyframe {
            time_fraction: 1.0,
            value: default_value,
        });
        changed = true;
    }
    if changed {
        Some(list)
    } else {
        None
    }
}

/// Position-keyframe counterpart of [`f32_keyframe_editor`] — each row edits `time_fraction`
/// plus both `x`/`y` components of the same [`avcore::Position`].
pub(super) fn position_keyframe_editor(
    ui: &mut egui::Ui,
    keyframes: &[avcore::Keyframe<avcore::Position>],
    locale: crate::i18n::Locale,
) -> Option<Vec<avcore::Keyframe<avcore::Position>>> {
    let mut list = keyframes.to_vec();
    let mut changed = false;
    let mut remove_index = None;
    for (i, kf) in list.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui
                .add(
                    egui::Slider::new(&mut kf.time_fraction, 0.0..=1.0)
                        .text(Text::KeyframeTime.tr(locale)),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut kf.value.x, -1.0..=1.0).text(Text::KeyframeX.tr(locale)),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut kf.value.y, -1.0..=1.0).text(Text::KeyframeY.tr(locale)),
                )
                .changed();
            if components::icon_button(
                ui,
                "🗑",
                Text::RemoveKeyframe.tr(locale),
                components::IconButtonOpts::default(),
            )
            .clicked()
            {
                remove_index = Some(i);
            }
        });
    }
    if let Some(i) = remove_index {
        list.remove(i);
        changed = true;
    }
    if ui.button(Text::AddKeyframe.tr(locale)).clicked() {
        list.push(avcore::Keyframe {
            time_fraction: 1.0,
            value: avcore::Position { x: 0.0, y: 0.0 },
        });
        changed = true;
    }
    if changed {
        Some(list)
    } else {
        None
    }
}

/// Vertex-list counterpart of [`f32_keyframe_editor`]/[`position_keyframe_editor`] — each row
/// edits one `(x, y)` vertex of a [`avcore::timeline::ShapeKind::Polygon`], in the shape's own
/// local unit square (see that variant's doc comment). Refuses to remove the third-to-last
/// vertex (a polygon needs at least 3 to stay a real shape) rather than letting the list
/// collapse to something degenerate.
pub(super) fn polygon_vertex_editor(
    ui: &mut egui::Ui,
    vertices: &[(f32, f32)],
    locale: crate::i18n::Locale,
) -> Option<Vec<(f32, f32)>> {
    let mut list = vertices.to_vec();
    let count = list.len();
    let mut changed = false;
    let mut remove_index = None;
    for (i, v) in list.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui
                .add(
                    egui::DragValue::new(&mut v.0)
                        .speed(0.01)
                        .range(-2.0..=2.0)
                        .prefix("x "),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut v.1)
                        .speed(0.01)
                        .range(-2.0..=2.0)
                        .prefix("y "),
                )
                .changed();
            if count > 3
                && components::icon_button(
                    ui,
                    "🗑",
                    Text::RemoveVertex.tr(locale),
                    components::IconButtonOpts::default(),
                )
                .clicked()
            {
                remove_index = Some(i);
            }
        });
    }
    if let Some(i) = remove_index {
        list.remove(i);
        changed = true;
    }
    if ui.button(Text::ShapeAddVertex.tr(locale)).clicked() {
        // Offset from the last vertex rather than stacking exactly on top of it, so the new
        // point is easy to spot and drag into place.
        let (last_x, last_y) = list.last().copied().unwrap_or((0.0, 0.0));
        list.push((last_x + 0.1, last_y + 0.1));
        changed = true;
    }
    if changed {
        Some(list)
    } else {
        None
    }
}

pub(super) fn mask_shape_label(
    shape: avcore::timeline::MaskShape,
    locale: crate::i18n::Locale,
) -> String {
    match shape {
        avcore::timeline::MaskShape::None => Text::MaskNone.tr(locale).to_string(),
        avcore::timeline::MaskShape::Circle => Text::MaskCircle.tr(locale).to_string(),
        avcore::timeline::MaskShape::RoundedRect => Text::MaskRoundedRect.tr(locale).to_string(),
    }
}

pub(super) fn color_filter_label(
    filter: avcore::timeline::ColorFilter,
    locale: crate::i18n::Locale,
) -> String {
    match filter {
        avcore::timeline::ColorFilter::None => Text::ColorFilterNone.tr(locale).to_string(),
        avcore::timeline::ColorFilter::BlackAndWhite => {
            Text::ColorFilterBlackAndWhite.tr(locale).to_string()
        }
        avcore::timeline::ColorFilter::Sepia => Text::ColorFilterSepia.tr(locale).to_string(),
    }
}

pub(super) fn transition_type_label(
    transition: avcore::timeline::TransitionType,
    locale: crate::i18n::Locale,
) -> String {
    match transition {
        avcore::timeline::TransitionType::None => Text::TransitionNone.tr(locale).to_string(),
        avcore::timeline::TransitionType::Fade => Text::TransitionFade.tr(locale).to_string(),
        avcore::timeline::TransitionType::HardCut => Text::TransitionHardCut.tr(locale).to_string(),
        avcore::timeline::TransitionType::Slide => Text::TransitionSlide.tr(locale).to_string(),
        avcore::timeline::TransitionType::Zoom => Text::TransitionZoom.tr(locale).to_string(),
    }
}
