//! Application state ([`NivelaApp`]) and the top-level `eframe::App` implementation that
//! drives one frame: tick the mock export-progress timer, draw the nav rail and breadcrumb,
//! then delegate to whichever [`Screen`] is currently active (see [`crate::screens`]).

use std::time::{Duration, Instant};

use eframe::egui;
use nivela_core::{sample, ExportJob, ExportJobStatus, MediaAsset, Project};

use crate::i18n::Locale;
use crate::screens;
use crate::theme;

/// Which of the app's five top-level views is currently showing. Drives both the central
/// panel content and which nav-rail button is highlighted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Editor,
    Library,
    Queue,
    Prefs,
}

/// The editor toolbar's active tool (Selecionar / Cortar / Aparar). Currently just tracked
/// for the toolbar's highlight state — Fase 3 wires it up to actual timeline interactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTool {
    Select,
    Cut,
    Trim,
}

/// User-configurable settings shown on the Ajustes screen. Not persisted yet (Fase 5 adds a
/// preferences file) — resets to [`PrefsState::default`] every launch.
pub struct PrefsState {
    /// Index into [`LUFS_PROFILES`].
    pub lufs_profile: usize,
    pub true_peak_limiter: bool,
    pub export_workers: u8,
    pub output_folder: String,
    pub autosave_minutes: u8,
}

impl Default for PrefsState {
    fn default() -> Self {
        Self {
            lufs_profile: 0,
            true_peak_limiter: true,
            export_workers: 1,
            output_folder: r"C:\Videos\PacoPaçoca\Export".to_string(),
            autosave_minutes: 5,
        }
    }
}

/// Loudness normalization presets offered in Preferences and shown on the Editor's "Ao
/// exportar" panel. `(label, target LUFS)` — the label is a loanword-heavy string
/// (`"YouTube"`/`"Podcast"`/`"Broadcast"`) that reads the same in both locales, so unlike
/// most UI text it isn't routed through `i18n::Text`.
pub const LUFS_PROFILES: [(&str, f32); 3] = [
    ("-14 LUFS · YouTube", -14.0),
    ("-16 LUFS · Podcast", -16.0),
    ("-23 LUFS · Broadcast", -23.0),
];

/// The whole application's state: which screen is showing, the loaded projects, the export
/// queue, and user preferences. `eframe` owns one instance of this for the app's lifetime
/// and calls [`NivelaApp::ui`](eframe::App::ui) on it every frame.
pub struct NivelaApp {
    pub screen: Screen,
    pub tool: EditorTool,
    pub locale: Locale,
    pub projects: Vec<Project>,
    pub active_project: usize,
    pub selected_asset_id: Option<u64>,
    pub export_jobs: Vec<ExportJob>,
    pub queue_workers: u8,
    pub prefs: PrefsState,
    last_tick: Instant,
}

impl NivelaApp {
    /// Builds the initial app state: applies the theme, loads the mock projects/export
    /// queue (see [`nivela_core::sample`]), and selects the first project's first asset.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let projects = sample::sample_projects();
        let selected_asset_id = projects
            .first()
            .and_then(|p| p.media_library.first())
            .map(|a| a.id);
        Self {
            screen: Screen::Home,
            tool: EditorTool::Select,
            locale: Locale::PtBr,
            projects,
            active_project: 0,
            selected_asset_id,
            export_jobs: sample::sample_export_jobs(),
            queue_workers: 1,
            prefs: PrefsState::default(),
            last_tick: Instant::now(),
        }
    }

    /// The project currently open in the Editor/Mídia screens.
    pub fn active_project(&self) -> &Project {
        &self.projects[self.active_project]
    }

    /// Mutable access to the project currently open in the Editor/Mídia screens — for
    /// imports, edits, and anything else that changes the active project in place.
    pub fn active_project_mut(&mut self) -> &mut Project {
        &mut self.projects[self.active_project]
    }

    /// The asset backing the Editor's "Clipe selecionado" panel, if any is selected.
    pub fn selected_asset(&self) -> Option<&MediaAsset> {
        let id = self.selected_asset_id?;
        self.active_project()
            .media_library
            .iter()
            .find(|a| a.id == id)
    }

    /// Switches the active project to `index` and navigates to the Editor screen — this is
    /// what a project card click on the Início screen does.
    pub fn open_project(&mut self, index: usize) {
        self.active_project = index;
        self.selected_asset_id = self.active_project().media_library.first().map(|a| a.id);
        self.screen = Screen::Editor;
    }

    /// Appends `project` to the project list and opens it — used for both "Novo projeto"
    /// (an empty project) and "Abrir projeto" (one just loaded from disk).
    pub fn add_and_open_project(&mut self, project: Project) {
        self.projects.push(project);
        self.open_project(self.projects.len() - 1);
    }

    /// Builds an empty project with a fresh id and opens it — what "Novo projeto" does.
    pub fn create_new_project(&mut self, name: String) {
        let id = self.projects.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        self.add_and_open_project(Project {
            id,
            name,
            last_edited: nivela_core::Recency::HoursAgo(0),
            summary: String::new(),
            media_library: Vec::new(),
            timeline: nivela_core::Timeline {
                tracks: Vec::new(),
                playhead_secs: 0.0,
            },
            file_path: None,
        });
    }

    /// Mirrors the mockup's `setInterval` progress bump on the rendering job, purely for
    /// visual demo purposes until Fase 4 wires up the real background render worker.
    fn tick_mock_progress(&mut self) {
        if self.last_tick.elapsed() < Duration::from_millis(900) {
            return;
        }
        self.last_tick = Instant::now();
        for job in &mut self.export_jobs {
            if let ExportJobStatus::Rendering { percent } = &mut job.status {
                if *percent < 95 {
                    *percent += 1;
                }
            }
        }
    }
}

impl eframe::App for NivelaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.tick_mock_progress();
        ui.ctx().request_repaint_after(Duration::from_millis(200));

        screens::nav_rail::show(self, ui);
        screens::breadcrumb::show(self, ui);

        egui::CentralPanel::default().show(ui, |ui| match self.screen {
            Screen::Home => screens::home::show(self, ui),
            Screen::Editor => screens::editor::show(self, ui),
            Screen::Library => screens::library::show(self, ui),
            Screen::Queue => screens::queue::show(self, ui),
            Screen::Prefs => screens::prefs::show(self, ui),
        });
    }
}
