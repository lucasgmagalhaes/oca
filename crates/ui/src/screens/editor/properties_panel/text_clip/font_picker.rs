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

use eframe::egui::{self, RichText};

use crate::i18n::Text;
use crate::theme;
/// FONT-01C: the categorized/searchable font picker body, shown inside the family
/// [`egui::ComboBox`]'s popup instead of a flat 43-item list. A search box filters by display
/// name, `family_id`, and catalog tags (e.g. typing "mono" surfaces every monospace family
/// regardless of category label); below it, matching families are grouped under their
/// [`avcore::font_catalog::FontCategory`] heading in the doc's fixed category order, each
/// category section omitted entirely when nothing in it matches. The search text is kept in
/// egui's own per-widget temp storage (keyed by this clip's id), not a new `App` field --
/// it's transient popup-local UI state, not project or even cross-frame app state.
pub(super) fn font_family_picker_body(
    ui: &mut egui::Ui,
    selected: &mut avcore::TextFontFamily,
    tc_id: u64,
    locale: crate::i18n::Locale,
) {
    let search_id = ui.id().with(("text_font_family_search", tc_id));
    let mut query = ui
        .data_mut(|d| d.get_temp::<String>(search_id))
        .unwrap_or_default();
    ui.add(
        egui::TextEdit::singleline(&mut query)
            .hint_text(Text::TextFontSearchHint.tr(locale))
            .desired_width(ui.available_width()),
    );
    ui.data_mut(|d| d.insert_temp(search_id, query.clone()));
    let needle = query.trim().to_lowercase();

    ui.add_space(4.0);
    egui::ScrollArea::vertical()
        .max_height(280.0)
        .show(ui, |ui| {
            for category in FONT_CATEGORY_ORDER {
                let matches: Vec<avcore::TextFontFamily> = avcore::TextFontFamily::ALL
                    .into_iter()
                    .filter(|family| font_family_category(family) == category)
                    .filter(|family| family_matches_search(family, &needle, locale))
                    .collect();
                if matches.is_empty() {
                    continue;
                }
                ui.label(
                    RichText::new(font_category_label(category, locale))
                        .size(10.5)
                        .color(theme::TEXT_MUTED),
                );
                for family in matches {
                    let label = text_font_family_label(&family, locale);
                    ui.selectable_value(selected, family, label);
                }
                ui.add_space(4.0);
            }
        });
}

const FONT_CATEGORY_ORDER: [avcore::font_catalog::FontCategory; 6] = [
    avcore::font_catalog::FontCategory::Sans,
    avcore::font_catalog::FontCategory::Display,
    avcore::font_catalog::FontCategory::Serif,
    avcore::font_catalog::FontCategory::Handwritten,
    avcore::font_catalog::FontCategory::Monospace,
    avcore::font_catalog::FontCategory::International,
];

/// Every family in [`avcore::TextFontFamily::ALL`] has a matching [`avcore::font_catalog::CATALOG`]
/// entry (see `font_catalog`'s own self-consistency test) -- the `Sans` fallback here only
/// guards against a future entry landing without one, never observed for a real family today.
fn font_family_category(family: &avcore::TextFontFamily) -> avcore::font_catalog::FontCategory {
    avcore::font_catalog::find_family(family.family_id())
        .map(|entry| entry.category)
        .unwrap_or(avcore::font_catalog::FontCategory::Sans)
}

fn font_category_label(
    category: avcore::font_catalog::FontCategory,
    locale: crate::i18n::Locale,
) -> &'static str {
    use avcore::font_catalog::FontCategory;
    match category {
        FontCategory::Sans => Text::FontCategorySans.tr(locale),
        FontCategory::Display => Text::FontCategoryDisplay.tr(locale),
        FontCategory::Serif => Text::FontCategorySerif.tr(locale),
        FontCategory::Handwritten => Text::FontCategoryHandwritten.tr(locale),
        FontCategory::Monospace => Text::FontCategoryMonospace.tr(locale),
        FontCategory::International => Text::FontCategoryInternational.tr(locale),
    }
}

fn family_matches_search(
    family: &avcore::TextFontFamily,
    needle: &str,
    locale: crate::i18n::Locale,
) -> bool {
    if needle.is_empty() {
        return true;
    }
    if text_font_family_label(family, locale)
        .to_lowercase()
        .contains(needle)
    {
        return true;
    }
    let Some(entry) = avcore::font_catalog::find_family(family.family_id()) else {
        return false;
    };
    entry.family_id.contains(needle) || entry.tags.iter().any(|tag| tag.contains(needle))
}

pub(super) fn text_font_family_label(
    family: &avcore::TextFontFamily,
    locale: crate::i18n::Locale,
) -> String {
    match family {
        avcore::TextFontFamily::Lato => Text::TextFontLato.tr(locale).to_string(),
        avcore::TextFontFamily::BebasNeue => Text::TextFontBebasNeue.tr(locale).to_string(),
        avcore::TextFontFamily::PlayfairDisplay => {
            Text::TextFontPlayfairDisplay.tr(locale).to_string()
        }
        avcore::TextFontFamily::PatrickHand => Text::TextFontPatrickHand.tr(locale).to_string(),
        avcore::TextFontFamily::AnonymousPro => Text::TextFontAnonymousPro.tr(locale).to_string(),
        avcore::TextFontFamily::ArchivoBlack => Text::TextFontArchivoBlack.tr(locale).to_string(),
        // FONT-01's own "unknown future ID... displays a missing-font warning" rule -- this
        // clip's persisted family_id (or, reading an old save, variant name) isn't one this
        // build recognizes; it renders as Lato (the fallback every unrecognized family gets)
        // but the picker still names the original id so the loss is visible, not silent.
        avcore::TextFontFamily::Unknown(id) => {
            format!("{} ({id})", Text::TextFontUnknown.tr(locale))
        }
        // FONT-01B's 37 new families reuse the catalog's own display_name directly rather than
        // a dedicated i18n key per family -- font family names are proper nouns, not
        // conventionally translated per-locale (there's no pt-BR equivalent for "Montserrat"),
        // so a per-locale key would just repeat the same string twice. No "· category" suffix
        // the original six have either -- that's cosmetic, not required, and FONT-01C's own
        // categorized/searchable selector (still open) is the real place a category grouping
        // belongs, not a string suffix on today's flat list.
        other => avcore::font_catalog::find_family(other.family_id())
            .map(|entry| entry.display_name.to_string())
            .unwrap_or_else(|| "Lato".to_string()),
    }
}
