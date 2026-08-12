//! Application state ([`OcaApp`]) and the top-level `eframe::App` implementation that
//! drives one frame: pump the background export queue, draw the nav rail and breadcrumb,
//! then delegate to whichever [`Screen`] is currently active (see [`crate::screens`]).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use avcore::{
    ClipInstance, ExportJob, ExportJobStatus, MediaAsset, Project, RenderOutcome, TrackKind,
};
use eframe::egui;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use tracing::{debug, error, info, warn};

use crate::i18n::{Locale, Text};
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

/// User-configurable settings shown on the Ajustes screen. Persisted to a JSON file in the
/// platform config dir — see [`OcaApp::save_prefs`] / [`load_prefs`].
#[derive(serde::Serialize, serde::Deserialize)]
pub struct PrefsState {
    /// Index into [`LUFS_PROFILES`].
    pub lufs_profile: usize,
    pub true_peak_limiter: bool,
    pub export_workers: u8,
    pub output_folder: String,
    pub autosave_minutes: u8,
    /// Absolute paths of recently opened project JSON files, most-recent first, capped at 10.
    /// Loaded at startup to restore the Home screen's project grid across sessions.
    #[serde(default)]
    pub recent_project_paths: Vec<String>,
    /// UI language, persisted so the user's choice survives restarts.
    #[serde(default)]
    pub locale: crate::i18n::Locale,
}

impl Default for PrefsState {
    fn default() -> Self {
        Self {
            lufs_profile: 0,
            true_peak_limiter: true,
            export_workers: 1,
            output_folder: String::new(),
            autosave_minutes: 5,
            recent_project_paths: Vec::new(),
            locale: crate::i18n::Locale::default(),
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

/// Slider bounds for the properties panel's per-block gain control (Fase 4's "ganho de volume
/// por bloco") — [`OcaApp::set_selected_clip_gain`] clamps to this range.
pub const GAIN_DB_RANGE: std::ops::RangeInclusive<f32> = -24.0..=24.0;

/// Slider bounds for the properties panel's per-block speed control (Fase 4's "Velocidade") —
/// [`OcaApp::set_selected_clip_speed`] clamps to this range.
pub const SPEED_FACTOR_RANGE: std::ops::RangeInclusive<f32> = 0.25..=4.0;

/// Minimum width/height the properties panel's crop controls allow for
/// [`avcore::timeline::ClipInstance::crop_w`]/`crop_h` (Fase 4's "Recorte") — keeps a drag from
/// collapsing the crop rect to nothing.
pub const CROP_MIN_SIZE: f32 = 0.05;

/// Slider bounds for the properties panel's mask corner-radius control (Fase 4's "Máscaras",
/// [`avcore::timeline::ClipInstance::mask_corner_radius`]).
pub const MASK_CORNER_RADIUS_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0;

/// Slider bounds for the properties panel's vignette control (Fase 4's "Efeitos visuais",
/// [`avcore::timeline::ClipInstance::vignette_intensity`]).
pub const VIGNETTE_INTENSITY_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0;

/// Slider bounds for the properties panel's brightness control (Fase 4's "Efeitos visuais",
/// [`avcore::timeline::ClipInstance::brightness`]).
pub const BRIGHTNESS_RANGE: std::ops::RangeInclusive<f32> = -1.0..=1.0;

/// Slider bounds for the properties panel's contrast control (Fase 4's "Efeitos visuais",
/// [`avcore::timeline::ClipInstance::contrast`]).
pub const CONTRAST_RANGE: std::ops::RangeInclusive<f32> = 0.0..=2.0;

/// Slider bounds for the properties panel's saturation control (Fase 4's "Efeitos visuais",
/// [`avcore::timeline::ClipInstance::saturation`]).
pub const SATURATION_RANGE: std::ops::RangeInclusive<f32> = 0.0..=2.0;

/// Slider bounds for the properties panel's sharpen control (Fase 4's "Efeitos visuais",
/// [`avcore::timeline::ClipInstance::sharpen`]).
pub const SHARPEN_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0;

/// Slider bounds for the properties panel's chroma key tolerance control (Fase 4's "Efeitos
/// visuais", [`avcore::timeline::ClipInstance::chroma_key_tolerance`]).
pub const CHROMA_KEY_TOLERANCE_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0;

/// Slider bounds for the properties panel's blur control (Fase 4's "Efeitos visuais",
/// [`avcore::timeline::ClipInstance::blur_intensity`]).
pub const BLUR_INTENSITY_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0;

/// Slider bounds for the properties panel's shake control (Fase 4's "Efeitos visuais",
/// [`avcore::timeline::ClipInstance::shake_intensity`]).
pub const SHAKE_INTENSITY_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0;

/// Slider bounds for the properties panel's glitch control (Fase 4's "Efeitos visuais",
/// [`avcore::timeline::ClipInstance::glitch_intensity`]).
pub const GLITCH_INTENSITY_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0;

/// Slider bounds for the properties panel's pixelize control (Fase 4's "Efeitos visuais",
/// [`avcore::timeline::ClipInstance::pixelize_intensity`]).
pub const PIXELIZE_INTENSITY_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0;

/// Slider bounds for the properties panel's transition duration control (Fase 4's "Efeitos
/// visuais", [`avcore::timeline::ClipInstance::transition_duration_secs`]).
pub const TRANSITION_DURATION_RANGE: std::ops::RangeInclusive<f32> = 0.1..=3.0;

/// Slider bounds for the properties panel's zoom controls (Fase 4's "Efeitos visuais",
/// [`avcore::timeline::ClipInstance::zoom_start`]/`zoom_end`).
pub const ZOOM_RANGE: std::ops::RangeInclusive<f32> = 1.0..=3.0;

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

// ClipFormatting is defined in core::timeline and re-exported as avcore::ClipFormatting;
// the type alias below is kept for in-module readability only.
use avcore::ClipFormatting;

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
    pub prefs: PrefsState,
    render_tx: UnboundedSender<RenderEvent>,
    render_rx: UnboundedReceiver<RenderEvent>,
    /// Cancellation flags for jobs a worker thread is currently rendering, keyed by job id.
    /// A job id present here is the source of truth for "how many workers are busy right
    /// now" — [`OcaApp::pump_export_queue`] uses its length against `prefs.export_workers`.
    active_renders: HashMap<u64, Arc<AtomicBool>>,
    /// The GStreamer pipeline for the clip currently covering the active sequence's timeline
    /// playhead, if it could be opened (`None` before any project has a clip at the playhead,
    /// before it's been lazily opened, and when `Preview::open` failed, e.g. a source file
    /// that's since been moved or deleted — see [`OcaApp::ensure_preview_loaded`]).
    preview: Option<avcore::preview::Preview>,
    /// The clip id [`OcaApp::ensure_preview_loaded`] last attempted to open a pipeline for,
    /// whether or not it succeeded — lets it tell "already tried and failed for this exact
    /// clip, don't retry every frame" apart from "the playhead moved onto a different clip, try
    /// again".
    preview_clip_id: Option<u64>,
    /// Uploaded from the latest [`avcore::preview::Preview::current_frame`] each frame the
    /// Editor screen is shown; `None` until the first frame decodes. Reset whenever
    /// [`OcaApp::ensure_preview_loaded`] reopens the pipeline for a different clip so a stale
    /// frame from the previous one never lingers.
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
    /// The last clip copied or cut via `Ctrl+C`/`Ctrl+X`/the timeline context menu, and the
    /// track kind it came from (so a paste lands on a matching-kind track — same rule as a
    /// drag-move). Not scoped to a project or sequence: pasting into a different tab, or even
    /// a different project, is what makes "copiar e colar entre abas" (`request.md`'s Fase 3
    /// spec) work for free, rather than needing separate cross-tab plumbing.
    clipboard_clip: Option<(avcore::timeline::ClipInstance, avcore::timeline::TrackKind)>,
    /// The last formatting (gain/freeze settings, not the clip itself) copied via
    /// `Ctrl+Shift+C`/the timeline context menu — [`OcaApp::paste_selected_clip_formatting`]
    /// applies it onto a different block, per `request.md`'s Fase 4 "copiar formatação" spec.
    formatting_clipboard: Option<ClipFormatting>,
    /// Clip ids picked (via `Ctrl+click`) as candidates for [`OcaApp::merge_into_composite`] —
    /// separate from `selected_clip_id`, which stays single-target for every other clip
    /// operation (trim, delete, copy, split). Cleared after a successful merge; not otherwise
    /// tied to `selected_clip_id`, so the last-clicked clip can be a multi-select member
    /// without also being "the" selection.
    pub multi_selected_clip_ids: HashSet<u64>,
    /// Set whenever [`OcaApp::active_project_mut`] is called; cleared after each autosave
    /// write. Guards [`OcaApp::pump_autosave`] from writing unchanged state to disk.
    project_dirty: bool,
    /// Timestamp of the most recent call to [`OcaApp::active_project_mut`] — the debounce
    /// start for the 2-second idle window in [`OcaApp::pump_autosave`].
    last_edit_instant: Option<Instant>,
    /// Timestamp of the last successful autosave write — used to enforce the 30-second
    /// ceiling that forces a save even during continuous editing.
    last_autosave_instant: Option<Instant>,
    /// Short-lived error messages shown as floating overlays at the bottom-right of the window.
    /// Each entry is `(message, born_at)`; [`OcaApp::show_toasts`] removes entries older than
    /// 4 seconds each frame. Use [`OcaApp::push_toast`] to add one.
    toasts: Vec<(String, Instant)>,
    /// Whether the preferences modal is currently open — toggled by the nav rail's ⚙ button.
    /// Kept separate from `screen` so the modal overlays whatever screen is currently active
    /// rather than replacing it with a dedicated route.
    pub prefs_open: bool,
    /// The value of `prefs_open` on the previous frame — lets [`OcaApp::ui`] detect the
    /// closing edge (true → false) and trigger a prefs save exactly once.
    prev_prefs_open: bool,
    /// Set to the autosave file path when opening a project that has a newer autosave on disk.
    /// [`OcaApp::pump_autosave_restore`] consumes it to show the restore/discard modal.
    autosave_restore_pending: Option<PathBuf>,
    /// `true` when a crash sentinel from a previous session was found at startup — consumed
    /// by [`OcaApp::ui`] to show a one-time toast, then cleared.
    crash_detected: bool,
    /// Target aspect ratio selected in the export queue's "Add Export" row. Defaults to
    /// `Original` (source dimensions). Persists between export invocations so the user doesn't
    /// have to re-select it every time.
    pub export_aspect_ratio: avcore::ExportAspectRatio,
}

impl OcaApp {
    /// Builds the initial app state: applies the theme and starts with an empty project list
    /// and export queue — every project, asset, and job comes from the user via "Novo
    /// projeto"/"Abrir projeto" and real imports, not mock data.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let sentinel = sentinel_path();
        let crash_detected = sentinel.exists();
        if let Some(dir) = sentinel.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Write the sentinel — deleted on clean exit via on_exit(). Survives a crash.
        let _ = std::fs::write(&sentinel, b"");
        let prefs = load_prefs();
        // Reload projects from the last session. Failures (moved/deleted files) are silently
        // skipped — the missing path will be pruned from recents next time prefs are saved.
        let projects: Vec<avcore::Project> = prefs
            .recent_project_paths
            .iter()
            .filter_map(|p| {
                let path = PathBuf::from(p);
                avcore::load_project_from_file(&path).ok().map(|mut proj| {
                    proj.file_path = Some(path);
                    proj
                })
            })
            .collect();
        let (render_tx, render_rx) = mpsc::unbounded_channel();
        let (import_tx, import_rx) = mpsc::unbounded_channel();
        let (thumbnail_tx, thumbnail_rx) = mpsc::unbounded_channel();
        Self {
            screen: Screen::Home,
            tool: EditorTool::Select,
            locale: prefs.locale,
            projects,
            active_project: 0,
            selected_asset_id: None,
            export_jobs: Vec::new(),
            prefs,
            render_tx,
            render_rx,
            active_renders: HashMap::new(),
            preview: None,
            preview_clip_id: None,
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
            clipboard_clip: None,
            formatting_clipboard: None,
            multi_selected_clip_ids: HashSet::new(),
            toasts: Vec::new(),
            prefs_open: false,
            prev_prefs_open: false,
            project_dirty: false,
            last_edit_instant: None,
            last_autosave_instant: None,
            autosave_restore_pending: None,
            crash_detected,
            export_aspect_ratio: avcore::ExportAspectRatio::default(),
        }
    }

    /// The project currently open in the Editor/Mídia screens.
    pub fn active_project(&self) -> &Project {
        &self.projects[self.active_project]
    }

    /// Guarantees `active_project()`/`active_project_mut()` resolve — the Editor and Mídia
    /// screens call this before touching either, since a fresh launch's `projects` starts
    /// empty and navigating straight there (nav rail, no "Novo projeto" first) would otherwise
    /// index out of bounds. Creates and opens an untitled project exactly like clicking "Novo
    /// projeto" would, only when none exists yet; a no-op once any project is open.
    ///
    /// `create_new_project` also forces `screen` to `Editor` (what the Home "Novo projeto"
    /// button relies on to navigate away from Home) — restored here so calling this from
    /// Mídia doesn't hijack the user back to the Editor screen mid-render.
    pub fn ensure_active_project(&mut self) {
        if self.projects.is_empty() {
            let screen_before = self.screen;
            self.create_new_project(Text::UntitledProject.tr(self.locale).to_string());
            self.screen = screen_before;
        }
    }

    /// Mutable access to the project currently open in the Editor/Mídia screens — for
    /// imports, edits, and anything else that changes the active project in place. Sets the
    /// autosave dirty flag so [`OcaApp::pump_autosave`] knows to write the recovery file.
    pub fn active_project_mut(&mut self) -> &mut Project {
        self.project_dirty = true;
        self.last_edit_instant = Some(Instant::now());
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

    /// The timeline clip backing the properties panel's per-block controls (gain, freeze), if
    /// `selected_clip_id` points at one on the active sequence.
    pub fn selected_clip(&self) -> Option<&avcore::timeline::ClipInstance> {
        let id = self.selected_clip_id?;
        self.active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .find(|c| c.id == id)
    }

    /// The kind of track `selected_clip_id` lives on, if any — lets the properties panel gate
    /// the freeze-frame toggle to video clips only ("Congelar" holds a video frame, audio
    /// clips have none).
    pub fn selected_clip_track_kind(&self) -> Option<avcore::timeline::TrackKind> {
        let id = self.selected_clip_id?;
        self.active_project()
            .timeline()
            .tracks
            .iter()
            .find(|t| t.clips.iter().any(|c| c.id == id))
            .map(|t| t.kind)
    }

    /// Selects a timeline clip and its backing asset together, so the properties panel's
    /// per-block controls and the preview stay in sync with a direct timeline click.
    pub fn select_timeline_clip(&mut self, id: u64) {
        let asset_id = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|track| &track.clips)
            .find(|clip| clip.id == id)
            .map(|clip| clip.asset_id);
        self.selected_clip_id = Some(id);
        self.select_asset(asset_id);
    }

    /// Switches the active project to `index` and navigates to the Editor screen — this is
    /// what a project card click on the Início screen does.
    pub fn open_project(&mut self, index: usize) {
        self.active_project = index;
        let project = self.active_project();
        info!(
            project_id = project.id,
            name = %project.name,
            path = ?project.file_path,
            "project opened"
        );
        let asset_id = project.media_library.first().map(|a| a.id);
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
    }

    /// Id of the clip covering the active sequence's timeline playhead, if any. Cheaper than
    /// [`OcaApp::current_preview_clip`] — no clones — used by [`OcaApp::ensure_preview_loaded`]
    /// for the early-exit check.
    fn current_preview_clip_id(&self) -> Option<u64> {
        let timeline = self.active_project().timeline();
        let track = timeline.tracks.iter().find(|t| t.kind == TrackKind::Video)?;
        Some(track.clip_at(timeline.playhead_secs)?.id)
    }

    /// The clip covering the active sequence's timeline playhead, and the asset it plays from,
    /// if both resolve — `None` if the video track is missing/empty, nothing covers the
    /// playhead ([`avcore::timeline::Track::clip_at`]), or the clip's `asset_id` isn't in the
    /// media library.
    fn current_preview_clip(&self) -> Option<(ClipInstance, MediaAsset)> {
        let project = self.active_project();
        let timeline = project.timeline();
        let track = timeline
            .tracks
            .iter()
            .find(|t| t.kind == TrackKind::Video)?;
        let clip = track.clip_at(timeline.playhead_secs)?;
        let asset = project
            .media_library
            .iter()
            .find(|a| a.id == clip.asset_id)?;
        Some((clip.clone(), asset.clone()))
    }

    /// Reopens the preview pipeline whenever the clip covering the timeline playhead
    /// ([`OcaApp::current_preview_clip`]) differs from the one last opened for
    /// (`preview_clip_id`) — called once per frame from the Editor's preview panel, right
    /// before it reads any preview state, so the first paint after the playhead moves onto a
    /// different clip is what actually triggers `Preview::open`. Prefers the editing proxy if
    /// one exists (lighter to decode), otherwise the original source file. Leaves `preview` as
    /// `None` without an error dialog if `Preview::open` fails (e.g. a source file that's been
    /// moved or deleted since import) — the Editor screen shows a muted "preview unavailable"
    /// label instead, and won't retry until the playhead moves onto a different clip. Seeks
    /// into the newly opened clip at the playhead's own offset, and resumes playback
    /// (`preview_playing` was already `true`) so crossing a cut doesn't pause playback, just
    /// hitches while the new pipeline opens.
    pub fn ensure_preview_loaded(&mut self) {
        let current_clip_id = self.current_preview_clip_id();
        if current_clip_id == self.preview_clip_id {
            return;
        }
        let current = self.current_preview_clip();
        self.preview = None;
        self.preview_texture = None;
        self.preview_clip_id = current_clip_id;

        let Some((clip, asset)) = current else {
            self.preview_playing = false;
            return;
        };
        let path = asset
            .proxy_path
            .clone()
            .unwrap_or_else(|| asset.source_path.clone());
        // A moved/deleted source file would otherwise still spin up a whole GStreamer pipeline
        // just to watch it fail to open a file that isn't there.
        if !path.exists() {
            return;
        }

        match avcore::preview::Preview::open(&path, Some(&clip)) {
            Ok(preview) => {
                debug!(path = %path.display(), clip_id = clip.id, "preview pipeline opened");
                let playhead = self.active_project().timeline().playhead_secs;
                let offset = clip.source_in_secs + (playhead - clip.start_secs);
                if let Err(e) = preview.seek(offset) {
                    warn!(error = %e, "failed to seek newly opened preview");
                }
                if self.preview_playing {
                    if let Err(e) = preview.play() {
                        warn!(error = %e, "failed to resume preview playback across a cut");
                    }
                }
                self.preview = Some(preview);
            }
            Err(e) => error!(path = %path.display(), error = %e, "failed to open preview pipeline"),
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
            Err(e) => warn!(error = %e, "failed to toggle preview playback"),
        }
    }

    /// Seeks to `position_secs` (timeline-relative). Takes the fast path — seeking the
    /// already-open pipeline directly — when `position_secs` still falls within the clip it's
    /// currently loaded for; otherwise updates the timeline playhead and lets
    /// [`OcaApp::ensure_preview_loaded`] open the right clip's pipeline next frame. A no-op if
    /// nothing is selected or the pipeline failed to open.
    pub fn seek_preview(&mut self, position_secs: f64) {
        let same_clip = self
            .preview_clip_id
            .zip(self.current_preview_clip())
            .filter(|(loaded_id, (clip, _))| *loaded_id == clip.id)
            .map(|(_, (clip, _))| clip);

        match (&self.preview, same_clip) {
            (Some(preview), Some(clip)) => {
                let offset = clip.source_in_secs + (position_secs - clip.start_secs);
                if let Err(e) = preview.seek(offset) {
                    warn!(error = %e, "failed to seek preview");
                }
                self.active_project_mut().timeline_mut().playhead_secs = position_secs;
            }
            _ => {
                self.active_project_mut().timeline_mut().playhead_secs = position_secs;
            }
        }
    }

    /// Whether the clip at the timeline playhead has a live preview pipeline — `false` before
    /// any clip covers the playhead, before [`OcaApp::ensure_preview_loaded`] has run for it,
    /// and when it couldn't open one.
    pub fn preview_available(&self) -> bool {
        self.preview.is_some()
    }

    /// Whether a clip currently covers the timeline playhead, whether or not its preview
    /// pipeline could actually be opened — distinguishes "nothing to preview here" from
    /// "something's here but its preview failed to open" for the Editor's empty-state label.
    pub fn preview_clip_present(&self) -> bool {
        self.preview_clip_id.is_some()
    }

    /// Pulls the latest decoded video frame (if any) into `preview_texture`, and — while
    /// playing — mirrors the pipeline's position into the active project's timeline playhead,
    /// converting from the clip-relative position `Preview` reports back to timeline time.
    /// Once the clip currently loaded (`preview_clip_id`) plays past its own
    /// `source_out_secs`, advances the playhead to that clip's end instead — the next frame's
    /// `ensure_preview_loaded` call then naturally opens whatever clip (if any) covers that
    /// position, continuing playback across the cut. Called once per frame from
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

        if !self.preview_playing {
            return;
        }
        let Some(position) = preview.position_secs() else {
            return;
        };
        let Some(clip_id) = self.preview_clip_id else {
            return;
        };
        let Some(clip) = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .find_map(|t| t.clips.iter().find(|c| c.id == clip_id))
            .cloned()
        else {
            return;
        };

        let new_playhead = if position >= clip.source_out_secs {
            clip.start_secs + clip.duration_secs()
        } else {
            clip.start_secs + (position - clip.source_in_secs)
        };
        self.active_project_mut().timeline_mut().playhead_secs = new_playhead;
    }

    /// Appends `project` to the project list and opens it — used for both "Novo projeto"
    /// (an empty project) and "Abrir projeto" (one just loaded from disk).
    pub fn add_and_open_project(&mut self, project: Project) {
        self.projects.push(project);
        let idx = self.projects.len() - 1;
        // Track the file path in recent projects before open_project() runs.
        if let Some(path) = self.projects[idx].file_path.clone() {
            let path_str = path.display().to_string();
            self.prefs.recent_project_paths.retain(|p| p != &path_str);
            self.prefs.recent_project_paths.insert(0, path_str);
            self.prefs.recent_project_paths.truncate(10);
            self.save_prefs();
        }
        self.open_project(idx);
    }

    /// Removes the project at `index` from the in-memory list and from `prefs.recent_project_paths`,
    /// then persists prefs. Adjusts `active_project` so it stays in bounds. Does NOT navigate —
    /// the caller (Home screen) decides whether to switch screens.
    pub fn remove_project(&mut self, index: usize) {
        if index >= self.projects.len() {
            return;
        }
        // Remove from recents before dropping the project.
        if let Some(path) = &self.projects[index].file_path {
            let path_str = path.display().to_string();
            self.prefs.recent_project_paths.retain(|p| p != &path_str);
        }
        self.projects.remove(index);
        // Keep active_project in bounds.
        if !self.projects.is_empty() && self.active_project >= self.projects.len() {
            self.active_project = self.projects.len() - 1;
        }
        self.save_prefs();
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
            sequences: vec![avcore::Sequence {
                id: 1,
                name: Text::DefaultSequenceName.tr(self.locale).to_string(),
                timeline: avcore::Timeline {
                    tracks: Vec::new(),
                    playhead_secs: 0.0,
                },
            }],
            active_sequence: 0,
            file_path: None,
        });
    }

    /// Appends a new, empty sequence tab to the active project and switches to it — what the
    /// Editor's tab bar "+" button does (per `request.md`'s Fase 3 "abas de projeto" spec).
    /// Named positionally (`Text::DefaultSequenceName` is reserved for a project's first,
    /// non-numbered tab).
    pub fn add_sequence(&mut self) {
        let locale = self.locale;
        let project = self.active_project_mut();
        let n = project.sequences.len() + 1;
        project.new_sequence(crate::i18n::sequence_name(locale, n));
        // Clip ids are only unique within a sequence (each one numbers its own clips from 1
        // via next_clip_id), so a selection left over from the previous tab could otherwise
        // spuriously highlight an unrelated clip if the ids happen to collide.
        self.selected_clip_id = None;
    }

    /// Switches the active project's tab to `index` — what clicking a tab in the Editor's tab
    /// bar does. A no-op if `index` is out of range.
    pub fn select_sequence(&mut self, index: usize) {
        let project = self.active_project_mut();
        if index < project.sequences.len() {
            project.active_sequence = index;
            // See add_sequence's comment on why a cross-sequence selection isn't safe to keep.
            self.selected_clip_id = None;
        }
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
        let timeline = self.active_project_mut().timeline_mut();
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
                composite_id: None,
                gain_db: 0.0,
                frozen: false,
                speed_factor: 1.0,
                crop_x: 0.0,
                crop_y: 0.0,
                crop_w: 1.0,
                crop_h: 1.0,
                mask_shape: avcore::timeline::MaskShape::None,
                mask_corner_radius: 0.0,
                flipped_h: false,
                color_filter: avcore::timeline::ColorFilter::None,
                vignette_intensity: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                sharpen: 0.0,
                chroma_key_enabled: false,
                chroma_key_color: [0, 255, 0],
                chroma_key_tolerance: 0.4,
                blur_intensity: 0.0,
                shake_intensity: 0.0,
                glitch_intensity: 0.0,
                pixelize_intensity: 0.0,
                transition_in: avcore::timeline::TransitionType::None,
                transition_duration_secs: 0.5,
                zoom_start: 1.0,
                zoom_end: 1.0,
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
        let timeline = self.active_project_mut().timeline_mut();
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
                composite_id: None,
                gain_db: 0.0,
                frozen: false,
                speed_factor: 1.0,
                crop_x: 0.0,
                crop_y: 0.0,
                crop_w: 1.0,
                crop_h: 1.0,
                mask_shape: avcore::timeline::MaskShape::None,
                mask_corner_radius: 0.0,
                flipped_h: false,
                color_filter: avcore::timeline::ColorFilter::None,
                vignette_intensity: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                sharpen: 0.0,
                chroma_key_enabled: false,
                chroma_key_color: [0, 255, 0],
                chroma_key_tolerance: 0.4,
                blur_intensity: 0.0,
                shake_intensity: 0.0,
                glitch_intensity: 0.0,
                pixelize_intensity: 0.0,
                transition_in: avcore::timeline::TransitionType::None,
                transition_duration_secs: 0.5,
                zoom_start: 1.0,
                zoom_end: 1.0,
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
        let at_secs = self.active_project().timeline().playhead_secs;
        let mut next_clip_id = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .map(|c| c.id)
            .max()
            .unwrap_or(0)
            + 1;

        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if track.split_clip_at(at_secs, next_clip_id) {
                next_clip_id += 1;
            }
        }
    }

    /// Removes `selected_clip_id` from whichever track has it and clears the selection — what
    /// pressing `Delete` on the timeline does. Leaves a gap rather than rippling later clips
    /// left, matching `split_at_playhead`'s equally simple non-ripple editing model. If the
    /// selected clip is a composite block member (`composite_id.is_some()`), every clip
    /// sharing that id is removed too — deleting one member deletes the whole block, per
    /// `request.md`'s Fase 3 "reutilizado ... como se fosse um clipe só" spec. A no-op if
    /// nothing is selected.
    pub fn delete_selected_clip(&mut self) {
        let Some(clip_id) = self.selected_clip_id else {
            return;
        };
        let timeline = self.active_project_mut().timeline_mut();
        let composite_id = timeline
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .find(|c| c.id == clip_id)
            .and_then(|c| c.composite_id);
        for track in &mut timeline.tracks {
            match composite_id {
                Some(group) => track.clips.retain(|c| c.composite_id != Some(group)),
                None => track.clips.retain(|c| c.id != clip_id),
            }
        }
        self.selected_clip_id = None;
    }

    /// Calls `f` with a mutable borrow of the selected clip, if any — the shared dispatch path
    /// for every `set_selected_clip_*` setter.
    fn with_selected_clip_mut(&mut self, f: impl FnOnce(&mut avcore::timeline::ClipInstance)) {
        let Some(clip_id) = self.selected_clip_id else {
            return;
        };
        if let Some(clip) = self.active_project_mut().timeline_mut().clip_mut(clip_id) {
            f(clip);
        }
    }

    /// Sets `selected_clip_id`'s [`avcore::timeline::ClipInstance::gain_db`], clamped to
    /// [`GAIN_DB_RANGE`] — what dragging the properties panel's gain slider does. A no-op if
    /// nothing is selected.
    pub fn set_selected_clip_gain(&mut self, gain_db: f32) {
        let gain_db = gain_db.clamp(*GAIN_DB_RANGE.start(), *GAIN_DB_RANGE.end());
        self.with_selected_clip_mut(|clip| clip.gain_db = gain_db);
    }

    /// Sets `selected_clip_id`'s [`avcore::timeline::ClipInstance::frozen`] — what checking the
    /// properties panel's "Congelar quadro" box does. A no-op if nothing is selected.
    pub fn set_selected_clip_frozen(&mut self, frozen: bool) {
        self.with_selected_clip_mut(|clip| clip.frozen = frozen);
    }

    /// Sets `selected_clip_id`'s [`avcore::timeline::ClipInstance::speed_factor`], clamped to
    /// [`SPEED_FACTOR_RANGE`] — what dragging the properties panel's speed slider does. A no-op
    /// if nothing is selected.
    pub fn set_selected_clip_speed(&mut self, speed_factor: f32) {
        let speed_factor =
            speed_factor.clamp(*SPEED_FACTOR_RANGE.start(), *SPEED_FACTOR_RANGE.end());
        self.with_selected_clip_mut(|clip| clip.speed_factor = speed_factor);
    }

    /// Sets `selected_clip_id`'s crop rect ([`avcore::timeline::ClipInstance::crop_x`]/`crop_y`/
    /// `crop_w`/`crop_h`), each independently clamped to `[0.0, 1.0]` (`crop_w`/`crop_h` floored
    /// at [`CROP_MIN_SIZE`]) — what dragging the properties panel's crop controls does. A no-op
    /// if nothing is selected.
    pub fn set_selected_clip_crop(&mut self, crop_x: f32, crop_y: f32, crop_w: f32, crop_h: f32) {
        let crop_x = crop_x.clamp(0.0, 1.0);
        let crop_y = crop_y.clamp(0.0, 1.0);
        let crop_w = crop_w.clamp(CROP_MIN_SIZE, 1.0);
        let crop_h = crop_h.clamp(CROP_MIN_SIZE, 1.0);
        self.with_selected_clip_mut(|clip| {
            clip.crop_x = crop_x;
            clip.crop_y = crop_y;
            clip.crop_w = crop_w;
            clip.crop_h = crop_h;
        });
    }

    /// Sets `selected_clip_id`'s layer mask ([`avcore::timeline::ClipInstance::mask_shape`]/
    /// `mask_corner_radius`, the latter clamped to [`MASK_CORNER_RADIUS_RANGE`]) — what picking
    /// a shape/dragging the corner-radius slider in the properties panel does. A no-op if
    /// nothing is selected.
    pub fn set_selected_clip_mask(
        &mut self,
        mask_shape: avcore::timeline::MaskShape,
        mask_corner_radius: f32,
    ) {
        let mask_corner_radius = mask_corner_radius.clamp(
            *MASK_CORNER_RADIUS_RANGE.start(),
            *MASK_CORNER_RADIUS_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| {
            clip.mask_shape = mask_shape;
            clip.mask_corner_radius = mask_corner_radius;
        });
    }

    /// Sets `selected_clip_id`'s horizontal mirroring
    /// ([`avcore::timeline::ClipInstance::flipped_h`]) — what checking the properties panel's
    /// "Espelhar" box does. A no-op if nothing is selected.
    pub fn set_selected_clip_flip_h(&mut self, flipped_h: bool) {
        self.with_selected_clip_mut(|clip| clip.flipped_h = flipped_h);
    }

    /// Sets `selected_clip_id`'s color filter
    /// ([`avcore::timeline::ClipInstance::color_filter`]) — what picking a filter in the
    /// properties panel does. A no-op if nothing is selected.
    pub fn set_selected_clip_color_filter(&mut self, color_filter: avcore::timeline::ColorFilter) {
        self.with_selected_clip_mut(|clip| clip.color_filter = color_filter);
    }

    /// Sets `selected_clip_id`'s vignette strength
    /// ([`avcore::timeline::ClipInstance::vignette_intensity`], clamped to
    /// [`VIGNETTE_INTENSITY_RANGE`]) — what dragging the properties panel's vignette slider
    /// does. A no-op if nothing is selected.
    pub fn set_selected_clip_vignette(&mut self, vignette_intensity: f32) {
        let vignette_intensity = vignette_intensity.clamp(
            *VIGNETTE_INTENSITY_RANGE.start(),
            *VIGNETTE_INTENSITY_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| clip.vignette_intensity = vignette_intensity);
    }

    /// Sets `selected_clip_id`'s sharpen strength
    /// ([`avcore::timeline::ClipInstance::sharpen`], clamped to [`SHARPEN_RANGE`]) — what
    /// dragging the properties panel's sharpen slider does. A no-op if nothing is selected.
    pub fn set_selected_clip_sharpen(&mut self, sharpen: f32) {
        let sharpen = sharpen.clamp(*SHARPEN_RANGE.start(), *SHARPEN_RANGE.end());
        self.with_selected_clip_mut(|clip| clip.sharpen = sharpen);
    }

    /// Sets `selected_clip_id`'s chroma key settings
    /// ([`avcore::timeline::ClipInstance::chroma_key_enabled`]/`chroma_key_color`/
    /// `chroma_key_tolerance`, the tolerance clamped to [`CHROMA_KEY_TOLERANCE_RANGE`]) — what
    /// toggling the checkbox or adjusting the color/tolerance controls in the properties panel
    /// does. A no-op if nothing is selected.
    pub fn set_selected_clip_chroma_key(
        &mut self,
        chroma_key_enabled: bool,
        chroma_key_color: [u8; 3],
        chroma_key_tolerance: f32,
    ) {
        let chroma_key_tolerance = chroma_key_tolerance.clamp(
            *CHROMA_KEY_TOLERANCE_RANGE.start(),
            *CHROMA_KEY_TOLERANCE_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| {
            clip.chroma_key_enabled = chroma_key_enabled;
            clip.chroma_key_color = chroma_key_color;
            clip.chroma_key_tolerance = chroma_key_tolerance;
        });
    }

    /// Sets `selected_clip_id`'s blur strength
    /// ([`avcore::timeline::ClipInstance::blur_intensity`], clamped to [`BLUR_INTENSITY_RANGE`])
    /// — what dragging the properties panel's blur slider does. A no-op if nothing is selected.
    pub fn set_selected_clip_blur(&mut self, blur_intensity: f32) {
        let blur_intensity =
            blur_intensity.clamp(*BLUR_INTENSITY_RANGE.start(), *BLUR_INTENSITY_RANGE.end());
        self.with_selected_clip_mut(|clip| clip.blur_intensity = blur_intensity);
    }

    /// Sets `selected_clip_id`'s camera-shake strength
    /// ([`avcore::timeline::ClipInstance::shake_intensity`], clamped to
    /// [`SHAKE_INTENSITY_RANGE`]) — what dragging the properties panel's shake slider does. A
    /// no-op if nothing is selected.
    pub fn set_selected_clip_shake(&mut self, shake_intensity: f32) {
        let shake_intensity =
            shake_intensity.clamp(*SHAKE_INTENSITY_RANGE.start(), *SHAKE_INTENSITY_RANGE.end());
        self.with_selected_clip_mut(|clip| clip.shake_intensity = shake_intensity);
    }

    /// Sets `selected_clip_id`'s glitch strength
    /// ([`avcore::timeline::ClipInstance::glitch_intensity`], clamped to
    /// [`GLITCH_INTENSITY_RANGE`]) — what dragging the properties panel's glitch slider does. A
    /// no-op if nothing is selected.
    pub fn set_selected_clip_glitch(&mut self, glitch_intensity: f32) {
        let glitch_intensity = glitch_intensity.clamp(
            *GLITCH_INTENSITY_RANGE.start(),
            *GLITCH_INTENSITY_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| clip.glitch_intensity = glitch_intensity);
    }

    /// Sets `selected_clip_id`'s pixelize/mosaic-censor strength
    /// ([`avcore::timeline::ClipInstance::pixelize_intensity`], clamped to
    /// [`PIXELIZE_INTENSITY_RANGE`]) — what dragging the properties panel's pixelize slider
    /// does. A no-op if nothing is selected.
    pub fn set_selected_clip_pixelize(&mut self, pixelize_intensity: f32) {
        let pixelize_intensity = pixelize_intensity.clamp(
            *PIXELIZE_INTENSITY_RANGE.start(),
            *PIXELIZE_INTENSITY_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| clip.pixelize_intensity = pixelize_intensity);
    }

    /// Sets `selected_clip_id`'s transition
    /// ([`avcore::timeline::ClipInstance::transition_in`]/`transition_duration_secs`, the
    /// duration clamped to [`TRANSITION_DURATION_RANGE`]) — what picking a transition type or
    /// dragging the duration slider in the properties panel does. A no-op if nothing is
    /// selected.
    pub fn set_selected_clip_transition(
        &mut self,
        transition_in: avcore::timeline::TransitionType,
        transition_duration_secs: f32,
    ) {
        let transition_duration_secs = transition_duration_secs.clamp(
            *TRANSITION_DURATION_RANGE.start(),
            *TRANSITION_DURATION_RANGE.end(),
        );
        self.with_selected_clip_mut(|clip| {
            clip.transition_in = transition_in;
            clip.transition_duration_secs = transition_duration_secs;
        });
    }

    /// Sets `selected_clip_id`'s zoom ([`avcore::timeline::ClipInstance::zoom_start`]/
    /// `zoom_end`), each independently clamped to [`ZOOM_RANGE`] — what dragging the properties
    /// panel's zoom sliders does. A no-op if nothing is selected.
    pub fn set_selected_clip_zoom(&mut self, zoom_start: f32, zoom_end: f32) {
        let zoom_start = zoom_start.clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
        let zoom_end = zoom_end.clamp(*ZOOM_RANGE.start(), *ZOOM_RANGE.end());
        self.with_selected_clip_mut(|clip| {
            clip.zoom_start = zoom_start;
            clip.zoom_end = zoom_end;
        });
    }

    /// Sets `selected_clip_id`'s brightness/contrast/saturation
    /// ([`avcore::timeline::ClipInstance::brightness`]/`contrast`/`saturation`), each
    /// independently clamped to its own range ([`BRIGHTNESS_RANGE`]/[`CONTRAST_RANGE`]/
    /// [`SATURATION_RANGE`]) — what dragging the properties panel's color-adjustment sliders
    /// does. A no-op if nothing is selected.
    pub fn set_selected_clip_color_adjust(
        &mut self,
        brightness: f32,
        contrast: f32,
        saturation: f32,
    ) {
        let brightness = brightness.clamp(*BRIGHTNESS_RANGE.start(), *BRIGHTNESS_RANGE.end());
        let contrast = contrast.clamp(*CONTRAST_RANGE.start(), *CONTRAST_RANGE.end());
        let saturation = saturation.clamp(*SATURATION_RANGE.start(), *SATURATION_RANGE.end());
        self.with_selected_clip_mut(|clip| {
            clip.brightness = brightness;
            clip.contrast = contrast;
            clip.saturation = saturation;
        });
    }

    /// Whether formatting is waiting in the clipboard for
    /// [`OcaApp::paste_selected_clip_formatting`] — lets the timeline context menu grey out
    /// "Colar formatação" otherwise.
    pub fn has_formatting_clipboard(&self) -> bool {
        self.formatting_clipboard.is_some()
    }

    /// Copies `selected_clip_id`'s gain/freeze/speed/crop/mask/flip/color-filter/vignette/
    /// color-adjustment settings to [`OcaApp::formatting_clipboard`] — what `Ctrl+Shift+C`/the
    /// context menu's "Copiar formatação" do. A no-op if nothing is selected.
    pub fn copy_selected_clip_formatting(&mut self) {
        let Some(clip) = self.selected_clip() else {
            return;
        };
        self.formatting_clipboard = Some(clip.formatting());
    }

    /// Applies [`OcaApp::formatting_clipboard`] onto `selected_clip_id`, without touching any
    /// other field (position, trim, composite membership) — what `Ctrl+Shift+V`/the context
    /// menu's "Colar formatação" do. A no-op if nothing is selected or the clipboard is empty.
    pub fn paste_selected_clip_formatting(&mut self) {
        let Some(formatting) = self.formatting_clipboard else {
            return;
        };
        self.with_selected_clip_mut(|clip| clip.apply_formatting(&formatting));
    }

    /// Adds/removes `clip_id` from [`OcaApp::multi_selected_clip_ids`] — what `Ctrl+click`ing
    /// a timeline clip does, building up a set of candidates for
    /// [`OcaApp::merge_into_composite`].
    pub fn toggle_multi_select(&mut self, clip_id: u64) {
        if !self.multi_selected_clip_ids.remove(&clip_id) {
            self.multi_selected_clip_ids.insert(clip_id);
        }
    }

    /// Merges every clip in [`OcaApp::multi_selected_clip_ids`] into one composite block —
    /// what the toolbar's "Mesclar em bloco composto" button does (per `request.md`'s Fase 3
    /// "blocos compostos" spec). Assigns them all a fresh `composite_id` and clears the
    /// multi-selection. A no-op, leaving the multi-selection untouched so the user can fix
    /// their pick, if fewer than two ids were selected or they aren't all on the same track —
    /// composite blocks don't span tracks yet.
    pub fn merge_into_composite(&mut self) {
        if self.multi_selected_clip_ids.len() < 2 {
            return;
        }
        let ids = self.multi_selected_clip_ids.clone();
        let timeline = self.active_project_mut().timeline_mut();
        let Some(track) = timeline
            .tracks
            .iter_mut()
            .find(|t| ids.iter().all(|id| t.clips.iter().any(|c| c.id == *id)))
        else {
            return;
        };
        let group_id = track
            .clips
            .iter()
            .filter_map(|c| c.composite_id)
            .max()
            .unwrap_or(0)
            + 1;
        for clip in &mut track.clips {
            if ids.contains(&clip.id) {
                clip.composite_id = Some(group_id);
            }
        }
        self.multi_selected_clip_ids.clear();
    }

    /// Whether a clip is waiting in the clipboard for [`OcaApp::paste_clip_at_playhead`] — lets
    /// the timeline context menu grey out "Colar" instead of pasting nothing.
    pub fn has_clipboard_clip(&self) -> bool {
        self.clipboard_clip.is_some()
    }

    /// Copies `selected_clip_id` (and the track kind it's on) to [`OcaApp::clipboard_clip`] —
    /// what `Ctrl+C`/the timeline context menu's "Copiar" do. A no-op if nothing is selected.
    pub fn copy_selected_clip(&mut self) {
        let Some(clip_id) = self.selected_clip_id else {
            return;
        };
        let found = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .find_map(|t| {
                t.clips
                    .iter()
                    .find(|c| c.id == clip_id)
                    .map(|c| (c.clone(), t.kind))
            });
        if let Some(copied) = found {
            self.clipboard_clip = Some(copied);
        }
    }

    /// [`OcaApp::copy_selected_clip`] followed by [`OcaApp::delete_selected_clip`] — what
    /// `Ctrl+X`/the context menu's "Recortar" do.
    pub fn cut_selected_clip(&mut self) {
        self.copy_selected_clip();
        self.delete_selected_clip();
    }

    /// Pastes [`OcaApp::clipboard_clip`] as a new, freshly-id'd clip at the playhead's current
    /// position on the active sequence — what `Ctrl+V`/the context menu's "Colar" do. Lands on
    /// a matching-kind track the same way [`OcaApp::add_asset_to_timeline`] does (first
    /// existing track of that kind, auto-created if none exists); always the playhead, not
    /// wherever the context menu happened to be opened — a known simplification. A no-op if
    /// the clipboard is empty. Works across sequence tabs and even across projects, since
    /// `clipboard_clip` isn't scoped to either.
    pub fn paste_clip_at_playhead(&mut self) {
        let Some((copied, kind)) = self.clipboard_clip.clone() else {
            return;
        };
        let playhead_secs = self.active_project().timeline().playhead_secs;
        let timeline = self.active_project_mut().timeline_mut();
        let track_index = resolve_or_create_track(timeline, kind, None);
        let clip_id = next_clip_id(timeline);
        timeline.tracks[track_index]
            .clips
            .push(avcore::timeline::ClipInstance {
                id: clip_id,
                asset_id: copied.asset_id,
                start_secs: playhead_secs,
                source_in_secs: copied.source_in_secs,
                source_out_secs: copied.source_out_secs,
                // A pasted clip is always standalone, even if the copied original was a
                // composite member — copy/paste doesn't replicate group membership (a known
                // gap short of request.md's "reutilizado ... como se fosse um clipe só").
                composite_id: None,
                gain_db: copied.gain_db,
                frozen: copied.frozen,
                speed_factor: copied.speed_factor,
                crop_x: copied.crop_x,
                crop_y: copied.crop_y,
                crop_w: copied.crop_w,
                crop_h: copied.crop_h,
                mask_shape: copied.mask_shape,
                mask_corner_radius: copied.mask_corner_radius,
                flipped_h: copied.flipped_h,
                color_filter: copied.color_filter,
                vignette_intensity: copied.vignette_intensity,
                brightness: copied.brightness,
                contrast: copied.contrast,
                saturation: copied.saturation,
                sharpen: copied.sharpen,
                chroma_key_enabled: copied.chroma_key_enabled,
                chroma_key_color: copied.chroma_key_color,
                chroma_key_tolerance: copied.chroma_key_tolerance,
                blur_intensity: copied.blur_intensity,
                shake_intensity: copied.shake_intensity,
                glitch_intensity: copied.glitch_intensity,
                pixelize_intensity: copied.pixelize_intensity,
                transition_in: copied.transition_in,
                transition_duration_secs: copied.transition_duration_secs,
                zoom_start: copied.zoom_start,
                zoom_end: copied.zoom_end,
            });
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
        for track in &mut self.active_project_mut().timeline_mut().tracks {
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
            .timeline()
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

        for track in &mut self.active_project_mut().timeline_mut().tracks {
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
        for track in &mut self.active_project_mut().timeline_mut().tracks {
            if track.move_clip(clip_id, new_start_secs) {
                return;
            }
        }
    }

    /// [`OcaApp::move_clip`], but if `clip_id` is a composite block member every other clip
    /// sharing its `composite_id` moves by the same delta, on the same track — what dragging a
    /// composite block's body does, so the whole block reads as one clip per `request.md`'s
    /// Fase 3 "blocos compostos" spec. A no-op if `clip_id` isn't found; falls back to a plain
    /// [`OcaApp::move_clip`] if it isn't a composite member.
    pub fn move_clip_with_group(&mut self, clip_id: u64, new_start_secs: f64) {
        let found = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .find_map(|t| {
                let dragged = t.clips.iter().find(|c| c.id == clip_id)?;
                Some((dragged.start_secs, dragged.composite_id, t))
            });
        let Some((old_start_secs, composite_id, track)) = found else {
            return;
        };
        let Some(group) = composite_id else {
            self.move_clip(clip_id, new_start_secs);
            return;
        };
        let delta = new_start_secs - old_start_secs;
        let updates: Vec<(u64, f64)> = track
            .clips
            .iter()
            .filter(|c| c.composite_id == Some(group))
            .map(|c| (c.id, c.start_secs + delta))
            .collect();
        for (id, start_secs) in updates {
            self.move_clip(id, start_secs);
        }
    }

    /// Moves `clip_id` onto `target_track_id` at `new_start_secs` — what dragging a timeline
    /// clip's body onto a *different* track does, once the editor has confirmed the drop
    /// target's row is a same-kind track. A no-op if the clip or target track aren't found,
    /// the kinds don't match, or `new_start_secs` is negative — see
    /// [`avcore::timeline::Timeline::move_clip_to_track`] for the exact rules.
    pub fn move_clip_to_track(&mut self, clip_id: u64, target_track_id: u64, new_start_secs: f64) {
        self.active_project_mut().timeline_mut().move_clip_to_track(
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
                    info!(
                        asset_id = next_id,
                        path = %asset.source_path.display(),
                        codec = %asset.codec,
                        duration_secs = asset.duration_secs,
                        "asset imported"
                    );
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
                        debug!(
                            asset_id,
                            has_loudness = loudness.is_some(),
                            has_proxy = proxy_path.is_some(),
                            "asset enrichment complete"
                        );
                        asset.loudness = loudness;
                        asset.proxy_path = proxy_path;
                        asset.waveform_peaks = waveform_peaks;
                    }
                }
                ImportEvent::Failed { path, message } => {
                    self.pending_imports = self.pending_imports.saturating_sub(1);
                    error!(path = %path.display(), error = %message, "asset import failed");
                    self.push_toast(format!(
                        "Import failed — {}: {message}",
                        path.file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.display().to_string())
                    ));
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

    /// Appends a new `Queued` job — what "Adicionar exportação" does, given `segments`/
    /// `canvas` already resolved from the active sequence (see
    /// `avcore::resolve_timeline_segments`) so this job renders the timeline as it was at the
    /// moment it entered the queue, not whatever it's edited to later. Picked up by
    /// [`OcaApp::pump_export_queue`] once a worker slot ([`OcaApp::queue_workers`])
    /// frees up.
    pub fn queue_export(
        &mut self,
        title: String,
        segments: Vec<avcore::ClipSegment>,
        canvas: avcore::Canvas,
        target_lufs: f32,
        output_path: String,
    ) {
        let id = self.export_jobs.iter().map(|j| j.id).max().unwrap_or(0) + 1;
        info!(job_id = id, title = %title, output = %output_path, "export job queued");
        self.export_jobs.push(ExportJob {
            id,
            title,
            segments,
            canvas,
            target_lufs,
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
                        info!(job_id, output = %job.output_path, "export job completed");
                        job.status = ExportJobStatus::Done;
                    }
                    self.active_renders.remove(&job_id);
                }
                RenderEvent::Failed { job_id, message } => {
                    if let Some(job) = self.export_jobs.iter_mut().find(|j| j.id == job_id) {
                        error!(job_id, output = %job.output_path, error = %message, "export job failed");
                        job.status = ExportJobStatus::Failed { message };
                    }
                    self.active_renders.remove(&job_id);
                }
                RenderEvent::Cancelled { job_id } => {
                    debug!(job_id, "export job cancelled");
                    self.export_jobs.retain(|j| j.id != job_id);
                    self.active_renders.remove(&job_id);
                }
            }
        }

        if self.active_renders.len() >= self.prefs.export_workers as usize {
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
        let segments = job.segments.clone();
        let canvas = job.canvas;
        let output_path = PathBuf::from(&job.output_path);
        let target_lufs = job.target_lufs;
        job.status = ExportJobStatus::Rendering { percent: 0 };

        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.active_renders.insert(job_id, Arc::clone(&cancel_flag));

        info!(job_id, output = %output_path.display(), "export render worker dispatched");
        let tx = self.render_tx.clone();
        std::thread::spawn(move || {
            let outcome = avcore::render_export_job(
                &segments,
                canvas,
                &output_path,
                target_lufs,
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

    /// Queues a short-lived error message to be shown as a floating overlay at the bottom-right
    /// of the window. Replaces silent `eprintln!` calls for user-facing errors.
    pub fn push_toast(&mut self, message: String) {
        warn!(message = %message, "user-facing error toast");
        self.toasts.push((message, Instant::now()));
    }

    /// Renders any active toasts and evicts ones older than 4 seconds. Called from [`ui`] each
    /// frame; uses `egui::Area` in `Foreground` order so toasts float above all panels.
    fn show_toasts(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        self.toasts
            .retain(|(_, born)| now.duration_since(*born) < Duration::from_secs(4));
        if self.toasts.is_empty() {
            return;
        }
        egui::Area::new(egui::Id::new("toasts"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    for (msg, born) in self.toasts.iter().rev().take(3) {
                        let age = now.duration_since(*born).as_secs_f32();
                        let alpha = if age > 3.0 { 1.0 - (age - 3.0) } else { 1.0 };
                        egui::Frame::new()
                            .fill(theme::ERROR.linear_multiply(alpha))
                            .corner_radius(6)
                            .inner_margin(egui::Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.set_max_width(360.0);
                                ui.label(
                                    egui::RichText::new(msg.as_str())
                                        .color(egui::Color32::WHITE.linear_multiply(alpha)),
                                );
                            });
                        ui.add_space(6.0);
                    }
                });
            });
    }

    /// Shows the preferences modal when `prefs_open` is set, overlaying whatever screen is
    /// currently active. Closing the modal (clicking outside or pressing Escape) clears the
    /// flag. Content is [`crate::screens::prefs::show`] — unchanged from when it was a full
    /// screen, just wrapped in an `egui::Modal` instead.
    fn show_prefs_modal(&mut self, ctx: &egui::Context) {
        if !self.prefs_open {
            return;
        }
        let modal = egui::Modal::new(egui::Id::new("prefs_modal"));
        let response = modal.show(ctx, |ui| {
            ui.set_width(600.0);
            screens::prefs::show(self, ui);
        });
        if response.should_close() {
            self.prefs_open = false;
        }
    }

    /// Writes the active project to a `<name>.autosave.json` recovery file next to the project's
    /// own save file, subject to a 2-second idle debounce and a 30-second forced-save ceiling.
    /// Skips silently if the project has never been saved (no `file_path` yet) or hasn't changed.
    /// Serializes on the calling thread (fast, in-memory) then writes on a background thread so
    /// the UI never blocks on file I/O.
    fn pump_autosave(&mut self) {
        if !self.project_dirty || self.projects.is_empty() {
            return;
        }
        let Some(file_path) = self.active_project().file_path.clone() else {
            return;
        };
        let now = Instant::now();
        let debounce_done = self
            .last_edit_instant
            .map(|t| now.duration_since(t) >= Duration::from_secs(2))
            .unwrap_or(false);
        let ceiling_hit = self
            .last_autosave_instant
            .map(|t| now.duration_since(t) >= Duration::from_secs(30))
            .unwrap_or(false);
        if !debounce_done && !ceiling_hit {
            return;
        }
        let json = match avcore::persistence::to_json(self.active_project()) {
            Ok(s) => s,
            Err(_) => return,
        };
        let autosave_path = file_path.with_extension("autosave.json");
        debug!(path = %autosave_path.display(), "writing autosave");
        self.project_dirty = false;
        self.last_autosave_instant = Some(now);
        std::thread::spawn(move || {
            if let Err(e) = std::fs::write(&autosave_path, json.as_bytes()) {
                error!(path = %autosave_path.display(), error = %e, "autosave write failed");
            }
        });
    }

    /// If a newer autosave was found when the active project was opened ([`autosave_restore_pending`]
    /// is set), shows a modal offering to restore or discard it. Restore replaces the active
    /// project's data in place (keeping its `file_path` and id). Discard deletes the autosave file.
    fn pump_autosave_restore(&mut self, ctx: &egui::Context) {
        let Some(autosave_path) = self.autosave_restore_pending.clone() else {
            return;
        };
        let locale = self.locale;
        let modal = egui::Modal::new(egui::Id::new("autosave_restore"));
        let response = modal.show(ctx, |ui| {
            ui.set_width(360.0);
            ui.label(crate::i18n::Text::AutosaveFound.tr(locale));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button(crate::i18n::Text::AutosaveRestore.tr(locale)).clicked() {
                    if let Ok(mut restored) =
                        avcore::load_project_from_file(&autosave_path)
                    {
                        let file_path = self.active_project().file_path.clone();
                        let id = self.active_project().id;
                        restored.file_path = file_path;
                        restored.id = id;
                        info!(path = %autosave_path.display(), "autosave restored");
                        *self.active_project_mut() = restored;
                        self.project_dirty = false;
                    }
                    self.autosave_restore_pending = None;
                }
                if ui.button(crate::i18n::Text::AutosaveDiscard.tr(locale)).clicked() {
                    info!(path = %autosave_path.display(), "autosave discarded");
                    let _ = std::fs::remove_file(&autosave_path);
                    self.autosave_restore_pending = None;
                }
            });
        });
        if response.should_close() {
            self.autosave_restore_pending = None;
        }
    }

    /// Flags that a project with `file_path` should offer autosave restoration on open, if
    /// `<file_path>.autosave.json` exists and is newer than the project file itself.
    pub fn check_autosave_on_open(&mut self, file_path: &Path) {
        let autosave_path = file_path.with_extension("autosave.json");
        if autosave_is_newer(&autosave_path, file_path) {
            self.autosave_restore_pending = Some(autosave_path);
        }
    }

    /// Serializes `prefs` to the platform config file on a background thread. Called whenever
    /// the preferences modal closes or a project is opened. Prunes `recent_project_paths`
    /// entries whose files no longer exist before serializing.
    pub fn save_prefs(&self) {
        let mut prefs_snapshot = serde_json::to_value(&self.prefs).unwrap_or_default();
        // Always capture the live locale (app.locale may differ from prefs.locale if the user
        // changed it this session without having previously saved).
        if let Ok(locale_val) = serde_json::to_value(self.locale) {
            prefs_snapshot["locale"] = locale_val;
        }
        // Prune stale recents (moved/deleted files) so the list stays clean.
        if let Some(arr) = prefs_snapshot
            .get_mut("recent_project_paths")
            .and_then(|v| v.as_array_mut())
        {
            arr.retain(|v| {
                v.as_str()
                    .map(|p| std::path::Path::new(p).exists())
                    .unwrap_or(false)
            });
        }
        let Ok(json) = serde_json::to_string_pretty(&prefs_snapshot) else {
            return;
        };
        let path = prefs_path();
        std::thread::spawn(move || {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = std::fs::write(&path, json.as_bytes()) {
                tracing::error!(path = %path.display(), error = %e, "failed to write prefs");
            } else {
                tracing::debug!(path = %path.display(), "prefs saved");
            }
        });
    }
}

/// Returns the path of the crash sentinel file. Its presence at startup means the previous
/// session exited uncleanly (crash, kill, power loss). Cleared by [`OcaApp::on_exit`].
fn sentinel_path() -> PathBuf {
    prefs_path()
        .parent()
        .map(|d| d.join("oca.running"))
        .unwrap_or_else(|| PathBuf::from("oca.running"))
}

/// Loads [`PrefsState`] from the platform config file, falling back to the default if the file
/// is absent or cannot be parsed.
pub fn load_prefs() -> PrefsState {
    let path = prefs_path();
    let Ok(bytes) = std::fs::read(&path) else {
        return PrefsState::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

/// Returns the platform-appropriate path for the oca preferences file.
///
/// - macOS:   `~/Library/Application Support/oca/prefs.json`
/// - Windows: `%APPDATA%\oca\prefs.json`
/// - Linux:   `~/.config/oca/prefs.json`
fn prefs_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join("Library/Application Support/oca/prefs.json");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("oca\\prefs.json");
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".config/oca/prefs.json");
        }
    }
    PathBuf::from("prefs.json")
}

/// Returns `true` if `autosave_path` exists and has a modification time strictly newer than
/// `project_path`. Returns `false` if either file's metadata can't be read or the timestamps
/// can't be compared.
fn autosave_is_newer(autosave_path: &Path, project_path: &Path) -> bool {
    let Ok(as_meta) = std::fs::metadata(autosave_path) else {
        return false;
    };
    let Ok(proj_meta) = std::fs::metadata(project_path) else {
        return true;
    };
    let Ok(as_time) = as_meta.modified() else {
        return false;
    };
    let Ok(proj_time) = proj_meta.modified() else {
        return false;
    };
    as_time > proj_time
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
    let preview = avcore::preview::Preview::open(path, None).ok()?;
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
        self.pump_autosave();
        self.pump_autosave_restore(ui.ctx());
        // Detect the prefs modal closing (true → false) and persist the new settings.
        if self.prev_prefs_open && !self.prefs_open {
            self.save_prefs();
        }
        self.prev_prefs_open = self.prefs_open;
        if self.crash_detected {
            self.crash_detected = false;
            tracing::warn!("crash sentinel found — previous session did not exit cleanly");
            self.push_toast(crate::i18n::Text::CrashDetected.tr(self.locale).to_string());
        }
        if self.preview_playing {
            // Smooth video needs every-frame repaints; the 200ms throttle below would show
            // it as a slideshow.
            ui.ctx().request_repaint();
        } else {
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }

        screens::nav_rail::show(self, ui);

        // Must run after `nav_rail::show` (which applies the click that changes `self.screen`)
        // and before `breadcrumb::show` — the breadcrumb reads `active_project()` too (project
        // name + unsaved-changes dot) whenever `screen == Editor`, and it renders before the
        // `editor`/`library` screens' own `ensure_active_project()` call gets a chance to.
        if matches!(self.screen, Screen::Editor | Screen::Library) {
            self.ensure_active_project();
        }

        screens::breadcrumb::show(self, ui);

        egui::CentralPanel::default().show(ui, |ui| match self.screen {
            Screen::Home => screens::home::show(self, ui),
            Screen::Editor => screens::editor::show(self, ui),
            Screen::Library => screens::library::show(self, ui),
            Screen::Queue => screens::queue::show(self, ui),
        });
        self.show_prefs_modal(ui.ctx());
        self.show_toasts(ui.ctx());
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Clean exit — remove the crash sentinel so the next launch doesn't think we crashed.
        let _ = std::fs::remove_file(sentinel_path());
        tracing::info!("clean exit — crash sentinel removed");
    }
}

#[cfg(test)]
#[path = "app/app_test.rs"]
mod tests;
