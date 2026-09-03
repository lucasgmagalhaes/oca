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

//! CF-07: parameterized motion-graphics templates
//! ([`spec/architecture/competitive-feature-plan.md`](../../../../spec/architecture/competitive-feature-plan.md)'s
//! own "Outcome: reusable channel assets such as lower thirds, scoreboards, subscribe prompts,
//! webcam frames").
//!
//! This module is CF-07's own slice 1: "Define a versioned declarative JSON format with
//! allowlisted primitives and editable parameters." A [`GraphicTemplate`] is a named, portable
//! JSON asset — deliberately plain `serde_json`, not this crate's usual gzip-MessagePack
//! `.ocproj` framing, since a template is meant to be inspected and shared, not just persisted
//! (CF-07 slice 3's own "package templates as data" goal). Its primitives are deliberately just
//! [`TemplateElement::Text`]/[`TemplateElement::Shape`] for this slice — the exact two overlay
//! kinds `crate::timeline`/`crate::overlay_render`/`crate::shape_render` already render, per
//! `spec/RULES.md`'s reuse-before-building rule. An `Image` primitive (slice 2's own scope) has
//! no existing overlay-clip kind to reuse yet — [`crate::timeline::TrackKind`] has no `Image`
//! variant — so adding it here now would mean inventing new overlay-rendering infrastructure
//! this slice was never meant to cover; a real, deliberate follow-up, not an oversight.
//!
//! `TemplateElement` is a closed enum (no `#[serde(other)]`, matching
//! [`crate::gameplay_events::GameplayEventKind`]'s own precedent) — an unrecognized primitive
//! kind fails to deserialize outright, per CF-07's own "Unknown primitives/parameters are
//! rejected rather than executed or ignored" acceptance criterion. The other half of that
//! criterion — an element bound to a parameter id nothing declares — is [`GraphicTemplate::
//! validate`]'s job. Templates contain no scripts or executable expressions anywhere in this
//! format; every field is a plain literal or a named reference into [`GraphicTemplate::
//! parameters`], matching the doc's own "no scripts or executable expressions" constraint for
//! this whole feature.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::timeline::{ShapeKind, TextFontFamily, TextFontStyle};

/// The only schema version this build understands — see [`TemplateValidationError::
/// UnsupportedSchemaVersion`]. Bumped, with an explicit migration, once a real format change
/// needs one (CF-07 slice 4's own "migration tests").
pub const TEMPLATE_SCHEMA_VERSION: u32 = 1;

/// The type of value a [`TemplateParameter`] expects — deliberately just the two CF-07 slice 1
/// calls out ("text, color" lead that list before "image, timing..." arrive in slice 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateParameterKind {
    Text,
    Color,
}

/// One named, editable slot a template exposes — e.g. "player_name" (`Text`) or
/// "team_color" (`Color`) for a scoreboard template. `id` is referenced by
/// [`TextBinding::Parameter`]/[`ColorBinding::Parameter`] elsewhere in the same
/// [`GraphicTemplate`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateParameter {
    pub id: String,
    /// User-facing label for this slot in an "apply template" form — not a translatable UI
    /// string (same reasoning `Project::summary` documents: this is user-authored template
    /// content, not application chrome).
    pub label: String,
    pub kind: TemplateParameterKind,
}

/// Where a text element's string comes from: a literal baked into the template, or a named
/// [`TemplateParameter`] (of [`TemplateParameterKind::Text`]) the applying user fills in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TextBinding {
    Fixed(String),
    Parameter(String),
}

/// Where a color field comes from — the same shape as [`TextBinding`], for
/// [`TemplateParameterKind::Color`] parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColorBinding {
    /// `[r, g, b, a]`, each 0–255 — same convention every other RGBA color field in this crate
    /// uses (e.g. [`crate::timeline::TextClip::color_rgba`]).
    Fixed([u8; 4]),
    Parameter(String),
}

/// A [`crate::timeline::TextClip`]-shaped template primitive. Position is a safe-area-agnostic
/// `0.0..=1.0` canvas fraction for this slice (matching `TextClip::pos_x`/`pos_y`'s own existing
/// convention) — CF-07 slice 2's own dedicated safe-area-anchor semantics are a real, separate
/// follow-up, not assumed here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateTextElement {
    /// Unique within the owning [`GraphicTemplate`] — see [`TemplateValidationError::
    /// DuplicateElementId`].
    pub id: String,
    pub text: TextBinding,
    pub color_rgba: ColorBinding,
    pub font_family: TextFontFamily,
    pub font_style: TextFontStyle,
    pub font_size: f32,
    pub pos_x: f32,
    pub pos_y: f32,
}

/// A [`crate::timeline::ShapeClip`]-shaped template primitive — the same field set, minus the
/// keyframe-animation lists (a template element is a static starting point; animating an
/// instantiated shape happens after placement, the same as any other `ShapeClip` today).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateShapeElement {
    pub id: String,
    pub shape_kind: ShapeKind,
    pub color_rgba: ColorBinding,
    pub center_x: f32,
    pub center_y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation_deg: f32,
    pub stroke_thickness_px: f32,
}

/// One allowlisted template primitive — see this module's own doc comment for why only these
/// two exist in this slice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemplateElement {
    Text(TemplateTextElement),
    Shape(TemplateShapeElement),
}

impl TemplateElement {
    fn id(&self) -> &str {
        match self {
            Self::Text(t) => &t.id,
            Self::Shape(s) => &s.id,
        }
    }
}

/// A reusable, parameterized graphic asset — CF-07's own "lower thirds, scoreboards, subscribe
/// prompts, webcam frames" — as plain declarative data. See this module's own doc comment for
/// the format's scope and constraints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphicTemplate {
    pub schema_version: u32,
    pub name: String,
    /// The canvas this template's `pos_x`/`pos_y`/`center_x`/`center_y` fractions were designed
    /// against — not yet a *variant* (CF-07 slice 2's "aspect-ratio variants"), just this
    /// template's own single native size.
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub parameters: Vec<TemplateParameter>,
    pub elements: Vec<TemplateElement>,
}

/// Why a [`GraphicTemplate`] failed [`GraphicTemplate::validate`] — each variant's
/// [`Display`](std::fmt::Display) names the offending id/value, matching this crate's
/// established convention for structured, actionable validation errors (e.g.
/// [`crate::gameplay_events::SidecarValidationError`]).
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateValidationError {
    UnsupportedSchemaVersion {
        found: u32,
    },
    InvalidCanvasSize {
        width: u32,
        height: u32,
    },
    DuplicateParameterId(String),
    DuplicateElementId(String),
    UnknownParameterReference {
        element_id: String,
        parameter_id: String,
    },
    ParameterKindMismatch {
        element_id: String,
        parameter_id: String,
        expected: TemplateParameterKind,
    },
    NonPositiveFontSize {
        element_id: String,
        value: f32,
    },
    NonPositiveShapeExtent {
        element_id: String,
        width: f32,
        height: f32,
    },
}

impl std::fmt::Display for TemplateValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion { found } => write!(
                f,
                "unsupported schema_version {found} (this build only understands \
                 {TEMPLATE_SCHEMA_VERSION})"
            ),
            Self::InvalidCanvasSize { width, height } => write!(
                f,
                "canvas_width/canvas_height must both be positive (got {width}x{height})"
            ),
            Self::DuplicateParameterId(id) => {
                write!(f, "duplicate parameter id {id:?}")
            }
            Self::DuplicateElementId(id) => {
                write!(f, "duplicate element id {id:?}")
            }
            Self::UnknownParameterReference {
                element_id,
                parameter_id,
            } => write!(
                f,
                "element {element_id:?} references parameter {parameter_id:?}, which this \
                 template does not declare"
            ),
            Self::ParameterKindMismatch {
                element_id,
                parameter_id,
                expected,
            } => write!(
                f,
                "element {element_id:?} references parameter {parameter_id:?} as {expected:?}, \
                 but that parameter is declared with a different kind"
            ),
            Self::NonPositiveFontSize { element_id, value } => write!(
                f,
                "text element {element_id:?}: font_size must be positive (got {value})"
            ),
            Self::NonPositiveShapeExtent {
                element_id,
                width,
                height,
            } => write!(
                f,
                "shape element {element_id:?}: width/height must both be positive (got \
                 {width}x{height})"
            ),
        }
    }
}

impl std::error::Error for TemplateValidationError {}

/// Either half of [`GraphicTemplate::parse_and_validate`]'s failure — kept distinct from
/// [`TemplateValidationError`] alone since a malformed-JSON failure and a well-formed-but-invalid
/// failure are different classes of problem for a caller to report (same reasoning
/// [`crate::gameplay_events::SidecarParseOrValidationError`] documents).
#[derive(Debug)]
pub enum TemplateParseOrValidationError {
    Parse(serde_json::Error),
    Validation(TemplateValidationError),
}

impl std::fmt::Display for TemplateParseOrValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "could not parse graphic template JSON: {e}"),
            Self::Validation(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for TemplateParseOrValidationError {}

impl From<TemplateValidationError> for TemplateParseOrValidationError {
    fn from(e: TemplateValidationError) -> Self {
        Self::Validation(e)
    }
}

fn parameter_kind(parameters: &[TemplateParameter], id: &str) -> Option<TemplateParameterKind> {
    parameters.iter().find(|p| p.id == id).map(|p| p.kind)
}

impl GraphicTemplate {
    /// Parses and validates `json` in one step — the only entry point external callers should
    /// use, so an accepted [`GraphicTemplate`] is always known-valid (same "sidecars are
    /// untrusted input" convention [`crate::gameplay_events::EventSidecar::parse_and_validate`]
    /// established).
    pub fn parse_and_validate(json: &str) -> Result<Self, TemplateParseOrValidationError> {
        let template: GraphicTemplate =
            serde_json::from_str(json).map_err(TemplateParseOrValidationError::Parse)?;
        template.validate()?;
        Ok(template)
    }

    /// Checks every CF-07 slice-1-relevant constraint: schema version, canvas size, unique
    /// parameter/element ids, and every parameter reference resolving to a declared parameter of
    /// the matching kind. An unrecognized primitive *kind* never reaches this function at all —
    /// it already fails at JSON deserialization, since [`TemplateElement`] is a closed enum.
    pub fn validate(&self) -> Result<(), TemplateValidationError> {
        if self.schema_version != TEMPLATE_SCHEMA_VERSION {
            return Err(TemplateValidationError::UnsupportedSchemaVersion {
                found: self.schema_version,
            });
        }
        if self.canvas_width == 0 || self.canvas_height == 0 {
            return Err(TemplateValidationError::InvalidCanvasSize {
                width: self.canvas_width,
                height: self.canvas_height,
            });
        }

        let mut seen_parameter_ids = std::collections::HashSet::new();
        for parameter in &self.parameters {
            if !seen_parameter_ids.insert(parameter.id.as_str()) {
                return Err(TemplateValidationError::DuplicateParameterId(
                    parameter.id.clone(),
                ));
            }
        }

        let mut seen_element_ids = std::collections::HashSet::new();
        for element in &self.elements {
            if !seen_element_ids.insert(element.id()) {
                return Err(TemplateValidationError::DuplicateElementId(
                    element.id().to_string(),
                ));
            }
            match element {
                TemplateElement::Text(t) => {
                    if t.font_size <= 0.0 {
                        return Err(TemplateValidationError::NonPositiveFontSize {
                            element_id: t.id.clone(),
                            value: t.font_size,
                        });
                    }
                    if let TextBinding::Parameter(pid) = &t.text {
                        self.check_reference(&t.id, pid, TemplateParameterKind::Text)?;
                    }
                    if let ColorBinding::Parameter(pid) = &t.color_rgba {
                        self.check_reference(&t.id, pid, TemplateParameterKind::Color)?;
                    }
                }
                TemplateElement::Shape(s) => {
                    if s.width <= 0.0 || s.height <= 0.0 {
                        return Err(TemplateValidationError::NonPositiveShapeExtent {
                            element_id: s.id.clone(),
                            width: s.width,
                            height: s.height,
                        });
                    }
                    if let ColorBinding::Parameter(pid) = &s.color_rgba {
                        self.check_reference(&s.id, pid, TemplateParameterKind::Color)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn check_reference(
        &self,
        element_id: &str,
        parameter_id: &str,
        expected: TemplateParameterKind,
    ) -> Result<(), TemplateValidationError> {
        match parameter_kind(&self.parameters, parameter_id) {
            None => Err(TemplateValidationError::UnknownParameterReference {
                element_id: element_id.to_string(),
                parameter_id: parameter_id.to_string(),
            }),
            Some(found) if found != expected => {
                Err(TemplateValidationError::ParameterKindMismatch {
                    element_id: element_id.to_string(),
                    parameter_id: parameter_id.to_string(),
                    expected,
                })
            }
            Some(_) => Ok(()),
        }
    }
}

/// A concrete value supplied for one [`TemplateParameter`] when instantiating a
/// [`GraphicTemplate`] — see [`instantiate`].
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterValue {
    Text(String),
    Color([u8; 4]),
}

/// One [`TemplateElement`] with every binding resolved to a concrete value — ready for a `ui`-
/// side caller to build a real [`crate::timeline::TextClip`]/[`crate::timeline::ShapeClip`] from
/// (assigning its own id/`start_secs`/`duration_secs` and placing it on a track — deliberately
/// left to that caller, since this module stays timeline/track-agnostic per its own "UI/render
/// code" boundary).
#[derive(Debug, Clone, PartialEq)]
pub enum InstantiatedElement {
    Text {
        source_id: String,
        text: String,
        color_rgba: [u8; 4],
        font_family: TextFontFamily,
        font_style: TextFontStyle,
        font_size: f32,
        pos_x: f32,
        pos_y: f32,
    },
    Shape {
        source_id: String,
        shape_kind: ShapeKind,
        color_rgba: [u8; 4],
        center_x: f32,
        center_y: f32,
        width: f32,
        height: f32,
        rotation_deg: f32,
        stroke_thickness_px: f32,
    },
}

/// Why [`instantiate`] couldn't resolve a template against the supplied `values` — a missing
/// value for a declared parameter, or a value of the wrong [`TemplateParameterKind`] for what an
/// element expects.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateInstantiateError {
    Validation(TemplateValidationError),
    MissingParameterValue(String),
    WrongParameterValueKind {
        parameter_id: String,
        expected: TemplateParameterKind,
    },
}

impl std::fmt::Display for TemplateInstantiateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(e) => write!(f, "{e}"),
            Self::MissingParameterValue(id) => {
                write!(f, "no value supplied for parameter {id:?}")
            }
            Self::WrongParameterValueKind {
                parameter_id,
                expected,
            } => write!(
                f,
                "value supplied for parameter {parameter_id:?} is not a {expected:?}"
            ),
        }
    }
}

impl std::error::Error for TemplateInstantiateError {}

fn resolve_text(
    binding: &TextBinding,
    values: &HashMap<String, ParameterValue>,
) -> Result<String, TemplateInstantiateError> {
    match binding {
        TextBinding::Fixed(text) => Ok(text.clone()),
        TextBinding::Parameter(id) => match values.get(id) {
            Some(ParameterValue::Text(text)) => Ok(text.clone()),
            Some(ParameterValue::Color(_)) => {
                Err(TemplateInstantiateError::WrongParameterValueKind {
                    parameter_id: id.clone(),
                    expected: TemplateParameterKind::Text,
                })
            }
            None => Err(TemplateInstantiateError::MissingParameterValue(id.clone())),
        },
    }
}

fn resolve_color(
    binding: &ColorBinding,
    values: &HashMap<String, ParameterValue>,
) -> Result<[u8; 4], TemplateInstantiateError> {
    match binding {
        ColorBinding::Fixed(rgba) => Ok(*rgba),
        ColorBinding::Parameter(id) => match values.get(id) {
            Some(ParameterValue::Color(rgba)) => Ok(*rgba),
            Some(ParameterValue::Text(_)) => {
                Err(TemplateInstantiateError::WrongParameterValueKind {
                    parameter_id: id.clone(),
                    expected: TemplateParameterKind::Color,
                })
            }
            None => Err(TemplateInstantiateError::MissingParameterValue(id.clone())),
        },
    }
}

/// Resolves every element of `template` against `values` into placement-ready
/// [`InstantiatedElement`]s — CF-07's "editable parameters" made real, not just declared.
/// Validates `template` first (so a caller never needs to call [`GraphicTemplate::validate`]
/// itself before this), then fails on the first missing/mismatched parameter value.
pub fn instantiate(
    template: &GraphicTemplate,
    values: &HashMap<String, ParameterValue>,
) -> Result<Vec<InstantiatedElement>, TemplateInstantiateError> {
    template
        .validate()
        .map_err(TemplateInstantiateError::Validation)?;

    template
        .elements
        .iter()
        .map(|element| match element {
            TemplateElement::Text(t) => Ok(InstantiatedElement::Text {
                source_id: t.id.clone(),
                text: resolve_text(&t.text, values)?,
                color_rgba: resolve_color(&t.color_rgba, values)?,
                font_family: t.font_family,
                font_style: t.font_style,
                font_size: t.font_size,
                pos_x: t.pos_x,
                pos_y: t.pos_y,
            }),
            TemplateElement::Shape(s) => Ok(InstantiatedElement::Shape {
                source_id: s.id.clone(),
                shape_kind: s.shape_kind.clone(),
                color_rgba: resolve_color(&s.color_rgba, values)?,
                center_x: s.center_x,
                center_y: s.center_y,
                width: s.width,
                height: s.height,
                rotation_deg: s.rotation_deg,
                stroke_thickness_px: s.stroke_thickness_px,
            }),
        })
        .collect()
}

#[cfg(test)]
#[path = "motion_template/motion_template_test.rs"]
mod tests;
