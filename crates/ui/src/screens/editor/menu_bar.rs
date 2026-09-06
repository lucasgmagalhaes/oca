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

//! The Editor's top File/Edit/View/Sequence/Clip/Markers/Graphics/Help menu bar, per
//! `spec/architecture/editor-ui-visual-redesign.md`'s Top bar mapping — a second way to reach
//! actions the toolbar (`super::toolbar`) and per-clip/per-tab context menus already expose,
//! not new business logic of its own. Coexists with the toolbar (confirmed with the user before
//! building this — the doc left "does the toolbar still exist alongside it" as an open
//! decision); nothing is removed from it. `Window` still isn't built: no panel-layout-save/
//! restore UI exists behind it yet, and this file is wiring, not a place to invent one. `Help`
//! *is* built — it turned out the redesign doc's original "no About/docs dialog anywhere in the
//! app" finding was stale (Fase 8's auto-update work had already added one, reachable only from
//! Preferences until now) — see `help_menu` below.

mod actions;
mod project;

use eframe::egui;

use crate::app::App;
use crate::i18n::Text;

use actions::{analyze_menu, clip_menu, graphics_menu, markers_menu};
use project::{file_menu, sequence_menu};

pub(crate) fn menu_bar(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    egui::MenuBar::new().ui(ui, |ui| {
        file_menu(app, ui, locale);
        edit_menu(app, ui, locale);
        view_menu(app, ui, locale);
        sequence_menu(app, ui, locale);
        clip_menu(app, ui, locale);
        markers_menu(app, ui, locale);
        graphics_menu(app, ui, locale);
        analyze_menu(app, ui, locale);
        help_menu(app, ui, locale);
    });
}

fn edit_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.menu_button(Text::MenuEdit.tr(locale), |ui| {
        if ui
            .add_enabled(
                app.can_undo(),
                egui::Button::new(Text::ShortcutUndo.tr(locale)),
            )
            .on_hover_text(app.prefs.key_bindings.undo.display())
            .clicked()
        {
            app.undo();
            ui.close();
        }
        if ui
            .add_enabled(
                app.can_redo(),
                egui::Button::new(Text::ShortcutRedo.tr(locale)),
            )
            .on_hover_text(app.prefs.key_bindings.redo.display())
            .clicked()
        {
            app.redo();
            ui.close();
        }
        ui.separator();
        if ui
            .button(Text::ContextMenuCopy.tr(locale))
            .on_hover_text("Ctrl+C")
            .clicked()
        {
            app.copy_selected_clip();
            ui.close();
        }
        if ui
            .button(Text::ContextMenuCut.tr(locale))
            .on_hover_text("Ctrl+X")
            .clicked()
        {
            app.cut_selected_clip();
            ui.close();
        }
        if ui
            .button(Text::ContextMenuPaste.tr(locale))
            .on_hover_text("Ctrl+V")
            .clicked()
        {
            app.paste_clip_at_playhead();
            ui.close();
        }
        ui.separator();
        if ui
            .button(Text::ContextMenuCopyFormatting.tr(locale))
            .on_hover_text(app.prefs.key_bindings.copy_formatting.display())
            .clicked()
        {
            app.copy_selected_clip_formatting();
            ui.close();
        }
        if ui
            .button(Text::ContextMenuPasteFormatting.tr(locale))
            .on_hover_text(app.prefs.key_bindings.paste_formatting.display())
            .clicked()
        {
            app.paste_selected_clip_formatting();
            ui.close();
        }
    });
}

fn view_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.menu_button(Text::MenuView.tr(locale), |ui| {
        if ui
            .checkbox(
                &mut app.timeline_index_open,
                Text::TimelineIndexToggle.tr(locale),
            )
            .clicked()
        {
            ui.close();
        }
        if ui
            .checkbox(
                &mut app.transcript_panel_open,
                Text::TranscriptPanelToggle.tr(locale),
            )
            .clicked()
        {
            ui.close();
        }
        if ui
            .checkbox(
                &mut app.preview_state.scopes_enabled,
                Text::PreviewScopesToggle.tr(locale),
            )
            .clicked()
        {
            ui.close();
        }
        ui.separator();
        if ui.button(Text::EnterFullscreenPreview.tr(locale)).clicked() {
            app.toggle_fullscreen_preview();
            ui.close();
        }
    });
}

/// "Help" — just the one real destination this app has: the About modal (version, update-check
/// status, install/restart flow — already fully built, `App::open_about`), previously only
/// reachable via Preferences. No separate "Documentation" entry: there's no hosted docs site for
/// this project to link to, and inventing one would be exactly the kind of unverified URL
/// CLAUDE.md's own rules say not to guess at.
fn help_menu(app: &mut App, ui: &mut egui::Ui, locale: crate::i18n::Locale) {
    ui.menu_button(Text::MenuHelp.tr(locale), |ui| {
        if ui.button(Text::MenuHelpAbout.tr(locale)).clicked() {
            app.open_about();
            ui.close();
        }
    });
}
