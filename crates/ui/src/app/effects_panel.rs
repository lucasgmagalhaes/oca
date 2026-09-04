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

//! Effects Panel (`CINECUT_PRODUCT_DECISIONS_v1.0.md` sections 9-12): a categorized, browsable
//! list of the clip effects this app actually supports — every entry maps to a real,
//! pre-existing `ClipInstance` field and setter (`clip_props.rs`), never a fake capability
//! invented to fill out a category. Categories with no supported effect are simply absent —
//! e.g. no "Stylize"/"Transform" category here, since nothing in the current effect set fits.

use avcore::timeline::ColorFilter;

use super::App;
use crate::i18n::{Locale, Text};

/// Applied by "Method A — Double-click" for every continuous-intensity effect in [`EffectPreset`]
/// (blur/sharpen/vignette/glitch/pixelize/shake/stabilization all share the same `0.0..=1.0`
/// range) — the doc defines double-click as "apply the effect" but doesn't specify a target
/// value for a slider-backed effect, so this picks the range midpoint: enough to be visibly
/// on, not a corner case, and a natural starting point for the Inspector's own slider once the
/// user fine-tunes it there.
pub const EFFECT_DEFAULT_INTENSITY: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectCategory {
    BlurSharpen,
    Color,
    Distortion,
    Keying,
    Utility,
}

impl EffectCategory {
    pub fn label(self, locale: Locale) -> &'static str {
        match self {
            EffectCategory::BlurSharpen => Text::EffectCategoryBlurSharpen.tr(locale),
            EffectCategory::Color => Text::EffectCategoryColor.tr(locale),
            EffectCategory::Distortion => Text::EffectCategoryDistortion.tr(locale),
            EffectCategory::Keying => Text::EffectCategoryKeying.tr(locale),
            EffectCategory::Utility => Text::EffectCategoryUtility.tr(locale),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectPreset {
    Blur,
    Sharpen,
    BlackAndWhite,
    Sepia,
    Vignette,
    Glitch,
    Pixelize,
    Shake,
    ChromaKey,
    BackgroundRemoval,
    Stabilization,
    Freeze,
    Deflicker,
}

impl EffectPreset {
    pub const ALL: [EffectPreset; 13] = [
        EffectPreset::Blur,
        EffectPreset::Sharpen,
        EffectPreset::BlackAndWhite,
        EffectPreset::Sepia,
        EffectPreset::Vignette,
        EffectPreset::Glitch,
        EffectPreset::Pixelize,
        EffectPreset::Shake,
        EffectPreset::ChromaKey,
        EffectPreset::BackgroundRemoval,
        EffectPreset::Stabilization,
        EffectPreset::Freeze,
        EffectPreset::Deflicker,
    ];

    pub fn category(self) -> EffectCategory {
        match self {
            EffectPreset::Blur | EffectPreset::Sharpen => EffectCategory::BlurSharpen,
            EffectPreset::BlackAndWhite | EffectPreset::Sepia | EffectPreset::Vignette => {
                EffectCategory::Color
            }
            EffectPreset::Glitch | EffectPreset::Pixelize | EffectPreset::Shake => {
                EffectCategory::Distortion
            }
            EffectPreset::ChromaKey | EffectPreset::BackgroundRemoval => EffectCategory::Keying,
            EffectPreset::Stabilization | EffectPreset::Freeze | EffectPreset::Deflicker => {
                EffectCategory::Utility
            }
        }
    }

    pub fn label(self, locale: Locale) -> &'static str {
        match self {
            EffectPreset::Blur => Text::EffectBlur.tr(locale),
            EffectPreset::Sharpen => Text::EffectSharpen.tr(locale),
            EffectPreset::BlackAndWhite => Text::EffectBlackAndWhite.tr(locale),
            EffectPreset::Sepia => Text::EffectSepia.tr(locale),
            EffectPreset::Vignette => Text::EffectVignette.tr(locale),
            EffectPreset::Glitch => Text::EffectGlitch.tr(locale),
            EffectPreset::Pixelize => Text::EffectPixelize.tr(locale),
            EffectPreset::Shake => Text::EffectShake.tr(locale),
            EffectPreset::ChromaKey => Text::EffectChromaKey.tr(locale),
            EffectPreset::BackgroundRemoval => Text::EffectBackgroundRemoval.tr(locale),
            EffectPreset::Stabilization => Text::EffectStabilization.tr(locale),
            EffectPreset::Freeze => Text::EffectFreeze.tr(locale),
            EffectPreset::Deflicker => Text::EffectDeflicker.tr(locale),
        }
    }

    /// `Freeze`/`Deflicker` only make sense for a video block — an audio clip has no frame to
    /// hold or deflicker, matching the same reasoning the Inspector's own toggles already use.
    pub fn video_only(self) -> bool {
        matches!(self, EffectPreset::Freeze | EffectPreset::Deflicker)
    }
}

impl App {
    /// Applies one [`EffectPreset`] to `selected_clip_id` (Section 10 "Method A/B" — same
    /// action either way, only the trigger UI differs). A no-op if nothing is selected; callers
    /// (the Effects Panel) are expected to check `selected_clip_id` first and show Section 11's
    /// "Select a clip to apply an effect." hint instead of calling this.
    pub fn apply_effect_preset(&mut self, preset: EffectPreset) {
        match preset {
            EffectPreset::Blur => self.set_selected_clip_blur(EFFECT_DEFAULT_INTENSITY),
            EffectPreset::Sharpen => self.set_selected_clip_sharpen(EFFECT_DEFAULT_INTENSITY),
            EffectPreset::BlackAndWhite => {
                self.set_selected_clip_color_filter(ColorFilter::BlackAndWhite)
            }
            EffectPreset::Sepia => self.set_selected_clip_color_filter(ColorFilter::Sepia),
            EffectPreset::Vignette => self.set_selected_clip_vignette(EFFECT_DEFAULT_INTENSITY),
            EffectPreset::Glitch => self.set_selected_clip_glitch(EFFECT_DEFAULT_INTENSITY),
            EffectPreset::Pixelize => self.set_selected_clip_pixelize(EFFECT_DEFAULT_INTENSITY),
            EffectPreset::Shake => self.set_selected_clip_shake(EFFECT_DEFAULT_INTENSITY),
            EffectPreset::ChromaKey => {
                let (color, tolerance) = self
                    .selected_clip()
                    .map(|c| (c.chroma_key_color, c.chroma_key_tolerance))
                    .unwrap_or(([0, 255, 0], EFFECT_DEFAULT_INTENSITY));
                self.set_selected_clip_chroma_key(true, color, tolerance);
            }
            EffectPreset::BackgroundRemoval => self.set_selected_clip_background_removal(true),
            EffectPreset::Stabilization => {
                self.set_selected_clip_stabilization(EFFECT_DEFAULT_INTENSITY)
            }
            EffectPreset::Freeze => self.set_selected_clip_frozen(true),
            EffectPreset::Deflicker => self.set_selected_clip_deflicker(true),
        }
    }
}
