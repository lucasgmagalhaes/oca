use eframe::egui::{self, RichText};

use crate::app::App;
use crate::components;
use crate::i18n::{Locale, Text};
use crate::icons;
use crate::theme;

use super::CLIP_COLOR_LABEL_PALETTE;

pub(super) struct TrackHeaderRequests<'a> {
    pub(super) toggle_visibility: &'a mut Vec<u64>,
    pub(super) toggle_lock: &'a mut Vec<u64>,
    pub(super) toggle_collapsed: &'a mut Vec<u64>,
    pub(super) audio_role: &'a mut Vec<(u64, avcore::AudioRole)>,
    pub(super) color_label: &'a mut Vec<(u64, Option<[u8; 3]>)>,
    pub(super) rename: &'a mut Vec<(u64, String)>,
    pub(super) duplicate: &'a mut Vec<u64>,
    pub(super) move_up: &'a mut Vec<u64>,
    pub(super) move_down: &'a mut Vec<u64>,
    pub(super) delete: &'a mut Vec<(u64, String)>,
}

pub(super) fn draw_track_header(
    app: &App,
    ui: &mut egui::Ui,
    track: &avcore::timeline::Track,
    row_height: f32,
    track_label_width: f32,
    locale: Locale,
    requests: TrackHeaderRequests<'_>,
) {
    ui.allocate_ui_with_layout(
        egui::vec2(track_label_width, row_height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            let track_id = track.id;
            let visible = track.visible;
            // Audio tracks read as "muted/unmuted" (a speaker glyph) rather than
            // "hidden/shown" (an eye) — same `visible` flag underneath, since an
            // invisible video track and a muted audio track are the same "doesn't
            // contribute to preview/export" concept. No Lucide speaker icon is
            // vendored (`spec/architecture/editor-ui-visual-redesign.md`'s Icon
            // set section only covers `eye`/`eye-off`) — and the raw "🔊"/"🔇"
            // emoji this used to fall back to is the same tofu class already
            // fixed elsewhere this session, so audio tracks now reuse the same
            // eye/eye-off icon as video tracks rather than a distinct glyph.
            let is_audio = track.kind == avcore::timeline::TrackKind::Audio;
            let (eye, eye_family) = if visible {
                (icons::EYE_STR, Some(icons::family()))
            } else {
                (icons::EYE_OFF_STR, Some(icons::family()))
            };
            let tooltip = match (is_audio, visible) {
                (true, true) => Text::TrackMute.tr(locale),
                (true, false) => Text::TrackUnmute.tr(locale),
                (false, true) => Text::TrackHide.tr(locale),
                (false, false) => Text::TrackShow.tr(locale),
            };
            if components::icon_button(
                ui,
                eye,
                tooltip,
                components::IconButtonOpts {
                    family: eye_family,
                    ..Default::default()
                },
            )
            .clicked()
            {
                requests.toggle_visibility.push(track_id);
            }
            let locked = track.locked;
            let lock_glyph = if locked {
                icons::LOCK_STR
            } else {
                icons::LOCK_OPEN_STR
            };
            let lock_tooltip = if locked {
                Text::TrackUnlock.tr(locale)
            } else {
                Text::TrackLock.tr(locale)
            };
            // Section 45's Track Lock spec: "Visual: lock icon becomes
            // active" — was glyph-only (open/closed padlock), same rest color
            // regardless of state, before this fix.
            if components::icon_button(
                ui,
                lock_glyph,
                lock_tooltip,
                components::IconButtonOpts {
                    family: Some(icons::family()),
                    color: locked.then_some(theme::ACCENT),
                    ..Default::default()
                },
            )
            .clicked()
            {
                requests.toggle_lock.push(track_id);
            }
            // Matches the mockup's `< >` track-header control (confirmed via
            // a real screenshot) — collapses/expands just this track's row
            // height. Plain ASCII ("v"/">"), not the Geometric-Shapes
            // chevrons the mockup itself uses — same confirmed-tofu class
            // (against this app's bundled default font) as every other icon
            // fixed this session.
            let collapsed = app.collapsed_track_ids.contains(&track_id);
            let (collapse_glyph, collapse_tooltip) = if collapsed {
                (">", Text::TrackExpand.tr(locale))
            } else {
                ("v", Text::TrackCollapse.tr(locale))
            };
            if components::icon_button(
                ui,
                collapse_glyph,
                collapse_tooltip,
                components::IconButtonOpts::default(),
            )
            .clicked()
            {
                requests.toggle_collapsed.push(track_id);
            }
            let name_response = ui.add(
                egui::Label::new(RichText::new(&track.name).size(11.0).color(
                    if let Some([r, g, b]) = track.color_label {
                        egui::Color32::from_rgb(r, g, b)
                    } else if visible {
                        theme::TEXT_SECONDARY
                    } else {
                        theme::TEXT_MUTED
                    },
                ))
                .truncate()
                .sense(egui::Sense::click()),
            );
            name_response.context_menu(|ui| {
                // Section 49's Track More Menu — Rename/Duplicate/Delete/
                // Move Up/Move Down, added to this existing track-color
                // context menu rather than a second right-click surface.
                if ui.button(Text::TrackCtxRename.tr(locale)).clicked() {
                    requests.rename.push((track_id, track.name.clone()));
                    ui.close();
                }
                if ui.button(Text::TrackCtxDuplicate.tr(locale)).clicked() {
                    requests.duplicate.push(track_id);
                    ui.close();
                }
                if ui.button(Text::TrackCtxMoveUp.tr(locale)).clicked() {
                    requests.move_up.push(track_id);
                    ui.close();
                }
                if ui.button(Text::TrackCtxMoveDown.tr(locale)).clicked() {
                    requests.move_down.push(track_id);
                    ui.close();
                }
                if ui.button(Text::TrackCtxDelete.tr(locale)).clicked() {
                    requests.delete.push((track_id, track.name.clone()));
                    ui.close();
                }
                ui.separator();
                for &[r, g, b] in CLIP_COLOR_LABEL_PALETTE {
                    let swatch = egui::Color32::from_rgb(r, g, b);
                    if ui.add(egui::Button::new("  ").fill(swatch)).clicked() {
                        requests.color_label.push((track_id, Some([r, g, b])));
                        ui.close();
                    }
                }
                ui.separator();
                if ui
                    .button(Text::ContextMenuColorLabelClear.tr(locale))
                    .clicked()
                {
                    requests.color_label.push((track_id, None));
                    ui.close();
                }
            });
            // D2 (`spec/architecture/differentiators.md`): which audio source
            // this track carries, if any — Text/Shape tracks never carry audio,
            // so they don't get the picker at all.
            if matches!(
                track.kind,
                avcore::timeline::TrackKind::Video | avcore::timeline::TrackKind::Audio
            ) {
                let mut role = track.audio_role;
                egui::ComboBox::from_id_salt(("track_audio_role", track_id))
                    .selected_text(audio_role_icon(role))
                    .width(28.0)
                    .show_ui(ui, |ui| {
                        for candidate in [
                            avcore::AudioRole::Unspecified,
                            avcore::AudioRole::GameAudio,
                            avcore::AudioRole::Mic,
                            avcore::AudioRole::Music,
                        ] {
                            ui.selectable_value(&mut role, candidate, audio_role_icon(candidate));
                        }
                    })
                    .response
                    .on_hover_text(audio_role_label(role, locale));
                if role != track.audio_role {
                    requests.audio_role.push((track_id, role));
                }
            }
        },
    );
}

pub(super) fn audio_role_icon(role: avcore::AudioRole) -> &'static str {
    match role {
        avcore::AudioRole::Unspecified => "-",
        avcore::AudioRole::GameAudio => "G",
        avcore::AudioRole::Mic => "V",
        avcore::AudioRole::Music => "N",
    }
}

pub(super) fn audio_role_label(role: avcore::AudioRole, locale: Locale) -> &'static str {
    match role {
        avcore::AudioRole::Unspecified => Text::AudioRoleUnspecified.tr(locale),
        avcore::AudioRole::GameAudio => Text::AudioRoleGameAudio.tr(locale),
        avcore::AudioRole::Mic => Text::AudioRoleMic.tr(locale),
        avcore::AudioRole::Music => Text::AudioRoleMusic.tr(locale),
    }
}

#[cfg(test)]
#[path = "track_header/track_header_test.rs"]
mod tests;
