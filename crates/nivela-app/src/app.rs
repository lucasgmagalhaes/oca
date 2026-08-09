//! Application state ([`NivelaApp`]) and the top-level `eframe::App` implementation that
//! drives one frame: pump the background export queue, draw the nav rail and breadcrumb,
//! then delegate to whichever [`Screen`] is currently active (see [`crate::screens`]).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use eframe::egui;
use nivela_core::{sample, ExportJob, ExportJobStatus, MediaAsset, Project, RenderOutcome};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

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

/// A message from a background render worker thread (see [`NivelaApp::pump_export_queue`])
/// back to the UI thread, sent over a plain `tokio::sync::mpsc` channel used purely
/// synchronously (`try_recv` on the UI side, `send` on the worker side) — no async runtime
/// needed, matching the execution plan's "tokio + canais assíncronos" without pulling egui's
/// synchronous frame loop into async code.
enum RenderEvent {
    Progress { job_id: u64, percent: u8 },
    Done { job_id: u64 },
    Failed { job_id: u64, message: String },
    Cancelled { job_id: u64 },
}

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
    render_tx: UnboundedSender<RenderEvent>,
    render_rx: UnboundedReceiver<RenderEvent>,
    /// Cancellation flags for jobs a worker thread is currently rendering, keyed by job id.
    /// A job id present here is the source of truth for "how many workers are busy right
    /// now" — [`NivelaApp::pump_export_queue`] uses its length against `queue_workers`.
    active_renders: HashMap<u64, Arc<AtomicBool>>,
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
        let (render_tx, render_rx) = mpsc::unbounded_channel();
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
            render_tx,
            render_rx,
            active_renders: HashMap::new(),
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

    /// Appends a new `Queued` job — what "Adicionar exportação" does. Picked up by
    /// [`NivelaApp::pump_export_queue`] once a worker slot ([`NivelaApp::queue_workers`])
    /// frees up.
    pub fn queue_export(
        &mut self,
        title: String,
        source_path: PathBuf,
        target_lufs: f32,
        bitrate_mbps: f32,
        output_path: String,
    ) {
        let id = self.export_jobs.iter().map(|j| j.id).max().unwrap_or(0) + 1;
        self.export_jobs.push(ExportJob {
            id,
            title,
            source_path,
            target_lufs,
            bitrate_mbps,
            output_path,
            status: ExportJobStatus::Queued,
        });
    }

    /// Stops a job: kills its `ffmpeg` process if it's actively rendering, or just removes it
    /// from the list if it hadn't started yet. There's no such thing as cancelling a `Done`/
    /// `Failed` job — callers only wire this to the buttons where it's meaningful.
    pub fn cancel_export_job(&mut self, job_id: u64) {
        match self.active_renders.get(&job_id) {
            Some(cancel_flag) => cancel_flag.store(true, Ordering::Relaxed),
            None => self.export_jobs.retain(|j| j.id != job_id),
        }
    }

    /// Applies events from render worker threads to `export_jobs`, then — if there's a free
    /// worker slot under `queue_workers` — dispatches the next `Queued` job to a new
    /// background thread. Called once per frame; this is the entire "background export
    /// queue" from the execution plan's Fase 4.
    fn pump_export_queue(&mut self) {
        while let Ok(event) = self.render_rx.try_recv() {
            match event {
                RenderEvent::Progress { job_id, percent } => {
                    if let Some(job) = self.export_jobs.iter_mut().find(|j| j.id == job_id) {
                        if let ExportJobStatus::Rendering { percent: p } = &mut job.status {
                            *p = percent;
                        }
                    }
                }
                RenderEvent::Done { job_id } => {
                    if let Some(job) = self.export_jobs.iter_mut().find(|j| j.id == job_id) {
                        job.status = ExportJobStatus::Done;
                    }
                    self.active_renders.remove(&job_id);
                }
                RenderEvent::Failed { job_id, message } => {
                    if let Some(job) = self.export_jobs.iter_mut().find(|j| j.id == job_id) {
                        job.status = ExportJobStatus::Failed { message };
                    }
                    self.active_renders.remove(&job_id);
                }
                RenderEvent::Cancelled { job_id } => {
                    self.export_jobs.retain(|j| j.id != job_id);
                    self.active_renders.remove(&job_id);
                }
            }
        }

        if self.active_renders.len() >= self.queue_workers as usize {
            return;
        }
        let Some(job) = self
            .export_jobs
            .iter_mut()
            .find(|j| j.status == ExportJobStatus::Queued)
        else {
            return;
        };

        let job_id = job.id;
        let source_path = job.source_path.clone();
        let output_path = PathBuf::from(&job.output_path);
        let target_lufs = job.target_lufs;
        job.status = ExportJobStatus::Rendering { percent: 0 };

        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.active_renders.insert(job_id, Arc::clone(&cancel_flag));

        let tx = self.render_tx.clone();
        std::thread::spawn(move || {
            let duration_secs = nivela_core::probe_media(&source_path)
                .map(|probed| probed.duration_secs)
                .unwrap_or(0.0);

            let outcome = nivela_core::render_export(
                &source_path,
                &output_path,
                target_lufs,
                duration_secs,
                &cancel_flag,
                |percent| {
                    let _ = tx.send(RenderEvent::Progress { job_id, percent });
                },
            );

            let event = match outcome {
                Ok(RenderOutcome::Completed) => RenderEvent::Done { job_id },
                Ok(RenderOutcome::Cancelled) => RenderEvent::Cancelled { job_id },
                Err(e) => RenderEvent::Failed {
                    job_id,
                    message: e.to_string(),
                },
            };
            let _ = tx.send(event);
        });
    }
}

impl eframe::App for NivelaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump_export_queue();
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
