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

use crate::app::{App, EditorTool, PropertiesTab};
use crate::components;
use crate::i18n::Text;
use crate::theme;

pub(crate) fn toolbar(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    ui.horizontal(|ui| {
        tool_button_icon_font(
            app,
            ui,
            EditorTool::Select,
            crate::icons::MOUSE_POINTER_2_STR,
            Text::ToolSelect.tr(locale),
        );
        let cut_job = components::icon_label_job(
            crate::icons::SCISSORS_STR,
            crate::icons::family(),
            14.0,
            theme::TEXT_PRIMARY,
            Text::ToolCut.tr(locale),
            14.0,
            theme::TEXT_PRIMARY,
        );
        if ui.button(cut_job).on_hover_text("Ctrl+B").clicked() {
            app.split_at_playhead();
        }
        // `CINECUT_UI_UX_SPEC_v1.0.md`'s product-decisions addendum, Section 2: "final toolbar
        // for the current version is: Selection, Razor, Trim, Text, Effects, Hand, Zoom" — this
        // replaces the previous Trim/Ripple/Roll/Slip/Slide five-button row. Ripple/Roll/Slip/
        // Slide stay real `EditorTool` variants (see that enum's own doc comment) — the spec's
        // instruction is about the toolbar's exposed buttons, not a mandate to delete working
        // trim behavior that has no other entry point — just no longer have a toolbar button of
        // their own.
        tool_button(app, ui, EditorTool::Razor, "C", Text::ToolRazor.tr(locale));
        // `fold-horizontal` is the mockup's resolved best-guess for the Trim tool-rail slot (see
        // `spec/architecture/editor-ui-visual-redesign.md`'s Icon set table).
        tool_button_icon_font(
            app,
            ui,
            EditorTool::Trim,
            crate::icons::FOLD_HORIZONTAL_STR,
            Text::ToolTrim.tr(locale),
        );
        tool_button(app, ui, EditorTool::Text, "T", Text::ToolText.tr(locale));
        tool_button(
            app,
            ui,
            EditorTool::Effects,
            "FX",
            Text::ToolEffects.tr(locale),
        );
        // Section 8: activating Effects mode also focuses the Effects Panel — no separate
        // click-to-open step. Idempotent (harmless to re-set every frame the tool stays active).
        if app.tool == EditorTool::Effects {
            app.properties_tab = PropertiesTab::Effects;
        }
        tool_button_icon_font(
            app,
            ui,
            EditorTool::Hand,
            crate::icons::HAND_STR,
            Text::ToolHand.tr(locale),
        );
        tool_button(
            app,
            ui,
            EditorTool::Zoom,
            "Z",
            Text::ToolZoomTool.tr(locale),
        );
        ui.separator();
        // Section 56's Snapping spec: "a timeline-level toggle... click magnet icon". No
        // vendored Lucide magnet icon exists, and this session's own tofu-glyph hunt already
        // showed a plain Unicode magnet symbol isn't a safe bet against this app's bundled
        // default font — a plain text toggle (matching this exact toolbar's own tool-button
        // convention of a visible label) is the honest choice here, not a risky icon guess.
        if ui
            .selectable_label(app.snap_enabled, Text::SnapToggle.tr(locale))
            .on_hover_text(Text::SnapToggleHint.tr(locale))
            .clicked()
        {
            app.snap_enabled = !app.snap_enabled;
        }
        ui.separator();
        // "↺"/"↻" read as tofu (same class already fixed elsewhere) -- no vendored undo/redo
        // icon exists either, so these two buttons show the already-i18n'd action name instead.
        if ui
            .add_enabled(
                app.can_undo(),
                egui::Button::new(Text::ShortcutUndo.tr(locale)),
            )
            .on_hover_text(Text::ShortcutUndo.tr(locale))
            .clicked()
        {
            app.undo();
        }
        if ui
            .add_enabled(
                app.can_redo(),
                egui::Button::new(Text::ShortcutRedo.tr(locale)),
            )
            .on_hover_text(Text::ShortcutRedo.tr(locale))
            .clicked()
        {
            app.redo();
        }
        ui.separator();
        // No "🏷"/"📝" prefix -- no vendored icon for either, and the raw emoji is the same
        // tofu class already fixed elsewhere; the label text alone still conveys the toggle.
        if ui
            .selectable_label(
                app.timeline_index_open,
                Text::TimelineIndexToggle.tr(locale),
            )
            .clicked()
        {
            app.toggle_timeline_index();
        }
        if ui
            .selectable_label(
                app.transcript_panel_open,
                Text::TranscriptPanelToggle.tr(locale),
            )
            .clicked()
        {
            app.toggle_transcript_panel();
        }
    });
}

/// Row of tabs, one per sequence in the active project (per `request.md`'s Fase 3 "abas de
/// projeto" spec) — click a tab to switch which sequence's timeline the rest of the Editor
/// screen shows, or the trailing "+" to append a new empty one and switch to it.
#[derive(Clone, Copy)]
struct SequenceTabDrag {
    sequence_id: u64,
}

pub(crate) fn sequence_tab_bar(app: &mut App, ui: &mut egui::Ui) {
    let locale = app.locale;
    let active_index = app.active_project().active_sequence;
    let mut select_index = None;
    let mut rename_index = None;
    let mut duplicate_index = None;
    let mut move_request = None;
    let mut drag_move_request = None;
    let mut delete_request = None;
    let mut add_requested = false;

    ui.horizontal(|ui| {
        let count = app.active_project().sequences.len();
        for index in 0..count {
            let active = index == active_index;
            let sequence = &app.active_project().sequences[index];
            let sequence_id = sequence.id;
            let name = sequence.name.clone();
            let text = RichText::new(&name).color(if active {
                theme::ACCENT
            } else {
                theme::TEXT_SECONDARY
            });
            let button = egui::Button::new(text)
                .fill(if active {
                    theme::SURFACE_2
                } else {
                    theme::SURFACE
                })
                .sense(egui::Sense::click_and_drag());
            let resp = ui
                .add(button)
                .on_hover_text(Text::SequenceTabDragHint.tr(locale));
            resp.dnd_set_drag_payload(SequenceTabDrag { sequence_id });
            if resp
                .dnd_hover_payload::<SequenceTabDrag>()
                .is_some_and(|payload| payload.sequence_id != sequence_id)
            {
                ui.painter().rect_stroke(
                    resp.rect,
                    4,
                    egui::Stroke::new(2.0, theme::ACCENT),
                    egui::StrokeKind::Inside,
                );
            }
            if let Some(payload) = resp.dnd_release_payload::<SequenceTabDrag>() {
                if payload.sequence_id != sequence_id {
                    drag_move_request = Some((payload.sequence_id, sequence_id));
                }
            }
            if resp.clicked() && !active {
                select_index = Some(index);
            }
            // Section 41's Timeline Tab Bar: "Close X" per tab — previously only reachable via
            // the tab's right-click context menu (still there, unchanged). No per-sequence
            // "unsaved changes" tracking exists in this codebase to gate the spec's own "if
            // unsaved, show save/discard/cancel confirmation" — this closes directly, same as
            // the context menu's own "Excluir" already did with no such confirmation either;
            // documented here rather than silently guessed at.
            if count > 1
                && components::icon_button(
                    ui,
                    "X",
                    Text::SequenceTabCtxDelete.tr(locale),
                    components::IconButtonOpts::default(),
                )
                .clicked()
            {
                delete_request = Some((sequence_id, name.clone()));
            }
            resp.context_menu(|ui| {
                if ui
                    .button(crate::i18n::Text::SequenceTabCtxRename.tr(locale))
                    .clicked()
                {
                    rename_index = Some((index, name.clone()));
                }
                if ui
                    .button(Text::SequenceTabCtxDuplicate.tr(locale))
                    .clicked()
                {
                    duplicate_index = Some(index);
                }
                ui.separator();
                if ui
                    .add_enabled(
                        index > 0,
                        egui::Button::new(Text::SequenceTabCtxMoveLeft.tr(locale)),
                    )
                    .clicked()
                {
                    move_request = Some((index, index - 1));
                }
                if ui
                    .add_enabled(
                        index + 1 < count,
                        egui::Button::new(Text::SequenceTabCtxMoveRight.tr(locale)),
                    )
                    .clicked()
                {
                    move_request = Some((index, index + 1));
                }
                ui.separator();
                if ui
                    .add_enabled(
                        count > 1,
                        egui::Button::new(Text::SequenceTabCtxDelete.tr(locale)),
                    )
                    .clicked()
                {
                    delete_request = Some((sequence_id, name.clone()));
                }
            });
        }
        if ui.button(Text::AddSequenceTab.tr(locale)).clicked() {
            add_requested = true;
        }
    });

    if let Some(index) = select_index {
        app.select_sequence(index);
    }
    if let Some((index, current_name)) = rename_index {
        app.renaming_sequence = Some((index, current_name));
    }
    if let Some(index) = duplicate_index {
        app.duplicate_sequence(index);
    }
    if let Some((from_index, target_index)) = move_request {
        app.move_sequence(from_index, target_index);
    }
    if let Some((source_id, target_id)) = drag_move_request {
        let source_index = app
            .active_project()
            .sequences
            .iter()
            .position(|sequence| sequence.id == source_id);
        let target_index = app
            .active_project()
            .sequences
            .iter()
            .position(|sequence| sequence.id == target_id);
        if let (Some(source_index), Some(target_index)) = (source_index, target_index) {
            app.move_sequence(source_index, target_index);
        }
    }
    if let Some(sequence) = delete_request {
        app.deleting_sequence = Some(sequence);
    }
    if add_requested {
        app.add_sequence();
    }
}

/// Saves the active project to its remembered [`avcore::Project::file_path`], or prompts
/// for a destination (and remembers it for next time) if it doesn't have one yet.
pub(crate) fn save_active_project(app: &mut App) {
    let path = match app.active_project().file_path.clone() {
        Some(path) => Some(path),
        None => rfd::FileDialog::new()
            .add_filter("oca project", &["ocproj"])
            .set_file_name(format!("{}.ocproj", app.active_project().name))
            .save_file(),
    };
    let Some(path) = path else { return };

    app.sync_panel_layout_into_active_project();
    match avcore::save_project_to_file(app.active_project(), &path) {
        Ok(()) => {
            tracing::info!(path = %path.display(), "project saved");
            let path_str = path.display().to_string();
            app.active_project_mut().file_path = Some(path);
            // Track in recents (covers first Save As, where file_path was previously None).
            app.prefs.recent_project_paths.retain(|p| p != &path_str);
            app.prefs.recent_project_paths.insert(0, path_str);
            app.prefs.recent_project_paths.truncate(10);
            app.save_prefs();
        }
        Err(e) => app.push_toast(format!("Failed to save project: {e}")),
    }
}

/// Prompts for a destination and writes the active sequence's text-track captions out as a
/// standalone `.srt` file (`request.md`'s "arquivo `.srt` separado" half of the subtitle
/// export ask — the embedded raster-overlay half already happens on every normal export). Doesn't
/// remember the chosen path the way project saves do — each export is a one-off action, not an
/// ongoing document with its own save location.
pub(crate) fn export_srt_for_active_sequence(app: &mut App) {
    let locale = app.locale;
    let sequence = &app.active_project().sequences[app.active_project().active_sequence];
    let srt = avcore::export_srt(&sequence.timeline);
    let default_name = format!("{}.srt", sequence.name);
    if srt.is_empty() {
        app.push_toast(Text::ExportSrtEmpty.tr(locale).to_string());
        return;
    }

    let mut dialog = rfd::FileDialog::new()
        .add_filter("SubRip", &["srt"])
        .set_file_name(default_name);
    if !app.prefs.output_folder.is_empty() {
        dialog = dialog.set_directory(&app.prefs.output_folder);
    }
    let Some(path) = dialog.save_file() else {
        return;
    };

    match std::fs::write(&path, srt) {
        Ok(()) => tracing::info!(path = %path.display(), "subtitles exported"),
        Err(e) => app.push_toast(format!("Failed to export subtitles: {e}")),
    }
}

/// Toolbar tool-select chip: a filled, bordered pill that reads active/inactive at a glance,
/// matching `oca-editor-mock.html`'s `.tb-btn`/`.tb-btn.active` (accent-tinted fill + accent
/// border when selected) instead of the plain color-only text button this used to be.
fn tool_button(app: &mut App, ui: &mut egui::Ui, tool: EditorTool, icon: &str, label: &str) {
    let active = app.tool == tool;
    let text = RichText::new(format!("{icon} {label}")).color(if active {
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
        app.tool = tool;
    }
}

/// Same active/inactive chip styling as [`tool_button`], for a tool that has a real vendored
/// Lucide icon (see `spec/architecture/editor-ui-visual-redesign.md`'s Icon set section) instead
/// of a plain-text/unicode glyph — `icon` is one of `crate::icons`' `_STR` constants, rendered
/// through the icon font via a [`components::icon_label_job`] `LayoutJob` since `RichText` can't
/// mix two fonts in one string.
fn tool_button_icon_font(
    app: &mut App,
    ui: &mut egui::Ui,
    tool: EditorTool,
    icon: &str,
    label: &str,
) {
    let active = app.tool == tool;
    let color = if active {
        theme::ACCENT
    } else {
        theme::TEXT_SECONDARY
    };
    let job = components::icon_label_job(
        icon,
        crate::icons::family(),
        14.0,
        color,
        label,
        14.0,
        color,
    );
    let button = egui::Button::new(job)
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
        app.tool = tool;
    }
}
