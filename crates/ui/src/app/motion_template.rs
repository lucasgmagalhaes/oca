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

//! CF-07 slice 3's `ui`-side apply flow: turns a resolved `avcore::motion_template::
//! GraphicTemplate` into real `TextClip`/`ShapeClip`s placed on the timeline —
//! `avcore::motion_template::instantiate`'s own doc comment leaves this step to a `ui`-side
//! caller by design, keeping that module timeline/track-agnostic. Distinct from `layer_templates`
//! (a different, pre-existing feature: saved per-layer *clip formatting*, not graphic-overlay
//! templates) — same "Template" word, unrelated data model.
//!
//! The toolbar's "🖼 Load graphic template" button (`App::load_graphic_template_from_file`) is
//! this slice's own real trigger, deliberately scoped to parameterless templates for now: it
//! applies with no supplied parameter values, so a template with any `Parameter`-bound field
//! surfaces `instantiate`'s own actionable "no value supplied for parameter ..." toast rather
//! than silently applying garbage. A real parameter-fill form (a dynamic modal, one row per
//! declared [`avcore::motion_template::TemplateParameter`]) is a genuine, separate follow-up —
//! this button is the honest, currently-reachable slice of that flow, not a stand-in for it.

use std::collections::HashMap;
use std::path::PathBuf;

use avcore::motion_template::{self, GraphicTemplate, InstantiatedElement, ParameterValue};
use avcore::timeline::{ShapeClip, TextClip, TrackKind};

use super::timeline_ops::{next_clip_id, resolve_or_create_track};
use super::App;

impl App {
    /// Reads `path`, parses and validates it as a [`GraphicTemplate`], and applies it with no
    /// supplied parameter values — see this module's own doc comment for why that's the honest
    /// scope of this button today. Toasts on a read failure, a parse/validation failure
    /// ([`GraphicTemplate::parse_and_validate`]), or an instantiation failure (surfaced by
    /// [`App::apply_graphic_template`] itself).
    pub fn load_graphic_template_from_file(&mut self, path: PathBuf) {
        let locale = self.locale;
        let json = match std::fs::read_to_string(&path) {
            Ok(json) => json,
            Err(e) => {
                self.push_toast(format!(
                    "{}: {e}",
                    crate::i18n::Text::GraphicTemplateReadFailed.tr(locale)
                ));
                return;
            }
        };
        let template = match GraphicTemplate::parse_and_validate(&json) {
            Ok(template) => template,
            Err(e) => {
                self.push_toast(format!(
                    "{}: {e}",
                    crate::i18n::Text::GraphicTemplateApplyInvalid.tr(locale)
                ));
                return;
            }
        };
        self.apply_graphic_template(&template, &HashMap::new());
    }

    /// Instantiates `template` against `values` and places every resolved element onto the
    /// timeline at the current playhead. Every `Text` element lands on the first-or-created text
    /// track, every `Shape` element on the first-or-created shape track — the same "first track
    /// of that kind" placement [`App::add_text_clip`]/[`App::add_shape_clip`] already use for a
    /// manually inserted overlay. Multiple template elements can share one track: nothing in
    /// this app's own timeline model requires same-kind clips to avoid overlapping in time, only
    /// each element's own template-defined position keeps them visually apart on screen.
    ///
    /// One [`App::push_undo_snapshot`] for the whole batch, matching Shorts Pack's own "one
    /// snapshot per batch, not per clip" precedent — never pushed at all if instantiation fails
    /// or the template has no elements, so a failed/empty apply leaves no undo-history noise.
    ///
    /// New clips default to a 3-second duration, the same default [`App::add_text_clip`]/
    /// [`App::add_shape_clip`] already use for a manually inserted overlay — CF-07 slice 2's own
    /// "timing" (animation in/out, and by extension a template-declared duration) remains open,
    /// a real, separate follow-up.
    pub fn apply_graphic_template(
        &mut self,
        template: &GraphicTemplate,
        values: &HashMap<String, ParameterValue>,
    ) {
        let locale = self.locale;
        let elements = match motion_template::instantiate(template, values) {
            Ok(elements) => elements,
            Err(e) => {
                self.push_toast(format!(
                    "{}: {e}",
                    crate::i18n::Text::GraphicTemplateApplyInvalid.tr(locale)
                ));
                return;
            }
        };
        if elements.is_empty() {
            return;
        }

        self.push_undo_snapshot();
        let playhead_secs = self.active_project().timeline().playhead_secs;
        let timeline = self.active_project_mut().timeline_mut();

        for element in elements {
            match element {
                InstantiatedElement::Text {
                    text,
                    color_rgba,
                    font_family,
                    font_style,
                    font_size,
                    pos_x,
                    pos_y,
                    ..
                } => {
                    let track_index = resolve_or_create_track(timeline, TrackKind::Text, None);
                    let clip_id = next_clip_id(timeline);
                    timeline.tracks[track_index].text_clips.push(TextClip {
                        id: clip_id,
                        start_secs: playhead_secs,
                        duration_secs: 3.0,
                        text,
                        font_size,
                        font_family,
                        font_style,
                        color_rgba,
                        background_rgba: [0, 0, 0, 0],
                        background_padding: 8.0,
                        background_corner_radius: 8.0,
                        pos_x,
                        pos_y,
                        words: Vec::new(),
                        highlight_enabled: false,
                        highlight_color_rgba: [255, 220, 0, 255],
                        opacity_keyframes: vec![],
                        pos_x_keyframes: vec![],
                        pos_y_keyframes: vec![],
                        scale_keyframes: vec![],
                        rotation_keyframes: vec![],
                        direction: Default::default(),
                        language: None,
                        text_align: Default::default(),
                    });
                }
                InstantiatedElement::Shape {
                    shape_kind,
                    color_rgba,
                    center_x,
                    center_y,
                    width,
                    height,
                    rotation_deg,
                    stroke_thickness_px,
                    ..
                } => {
                    let track_index = resolve_or_create_track(timeline, TrackKind::Shape, None);
                    let clip_id = next_clip_id(timeline);
                    timeline.tracks[track_index].shape_clips.push(ShapeClip {
                        id: clip_id,
                        start_secs: playhead_secs,
                        duration_secs: 3.0,
                        shape_kind,
                        center_x,
                        center_y,
                        center_x_keyframes: vec![],
                        center_y_keyframes: vec![],
                        width_keyframes: vec![],
                        height_keyframes: vec![],
                        rotation_keyframes: vec![],
                        width,
                        height,
                        rotation_deg,
                        color_rgba,
                        stroke_thickness_px,
                    });
                }
            }
        }

        self.selected_clip_id = None;
        self.selected_text_clip_id = None;
        self.selected_shape_clip_id = None;
        self.invalidate_preview_rendering();
    }
}
