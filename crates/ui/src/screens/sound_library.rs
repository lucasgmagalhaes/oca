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

use avcore::media::format_timecode;
use avcore::sound_library::{LibraryTrack, SoundCategory};
use eframe::egui::{self, RichText};

use crate::app::App;
use crate::components;
use crate::i18n::Text;
use crate::theme;

/// Renders the Música/SFX screen: a local catalog of tracks found under
/// `prefs.sound_library_path`, grouped by category, each addable to the timeline with one
/// click. See `avcore::sound_library`'s module docs for why nothing ships bundled.
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    app.ensure_active_project();
    let locale = app.locale;
    let mut add_clicked: Option<LibraryTrack> = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(theme::SPACE_LG);
        ui.horizontal(|ui| {
            components::page_title(ui, Text::SoundLibraryTitle.tr(locale));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(Text::SoundLibraryRescan.tr(locale)).clicked() {
                    app.rescan_sound_library();
                }
            });
        });
        ui.add_space(16.0);

        if app.prefs.sound_library_path.is_empty() {
            ui.label(
                RichText::new(Text::SoundLibraryNoFolderConfigured.tr(locale))
                    .color(theme::TEXT_MUTED),
            );
            return;
        }

        if app.sound_library_tracks.is_empty() {
            ui.label(RichText::new(Text::SoundLibraryEmpty.tr(locale)).color(theme::TEXT_MUTED));
            return;
        }

        for (category, label) in [
            (SoundCategory::Music, Text::SoundLibraryMusic),
            (SoundCategory::Sfx, Text::SoundLibrarySfx),
        ] {
            let tracks: Vec<&LibraryTrack> = app
                .sound_library_tracks
                .iter()
                .filter(|t| t.category == category)
                .collect();
            if tracks.is_empty() {
                continue;
            }

            components::section_label(ui, label.tr(locale));
            for track in tracks {
                egui::Frame::new()
                    .fill(theme::SURFACE)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER))
                    .corner_radius(theme::RADIUS_MD)
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&track.name).size(13.0));
                                ui.label(
                                    RichText::new(format_timecode(track.duration_secs))
                                        .size(10.5)
                                        .color(theme::TEXT_MUTED),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .button(Text::SoundLibraryAddToTimeline.tr(locale))
                                        .clicked()
                                    {
                                        add_clicked = Some(track.clone());
                                    }
                                },
                            );
                        });
                    });
                ui.add_space(6.0);
            }
            ui.add_space(10.0);
        }
    });

    if let Some(track) = add_clicked {
        app.add_sound_library_track_to_timeline(&track);
    }
}
