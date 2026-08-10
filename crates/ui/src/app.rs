//! Application state ([`OcaApp`]) and the top-level `eframe::App` implementation that
//! drives one frame: pump the background export queue, draw the nav rail and breadcrumb,
//! then delegate to whichever [`Screen`] is currently active (see [`crate::screens`]).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
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

/// The editor toolbar's active tool (Selecionar / Aparar). Currently just tracked for the
/// toolbar's highlight state — Fase 3 wires it up to actual timeline interactions. Splitting
/// ("Cortar") isn't a persistent mode like these two — it's a one-shot action, performed
/// directly by [`OcaApp::split_at_playhead`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTool {
    Select,
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

/// A message from a background import worker thread (see [`OcaApp::spawn_import`]) back to
/// the UI thread. `project_id` (rather than an index into `projects`) is what the result gets
/// applied to, since the active project can change while a slow import is still running.
enum ImportEvent {
    AssetReady { project_id: u64, asset: MediaAsset },
    Failed { path: PathBuf, message: String },
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
    import_tx: UnboundedSender<ImportEvent>,
    import_rx: UnboundedReceiver<ImportEvent>,
    /// How many files a call to [`OcaApp::spawn_import`] is still probing/measuring/
    /// generating a proxy for, in the background. The Mídia screen shows a busy note while
    /// this is nonzero so a large import (which used to freeze the whole app) reads as "still
    /// working" instead of "did nothing".
    pub pending_imports: usize,
    /// The timeline clip currently highlighted in the Editor's timeline strip, if any — a
    /// separate concept from `selected_asset_id` (that's the media-library selection driving
    /// the preview panel; this is a placed [`avcore::timeline::ClipInstance`]). `Delete`
    /// removes whichever clip this points at.
    pub selected_clip_id: Option<u64>,
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
        let (import_tx, import_rx) = mpsc::unbounded_channel();
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
            import_tx,
            import_rx,
            pending_imports: 0,
            selected_clip_id: None,
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

    /// Splits whichever clip covers the timeline playhead, on every track that has one there,
    /// into two — what `Ctrl+B` and the toolbar's "Cortar / Split" button do. Cutting every
    /// track at once (rather than just a clicked clip) keeps V1/A1/A2 in sync, which is the
    /// point of a gameplay edit. A no-op on any track where nothing covers the playhead.
    pub fn split_at_playhead(&mut self) {
        let at_secs = self.active_project().timeline.playhead_secs;
        let mut next_clip_id = self
            .active_project()
            .timeline
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .map(|c| c.id)
            .max()
            .unwrap_or(0)
            + 1;

        for track in &mut self.active_project_mut().timeline.tracks {
            if track.split_clip_at(at_secs, next_clip_id) {
                next_clip_id += 1;
            }
        }
    }

    /// Removes `selected_clip_id` from whichever track has it and clears the selection — what
    /// pressing `Delete` on the timeline does. Leaves a gap rather than rippling later clips
    /// left, matching `split_at_playhead`'s equally simple non-ripple editing model. A no-op
    /// if nothing is selected.
    pub fn delete_selected_clip(&mut self) {
        let Some(clip_id) = self.selected_clip_id else {
            return;
        };
        for track in &mut self.active_project_mut().timeline.tracks {
            track.clips.retain(|c| c.id != clip_id);
        }
        self.selected_clip_id = None;
    }

    /// Minimum clip duration a drag-trim is allowed to shrink a clip to — small enough to feel
    /// unrestrictive, large enough that a clip can't accidentally get dragged down to
    /// (near-)zero length.
    const MIN_TRIM_DURATION_SECS: f64 = 0.1;

    /// Drags `clip_id`'s left edge to `new_start_secs` — what dragging the left handle on a
    /// timeline clip does. A no-op if the clip isn't found or the drag would violate
    /// [`avcore::timeline::ClipInstance::trim_start`]'s bounds (start/source-in going
    /// negative, or shrinking past [`OcaApp::MIN_TRIM_DURATION_SECS`]).
    pub fn trim_clip_start(&mut self, clip_id: u64, new_start_secs: f64) {
        for track in &mut self.active_project_mut().timeline.tracks {
            if let Some(clip) = track.clip_mut(clip_id) {
                clip.trim_start(new_start_secs.max(0.0), Self::MIN_TRIM_DURATION_SECS);
                return;
            }
        }
    }

    /// Drags `clip_id`'s right edge to `new_end_secs` — what dragging the right handle on a
    /// timeline clip does. Bounded above by the clip's source asset's own duration (looked up
    /// via the clip's `asset_id`), so a trim can't ask the source media for footage past its
    /// actual end; skipped if the asset can't be found (best effort rather than blocking the
    /// drag entirely).
    pub fn trim_clip_end(&mut self, clip_id: u64, new_end_secs: f64) {
        let asset_id = self
            .active_project()
            .timeline
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .find(|c| c.id == clip_id)
            .map(|c| c.asset_id);
        let Some(asset_id) = asset_id else {
            return;
        };
        let max_source_out_secs = self
            .active_project()
            .media_library
            .iter()
            .find(|a| a.id == asset_id)
            .map(|a| a.duration_secs);

        for track in &mut self.active_project_mut().timeline.tracks {
            if let Some(clip) = track.clip_mut(clip_id) {
                clip.trim_end(
                    new_end_secs,
                    Self::MIN_TRIM_DURATION_SECS,
                    max_source_out_secs,
                );
                return;
            }
        }
    }

    /// Probes, measures loudness, and (for video) generates an editing proxy for each of
    /// `paths` on a background thread — what "Importar arquivos" does. These are synchronous
    /// FFI calls that can take minutes for a large source file (a multi-GB capture), and used
    /// to run directly on the UI thread, freezing the whole app for that long. Results are
    /// applied to the target project's media library as they arrive
    /// ([`OcaApp::pump_import_queue`]), keyed by project id rather than the active project
    /// index, which could change before a slow import finishes.
    pub fn spawn_import(&mut self, paths: Vec<PathBuf>) {
        let project_id = self.active_project().id;
        let proxy_dir = avcore::proxy::cache_dir_for_project(self.active_project());
        let tx = self.import_tx.clone();
        self.pending_imports += paths.len();

        std::thread::spawn(move || {
            for path in paths {
                let _ = tx.send(import_one(&path, project_id, &proxy_dir));
            }
        });
    }

    /// Applies finished imports to their target project's media library. Called once per
    /// frame from [`eframe::App::ui`], same as [`OcaApp::pump_export_queue`].
    fn pump_import_queue(&mut self) {
        while let Ok(event) = self.import_rx.try_recv() {
            self.pending_imports = self.pending_imports.saturating_sub(1);
            match event {
                ImportEvent::AssetReady {
                    project_id,
                    mut asset,
                } => {
                    let Some(project) = self.projects.iter_mut().find(|p| p.id == project_id)
                    else {
                        continue;
                    };
                    let next_id = project
                        .media_library
                        .iter()
                        .map(|a| a.id)
                        .max()
                        .unwrap_or(0)
                        + 1;
                    asset.id = next_id;
                    project.media_library.push(asset);
                }
                ImportEvent::Failed { path, message } => {
                    eprintln!("failed to import {}: {message}", path.display());
                }
            }
        }
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

/// Runs on [`OcaApp::spawn_import`]'s background thread — probes `path`, measures its
/// loudness, and (for video) generates an editing proxy, returning whichever
/// [`ImportEvent`] the result maps to. A file that fails to probe is reported as
/// [`ImportEvent::Failed`] rather than aborting the rest of the batch; a proxy or loudness
/// measurement that fails just leaves that one field unset — the asset is still fully usable,
/// just not as light to scrub or already loudness-tagged.
fn import_one(path: &Path, project_id: u64, proxy_dir: &Path) -> ImportEvent {
    let probed = match avcore::probe_media(path) {
        Ok(probed) => probed,
        Err(e) => {
            return ImportEvent::Failed {
                path: path.to_path_buf(),
                message: e.to_string(),
            };
        }
    };

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut asset = probed.into_media_asset(0, file_name, path.to_path_buf());

    match avcore::measure_loudness(path) {
        Ok(metrics) => asset.loudness = Some(metrics),
        Err(e) => eprintln!("failed to measure loudness for {}: {e}", path.display()),
    }
    if asset.kind == avcore::MediaKind::Video {
        match avcore::ensure_proxy(path, proxy_dir) {
            Ok(proxy_path) => asset.proxy_path = Some(proxy_path),
            Err(e) => eprintln!("failed to generate proxy for {}: {e}", path.display()),
        }
    }

    ImportEvent::AssetReady { project_id, asset }
}

impl eframe::App for OcaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump_export_queue();
        self.pump_import_queue();
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
