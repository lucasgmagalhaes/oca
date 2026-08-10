//! Application state ([`OcaApp`]) and the top-level `eframe::App` implementation that
//! drives one frame: pump the background export queue, draw the nav rail and breadcrumb,
//! then delegate to whichever [`Screen`] is currently active (see [`crate::screens`]).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use avcore::{sample, ExportJob, ExportJobStatus, MediaAsset, Project, RenderOutcome};
use eframe::egui;
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

/// A message from a background render worker thread (see [`OcaApp::pump_export_queue`])
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
/// and calls [`OcaApp::ui`](eframe::App::ui) on it every frame.
pub struct OcaApp {
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
    /// now" — [`OcaApp::pump_export_queue`] uses its length against `queue_workers`.
    active_renders: HashMap<u64, Arc<AtomicBool>>,
    /// The GStreamer pipeline for `selected_asset_id`, if it could be opened (`None` both
    /// before any selection and when `Preview::open` failed, e.g. the sample-data assets'
    /// placeholder paths — see [`OcaApp::select_asset`]).
    preview: Option<avcore::preview::Preview>,
    /// Uploaded from the latest [`avcore::preview::Preview::current_frame`] each frame the
    /// Editor screen is shown; `None` until the first frame decodes. Reset on every
    /// [`OcaApp::select_asset`] call so a stale frame from the previous clip never lingers.
    pub preview_texture: Option<egui::TextureHandle>,
    /// Whether the preview pipeline is in `Playing` state. `Preview` has no state getter of
    /// its own, so the Editor's play/pause button and [`OcaApp::pump_export_queue`]'s repaint
    /// cadence both rely on this instead.
    pub preview_playing: bool,
}

impl OcaApp {
    /// Builds the initial app state: applies the theme, loads the mock projects/export
    /// queue (see [`avcore::sample`]), and selects the first project's first asset.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let projects = sample::sample_projects();
        let selected_asset_id = projects
            .first()
            .and_then(|p| p.media_library.first())
            .map(|a| a.id);
        let (render_tx, render_rx) = mpsc::unbounded_channel();
        let mut app = Self {
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
            preview: None,
            preview_texture: None,
            preview_playing: false,
        };
        app.reload_preview();
        app
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
        let asset_id = self.active_project().media_library.first().map(|a| a.id);
        self.select_asset(asset_id);
        self.screen = Screen::Editor;
    }

    /// Selects `id` as the Editor's active clip and (re)opens the preview pipeline for it —
    /// what clicking an asset in the media library panel does, and what [`OcaApp::open_project`]
    /// uses to select the newly-opened project's first asset. `None` clears the selection
    /// (empty media library).
    pub fn select_asset(&mut self, id: Option<u64>) {
        self.selected_asset_id = id;
        self.reload_preview();
    }

    /// Tears down the current preview pipeline (if any) and, if `selected_asset_id` points at
    /// an asset, opens a new one for it — from the editing proxy if one exists (lighter to
    /// decode), otherwise the original source file. Left as `None` without an error dialog if
    /// `Preview::open` fails (e.g. the sample-data projects' placeholder paths, which don't
    /// exist on disk) — the Editor screen shows a muted "preview unavailable" label instead.
    fn reload_preview(&mut self) {
        self.preview = None;
        self.preview_texture = None;
        self.preview_playing = false;

        let Some(asset) = self.selected_asset() else {
            return;
        };
        let path = asset
            .proxy_path
            .clone()
            .unwrap_or_else(|| asset.source_path.clone());
        // Sample-data projects (see avcore::sample) point at placeholder paths that don't
        // exist on disk. Checking first avoids spinning up a whole GStreamer pipeline just to
        // watch it fail to open a file that was never there.
        if !path.exists() {
            return;
        }

        match avcore::preview::Preview::open(&path) {
            Ok(preview) => self.preview = Some(preview),
            Err(e) => eprintln!("failed to open preview for {}: {e}", path.display()),
        }
    }

    /// Toggles play/pause on the current preview pipeline. A no-op if nothing is selected or
    /// the pipeline failed to open.
    pub fn toggle_preview_playback(&mut self) {
        let Some(preview) = &self.preview else {
            return;
        };
        let result = if self.preview_playing {
            preview.pause()
        } else {
            preview.play()
        };
        match result {
            Ok(()) => self.preview_playing = !self.preview_playing,
            Err(e) => eprintln!("failed to toggle preview playback: {e}"),
        }
    }

    /// Seeks the current preview pipeline to `position_secs`. A no-op if nothing is selected
    /// or the pipeline failed to open.
    pub fn seek_preview(&mut self, position_secs: f64) {
        let Some(preview) = &self.preview else {
            return;
        };
        if let Err(e) = preview.seek(position_secs) {
            eprintln!("failed to seek preview: {e}");
        }
    }

    /// Whether the currently selected asset has a live preview pipeline — `false` both before
    /// any selection and when [`OcaApp::reload_preview`] couldn't open one.
    pub fn preview_available(&self) -> bool {
        self.preview.is_some()
    }

    pub fn preview_duration_secs(&self) -> Option<f64> {
        self.preview.as_ref().and_then(|p| p.duration_secs())
    }

    pub fn preview_position_secs(&self) -> Option<f64> {
        self.preview.as_ref().and_then(|p| p.position_secs())
    }

    /// Pulls the latest decoded video frame (if any) into `preview_texture`, and — while
    /// playing — mirrors the pipeline's position into the active project's timeline playhead
    /// so the Editor's timecode label stays in sync. Called once per frame from
    /// [`eframe::App::ui`], before the screens draw.
    fn pump_preview_frame(&mut self, ctx: &egui::Context) {
        let Some(preview) = &self.preview else {
            return;
        };

        if let Some(frame) = preview.current_frame() {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [frame.width as usize, frame.height as usize],
                &frame.rgba,
            );
            match &mut self.preview_texture {
                Some(texture) => texture.set(image, egui::TextureOptions::LINEAR),
                None => {
                    self.preview_texture =
                        Some(ctx.load_texture("preview", image, egui::TextureOptions::LINEAR));
                }
            }
        }

        if self.preview_playing {
            if let Some(position) = preview.position_secs() {
                self.active_project_mut().timeline.playhead_secs = position;
            }
        }
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
            last_edited: avcore::Recency::HoursAgo(0),
            summary: String::new(),
            media_library: Vec::new(),
            timeline: avcore::Timeline {
                tracks: Vec::new(),
                playhead_secs: 0.0,
            },
            file_path: None,
        });
    }

    /// Appends a new `Queued` job — what "Adicionar exportação" does. Picked up by
    /// [`OcaApp::pump_export_queue`] once a worker slot ([`OcaApp::queue_workers`])
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
            let duration_secs = avcore::probe_media(&source_path)
                .map(|probed| probed.duration_secs)
                .unwrap_or(0.0);

            let outcome = avcore::render_export(
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

impl eframe::App for OcaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump_export_queue();
        self.pump_preview_frame(ui.ctx());
        if self.preview_playing {
            // Smooth video needs every-frame repaints; the 200ms throttle below would show
            // it as a slideshow.
            ui.ctx().request_repaint();
        } else {
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }

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

#[cfg(test)]
#[path = "app/app_test.rs"]
mod tests;
