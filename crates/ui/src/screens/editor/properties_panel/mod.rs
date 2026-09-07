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

mod background_effects;
mod chrome;
mod crop;
mod effects;
mod keyframe_editors;
mod motion_tracking;
mod privacy_blur;
mod shape_clip;
mod text_clip;
mod visual_effects;
mod voice_cleanup_preview;

pub(super) use chrome::stereo_db_meter;
use chrome::{effects_panel_browser, prop_row, properties_tab_bar};
use crop::crop_properties;
use effects::effects_properties;

use eframe::egui::{self, RichText};

use crate::app::{
    App, GAIN_DB_RANGE, MASK_CORNER_RADIUS_RANGE, SPEED_FACTOR_RANGE, VOICE_CLEANUP_CEILING_RANGE,
    VOICE_CLEANUP_COMPRESSOR_RATIO_RANGE, VOICE_CLEANUP_COMPRESSOR_THRESHOLD_RANGE,
    VOICE_CLEANUP_NOISE_FLOOR_RANGE,
};
use crate::components;
use crate::i18n::Text;
use crate::theme;
use keyframe_editors::{
    blend_mode_label, f32_keyframe_editor, mask_shape_label, position_keyframe_editor,
};
use motion_tracking::motion_tracking_properties;
use shape_clip::shape_clip_properties;
use text_clip::text_clip_properties;
use voice_cleanup_preview::voice_cleanup_preview;

pub(super) fn properties_panel(app: &mut App, ui: &mut egui::Ui, width: f32, height: f32) {
    let locale = app.locale;
    components::panel_frame().show(ui, |ui| {
        ui.set_width(width);
        ui.set_height(height);
        // Real clip selections carry a long, non-collapsible stack of property sections
        // (Transform/Crop/Composite/Speed/Effects/Audio, each with its own keyframe editor)
        // that easily exceeds a typical body_height — without a ScrollArea, this content
        // isn't clipped, it just paints straight past the panel's own allocated rect and
        // visually overlaps whatever sits below it in the Editor's vertical layout (the
        // timeline strip), which read as "the timeline disappeared" until traced back here.
        //
        // The ScrollArea is nested inside a plain `ui.vertical` rather than replacing it
        // directly (matching `media_library_panel`'s own working ScrollArea nesting one
        // level in) — putting `ScrollArea::show` directly as the first thing inside the
        // Frame's `set_width`/`set_height`'d ui measurably corrupted the content's
        // horizontal position (confirmed via a real-egui-rect debug log: labels rendered
        // squeezed into the panel's rightmost ~30px instead of starting at its left margin)
        // rather than merely its width, for reasons not fully root-caused — the vertical
        // wrapper is the fix a real precedent in this codebase already demonstrates works.
        ui.vertical(|ui| {
            egui::ScrollArea::vertical()
                .id_salt("properties_panel_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Keep the inspector chrome visible even before the user selects a clip.
                    // The mockup makes these three workspaces a stable landmark, and leaving
                    // them out turned the right column into an ambiguous empty panel on a new
                    // project. Effects remains browsable without a selection; Inspector and
                    // Audio retain their honest no-selection state below the shared tab strip.
                    properties_tab_bar(app, ui, locale);
                    // Text/shape clip properties take priority when one is selected — only one of
                    // `selected_clip_id`/`selected_text_clip_id`/`selected_shape_clip_id` is ever
                    // `Some` at a time (see `App`'s doc comments on those fields).
                    if let Some(tc_id) = app.selected_text_clip_id {
                        text_clip_properties(app, ui, tc_id, locale);
                        return;
                    }
                    if let Some(sc_id) = app.selected_shape_clip_id {
                        shape_clip_properties(app, ui, sc_id, locale);
                        return;
                    }

                    // Section 11: the Effects Panel stays browsable with no clip selected —
                    // unlike every other tab, which requires a selection to show anything —
                    // so this short-circuits before the `NoClipSelected` early-return below.
                    if app.properties_tab == crate::app::PropertiesTab::Effects
                        && app.selected_clip_id.is_none()
                    {
                        effects_panel_browser(app, ui, locale, false, None);
                        return;
                    }

                    components::section_label(ui, Text::SelectedClip.tr(locale));
                    let Some(asset) = app.selected_asset() else {
                        ui.label(
                            RichText::new(Text::NoClipSelected.tr(locale)).color(theme::TEXT_MUTED),
                        );
                        return;
                    };
                    ui.label(RichText::new(&asset.file_name).size(13.0));
                    ui.add_space(8.0);

                    prop_row(ui, Text::PropCodec.tr(locale), &asset.codec);
                    prop_row(
                        ui,
                        Text::PropSourceBitrate.tr(locale),
                        &format!("{:.0} Mbps", asset.source_bitrate_mbps),
                    );
                    if let Some((w, h)) = asset.resolution {
                        prop_row(ui, Text::PropResolution.tr(locale), &format!("{w}x{h}"));
                    }
                    if let Some(fps) = asset.fps {
                        prop_row(ui, Text::PropFps.tr(locale), &format!("{fps:.0}"));
                    }
                    if let Some(l) = &asset.loudness {
                        prop_row(
                            ui,
                            Text::PropLoudness.tr(locale),
                            &format!("{:.1} LUFS", l.integrated_lufs),
                        );
                    }
                    prop_row(
                        ui,
                        Text::PropProxy.tr(locale),
                        if asset.proxy_path.is_some() {
                            Text::ProxyPresent.tr(locale)
                        } else {
                            Text::ProxyAbsent.tr(locale)
                        },
                    );

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);
                    components::section_label(ui, Text::OnExport.tr(locale));
                    ui.horizontal_wrapped(|ui| {
                        let target = app.active_sequence_export_settings().target_lufs;
                        // "→" (U+2192) reads as tofu here — confirmed via a real screenshot.
                        // egui's bundled default font only covers a curated symbol subset, not
                        // arbitrary Unicode blocks (not even this common an arrow), so plain
                        // ASCII is the only glyph choice that's actually guaranteed, not merely
                        // "probably fine" — the lesson this session's whole run of tofu bugs
                        // (nav rail, toolbar, titlebar) converges on.
                        components::tag_accent(
                            ui,
                            &format!("{} -> {target:.0} LUFS", Text::NormalizeTo.tr(locale)),
                        );
                        components::tag_outline(
                            ui,
                            &format!(
                                "{} ({:.0} Mbps)",
                                Text::BitrateFromSource.tr(locale),
                                asset.source_bitrate_mbps
                            ),
                        );
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(Text::ExportAutoNote.tr(locale))
                            .size(10.5)
                            .color(theme::TEXT_MUTED),
                    );

                    if let Some(clip) = app.selected_clip() {
                        let clip_id = clip.id;
                        let mut gain_db = clip.gain_db;
                        let mut frozen = clip.frozen;
                        let mut deflicker_enabled = clip.deflicker_enabled;
                        let mut speed_factor = clip.speed_factor;
                        let mut mask_shape = clip.mask_shape;
                        let mut mask_corner_radius = clip.mask_corner_radius;
                        let mut flipped_h = clip.flipped_h;
                        let mut blend_mode = clip.blend_mode;
                        let (mut anchor_x, mut anchor_y) = (clip.anchor_x, clip.anchor_y);
                        let mut voice_cleanup_enabled = clip.voice_cleanup_enabled;
                        let mut voice_cleanup_noise_floor_db = clip.voice_cleanup_noise_floor_db;
                        let mut voice_cleanup_compressor_threshold_db =
                            clip.voice_cleanup_compressor_threshold_db;
                        let mut voice_cleanup_compressor_ratio =
                            clip.voice_cleanup_compressor_ratio;
                        let mut voice_cleanup_ceiling_linear = clip.voice_cleanup_ceiling_linear;
                        // Cloned out up front (like every other field above) rather than read from
                        // `clip` later, so this immutable borrow of `app` doesn't need to stay alive
                        // across the `app.set_selected_clip_*` mutable calls further down.
                        let position_keyframes = clip.position_keyframes.clone();
                        let scale_keyframes = clip.scale_keyframes.clone();
                        let rotation_keyframes = clip.rotation_keyframes.clone();
                        let opacity_keyframes = clip.opacity_keyframes.clone();
                        let gain_keyframes = clip.gain_keyframes.clone();

                        let tab = app.properties_tab;

                        if tab == crate::app::PropertiesTab::Audio {
                            if components::property_section(
                                ui,
                                clip_id,
                                Text::PropGain.tr(locale),
                                Text::GainExportNote.tr(locale),
                                gain_db != 0.0,
                                |ui| {
                                    ui.add(
                                        egui::Slider::new(&mut gain_db, GAIN_DB_RANGE)
                                            .suffix(" dB")
                                            .fixed_decimals(1),
                                    )
                                    .changed()
                                },
                            ) {
                                app.set_selected_clip_gain(gain_db);
                            }

                            let mut new_gain_keyframes = None;
                            if components::property_section(
                                ui,
                                clip_id,
                                Text::PropGainKeyframes.tr(locale),
                                Text::GainKeyframesExportNote.tr(locale),
                                !gain_keyframes.is_empty(),
                                |ui| {
                                    new_gain_keyframes = f32_keyframe_editor(
                                        ui,
                                        &gain_keyframes,
                                        GAIN_DB_RANGE,
                                        0.0,
                                        locale,
                                    );
                                    new_gain_keyframes.is_some()
                                },
                            ) {
                                if let Some(kfs) = new_gain_keyframes {
                                    app.set_selected_clip_gain_keyframes(kfs);
                                }
                            }
                        }

                        // "Congelar" only makes sense for a video block — audio clips have no
                        // frame to hold.
                        if tab == crate::app::PropertiesTab::Effects
                            && app.selected_clip_track_kind()
                                == Some(avcore::timeline::TrackKind::Video)
                            && components::property_toggle(
                                ui,
                                Text::PropFreeze.tr(locale),
                                Text::FreezeExportNote.tr(locale),
                                &mut frozen,
                            )
                        {
                            app.set_selected_clip_frozen(frozen);
                        }

                        // Deflicker only makes sense for a video block, same reasoning as "Congelar".
                        if tab == crate::app::PropertiesTab::Effects
                            && app.selected_clip_track_kind()
                                == Some(avcore::timeline::TrackKind::Video)
                            && components::property_toggle(
                                ui,
                                Text::PropDeflicker.tr(locale),
                                Text::DeflickerExportNote.tr(locale),
                                &mut deflicker_enabled,
                            )
                        {
                            app.set_selected_clip_deflicker(deflicker_enabled);
                        }

                        if tab == crate::app::PropertiesTab::Inspector
                            && components::property_section(
                                ui,
                                clip_id,
                                Text::PropSpeed.tr(locale),
                                Text::SpeedExportNote.tr(locale),
                                (speed_factor - 1.0).abs() > 1e-4,
                                |ui| {
                                    ui.add(
                                        egui::Slider::new(&mut speed_factor, SPEED_FACTOR_RANGE)
                                            .suffix("x")
                                            .fixed_decimals(2),
                                    )
                                    .changed()
                                },
                            )
                        {
                            app.set_selected_clip_speed(speed_factor);
                        }

                        // CF-03 voice cleanup (noise reduction + compressor + limiter, the proven
                        // Watch-Gameplay.ps1 chain) only makes sense on an audio block — no meaning
                        // for a video block's own embedded audio, same "clip-level toggle regardless
                        // of AudioRole" scope this field's own doc comment on ClipInstance describes.
                        let is_audio_clip = app.selected_clip_track_kind()
                            == Some(avcore::timeline::TrackKind::Audio);
                        if tab == crate::app::PropertiesTab::Audio
                            && is_audio_clip
                            && !voice_cleanup_enabled
                            && app.selected_clip_track_audio_role() == Some(avcore::AudioRole::Mic)
                        {
                            // A clip that predates its track's Mic role (or was moved there after
                            // creation) never gets `App::add_asset_to_timeline`'s "Mic by default"
                            // treatment — this is the non-blocking nudge for that case, never an
                            // auto-toggle, since the field stays a plain always-overridable per-clip
                            // setting either way.
                            ui.label(
                                RichText::new(Text::VoiceCleanupMicRoleSuggestion.tr(locale))
                                    .size(10.5)
                                    .color(theme::WARNING),
                            );
                        }
                        if tab == crate::app::PropertiesTab::Audio
                            && is_audio_clip
                            && components::property_block(
                                ui,
                                Text::VoiceCleanupExportNote.tr(locale),
                                |ui| {
                                    let mut changed = ui
                                        .checkbox(
                                            &mut voice_cleanup_enabled,
                                            Text::PropVoiceCleanup.tr(locale),
                                        )
                                        .changed();
                                    if voice_cleanup_enabled {
                                        changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut voice_cleanup_noise_floor_db,
                                                    VOICE_CLEANUP_NOISE_FLOOR_RANGE,
                                                )
                                                .suffix(" dB")
                                                .text(Text::VoiceCleanupNoiseFloor.tr(locale)),
                                            )
                                            .changed();
                                        changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut voice_cleanup_compressor_threshold_db,
                                                    VOICE_CLEANUP_COMPRESSOR_THRESHOLD_RANGE,
                                                )
                                                .suffix(" dB")
                                                .text(
                                                    Text::VoiceCleanupCompressorThreshold
                                                        .tr(locale),
                                                ),
                                            )
                                            .changed();
                                        changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut voice_cleanup_compressor_ratio,
                                                    VOICE_CLEANUP_COMPRESSOR_RATIO_RANGE,
                                                )
                                                .suffix(":1")
                                                .text(Text::VoiceCleanupCompressorRatio.tr(locale)),
                                            )
                                            .changed();
                                        changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    &mut voice_cleanup_ceiling_linear,
                                                    VOICE_CLEANUP_CEILING_RANGE,
                                                )
                                                .fixed_decimals(2)
                                                .text(Text::VoiceCleanupCeiling.tr(locale)),
                                            )
                                            .changed();
                                    }
                                    changed
                                },
                            )
                        {
                            app.set_selected_clip_voice_cleanup(
                                voice_cleanup_enabled,
                                voice_cleanup_noise_floor_db,
                                voice_cleanup_compressor_threshold_db,
                                voice_cleanup_compressor_ratio,
                                voice_cleanup_ceiling_linear,
                            );
                        }
                        // CF-03 slice 3: A/B preview + measured before/after loudness/peak, the
                        // acceptance criterion this effect's own doc comment previously left
                        // open (`VoiceCleanupExportNote`'s "no live preview effect" still holds —
                        // this renders two real, disposable samples instead, through the exact
                        // mixing path a real export uses, rather than a live pipeline change).
                        if tab == crate::app::PropertiesTab::Audio
                            && is_audio_clip
                            && voice_cleanup_enabled
                        {
                            voice_cleanup_preview(app, ui, clip_id, locale);
                        }
                        if app.selected_clip_track_kind()
                            == Some(avcore::timeline::TrackKind::Video)
                        {
                            crop_properties(app, ui, clip_id, locale);

                            if tab == crate::app::PropertiesTab::Inspector {
                                let mask_changed = components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropMask.tr(locale),
                                    Text::MaskExportNote.tr(locale),
                                    mask_shape != avcore::timeline::MaskShape::None,
                                    |ui| {
                                        let mut mask_changed = components::enum_combo(
                                            ui,
                                            "mask_shape",
                                            &[
                                                avcore::timeline::MaskShape::None,
                                                avcore::timeline::MaskShape::Circle,
                                                avcore::timeline::MaskShape::RoundedRect,
                                            ],
                                            &mut mask_shape,
                                            |shape| mask_shape_label(shape, locale),
                                        );
                                        if mask_shape == avcore::timeline::MaskShape::RoundedRect {
                                            mask_changed |= ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut mask_corner_radius,
                                                        MASK_CORNER_RADIUS_RANGE,
                                                    )
                                                    .text(Text::MaskCornerRadius.tr(locale)),
                                                )
                                                .changed();
                                        }
                                        mask_changed
                                    },
                                );
                                if mask_changed {
                                    app.set_selected_clip_mask(mask_shape, mask_corner_radius);
                                }

                                if components::property_toggle(
                                    ui,
                                    Text::PropFlip.tr(locale),
                                    Text::FlipExportNote.tr(locale),
                                    &mut flipped_h,
                                ) {
                                    app.set_selected_clip_flip_h(flipped_h);
                                }

                                // Composite section (mockup: "COMPOSITE" groups Blend Mode with
                                // Opacity — opacity_keyframes lives further down, alongside the
                                // other keyframe editors, per this panel's existing grouping).
                                let blend_mode_changed = components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropBlendMode.tr(locale),
                                    Text::BlendModeExportNote.tr(locale),
                                    blend_mode != avcore::timeline::BlendMode::Normal,
                                    |ui| {
                                        components::enum_combo(
                                            ui,
                                            "blend_mode",
                                            &avcore::timeline::BlendMode::ALL,
                                            &mut blend_mode,
                                            |mode| blend_mode_label(mode).to_string(),
                                        )
                                    },
                                );
                                if blend_mode_changed {
                                    app.set_selected_clip_blend_mode(blend_mode);
                                }
                            }

                            effects_properties(app, ui, clip_id, locale);

                            if tab == crate::app::PropertiesTab::Inspector {
                                let mut new_position_keyframes = None;
                                if components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropPositionKeyframes.tr(locale),
                                    Text::PositionExportNote.tr(locale),
                                    !position_keyframes.is_empty(),
                                    |ui| {
                                        new_position_keyframes = position_keyframe_editor(
                                            ui,
                                            &position_keyframes,
                                            locale,
                                        );
                                        new_position_keyframes.is_some()
                                    },
                                ) {
                                    if let Some(kfs) = new_position_keyframes {
                                        app.set_selected_clip_position_keyframes(kfs);
                                    }
                                }
                                motion_tracking_properties(app, ui, clip_id, locale);

                                let mut new_scale_keyframes = None;
                                if components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropScaleKeyframes.tr(locale),
                                    Text::ScaleExportNote.tr(locale),
                                    !scale_keyframes.is_empty(),
                                    |ui| {
                                        new_scale_keyframes = f32_keyframe_editor(
                                            ui,
                                            &scale_keyframes,
                                            crate::app::SCALE_RANGE,
                                            1.0,
                                            locale,
                                        );
                                        new_scale_keyframes.is_some()
                                    },
                                ) {
                                    if let Some(kfs) = new_scale_keyframes {
                                        app.set_selected_clip_scale_keyframes(kfs);
                                    }
                                }

                                let mut new_rotation_keyframes = None;
                                if components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropRotationKeyframes.tr(locale),
                                    Text::RotationExportNote.tr(locale),
                                    !rotation_keyframes.is_empty(),
                                    |ui| {
                                        new_rotation_keyframes = f32_keyframe_editor(
                                            ui,
                                            &rotation_keyframes,
                                            -180.0..=180.0,
                                            0.0,
                                            locale,
                                        );
                                        new_rotation_keyframes.is_some()
                                    },
                                ) {
                                    if let Some(kfs) = new_rotation_keyframes {
                                        app.set_selected_clip_rotation_keyframes(kfs);
                                    }
                                }

                                let anchor_changed = components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropAnchor.tr(locale),
                                    Text::AnchorExportNote.tr(locale),
                                    (anchor_x - 0.5).abs() > 1e-4 || (anchor_y - 0.5).abs() > 1e-4,
                                    |ui| {
                                        let mut changed = false;
                                        changed |= ui
                                            .add(
                                                egui::Slider::new(&mut anchor_x, 0.0..=1.0)
                                                    .text(Text::KeyframeX.tr(locale)),
                                            )
                                            .changed();
                                        changed |= ui
                                            .add(
                                                egui::Slider::new(&mut anchor_y, 0.0..=1.0)
                                                    .text(Text::KeyframeY.tr(locale)),
                                            )
                                            .changed();
                                        changed
                                    },
                                );
                                if anchor_changed {
                                    app.set_selected_clip_anchor(anchor_x, anchor_y);
                                }

                                let mut new_opacity_keyframes = None;
                                if components::property_section(
                                    ui,
                                    clip_id,
                                    Text::PropOpacityKeyframes.tr(locale),
                                    Text::OpacityExportNote.tr(locale),
                                    !opacity_keyframes.is_empty(),
                                    |ui| {
                                        new_opacity_keyframes = f32_keyframe_editor(
                                            ui,
                                            &opacity_keyframes,
                                            0.0..=1.0,
                                            1.0,
                                            locale,
                                        );
                                        new_opacity_keyframes.is_some()
                                    },
                                ) {
                                    if let Some(kfs) = new_opacity_keyframes {
                                        app.set_selected_clip_opacity_keyframes(kfs);
                                    }
                                }
                            }
                        }
                    }
                });
        });
    });
}
