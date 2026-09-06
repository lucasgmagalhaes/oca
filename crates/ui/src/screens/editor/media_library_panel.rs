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

use avcore::media::format_timecode;
use eframe::egui::{self, RichText};

use crate::app::{App, MediaLibraryFilter, MediaViewMode};
use crate::components;
use crate::i18n::Text;
use crate::icons;
use crate::theme;

const ASSET_THUMB_SIZE: egui::Vec2 = egui::vec2(48.0, 28.0);
/// Thumbnail size for [`MediaViewMode::Grid`]'s tiles — bigger than [`ASSET_THUMB_SIZE`]'s list
/// rows since a grid tile has no adjacent filename/metadata column competing for width.
const GRID_ASSET_THUMB_SIZE: egui::Vec2 = egui::vec2(120.0, 72.0);

fn asset_thumb(
    ui: &mut egui::Ui,
    asset: &avcore::media::MediaAsset,
    thumbnail: Option<&egui::TextureHandle>,
) {
    asset_thumb_sized(ui, asset, ASSET_THUMB_SIZE, thumbnail);
}

fn asset_thumb_sized(
    ui: &mut egui::Ui,
    asset: &avcore::media::MediaAsset,
    size: egui::Vec2,
    thumbnail: Option<&egui::TextureHandle>,
) {
    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter_at(rect);
    match thumbnail {
        Some(texture) => {
            // The image fully covers `rect` below, so the rounded fill underneath never
            // actually shows through -- kept anyway so a texture with any transparency (none
            // today, but poster frames are plain RGBA) doesn't reveal square corners.
            painter.rect_filled(
                rect,
                egui::CornerRadius::same(theme::RADIUS_SM),
                theme::SURFACE_2,
            );
            painter.image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
        None => {
            let (fill, glyph) = match asset.kind {
                avcore::media::MediaKind::Video => (theme::SURFACE_2, icons::PLAY_STR),
                avcore::media::MediaKind::Audio => {
                    (theme::ACCENT_2.gamma_multiply(0.25), icons::MUSIC_STR)
                }
            };
            painter.rect_filled(rect, egui::CornerRadius::same(theme::RADIUS_SM), fill);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                glyph,
                egui::FontId::new(11.0, icons::family()),
                theme::TEXT_MUTED,
            );
        }
    }
    let badge_text = asset.duration_label();
    let badge_pos = rect.right_bottom() - egui::vec2(2.0, 2.0);
    painter.text(
        badge_pos,
        egui::Align2::RIGHT_BOTTOM,
        &badge_text,
        egui::FontId::proportional(8.0),
        theme::TEXT_PRIMARY,
    );
}

/// One tab-style filter chip in the media library's header row — the OCA mockup's "MEDIA / Bins
/// / Favorites / Recent" tab strip, not the plain `selectable_label` pill this used to be.
/// Reuses `properties_panel::properties_tab_bar`'s own accent-tint-fill/accent-stroke-when-active
/// convention (this codebase's one established "tab" look) instead of inventing a second one.
/// Returns whether it was clicked.
fn media_filter_tab(ui: &mut egui::Ui, active: bool, label: &str) -> bool {
    let text = RichText::new(label).color(if active {
        theme::ACCENT
    } else {
        theme::TEXT_SECONDARY
    });
    let button = egui::Button::new(text)
        .fill(if active {
            theme::ACCENT_TINT
        } else {
            egui::Color32::TRANSPARENT
        })
        .stroke(egui::Stroke::new(
            1.0,
            if active {
                theme::ACCENT
            } else {
                egui::Color32::TRANSPARENT
            },
        ));
    ui.add(button).clicked()
}

pub(super) fn media_library_panel(app: &mut App, ui: &mut egui::Ui, width: f32, height: f32) {
    let mut clicked_id = None;
    let mut add_to_timeline_id = None;
    let mut dropped_asset = None;
    let mut selected_filter = None;
    let mut edited_bin_id = None;
    let mut new_bin_clicked = false;
    let mut toggled_favorite_id = None;
    // Collected during the asset-list closure below, applied after it ends -- same
    // "collect-during-loop, apply-after" shape `screens::library`/`timeline_panel` use for
    // their own poster-frame requests, needed here because `App::request_thumbnail`/
    // `App::touch_thumbnails` take `&mut self` while the loop below iterates a shared borrow
    // of `app.active_project().media_library`.
    let mut thumbnail_requests: Vec<u64> = Vec::new();
    let mut thumbnail_touches: Vec<(u64, u64, i64)> = Vec::new();
    let project_id = app.active_project().id;

    components::panel_frame().show(ui, |ui| {
        ui.set_width(width);
        ui.set_height(height);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                components::section_label(ui, Text::MediaLibrary.tr(app.locale));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(crate::i18n::media_item_count_label(
                            app.locale,
                            app.active_project().media_library.len(),
                        ))
                        .size(11.0)
                        .color(theme::TEXT_MUTED),
                    );
                    ui.add_space(theme::SPACE_SM);
                    // ASCII "#"/"=" -- no vendored grid/list icon exists, and the raw "▦"/"☰"
                    // glyphs are the same tofu class already fixed elsewhere this session.
                    if ui
                        .selectable_label(app.media_view_mode == MediaViewMode::Grid, "#")
                        .on_hover_text(Text::MediaViewGrid.tr(app.locale))
                        .clicked()
                    {
                        app.media_view_mode = MediaViewMode::Grid;
                    }
                    if ui
                        .selectable_label(app.media_view_mode == MediaViewMode::List, "=")
                        .on_hover_text(Text::MediaViewList.tr(app.locale))
                        .clicked()
                    {
                        app.media_view_mode = MediaViewMode::List;
                    }
                });
            });
            ui.add_space(theme::SPACE_SM);
            ui.add(
                egui::TextEdit::singleline(&mut app.media_search)
                    .hint_text(Text::SearchMediaPlaceholder.tr(app.locale))
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(4.0);
            // Smart bins (P4 item 22) plus the Favorites/Recent filters -- a row of filter
            // chips above the asset list, single-selection (see MediaLibraryFilter's own
            // doc comment). "All" clears the filter; each bin is click-to-select,
            // double-click-to-edit (the rules, not the assets themselves -- there's nothing
            // else to double-click a filter chip for).
            ui.horizontal_wrapped(|ui| {
                if media_filter_tab(
                    ui,
                    app.media_filter == MediaLibraryFilter::All,
                    Text::SmartBinAll.tr(app.locale),
                ) {
                    selected_filter = Some(MediaLibraryFilter::All);
                }
                if media_filter_tab(
                    ui,
                    app.media_filter == MediaLibraryFilter::Favorites,
                    Text::MediaFilterFavorites.tr(app.locale),
                ) {
                    selected_filter = Some(MediaLibraryFilter::Favorites);
                }
                if media_filter_tab(
                    ui,
                    app.media_filter == MediaLibraryFilter::Recent,
                    Text::MediaFilterRecent.tr(app.locale),
                ) {
                    selected_filter = Some(MediaLibraryFilter::Recent);
                }
                for bin in &app.active_project().smart_bins {
                    let response = ui.selectable_label(
                        app.media_filter == MediaLibraryFilter::SmartBin(bin.id),
                        &bin.name,
                    );
                    if response.clicked() {
                        selected_filter = Some(MediaLibraryFilter::SmartBin(bin.id));
                    }
                    if response.double_clicked() {
                        edited_bin_id = Some(bin.id);
                    }
                }
                if ui.button(Text::SmartBinNew.tr(app.locale)).clicked() {
                    new_bin_clicked = true;
                }
            });
            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .id_salt("media_library_scroll")
                .show(ui, |ui| {
                    // Only the (small) bin rule is cloned here, not the assets it filters --
                    // `active_project()` is borrowed again right below for the actual iteration,
                    // which is fine since both borrows are immutable.
                    let bin = match app.media_filter {
                        MediaLibraryFilter::SmartBin(id) => app
                            .active_project()
                            .smart_bins
                            .iter()
                            .find(|b| b.id == id)
                            .cloned(),
                        _ => None,
                    };
                    let recent_asset_ids = app.active_project().recent_asset_ids.clone();
                    let media_filter = app.media_filter;
                    let search = app.media_search.to_lowercase();
                    let mut assets: Vec<_> = app
                        .active_project()
                        .media_library
                        .iter()
                        .filter(|a| {
                            let matches_filter = match media_filter {
                                MediaLibraryFilter::All => true,
                                MediaLibraryFilter::SmartBin(_) => {
                                    bin.as_ref().is_none_or(|b| b.matches(a))
                                }
                                MediaLibraryFilter::Favorites => a.favorited,
                                MediaLibraryFilter::Recent => recent_asset_ids.contains(&a.id),
                            };
                            matches_filter
                                && (search.is_empty()
                                    || a.file_name.to_lowercase().contains(&search))
                        })
                        .collect();
                    // Recent is most-recently-used-first, not the library's own insertion
                    // order -- everything else keeps that default order unchanged.
                    if media_filter == MediaLibraryFilter::Recent {
                        assets.sort_by_key(|a| {
                            recent_asset_ids
                                .iter()
                                .position(|&id| id == a.id)
                                .unwrap_or(usize::MAX)
                        });
                    }

                    // Shared across both layouts below: every asset's click/double-click/drag/
                    // drop behavior is identical, only the Frame's own content (list row vs.
                    // grid tile) differs.
                    let mut handle_interaction =
                        |ui: &egui::Ui,
                         asset: &avcore::media::MediaAsset,
                         response: egui::Response| {
                            if response.clicked() {
                                clicked_id = Some(asset.id);
                            }
                            if response.double_clicked() {
                                add_to_timeline_id = Some(asset.id);
                            }
                            if response.dragged() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                if let Some(pos) = response.interact_pointer_pos() {
                                    egui::Area::new(ui.id().with(("asset_drag_ghost", asset.id)))
                                        .fixed_pos(pos + egui::vec2(12.0, 12.0))
                                        .order(egui::Order::Tooltip)
                                        .interactable(false)
                                        .show(ui.ctx(), |ui| {
                                            egui::Frame::new()
                                                .fill(theme::SURFACE_2)
                                                .corner_radius(theme::RADIUS_SM)
                                                .inner_margin(egui::Margin::symmetric(8, 4))
                                                .show(ui, |ui| {
                                                    ui.label(
                                                        RichText::new(&asset.file_name).size(11.0),
                                                    );
                                                });
                                        });
                                }
                            }
                            if response.drag_stopped() {
                                if let Some(pos) = response.interact_pointer_pos() {
                                    dropped_asset = Some((asset.id, pos));
                                }
                            }
                        };

                    // Same poster-frame lookup for both layouts below: a cached texture is
                    // touched (keeps it alive in the LRU), a missing one for a video asset is
                    // queued for extraction -- both applied after this closure returns, since
                    // `App::touch_thumbnails`/`App::request_thumbnail` need `&mut app`.
                    let mut resolve_thumbnail = |asset: &avcore::media::MediaAsset| {
                        if asset.kind != avcore::media::MediaKind::Video {
                            return None;
                        }
                        let key = (project_id, asset.id, 0);
                        match app.thumbnail_state.thumbnail_textures.get(&key) {
                            Some(texture) => {
                                thumbnail_touches.push(key);
                                Some(texture)
                            }
                            None => {
                                thumbnail_requests.push(asset.id);
                                None
                            }
                        }
                    };

                    match app.media_view_mode {
                        MediaViewMode::List => {
                            for asset in assets.iter().copied() {
                                let selected = app.selected_asset_id == Some(asset.id);
                                let bg = if selected {
                                    theme::ACCENT.gamma_multiply(0.18)
                                } else {
                                    theme::SURFACE
                                };
                                let thumbnail = resolve_thumbnail(asset);
                                let response = egui::Frame::new()
                                    .fill(bg)
                                    .corner_radius(theme::RADIUS_MD)
                                    .inner_margin(egui::Margin::same(4))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            asset_thumb(ui, asset, thumbnail);
                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        RichText::new(&asset.file_name).size(12.0),
                                                    );
                                                    if components::icon_button(
                                                        ui,
                                                        icons::STAR_STR,
                                                        Text::ToggleFavorite.tr(app.locale),
                                                        components::IconButtonOpts {
                                                            family: Some(icons::family()),
                                                            color: Some(if asset.favorited {
                                                                theme::ACCENT
                                                            } else {
                                                                theme::TEXT_MUTED
                                                            }),
                                                            ..Default::default()
                                                        },
                                                    )
                                                    .clicked()
                                                    {
                                                        toggled_favorite_id = Some(asset.id);
                                                    }
                                                });
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} | {}",
                                                        asset.duration_label(),
                                                        asset
                                                            .resolution
                                                            .map(|(w, h)| format!("{w}x{h}"))
                                                            .unwrap_or_else(|| asset
                                                                .sample_rate_khz
                                                                .map(|k| format!("{k:.0}kHz"))
                                                                .unwrap_or_default())
                                                    ))
                                                    .size(10.0)
                                                    .color(theme::TEXT_MUTED),
                                                );
                                            });
                                        });
                                    })
                                    .response
                                    .interact(egui::Sense::click_and_drag());
                                handle_interaction(ui, asset, response);
                                ui.add_space(4.0);
                            }
                        }
                        MediaViewMode::Grid => {
                            ui.horizontal_wrapped(|ui| {
                                for asset in assets.iter().copied() {
                                    let selected = app.selected_asset_id == Some(asset.id);
                                    let bg = if selected {
                                        theme::ACCENT.gamma_multiply(0.18)
                                    } else {
                                        theme::SURFACE
                                    };
                                    let thumbnail = resolve_thumbnail(asset);
                                    let response = egui::Frame::new()
                                        .fill(bg)
                                        .corner_radius(theme::RADIUS_MD)
                                        .inner_margin(egui::Margin::same(4))
                                        .show(ui, |ui| {
                                            ui.set_max_width(GRID_ASSET_THUMB_SIZE.x);
                                            ui.vertical(|ui| {
                                                asset_thumb_sized(
                                                    ui,
                                                    asset,
                                                    GRID_ASSET_THUMB_SIZE,
                                                    thumbnail,
                                                );
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        RichText::new(&asset.file_name)
                                                            .size(10.0)
                                                            .color(theme::TEXT_PRIMARY),
                                                    );
                                                    if components::icon_button(
                                                        ui,
                                                        icons::STAR_STR,
                                                        Text::ToggleFavorite.tr(app.locale),
                                                        components::IconButtonOpts {
                                                            family: Some(icons::family()),
                                                            color: Some(if asset.favorited {
                                                                theme::ACCENT
                                                            } else {
                                                                theme::TEXT_MUTED
                                                            }),
                                                            ..Default::default()
                                                        },
                                                    )
                                                    .clicked()
                                                    {
                                                        toggled_favorite_id = Some(asset.id);
                                                    }
                                                });
                                            });
                                        })
                                        .response
                                        .interact(egui::Sense::click_and_drag());
                                    handle_interaction(ui, asset, response);
                                }
                            });
                        }
                    }
                });
        });
    });

    app.touch_thumbnails(&thumbnail_touches);
    for asset_id in thumbnail_requests {
        app.request_thumbnail(project_id, asset_id, 0);
    }

    if let Some(id) = clicked_id {
        app.select_asset(Some(id));
    }
    if let Some(dropped) = dropped_asset {
        app.pending_asset_drop = Some(dropped);
    }
    if let Some(id) = add_to_timeline_id {
        app.add_asset_to_timeline(id);
    }
    if let Some(filter) = selected_filter {
        app.media_filter = filter;
    }
    if let Some(bin_id) = edited_bin_id {
        app.begin_edit_smart_bin(bin_id);
    }
    if new_bin_clicked {
        app.begin_new_smart_bin();
    }
    if let Some(asset_id) = toggled_favorite_id {
        app.toggle_asset_favorite(asset_id);
    }
}
