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

//! One module per screen/chrome piece, each exposing a `show(app, ui)` function that renders
//! it into the current frame. [`crate::app::App::ui`] calls `nav_rail` and `breadcrumb`
//! unconditionally every frame, then dispatches to exactly one of the five screen modules
//! based on `app.screen`.

/// oca's custom title bar — replaces the OS window chrome (`main.rs` disables it): app name ›
/// screen title › project name when in the Editor, drag-to-move, and minimize/maximize/close
/// buttons. Also exposes the window's custom edge-resize borders.
pub mod breadcrumb;
/// The Editor screen: toolbar, media library sidebar, preview, clip properties, timeline.
pub mod editor;
/// The Início screen: the grid of recent-project cards.
pub mod home;
/// The Mídia screen: a grid of every asset in the active project's media library.
pub mod library;
/// The left icon rail used to switch between screens.
pub mod nav_rail;
/// The Ajustes screen: language, audio, export, project and shortcut settings.
pub mod prefs;
/// The Fila screen: the export queue's job list and worker-count control.
pub mod queue;
/// The Música/SFX screen: a local catalog of music/SFX tracks, scanned from a user-configured
/// folder, addable to the timeline directly.
pub mod sound_library;
