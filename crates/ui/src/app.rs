//! Application state ([`OcaApp`]) and the top-level `eframe::App` implementation that
//! drives one frame: pump the background export queue, draw the nav rail and breadcrumb,
//! then delegate to whichever [`Screen`] is currently active (see [`crate::screens`]).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    /// Sent as soon as `path`'s cheap probe comes back — the asset is usable immediately
    /// (correct duration/resolution/kind), just without loudness or a proxy yet.
    AssetReady {
        project_id: u64,
        import_token: u64,
        asset: MediaAsset,
    },
    /// Sent after the same file's loudness measurement and (for video) proxy generation
    /// finish, however much later — both fields are `None` if that particular step failed,
    /// same as before this was split from `AssetReady`.
    Enriched {
        project_id: u64,
        import_token: u64,
        loudness: Option<avcore::LoudnessMetrics>,
        proxy_path: Option<PathBuf>,
        waveform_peaks: Option<Vec<(f32, f32)>>,
    },
    Failed {
        path: PathBuf,
        message: String,
    },
}

/// A poster frame extracted on a background thread (see [`OcaApp::request_thumbnail`]),
/// ready to upload as an egui texture. No failure variant — a (asset, bucket) that couldn't be
/// extracted just never gets a texture; [`OcaApp::requested_thumbnails`] already stops it from
/// being retried every frame.
struct ThumbnailReady {
    asset_id: u64,
    bucket: i64,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
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
    /// The GStreamer pipeline for `selected_asset_id`, if it could be opened (`None` before any
    /// selection, before it's been lazily opened, and when `Preview::open` failed, e.g. the
    /// sample-data assets' placeholder paths — see [`OcaApp::ensure_preview_loaded`]).
    preview: Option<avcore::preview::Preview>,
    /// The asset id [`OcaApp::ensure_preview_loaded`] last attempted to open a pipeline for,
    /// whether or not it succeeded — lets it tell "already tried and failed for this exact
    /// selection, don't retry every frame" apart from "selection changed, try again".
    /// `select_asset` clears this back to `None` so a fresh selection always gets one attempt.
    preview_attempted_for: Option<u64>,
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
    /// How many files a call to [`OcaApp::spawn_import`] haven't been probed yet, in the
    /// background. The Mídia screen shows a busy note while this is nonzero. Reaches zero as
    /// soon as each file's cheap probe comes back and it's added to the library — loudness
    /// measurement and proxy generation keep running after that in the background (see
    /// [`OcaApp::pending_enrichment`]) without holding this counter up, since the asset is
    /// already usable by then.
    pub pending_imports: usize,
    /// The next id to hand out in [`OcaApp::spawn_import`], one per file in the batch —
    /// correlates a file's `ImportEvent::AssetReady` with its later `ImportEvent::Enriched`
    /// once [`OcaApp::pump_import_queue`] knows the asset's real (project-assigned) id.
    next_import_token: u64,
    /// Import tokens awaiting their `ImportEvent::Enriched` (loudness + proxy), mapped to the
    /// asset id they were assigned when their `AssetReady` landed — removed once the
    /// enrichment arrives and gets applied, or left dangling harmlessly if the asset is gone
    /// by then (media library has no delete yet, so that can't currently happen).
    pending_enrichment: HashMap<u64, u64>,
    /// The timeline clip currently highlighted in the Editor's timeline strip, if any — a
    /// separate concept from `selected_asset_id` (that's the media-library selection driving
    /// the preview panel; this is a placed [`avcore::timeline::ClipInstance`]). `Delete`
    /// removes whichever clip this points at.
    pub selected_clip_id: Option<u64>,
    /// Horizontal scale of the timeline strip and its ruler, in pixels per second. Adjusted by
    /// `Ctrl` + scroll over the timeline (per `request.md`'s Fase 3 spec) — more zoom for
    /// frame-accurate edits, less to see the whole project at once.
    pub timeline_px_per_sec: f32,
    /// Width, in points, of the Editor's media-library column — dragged via the divider
    /// between it and the preview column (`editor.rs::resizable_divider`). Clamped to the
    /// window's current size every frame (`editor.rs::show`), not persisted across restarts —
    /// a known simplification short of `request.md`'s "layout salvo por projeto ou por
    /// usuário" (per-project/per-user persistence isn't wired up yet).
    pub lib_panel_width: f32,
    /// Same idea as `lib_panel_width`, for the clip-properties column on the right.
    pub props_panel_width: f32,
    /// Height, in points, of the timeline strip — dragged via the horizontal divider above it.
    /// Same persistence caveat as `lib_panel_width`.
    pub timeline_height: f32,
    thumbnail_tx: UnboundedSender<ThumbnailReady>,
    thumbnail_rx: UnboundedReceiver<ThumbnailReady>,
    /// Filmstrip tile textures for video clips on the timeline, keyed by `(asset_id, bucket)`
    /// where `bucket` is a source-time offset quantized to [`THUMBNAIL_BUCKET_SECS`] (see
    /// `editor.rs::draw_filmstrip`) — shared across every clip on that asset, and stable across
    /// zoom changes rather than tied to a particular on-screen tile. A known simplification:
    /// the bucket size is fixed, not adapted to the current zoom, so a very zoomed-in filmstrip
    /// can repeat the same tile a few times in a row instead of showing a unique frame each.
    pub thumbnail_textures: HashMap<(u64, i64), egui::TextureHandle>,
    /// `(asset_id, bucket)` pairs a thumbnail has already been requested for, successfully or
    /// not — stops [`OcaApp::request_thumbnail`] from spawning a new extraction thread every
    /// frame for a bucket that's still pending, or that already failed once.
    requested_thumbnails: HashSet<(u64, i64)>,
    /// Set by the media library panel on the frame a dragged asset is released (screen-space
    /// pointer position), consumed by the timeline panel later in the same frame to place it —
    /// how dragging an asset out of the library and dropping it on the timeline works. Always
    /// `None` between frames.
    pub pending_asset_drop: Option<(u64, egui::Pos2)>,
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
        let (thumbnail_tx, thumbnail_rx) = mpsc::unbounded_channel();
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
            preview: None,
            preview_attempted_for: None,
            preview_texture: None,
            preview_playing: false,
            import_tx,
            import_rx,
            pending_imports: 0,
            next_import_token: 0,
            pending_enrichment: HashMap::new(),
            selected_clip_id: None,
            timeline_px_per_sec: 4.0,
            lib_panel_width: 220.0,
            props_panel_width: 240.0,
            timeline_height: 190.0,
            thumbnail_tx,
            thumbnail_rx,
            thumbnail_textures: HashMap::new(),
            requested_thumbnails: HashSet::new(),
            pending_asset_drop: None,
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
        let asset_id = self.active_project().media_library.first().map(|a| a.id);
        self.select_asset(asset_id);
        self.screen = Screen::Editor;
    }

    /// Selects `id` as the Editor's active clip and clears out whatever pipeline/texture/
    /// playback state belonged to the previous one — what clicking an asset in the media
    /// library panel does, and what [`OcaApp::open_project`] uses to select the newly-opened
    /// project's first asset. `None` clears the selection (empty media library). Does *not*
    /// itself open a pipeline for the new selection — [`OcaApp::ensure_preview_loaded`] does
    /// that lazily, the next time the Editor's preview panel actually draws, so switching
    /// projects or asking for a project at startup never pays GStreamer's open cost for an
    /// asset nobody's looking at yet.
    pub fn select_asset(&mut self, id: Option<u64>) {
        self.selected_asset_id = id;
        self.preview = None;
        self.preview_attempted_for = None;
        self.preview_texture = None;
        self.preview_playing = false;
    }

    /// Opens a pipeline for `selected_asset_id` if one isn't already open and hasn't already
    /// been tried for this exact selection — called once per frame from the Editor's preview
    /// panel, right before it reads any preview state, so the first paint after a selection
    /// change is what actually triggers `Preview::open` rather than `select_asset` itself.
    /// Prefers the editing proxy if one exists (lighter to decode), otherwise the original
    /// source file. Leaves `preview` as `None` without an error dialog if `Preview::open` fails
    /// (e.g. the sample-data projects' placeholder paths, which don't exist on disk) — the
    /// Editor screen shows a muted "preview unavailable" label instead, and won't retry until
    /// the selection changes again.
    pub fn ensure_preview_loaded(&mut self) {
        if self.preview.is_some() || self.preview_attempted_for == self.selected_asset_id {
            return;
        }
        self.preview_attempted_for = self.selected_asset_id;

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

    /// Whether the currently selected asset has a live preview pipeline — `false` before any
    /// selection, before [`OcaApp::ensure_preview_loaded`] has run for it, and when it
    /// couldn't open one.
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

    /// Appends `asset_id` to the timeline as a new, untrimmed clip — what double-clicking an
    /// asset in the media library panel does. Lands on the first track whose kind matches the
    /// asset (video onto video, audio onto audio), auto-creating one (`"V1"`/`"A1"`) if none
    /// exists yet, right after whatever's already there
    /// ([`avcore::timeline::Track::duration_secs`] — `0.0` for an empty/new track). A no-op if
    /// `asset_id` isn't in the active project's media library.
    pub fn add_asset_to_timeline(&mut self, asset_id: u64) {
        let Some((kind, duration_secs)) = self.asset_kind_and_duration(asset_id) else {
            return;
        };
        let timeline = &mut self.active_project_mut().timeline;
        let track_index = resolve_or_create_track(timeline, kind, None);
        let start_secs = timeline.tracks[track_index].duration_secs();
        let clip_id = next_clip_id(timeline);
        timeline.tracks[track_index]
            .clips
            .push(avcore::timeline::ClipInstance {
                id: clip_id,
                asset_id,
                start_secs,
                source_in_secs: 0.0,
                source_out_secs: duration_secs,
            });
    }

    /// Inserts `asset_id` onto the timeline at `start_secs` — what dropping an asset dragged
    /// out of the media library onto the timeline strip does. Prefers `preferred_track_id` if
    /// it exists and matches the asset's kind (the track row the drop landed on); otherwise
    /// falls back to the same track-resolution as [`OcaApp::add_asset_to_timeline`] (first
    /// existing track of matching kind, auto-created if none exists). A no-op if `asset_id`
    /// isn't in the active project's media library or `start_secs` is negative.
    pub fn add_asset_to_timeline_at(
        &mut self,
        asset_id: u64,
        preferred_track_id: Option<u64>,
        start_secs: f64,
    ) {
        if start_secs < 0.0 {
            return;
        }
        let Some((kind, duration_secs)) = self.asset_kind_and_duration(asset_id) else {
            return;
        };
        let timeline = &mut self.active_project_mut().timeline;
        let track_index = resolve_or_create_track(timeline, kind, preferred_track_id);
        let clip_id = next_clip_id(timeline);
        timeline.tracks[track_index]
            .clips
            .push(avcore::timeline::ClipInstance {
                id: clip_id,
                asset_id,
                start_secs,
                source_in_secs: 0.0,
                source_out_secs: duration_secs,
            });
    }

    /// Looks up `asset_id` in the active project's media library and returns its track kind
    /// and duration, or `None` if it isn't there.
    fn asset_kind_and_duration(&self, asset_id: u64) -> Option<(avcore::timeline::TrackKind, f64)> {
        let asset = self
            .active_project()
            .media_library
            .iter()
            .find(|a| a.id == asset_id)?;
        let kind = match asset.kind {
            avcore::MediaKind::Video => avcore::timeline::TrackKind::Video,
            avcore::MediaKind::Audio => avcore::timeline::TrackKind::Audio,
        };
        Some((kind, asset.duration_secs))
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

    /// Repositions `clip_id` to `new_start_secs` on its own track — what dragging a timeline
    /// clip's body does when it's dropped back on the same track it started on. A no-op if
    /// the clip isn't found or `new_start_secs` is negative.
    pub fn move_clip(&mut self, clip_id: u64, new_start_secs: f64) {
        for track in &mut self.active_project_mut().timeline.tracks {
            if track.move_clip(clip_id, new_start_secs) {
                return;
            }
        }
    }

    /// Moves `clip_id` onto `target_track_id` at `new_start_secs` — what dragging a timeline
    /// clip's body onto a *different* track does, once the editor has confirmed the drop
    /// target's row is a same-kind track. A no-op if the clip or target track aren't found,
    /// the kinds don't match, or `new_start_secs` is negative — see
    /// [`avcore::timeline::Timeline::move_clip_to_track`] for the exact rules.
    pub fn move_clip_to_track(&mut self, clip_id: u64, target_track_id: u64, new_start_secs: f64) {
        self.active_project_mut().timeline.move_clip_to_track(
            clip_id,
            target_track_id,
            new_start_secs,
        );
    }

    /// Probes each of `paths` on its own background thread — what "Importar arquivos" does.
    /// Each file becomes usable in the media library as soon as its probe (cheap: container/
    /// stream metadata only, no decode) comes back, the same way other NLEs show an imported
    /// file instantly and refine it afterward; loudness measurement and (for video) proxy
    /// generation, both full decode passes that can take minutes on a multi-GB capture, keep
    /// running in the background past that point and update the asset in place once they
    /// finish (see [`ImportEvent`]/[`OcaApp::pump_import_queue`]). One thread per file rather
    /// than one thread for the whole batch, so a multi-file import isn't serialized behind
    /// its slowest file either. Applied to the target project by id rather than the active
    /// project index, which could change before a slow import finishes.
    pub fn spawn_import(&mut self, paths: Vec<PathBuf>) {
        let project_id = self.active_project().id;
        let proxy_dir = avcore::proxy::cache_dir_for_project(self.active_project());
        self.pending_imports += paths.len();

        for path in paths {
            let import_token = self.next_import_token;
            self.next_import_token += 1;
            let tx = self.import_tx.clone();
            let proxy_dir = proxy_dir.clone();
            std::thread::spawn(move || {
                import_one(&path, project_id, import_token, &proxy_dir, &tx);
            });
        }
    }

    /// Applies import progress to its target project's media library as it arrives. Called
    /// once per frame from [`eframe::App::ui`], same as [`OcaApp::pump_export_queue`].
    fn pump_import_queue(&mut self) {
        while let Ok(event) = self.import_rx.try_recv() {
            match event {
                ImportEvent::AssetReady {
                    project_id,
                    import_token,
                    mut asset,
                } => {
                    self.pending_imports = self.pending_imports.saturating_sub(1);
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
                    self.pending_enrichment.insert(import_token, next_id);
                }
                ImportEvent::Enriched {
                    project_id,
                    import_token,
                    loudness,
                    proxy_path,
                    waveform_peaks,
                } => {
                    let Some(asset_id) = self.pending_enrichment.remove(&import_token) else {
                        continue;
                    };
                    let Some(project) = self.projects.iter_mut().find(|p| p.id == project_id)
                    else {
                        continue;
                    };
                    if let Some(asset) = project.media_library.iter_mut().find(|a| a.id == asset_id)
                    {
                        asset.loudness = loudness;
                        asset.proxy_path = proxy_path;
                        asset.waveform_peaks = waveform_peaks;
                    }
                }
                ImportEvent::Failed { path, message } => {
                    self.pending_imports = self.pending_imports.saturating_sub(1);
                    eprintln!("failed to import {}: {message}", path.display());
                }
            }
        }
    }

    /// Requests a poster-frame thumbnail for a timeline clip — what `timeline_panel` calls for
    /// every visible filmstrip tile that doesn't have a texture cached yet (see
    /// `editor.rs::draw_filmstrip`). A no-op if `(asset_id, bucket)` was already requested
    /// (successfully or not; see [`OcaApp::requested_thumbnails`]). Extraction (open the
    /// asset's proxy-or-source file, seek to the bucket's representative time, grab a frame,
    /// downscale) runs on a background thread — the same reasoning as
    /// [`OcaApp::spawn_import`]: this is FFI/decode work that must not run on the UI thread.
    pub fn request_thumbnail(&mut self, asset_id: u64, bucket: i64) {
        let key = (asset_id, bucket);
        if self.requested_thumbnails.contains(&key) {
            return;
        }
        self.requested_thumbnails.insert(key);

        let Some(asset) = self
            .active_project()
            .media_library
            .iter()
            .find(|a| a.id == asset_id)
        else {
            return;
        };
        let path = asset
            .proxy_path
            .clone()
            .unwrap_or_else(|| asset.source_path.clone());
        let at_secs = bucket as f64 * THUMBNAIL_BUCKET_SECS;

        let tx = self.thumbnail_tx.clone();
        std::thread::spawn(move || {
            if let Some((width, height, rgba)) = extract_thumbnail(&path, at_secs) {
                let _ = tx.send(ThumbnailReady {
                    asset_id,
                    bucket,
                    width,
                    height,
                    rgba,
                });
            }
        });
    }

    /// Uploads finished thumbnail extractions as egui textures. Called once per frame from
    /// [`eframe::App::ui`], same as [`OcaApp::pump_import_queue`].
    fn pump_thumbnail_queue(&mut self, ctx: &egui::Context) {
        while let Ok(ready) = self.thumbnail_rx.try_recv() {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [ready.width as usize, ready.height as usize],
                &ready.rgba,
            );
            let texture = ctx.load_texture(
                format!("thumb-{}-{}", ready.asset_id, ready.bucket),
                image,
                egui::TextureOptions::LINEAR,
            );
            self.thumbnail_textures
                .insert((ready.asset_id, ready.bucket), texture);
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

/// Finds the track to place a new clip of `kind` on, for [`OcaApp::add_asset_to_timeline`] and
/// [`OcaApp::add_asset_to_timeline_at`]: `preferred_track_id` if it exists and matches `kind`,
/// else the first existing track of that kind, else a newly created `"V1"`/`"A1"` track
/// appended to `timeline.tracks`. Returns the resolved track's index.
fn resolve_or_create_track(
    timeline: &mut avcore::timeline::Timeline,
    kind: avcore::timeline::TrackKind,
    preferred_track_id: Option<u64>,
) -> usize {
    if let Some(id) = preferred_track_id {
        if let Some(index) = timeline
            .tracks
            .iter()
            .position(|t| t.id == id && t.kind == kind)
        {
            return index;
        }
    }
    if let Some(index) = timeline.tracks.iter().position(|t| t.kind == kind) {
        return index;
    }
    let track_id = timeline.tracks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    let name = match kind {
        avcore::timeline::TrackKind::Video => "V1",
        avcore::timeline::TrackKind::Audio => "A1",
    };
    timeline.tracks.push(avcore::timeline::Track {
        id: track_id,
        name: name.to_string(),
        kind,
        clips: Vec::new(),
    });
    timeline.tracks.len() - 1
}

/// The next free clip id across every track in `timeline` — one past the current max, `1` if
/// the timeline has no clips yet.
fn next_clip_id(timeline: &avcore::timeline::Timeline) -> u64 {
    timeline
        .tracks
        .iter()
        .flat_map(|t| &t.clips)
        .map(|c| c.id)
        .max()
        .unwrap_or(0)
        + 1
}

/// Runs on one of [`OcaApp::spawn_import`]'s per-file background threads, in two phases.
/// Phase one probes `path` and sends [`ImportEvent::AssetReady`] the moment that (cheap)
/// call returns — a file that fails to probe sends [`ImportEvent::Failed`] instead and skips
/// phase two entirely. Phase two measures loudness, (for video) generates an editing proxy,
/// and computes a waveform peak table, then sends [`ImportEvent::Enriched`] with whatever came
/// of it; a step that fails just leaves that one field `None` — the asset was already fully
/// usable from phase one, just not as light to scrub, loudness-tagged, or waveform-drawn yet.
fn import_one(
    path: &Path,
    project_id: u64,
    import_token: u64,
    proxy_dir: &Path,
    tx: &UnboundedSender<ImportEvent>,
) {
    let probed = match avcore::probe_media(path) {
        Ok(probed) => probed,
        Err(e) => {
            let _ = tx.send(ImportEvent::Failed {
                path: path.to_path_buf(),
                message: e.to_string(),
            });
            return;
        }
    };

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let asset = probed.into_media_asset(0, file_name, path.to_path_buf());
    let kind = asset.kind;
    let _ = tx.send(ImportEvent::AssetReady {
        project_id,
        import_token,
        asset,
    });

    let loudness = match avcore::measure_loudness(path) {
        Ok(metrics) => Some(metrics),
        Err(e) => {
            eprintln!("failed to measure loudness for {}: {e}", path.display());
            None
        }
    };
    let proxy_path = if kind == avcore::MediaKind::Video {
        match avcore::ensure_proxy(path, proxy_dir) {
            Ok(proxy_path) => Some(proxy_path),
            Err(e) => {
                eprintln!("failed to generate proxy for {}: {e}", path.display());
                None
            }
        }
    } else {
        None
    };
    let waveform_peaks = match avcore::generate_waveform(path) {
        Ok(peaks) => Some(peaks),
        Err(e) => {
            // Also hit by a video-only asset (no audio track to compute a waveform from) —
            // not worth a distinct log line from an actual decode failure, same as
            // measure_loudness's NoAudioStream case above.
            eprintln!("failed to compute waveform for {}: {e}", path.display());
            None
        }
    };
    let _ = tx.send(ImportEvent::Enriched {
        project_id,
        import_token,
        loudness,
        proxy_path,
        waveform_peaks,
    });
}

/// Longest side, in pixels, a generated thumbnail is downscaled to — tiny on purpose, these
/// are drawn small and tiled, not viewed full-size.
const THUMBNAIL_MAX_DIM: u32 = 96;

/// Source-time width, in seconds, of one filmstrip thumbnail bucket (see
/// [`OcaApp::thumbnail_textures`]). Fixed rather than zoom-dependent — simpler cache
/// invalidation (a bucket's key never changes as the user zooms) at the cost of some tiles
/// repeating when zoomed in past roughly one tile per this many seconds.
pub const THUMBNAIL_BUCKET_SECS: f64 = 1.0;

/// Runs on [`OcaApp::request_thumbnail`]'s background thread — opens `path`, seeks to
/// `at_secs`, and grabs the first frame that decodes, downscaled. `None` if the file doesn't
/// exist, fails to open, or no frame arrives within the poll deadline.
fn extract_thumbnail(path: &Path, at_secs: f64) -> Option<(u32, u32, Vec<u8>)> {
    if !path.exists() {
        return None;
    }
    let preview = avcore::preview::Preview::open(path).ok()?;
    let _ = preview.seek(at_secs.max(0.0));

    // current_frame() is non-blocking (see ensure_preview_loaded's doc comment on the same
    // choice) — a frame isn't necessarily ready the instant seek() returns, so poll briefly
    // for one.
    let deadline = Instant::now() + Duration::from_millis(800);
    let frame = loop {
        if let Some(frame) = preview.current_frame() {
            break frame;
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    Some(downscale_rgba(&frame, THUMBNAIL_MAX_DIM))
}

/// Nearest-neighbor downscale of a decoded frame's RGBA buffer so its longest side is at most
/// `max_dim`. Keeps the uploaded texture tiny regardless of the source's actual resolution —
/// fine for something drawn at thumbnail size, and avoids nearest-neighbor's usual aliasing
/// mattering at that scale.
fn downscale_rgba(frame: &avcore::preview::VideoFrame, max_dim: u32) -> (u32, u32, Vec<u8>) {
    let scale = (max_dim as f32 / frame.width.max(frame.height) as f32).min(1.0);
    let new_width = ((frame.width as f32 * scale) as u32).max(1);
    let new_height = ((frame.height as f32 * scale) as u32).max(1);

    let mut rgba = vec![0u8; (new_width * new_height * 4) as usize];
    for y in 0..new_height {
        let src_y = (y * frame.height / new_height).min(frame.height - 1);
        for x in 0..new_width {
            let src_x = (x * frame.width / new_width).min(frame.width - 1);
            let src = ((src_y * frame.width + src_x) * 4) as usize;
            let dst = ((y * new_width + x) * 4) as usize;
            rgba[dst..dst + 4].copy_from_slice(&frame.rgba[src..src + 4]);
        }
    }
    (new_width, new_height, rgba)
}

impl eframe::App for OcaApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump_export_queue();
        self.pump_import_queue();
        self.pump_thumbnail_queue(ui.ctx());
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
