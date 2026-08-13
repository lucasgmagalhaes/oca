//! One module per screen/chrome piece, each exposing a `show(app, ui)` function that renders
//! it into the current frame. [`crate::app::App::ui`] calls `nav_rail` and `breadcrumb`
//! unconditionally every frame, then dispatches to exactly one of the five screen modules
//! based on `app.screen`.

/// Top breadcrumb bar (app name › screen title › project name when in the Editor).
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
