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

//! Layer-group templates (`request.md`'s Fase 4 "Templates de grupo de camadas") — save the
//! per-layer settings of a multi-selected group of clips (position, scale, crop, effects, via
//! [`avcore::ClipFormatting`]) as a named, reusable [`avcore::LayerTemplate`], then apply it to
//! fresh footage later by picking one source asset per saved layer. Persisted app-wide in
//! `prefs.saved_layer_templates`, not per-project — the whole point is reusing the same layer
//! group (e.g. "webcam recortada + fundo com blur + jogo centralizado") across different shorts.

use eframe::egui;

use crate::components;
use crate::i18n::Text;
use crate::theme;

use super::timeline_ops::{create_new_track, next_clip_id};
use super::App;

impl App {
    /// Shows the saved-templates list popup when [`App::layer_templates_menu_open`] is set —
    /// picking "Aplicar" on a row opens the apply-template modal ([`App::begin_apply_layer_template`])
    /// and closes this one; "🗑" deletes that entry immediately.
    pub(super) fn show_layer_templates_menu(&mut self, ctx: &egui::Context) {
        if !self.layer_templates_menu_open {
            return;
        }
        let locale = self.locale;
        let modal = egui::Modal::new(egui::Id::new("layer_templates_menu"));
        let mut apply_index = None;
        let mut delete_index = None;
        let response = modal.show(ctx, |ui| {
            ui.set_width(320.0);
            components::modal_title(ui, Text::Templates.tr(locale));
            ui.add_space(8.0);
            if self.prefs.saved_layer_templates.is_empty() {
                ui.label(
                    egui::RichText::new(Text::NoSavedTemplates.tr(locale)).color(theme::TEXT_MUTED),
                );
            }
            for (index, template) in self.prefs.saved_layer_templates.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(&template.name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(Text::DeleteTemplate.tr(locale)).clicked() {
                            delete_index = Some(index);
                        }
                        if components::primary_button(ui, Text::ApplyTemplate.tr(locale)).clicked()
                        {
                            apply_index = Some(index);
                        }
                    });
                });
            }
            ui.add_space(8.0);
            if ui.button(Text::CancelJob.tr(locale)).clicked() {
                self.layer_templates_menu_open = false;
            }
        });
        if response.should_close() {
            self.layer_templates_menu_open = false;
        }
        if let Some(index) = delete_index {
            self.delete_layer_template(index);
        }
        if let Some(index) = apply_index {
            self.begin_apply_layer_template(index);
        }
    }

    /// Shows the apply-template modal when [`App::applying_layer_template`] is `Some` — one
    /// asset dropdown per saved layer, filtered to media-library assets of that layer's own
    /// [`avcore::timeline::TrackKind`]. "Criar camadas" is only enabled once every slot has a
    /// pick; confirming calls [`App::confirm_apply_layer_template`].
    pub(super) fn show_apply_layer_template_modal(&mut self, ctx: &egui::Context) {
        let Some((template_index, _)) = self.applying_layer_template.as_ref() else {
            return;
        };
        let Some(template) = self
            .prefs
            .saved_layer_templates
            .get(*template_index)
            .cloned()
        else {
            self.applying_layer_template = None;
            return;
        };
        let locale = self.locale;
        let media_library = self.active_project().media_library.clone();
        let modal = egui::Modal::new(egui::Id::new("apply_layer_template_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(360.0);
            components::modal_title(
                ui,
                &format!("{}: {}", Text::ApplyTemplateTitle.tr(locale), template.name),
            );
            ui.add_space(8.0);
            let layer_asset_ids = &mut self.applying_layer_template.as_mut().unwrap().1;
            for (index, (kind, _formatting)) in template.layers.iter().enumerate() {
                let kind_label = match kind {
                    avcore::timeline::TrackKind::Video => Text::TrackKindVideo.tr(locale),
                    avcore::timeline::TrackKind::Audio => Text::TrackKindAudio.tr(locale),
                    avcore::timeline::TrackKind::Text => Text::TrackKindText.tr(locale),
                    avcore::timeline::TrackKind::Shape => Text::TrackKindShape.tr(locale),
                };
                ui.label(format!(
                    "{} {} ({kind_label})",
                    Text::ApplyTemplateLayerLabel.tr(locale),
                    index + 1
                ));
                let selected_name = layer_asset_ids[index]
                    .and_then(|id| media_library.iter().find(|a| a.id == id))
                    .map(|a| a.file_name.clone())
                    .unwrap_or_else(|| Text::ApplyTemplatePickAsset.tr(locale).to_string());
                egui::ComboBox::from_id_salt(("apply_template_layer_asset", index))
                    .selected_text(selected_name)
                    .show_ui(ui, |ui| {
                        for asset in media_library
                            .iter()
                            .filter(|a| media_kind_matches(a.kind, *kind))
                        {
                            ui.selectable_value(
                                &mut layer_asset_ids[index],
                                Some(asset.id),
                                &asset.file_name,
                            );
                        }
                    });
                ui.add_space(4.0);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let all_picked = layer_asset_ids.iter().all(|a| a.is_some());
                if ui
                    .add_enabled(
                        all_picked,
                        egui::Button::new(Text::ApplyTemplateConfirm.tr(locale)),
                    )
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.applying_layer_template = None;
            return;
        }
        if confirmed {
            self.confirm_apply_layer_template();
        }
    }

    /// Shows the parameter fill-in modal when [`App::pending_graphic_template_apply`] is `Some`
    /// (CF-07 slice 3) — one row per declared [`avcore::motion_template::TemplateParameter`], a
    /// plain text field for `Text` and a color picker for `Color`. Confirming calls
    /// [`App::confirm_apply_graphic_template`]; cancelling or Escape calls [`App::
    /// cancel_apply_graphic_template`].
    pub(super) fn show_apply_graphic_template_modal(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending_graphic_template_apply.as_ref() else {
            return;
        };
        let locale = self.locale;
        let template_name = pending.template.name.clone();
        let parameters = pending.template.parameters.clone();
        let modal = egui::Modal::new(egui::Id::new("apply_graphic_template_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(340.0);
            components::modal_title(
                ui,
                &format!(
                    "{}: {}",
                    Text::GraphicTemplateApplyTitle.tr(locale),
                    template_name
                ),
            );
            ui.add_space(8.0);
            let pending = self.pending_graphic_template_apply.as_mut().unwrap();
            for parameter in &parameters {
                ui.label(&parameter.label);
                match parameter.kind {
                    avcore::motion_template::TemplateParameterKind::Text => {
                        let value = pending.text_values.entry(parameter.id.clone()).or_default();
                        ui.text_edit_singleline(value);
                    }
                    avcore::motion_template::TemplateParameterKind::Color => {
                        let value = pending
                            .color_values
                            .entry(parameter.id.clone())
                            .or_insert([255, 255, 255, 255]);
                        let mut color = egui::Color32::from_rgba_premultiplied(
                            value[0], value[1], value[2], value[3],
                        );
                        if ui.color_edit_button_srgba(&mut color).changed() {
                            *value = [color.r(), color.g(), color.b(), color.a()];
                        }
                    }
                }
                ui.add_space(4.0);
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if components::primary_button(ui, Text::GraphicTemplateApplyConfirm.tr(locale))
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.cancel_apply_graphic_template();
            return;
        }
        if confirmed {
            self.confirm_apply_graphic_template();
        }
    }
    /// Shows the "save as template" naming modal when [`App::saving_layer_template`] is
    /// `Some`. Commits via [`App::commit_save_layer_template`] on Enter or the Save button
    /// (a no-op, leaving the modal open, while the name is blank); discards on Escape/Cancel.
    pub(super) fn show_save_layer_template_modal(&mut self, ctx: &egui::Context) {
        if self.saving_layer_template.is_none() {
            return;
        }
        let locale = self.locale;
        let modal = egui::Modal::new(egui::Id::new("save_layer_template_modal"));
        let mut confirmed = false;
        let mut cancelled = false;
        let response = modal.show(ctx, |ui| {
            ui.set_width(320.0);
            components::modal_title(ui, Text::SaveTemplateTitle.tr(locale));
            ui.add_space(8.0);
            ui.label(Text::TemplateNameLabel.tr(locale));
            let buf = &mut self.saving_layer_template.as_mut().unwrap().1;
            let name_edit = ui.add(egui::TextEdit::singleline(buf).desired_width(f32::INFINITY));
            name_edit.request_focus();
            if name_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                confirmed = true;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                cancelled = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let name_blank = self
                    .saving_layer_template
                    .as_ref()
                    .is_some_and(|(_, n)| n.trim().is_empty());
                if ui
                    .add_enabled(
                        !name_blank,
                        egui::Button::new(Text::SaveTemplateConfirm.tr(locale)),
                    )
                    .clicked()
                {
                    confirmed = true;
                }
                if ui.button(Text::CancelJob.tr(locale)).clicked() {
                    cancelled = true;
                }
            });
        });
        if response.should_close() || cancelled {
            self.saving_layer_template = None;
            return;
        }
        if confirmed {
            self.commit_save_layer_template();
        }
    }
    /// Snapshots `multi_selected_clip_ids`' per-layer `(TrackKind, ClipFormatting)`, ordered by
    /// track position then start time, and stages it in [`App::saving_layer_template`] for the
    /// naming modal — what the toolbar's "Salvar como template" button does. A no-op if nothing
    /// is multi-selected.
    pub fn begin_save_layer_template(&mut self) {
        if self.multi_selected_clip_ids.is_empty() {
            return;
        }
        let ids = self.multi_selected_clip_ids.clone();
        let timeline = self.active_project().timeline();
        let mut layers: Vec<(
            usize,
            f64,
            avcore::timeline::TrackKind,
            avcore::ClipFormatting,
        )> = Vec::new();
        for (track_idx, track) in timeline.tracks.iter().enumerate() {
            for clip in &track.clips {
                if ids.contains(&clip.id) {
                    layers.push((track_idx, clip.start_secs, track.kind, clip.formatting()));
                }
            }
        }
        if layers.is_empty() {
            return;
        }
        layers.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let layers = layers
            .into_iter()
            .map(|(_, _, kind, f)| (kind, f))
            .collect();
        self.saving_layer_template = Some((layers, String::new()));
    }

    /// Commits [`App::saving_layer_template`] as a new entry in `prefs.saved_layer_templates`
    /// and persists it — what the save-template modal's confirm button does. A no-op (leaving
    /// the modal open) if the name is blank; the caller is expected to have already validated
    /// this before offering the confirm button, but this re-checks rather than trust that.
    pub fn commit_save_layer_template(&mut self) {
        let Some((layers, name)) = self.saving_layer_template.clone() else {
            return;
        };
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        self.prefs
            .saved_layer_templates
            .push(avcore::LayerTemplate { name, layers });
        self.saving_layer_template = None;
        self.save_prefs();
    }

    /// Removes `prefs.saved_layer_templates[index]` and persists the change — what the
    /// templates list popup's delete button does. A no-op if `index` is out of range.
    pub fn delete_layer_template(&mut self, index: usize) {
        if index >= self.prefs.saved_layer_templates.len() {
            return;
        }
        self.prefs.saved_layer_templates.remove(index);
        self.save_prefs();
    }

    /// Opens the apply-template modal for `prefs.saved_layer_templates[template_index]`, with
    /// one empty asset slot per saved layer for the user to fill in. A no-op if `template_index`
    /// is out of range.
    pub fn begin_apply_layer_template(&mut self, template_index: usize) {
        let Some(template) = self.prefs.saved_layer_templates.get(template_index) else {
            return;
        };
        self.applying_layer_template = Some((template_index, vec![None; template.layers.len()]));
        self.layer_templates_menu_open = false;
    }

    /// Applies [`App::applying_layer_template`]: for every layer with an asset chosen, creates a
    /// brand-new track of that layer's kind ([`create_new_track`] — never reuses an existing
    /// track, so a multi-video-layer template doesn't collapse its layers onto the same track),
    /// adds a clip for the chosen asset at the current playhead, then applies the layer's saved
    /// [`avcore::ClipFormatting`] onto it. Layers left without an asset are silently skipped —
    /// the modal's own "Aplicar" button is expected to gate on every slot being filled, but this
    /// degrades gracefully (partial apply) rather than refusing outright if it isn't. A no-op if
    /// nothing was staged or the referenced template no longer exists (deleted mid-modal).
    pub fn confirm_apply_layer_template(&mut self) {
        let Some((template_index, layer_asset_ids)) = self.applying_layer_template.take() else {
            return;
        };
        let Some(template) = self
            .prefs
            .saved_layer_templates
            .get(template_index)
            .cloned()
        else {
            return;
        };
        let playhead_secs = self.active_project().timeline().playhead_secs;
        for (layer_formatting, asset_id) in template
            .layers
            .iter()
            .map(|(_, formatting)| formatting)
            .zip(layer_asset_ids.iter())
        {
            let Some(asset_id) = asset_id else { continue };
            let Some((kind, duration_secs)) = self.asset_kind_and_duration(*asset_id) else {
                continue;
            };
            let timeline = self.active_project_mut().timeline_mut();
            let track_index = create_new_track(timeline, kind);
            let clip_id = next_clip_id(timeline);
            timeline.tracks[track_index]
                .clips
                .push(avcore::timeline::ClipInstance {
                    id: clip_id,
                    asset_id: *asset_id,
                    start_secs: playhead_secs,
                    source_in_secs: 0.0,
                    source_out_secs: duration_secs,
                    composite_id: None,
                    color_label: None,
                    gain_db: 0.0,
                    frozen: false,
                    speed_factor: 1.0,
                    speed_ramp_end_factor: None,
                    nested_sequence_id: None,
                    crop_x: 0.0,
                    crop_y: 0.0,
                    crop_w: 1.0,
                    crop_h: 1.0,
                    mask_shape: avcore::timeline::MaskShape::None,
                    mask_corner_radius: 0.0,
                    flipped_h: false,
                    color_filter: avcore::timeline::ColorFilter::None,
                    vignette_intensity: 0.0,
                    brightness: 0.0,
                    contrast: 1.0,
                    saturation: 1.0,
                    sharpen: 0.0,
                    chroma_key_enabled: false,
                    chroma_key_color: [0, 255, 0],
                    chroma_key_tolerance: 0.4,
                    blur_intensity: 0.0,
                    shake_intensity: 0.0,
                    glitch_intensity: 0.0,
                    pixelize_intensity: 0.0,
                    transition_in: avcore::timeline::TransitionType::None,
                    transition_duration_secs: 0.5,
                    position_keyframes: vec![],
                    scale_keyframes: vec![],
                    rotation_keyframes: vec![],
                    opacity_keyframes: vec![],
                    gain_keyframes: vec![],
                    voice_cleanup_enabled: false,
                    voice_cleanup_noise_floor_db: -30.0,
                    voice_cleanup_compressor_threshold_db: -18.0,
                    voice_cleanup_compressor_ratio: 3.0,
                    voice_cleanup_ceiling_linear: 0.95,
                    brightness_keyframes: vec![],
                    contrast_keyframes: vec![],
                    saturation_keyframes: vec![],
                    crop_x_keyframes: vec![],
                    crop_y_keyframes: vec![],
                    crop_w_keyframes: vec![],
                    crop_h_keyframes: vec![],
                    deflicker_enabled: false,
                    lut_path: String::new(),
                    layer_scale_x: 1.0,
                    layer_scale_y: 1.0,
                    stabilization_intensity: 0.0,
                    background_removal_enabled: false,
                    background_removal_mask_path: String::new(),
                    blend_mode: avcore::timeline::BlendMode::Normal,
                    anchor_x: 0.5,
                    anchor_y: 0.5,
                    reframe_seed_point: None,
                    privacy_blur_enabled: false,
                    privacy_blur_mask_path: String::new(),
                    privacy_blur_sigma: 15.0,
                    privacy_blur_seed_vertices: Vec::new(),
                    privacy_blur_seed_center_x_frac: 0.0,
                    privacy_blur_seed_center_y_frac: 0.0,
                });
            timeline.tracks[track_index]
                .clips
                .last_mut()
                .expect("just pushed")
                .apply_formatting(layer_formatting);
        }
    }
}

/// Returns whether a media asset can fill a template layer of the given track kind.
fn media_kind_matches(kind: avcore::MediaKind, track_kind: avcore::timeline::TrackKind) -> bool {
    matches!(
        (kind, track_kind),
        (avcore::MediaKind::Video, avcore::timeline::TrackKind::Video)
            | (avcore::MediaKind::Audio, avcore::timeline::TrackKind::Audio)
    )
}

#[cfg(test)]
#[path = "layer_templates/layer_templates_test.rs"]
mod tests;
