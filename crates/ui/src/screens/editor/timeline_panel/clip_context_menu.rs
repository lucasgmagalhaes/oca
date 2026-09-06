use eframe::egui;

use crate::i18n::{Locale, Text};

use super::CLIP_COLOR_LABEL_PALETTE;

pub(super) struct ClipContextMenuRequests<'a> {
    pub(super) clicked: &'a mut Option<u64>,
    pub(super) split_at_playhead: &'a mut bool,
    pub(super) copies: &'a mut Vec<u64>,
    pub(super) cuts: &'a mut Vec<u64>,
    pub(super) paste: &'a mut bool,
    pub(super) merge_into_composite: &'a mut bool,
    pub(super) detach_audio: &'a mut Vec<u64>,
    pub(super) create_compound_clip: &'a mut bool,
    pub(super) open_nested_sequence: &'a mut Option<u64>,
    pub(super) speed_ramps: &'a mut Vec<(u64, f32, f32)>,
    pub(super) custom_speed_ramp: &'a mut Option<u64>,
    pub(super) copy_formatting: &'a mut Vec<u64>,
    pub(super) paste_formatting: &'a mut Vec<u64>,
    pub(super) color_labels: &'a mut Vec<(u64, Option<[u8; 3]>)>,
    pub(super) deletes: &'a mut Vec<u64>,
}

pub(super) fn show_clip_context_menu(
    response: egui::Response,
    clip: &avcore::timeline::ClipInstance,
    track: &avcore::timeline::Track,
    locale: Locale,
    covers_playhead: bool,
    has_clipboard_clip: bool,
    has_formatting_clipboard: bool,
    multi_selected_count: usize,
    requests: ClipContextMenuRequests<'_>,
) {
    response.context_menu(|ui| {
        *requests.clicked = Some(clip.id);
        if ui
            .add_enabled(
                covers_playhead,
                egui::Button::new(Text::ContextMenuSplit.tr(locale)),
            )
            .clicked()
        {
            *requests.split_at_playhead = true;
            ui.close();
        }
        if ui.button(Text::ContextMenuCopy.tr(locale)).clicked() {
            requests.copies.push(clip.id);
            ui.close();
        }
        if ui.button(Text::ContextMenuCut.tr(locale)).clicked() {
            requests.cuts.push(clip.id);
            ui.close();
        }
        if ui
            .add_enabled(
                has_clipboard_clip,
                egui::Button::new(Text::ContextMenuPaste.tr(locale)),
            )
            .clicked()
        {
            *requests.paste = true;
            ui.close();
        }
        ui.separator();
        if ui
            .add_enabled(
                multi_selected_count >= 2,
                egui::Button::new(Text::MergeIntoComposite.tr(locale)),
            )
            .clicked()
        {
            *requests.merge_into_composite = true;
            ui.close();
        }
        if track.kind == avcore::timeline::TrackKind::Video
            && ui.button(Text::ContextMenuDetachAudio.tr(locale)).clicked()
        {
            requests.detach_audio.push(clip.id);
            ui.close();
        }
        if track.kind == avcore::timeline::TrackKind::Video
            && clip.nested_sequence_id.is_none()
            && ui
                .button(Text::ContextMenuCreateCompoundClip.tr(locale))
                .clicked()
        {
            *requests.clicked = Some(clip.id);
            *requests.create_compound_clip = true;
            ui.close();
        }
        if let Some(nested_id) = clip.nested_sequence_id {
            if ui
                .button(Text::ContextMenuOpenCompoundClip.tr(locale))
                .clicked()
            {
                *requests.open_nested_sequence = Some(nested_id);
                ui.close();
            }
        }
        ui.menu_button(Text::ContextMenuSpeedRamp.tr(locale), |ui| {
            if ui.button(Text::SpeedRampSlowToFast.tr(locale)).clicked() {
                requests.speed_ramps.push((clip.id, 0.5, 2.0));
                ui.close();
            }
            if ui.button(Text::SpeedRampFastToSlow.tr(locale)).clicked() {
                requests.speed_ramps.push((clip.id, 2.0, 0.5));
                ui.close();
            }
            ui.separator();
            if ui.button(Text::SpeedRampCustom.tr(locale)).clicked() {
                *requests.custom_speed_ramp = Some(clip.id);
                ui.close();
            }
        });
        ui.separator();
        if ui
            .button(Text::ContextMenuCopyFormatting.tr(locale))
            .clicked()
        {
            requests.copy_formatting.push(clip.id);
            ui.close();
        }
        if ui
            .add_enabled(
                has_formatting_clipboard,
                egui::Button::new(Text::ContextMenuPasteFormatting.tr(locale)),
            )
            .clicked()
        {
            requests.paste_formatting.push(clip.id);
            ui.close();
        }
        ui.separator();
        ui.menu_button(Text::ContextMenuColorLabel.tr(locale), |ui| {
            for &[r, g, b] in CLIP_COLOR_LABEL_PALETTE {
                let swatch = egui::Color32::from_rgb(r, g, b);
                if ui.add(egui::Button::new("  ").fill(swatch)).clicked() {
                    requests.color_labels.push((clip.id, Some([r, g, b])));
                    ui.close();
                }
            }
            ui.separator();
            if ui
                .button(Text::ContextMenuColorLabelClear.tr(locale))
                .clicked()
            {
                requests.color_labels.push((clip.id, None));
                ui.close();
            }
        });
        ui.separator();
        if ui.button(Text::ContextMenuDelete.tr(locale)).clicked() {
            requests.deletes.push(clip.id);
            ui.close();
        }
    });
}
