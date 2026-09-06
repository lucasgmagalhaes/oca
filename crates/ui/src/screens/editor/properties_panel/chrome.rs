use eframe::egui::{self, RichText};

use crate::app::App;
use crate::components;
use crate::i18n::Text;
use crate::theme;
pub(super) fn effects_panel_browser(
    app: &mut App,
    ui: &mut egui::Ui,
    locale: crate::i18n::Locale,
    has_selection: bool,
    track_kind: Option<avcore::timeline::TrackKind>,
) {
    use crate::app::effects_panel::EffectPreset;

    components::section_label(ui, Text::EffectsPanelTitle.tr(locale));
    if !has_selection {
        ui.label(
            RichText::new(Text::EffectsPanelNoSelectionHint.tr(locale))
                .size(11.0)
                .color(theme::TEXT_MUTED),
        );
    }
    ui.add_space(4.0);

    let mut applied: Option<EffectPreset> = None;
    let mut current_category = None;
    for preset in EffectPreset::ALL {
        if preset.video_only() && track_kind == Some(avcore::timeline::TrackKind::Audio) {
            continue;
        }
        if current_category != Some(preset.category()) {
            current_category = Some(preset.category());
            ui.add_space(if current_category.is_some() { 6.0 } else { 0.0 });
            ui.label(
                RichText::new(preset.category().label(locale))
                    .size(10.5)
                    .color(theme::TEXT_SECONDARY),
            );
        }
        let resp = ui.add(
            egui::Button::new(RichText::new(preset.label(locale)).size(12.0))
                .min_size(egui::vec2(ui.available_width(), 22.0))
                .fill(egui::Color32::TRANSPARENT),
        );
        if resp.double_clicked() && has_selection {
            applied = Some(preset);
        }
    }

    if let Some(preset) = applied {
        app.apply_effect_preset(preset);
    }
}

/// Inspector/Effects/Audio tab strip (`editor-ui-visual-redesign.md`'s Inspector mapping) —
/// a pure regrouping of the property sections already drawn below, matching
/// [`tool_button`](super::tool_button)'s active/inactive chip styling so the panel's chrome
/// stays consistent with the toolbar's own tab-like controls.
pub(super) fn properties_tab_bar(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    use crate::app::PropertiesTab;
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        for (tab, label) in [
            (
                PropertiesTab::Inspector,
                Text::PropertiesTabInspector.tr(locale),
            ),
            (
                PropertiesTab::Effects,
                Text::PropertiesTabEffects.tr(locale),
            ),
            (PropertiesTab::Audio, Text::PropertiesTabAudio.tr(locale)),
        ] {
            let active = app.properties_tab == tab;
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
            if ui.add(button).clicked() {
                app.properties_tab = tab;
            }
        }
    });
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);
}

pub(super) fn prop_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(12.0).color(theme::TEXT_MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(12.0));
        });
    });
}

/// The OCA mockup's "vertical stereo (L/R) audio meter with dB ticks"
/// (`spec/architecture/editor-ui-visual-redesign.md`'s Inspector section) — a real per-channel
/// meter now that `avcore::preview`'s buffer probe reports one (`AudioLevel::peak_l`/`rms_l`/
/// `peak_r`/`rms_r`), not the same mono value drawn twice into two bars the doc explicitly
/// called out as the thing not to do. `DB_TICKS`/`DB_FLOOR` set a fixed -60..0 dB display range,
/// matching a small meter widget's scope rather than a full calibrated broadcast meter.
/// Renders at `meter_height` tall — the caller decides how much vertical space it gets. Moved
/// out of the Audio tab's own content (where it used to be tab-gated, `stereo_db_meter(ui,
/// app.current_audio_level())` inside `if tab == PropertiesTab::Audio`) into its own persistent
/// column, per `spec/architecture/editor-ui-visual-redesign.md`'s Inspector section: the OCA
/// mockup shows this meter as a fixed vertical strip along the whole editor body's right edge,
/// visible regardless of which Inspector/Effects/Audio tab is active or whether a clip is even
/// selected — not nested inside one tab's content. See `screens::editor::audio_meter_column`.
pub(crate) fn stereo_db_meter(ui: &mut egui::Ui, level: avcore::AudioLevel, meter_height: f32) {
    const BAR_WIDTH: f32 = 16.0;
    const BAR_GAP: f32 = 4.0;
    const LABEL_WIDTH: f32 = 26.0;
    const DB_FLOOR: f32 = -60.0;
    const DB_TICKS: [f32; 5] = [0.0, -6.0, -12.0, -24.0, -48.0];

    fn amplitude_to_unit(amplitude: f32) -> f32 {
        if amplitude <= 0.0 {
            return 0.0;
        }
        let db = 20.0 * amplitude.log10();
        ((db - DB_FLOOR) / -DB_FLOOR).clamp(0.0, 1.0)
    }

    let (rect, _response) = ui.allocate_exact_size(
        egui::vec2(LABEL_WIDTH + BAR_WIDTH * 2.0 + BAR_GAP, meter_height),
        egui::Sense::hover(),
    );
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter();
    let bars_left = rect.left() + LABEL_WIDTH;
    let l_rect = egui::Rect::from_min_size(
        egui::pos2(bars_left, rect.top()),
        egui::vec2(BAR_WIDTH, meter_height),
    );
    let r_rect = egui::Rect::from_min_size(
        egui::pos2(bars_left + BAR_WIDTH + BAR_GAP, rect.top()),
        egui::vec2(BAR_WIDTH, meter_height),
    );

    for db in DB_TICKS {
        let unit = ((db - DB_FLOOR) / -DB_FLOOR).clamp(0.0, 1.0);
        let y = rect.bottom() - unit * meter_height;
        painter.text(
            egui::pos2(rect.left(), y),
            egui::Align2::LEFT_CENTER,
            format!("{db:.0}"),
            egui::FontId::monospace(8.5),
            theme::TEXT_MUTED,
        );
        painter.hline(
            l_rect.left()..=r_rect.right(),
            y,
            egui::Stroke::new(1.0, theme::BORDER.gamma_multiply(0.6)),
        );
    }

    for (bar_rect, peak, rms) in [
        (l_rect, level.peak_l, level.rms_l),
        (r_rect, level.peak_r, level.rms_r),
    ] {
        painter.rect_filled(bar_rect, theme::RADIUS_XS, theme::SURFACE_2);
        let rms_unit = amplitude_to_unit(rms);
        if rms_unit > 0.0 {
            let filled_h = rms_unit * bar_rect.height();
            let filled_rect = egui::Rect::from_min_size(
                egui::pos2(bar_rect.left(), bar_rect.bottom() - filled_h),
                egui::vec2(bar_rect.width(), filled_h),
            );
            // Standard green/yellow/red audio-meter convention — was a flat `theme::ACCENT`
            // regardless of level (confirmed via a real screenshot: a thick, unvarying purple
            // bar, not a meter). `ACCENT`/Petroleum Blue means interaction, not a passive level
            // readout, per the design doc's own "don't flood the interface with the accent"
            // rule anyway.
            let fill_color = if rms_unit > 0.9 {
                theme::ERROR
            } else if rms_unit > 0.7 {
                theme::WARNING
            } else {
                theme::SUCCESS
            };
            painter.rect_filled(filled_rect, theme::RADIUS_XS, fill_color);
        }
        let peak_unit = amplitude_to_unit(peak);
        if peak_unit > 0.0 {
            let peak_y = bar_rect.bottom() - peak_unit * bar_rect.height();
            let color = if peak > 0.98 {
                theme::ERROR
            } else {
                theme::ACCENT_2
            };
            painter.hline(bar_rect.x_range(), peak_y, egui::Stroke::new(2.0, color));
        }
    }
}
