//! Application state ([`App`]) and the top-level `eframe::App` implementation that
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

mod auto_reframe;
mod background_removal;
mod clip_props;
pub mod export;
mod import;
mod layer_templates;
mod modals;
mod model_download;
mod motion_tracking;
mod preview;
mod sound_library;
mod telemetry;
mod text_to_speech;
mod timeline_ops;
mod transcribe;
mod update_check;

/// Which of the app's five top-level views is currently showing. Drives both the central
/// panel content and which nav-rail button is highlighted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Editor,
    Library,
    SoundLibrary,
    Queue,
}

/// The editor toolbar's active tool (Selecionar / Aparar). Currently just tracked for the
/// toolbar's highlight state — Fase 3 wires it up to actual timeline interactions. Splitting
/// ("Cortar") isn't a persistent mode like these two — it's a one-shot action, performed
/// directly by [`App::split_at_playhead`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTool {
    Select,
    Trim,
}

/// A key + modifier combination that can be assigned to a bindable action. `key_name` is
/// the value returned by [`egui::Key::name`] (e.g. `"Space"`, `"B"`) and accepted by
/// [`egui::Key::from_name`], making it stable across egui versions for common keys.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KeyCombo {
    pub ctrl: bool,
    pub shift: bool,
    pub key_name: String,
}

impl KeyCombo {
    /// Returns `true` if this combo was just pressed on the current frame.
    pub fn matches(&self, i: &egui::InputState) -> bool {
        let Some(key) = egui::Key::from_name(&self.key_name) else {
            return false;
        };
        i.modifiers.ctrl == self.ctrl && i.modifiers.shift == self.shift && i.key_pressed(key)
    }

    /// Human-readable label, e.g. `"Ctrl+Shift+C"` or `"Space"`.
    pub fn display(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl+");
        }
        if self.shift {
            s.push_str("Shift+");
        }
        s.push_str(&self.key_name);
        s
    }
}

/// The four editor actions whose bindings the user can change in Preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindableAction {
    PlayPause,
    SplitAtPlayhead,
    CopyFormatting,
    PasteFormatting,
}

/// User-configurable key bindings for the four main editor shortcuts. Persisted as part of
/// [`PrefsState`] so changes survive restarts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyBindings {
    pub play_pause: KeyCombo,
    pub split_at_playhead: KeyCombo,
    pub copy_formatting: KeyCombo,
    pub paste_formatting: KeyCombo,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            play_pause: KeyCombo {
                ctrl: false,
                shift: false,
                key_name: "Space".to_string(),
            },
            split_at_playhead: KeyCombo {
                ctrl: true,
                shift: false,
                key_name: "B".to_string(),
            },
            copy_formatting: KeyCombo {
                ctrl: true,
                shift: true,
                key_name: "C".to_string(),
            },
            paste_formatting: KeyCombo {
                ctrl: true,
                shift: true,
                key_name: "V".to_string(),
            },
        }
    }
}

/// User-configurable settings shown on the Ajustes screen. Persisted to `prefs.oc` (the same
/// binary framing as a `.ocproj` project) in the platform config dir — see [`App::save_prefs`] /
/// [`load_prefs`].
#[derive(Clone, serde::Serialize, serde::Deserialize)]
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
    /// Key bindings for the four configurable editor shortcuts.
    #[serde(default)]
    pub key_bindings: KeyBindings,
    /// Preferred video encoder for exports — hardware-accelerated (NVENC/Quick Sync/AMF) with
    /// an automatic CPU (libopenh264) fallback, or a specific choice. See
    /// [`avcore::GpuEncoderPreference`].
    #[serde(default)]
    pub gpu_encoder: avcore::GpuEncoderPreference,
    /// Path to a local GGML Whisper model file (e.g. `ggml-base.bin`) for automatic
    /// transcription ([`App::spawn_transcribe`]). Empty when not configured — the model isn't
    /// bundled with the app (see `avcore::transcribe`'s module docs), so this is a one-time
    /// manual setup step in Preferences.
    #[serde(default)]
    pub whisper_model_path: String,
    /// Path to a local UltraFace ONNX model file for auto-reframe
    /// ([`App::spawn_auto_reframe_selected_clip`]). Empty when not configured — same one-time
    /// manual setup step as `whisper_model_path` above (see `avcore::auto_reframe`'s module
    /// docs for why it isn't bundled yet).
    #[serde(default)]
    pub reframe_model_path: String,
    /// Path to a local MODNet ONNX model file for AI background removal
    /// (`avcore::background_removal::segment_person`). Empty when not configured — same
    /// one-time manual setup step as `whisper_model_path`/`reframe_model_path` above (see
    /// `avcore::background_removal`'s module docs for why it isn't bundled yet).
    #[serde(default)]
    pub background_removal_model_path: String,
    /// Path to a local Piper voice `.onnx` model file for text-to-speech
    /// (`avcore::text_to_speech::synthesize`). Its `.onnx.json` config sidecar is expected right
    /// next to it (`<this path>.json`), same layout `avcore::download_tts_voice` downloads into.
    /// Empty when not configured — same one-time manual setup step as the other model paths
    /// above (see `avcore::text_to_speech`'s module docs for why it isn't bundled yet).
    #[serde(default)]
    pub tts_model_path: String,
    /// Saved layer-group templates (`request.md`'s Fase 4 "Templates de grupo de camadas") —
    /// app-wide, not per-project, since the whole point is reapplying the same layer group
    /// (position/scale/crop/effects per layer) to fresh footage across different shorts.
    #[serde(default)]
    pub saved_layer_templates: Vec<avcore::timeline::LayerTemplate>,
    /// Folder scanned for the Music & SFX screen's local catalog (see
    /// [`avcore::sound_library::scan_library_dir`]) — expects a `music/` and/or `sfx/`
    /// subfolder inside it. Empty when not configured yet; no bundled tracks ship with the app
    /// (see `avcore::sound_library`'s module docs), so this is a one-time manual setup step,
    /// the same shape as `whisper_model_path` above.
    #[serde(default)]
    pub sound_library_path: String,
    /// Editing-proxy/preview resolution (`request.md`'s Fase 7 "Qualidade do preview
    /// selecionável"), applied by [`App::spawn_import`] to every proxy generated from then on
    /// — see [`avcore::PreviewQuality`]. Changing it doesn't retroactively regenerate proxies
    /// for assets already imported; only newly imported files pick up the new setting.
    #[serde(default)]
    pub preview_quality: avcore::PreviewQuality,
    /// Whether local runtime telemetry (`request.md`'s Fase 7 "Telemetria de runtime" — import/
    /// export duration, sampled preview frame time, error events) is recorded to
    /// `telemetry.jsonl`. Stays on-device either way — this only controls whether it's
    /// collected at all. On by default, matching `request.md`'s "fica no dispositivo por
    /// padrão" framing (an on-device log, not an opt-in analytics pipeline), but user-visible
    /// and toggleable in Preferences either way.
    #[serde(default = "default_telemetry_enabled")]
    pub telemetry_enabled: bool,
    /// Persisted Editor panel layout — `request.md`'s Fase 3 "Painéis de UI redimensionáveis"
    /// explicitly asks for this to survive restarts ("layout salvo por projeto ou por
    /// usuário"); this is the per-user half, the simpler of the two to wire up since it reuses
    /// `PrefsState`'s existing save/load machinery rather than touching the `.ocproj` project
    /// format. `App::lib_panel_width`/`props_panel_width`/`timeline_height` are the live,
    /// actively-dragged values during a session; `App::save_prefs` copies them in here at save
    /// time (same technique it already uses for `locale`), and `App::new` seeds the live
    /// fields from these at startup.
    #[serde(default = "default_lib_panel_width")]
    pub lib_panel_width: f32,
    #[serde(default = "default_props_panel_width")]
    pub props_panel_width: f32,
    #[serde(default = "default_timeline_height")]
    pub timeline_height: f32,
}

fn default_telemetry_enabled() -> bool {
    true
}

fn default_lib_panel_width() -> f32 {
    220.0
}

fn default_props_panel_width() -> f32 {
    240.0
}

fn default_timeline_height() -> f32 {
    190.0
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
            key_bindings: KeyBindings::default(),
            gpu_encoder: avcore::GpuEncoderPreference::default(),
            whisper_model_path: String::new(),
            reframe_model_path: String::new(),
            background_removal_model_path: String::new(),
            tts_model_path: String::new(),
            saved_layer_templates: Vec::new(),
            sound_library_path: String::new(),
            preview_quality: avcore::PreviewQuality::default(),
            telemetry_enabled: true,
            lib_panel_width: default_lib_panel_width(),
            props_panel_width: default_props_panel_width(),
            timeline_height: default_timeline_height(),
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
/// por bloco") — [`App::set_selected_clip_gain`] clamps to this range.
pub const GAIN_DB_RANGE: std::ops::RangeInclusive<f32> = -24.0..=24.0;

/// Slider bounds for the properties panel's per-block speed control (Fase 4's "Velocidade") —
/// [`App::set_selected_clip_speed`] clamps to this range.
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

/// Slider bounds for the properties panel's scale-keyframe controls (Fase 4's "Keyframes",
/// [`avcore::timeline::ClipInstance::scale_keyframes`]).
pub const SCALE_RANGE: std::ops::RangeInclusive<f32> = 1.0..=3.0;

/// Minimum gap between sampled `PreviewFrameTime` telemetry events (Fase 7's "Telemetria de
/// runtime") — recording every single preview frame would flood `telemetry.jsonl` for no
/// analytical benefit over a periodic sample.
pub const PREVIEW_FRAME_TELEMETRY_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

/// Slider bounds for the properties panel's layer-resize controls
/// ([`avcore::timeline::ClipInstance::layer_scale_x`]/`_y`) — unlike [`SCALE_RANGE`]'s
/// Ken-Burns zoom (which only ever enlarges), a layer's on-canvas footprint can shrink well
/// below native size (e.g. a small webcam corner) as well as grow.
pub const LAYER_SCALE_RANGE: std::ops::RangeInclusive<f32> = 0.1..=3.0;

/// Slider bounds for the properties panel's video-stabilization control
/// ([`avcore::timeline::ClipInstance::stabilization_intensity`]).
pub const STABILIZATION_INTENSITY_RANGE: std::ops::RangeInclusive<f32> = 0.0..=1.0;

/// Slider bounds for the properties panel's motion-tracking region width/height controls
/// (`App::motion_track_width`/`_height`, `avcore::track_region`'s `template_width_frac`/
/// `template_height_frac`) — kept well under `1.0` so the template can always slide within the
/// frame during search, and above a few percent so it still covers enough texture to match
/// against. Shared by both the width and height sliders.
pub const MOTION_TRACK_SIZE_RANGE: std::ops::RangeInclusive<f32> = 0.05..=0.6;

/// Slider bounds for the properties panel's motion-tracking search-radius control
/// (`App::motion_track_search_radius`, `avcore::track_region`'s `search_radius_frac`).
pub const MOTION_TRACK_SEARCH_RADIUS_RANGE: std::ops::RangeInclusive<f32> = 0.02..=0.3;

/// A message from a background render worker thread (see [`App::pump_export_queue`])
/// back to the UI thread, sent over a plain `tokio::sync::mpsc` channel used purely
/// synchronously (`try_recv` on the UI side, `send` on the worker side) — no async runtime
/// needed, matching the execution plan's "tokio + canais assíncronos" without pulling egui's
/// synchronous frame loop into async code.
enum RenderEvent {
    Progress {
        job_id: u64,
        percent: u8,
    },
    Done {
        job_id: u64,
        duration_ms: u64,
        output_duration_secs: f64,
    },
    Failed {
        job_id: u64,
        message: String,
        duration_ms: u64,
        output_duration_secs: f64,
    },
    Cancelled {
        job_id: u64,
    },
}

/// A message from a background import worker thread (see [`App::spawn_import`]) back to
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
        duration_ms: u64,
    },
    Failed {
        path: PathBuf,
        message: String,
    },
}

/// A message from a background transcription worker thread (see [`App::spawn_transcribe`])
/// back to the UI thread.
enum TranscribeEvent {
    Done {
        asset_id: u64,
        segments: Vec<avcore::TranscribeSegment>,
    },
    Failed {
        asset_id: u64,
        message: String,
    },
}

/// Which model a [`ModelDownloadEvent::Done`] belongs to — [`App::pump_model_download`] routes
/// the finished path to the matching `prefs` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModelKind {
    Whisper,
    Reframe,
    BackgroundRemoval,
    Tts,
}

/// A message from a background auto-reframe worker thread (see
/// [`App::spawn_auto_reframe_selected_clip`]) back to the UI thread.
enum AutoReframeEvent {
    Done {
        clip_id: u64,
        crop: avcore::CropRect,
        /// Whether a face was actually detected — `false` means [`avcore::compute_reframe_crop`]
        /// fell back to a plain center crop, worth telling the user about.
        subject_found: bool,
    },
    Failed {
        message: String,
    },
}

/// A message from a background motion-tracking worker thread (see
/// [`App::spawn_motion_track_selected_clip`]) back to the UI thread.
enum MotionTrackEvent {
    Done {
        clip_id: u64,
        keyframes: Vec<avcore::Keyframe<avcore::Position>>,
    },
}

/// A message from a background AI-background-removal matte-generation worker thread (see
/// [`App::spawn_generate_matte_for_selected_clip`]) back to the UI thread.
enum MatteGenerationEvent {
    Done { clip_id: u64, mask_path: PathBuf },
    Failed { message: String },
}

/// A message from the background update-check thread (see [`App::spawn_update_check`]) back to
/// the UI thread. Only sent when a newer version actually exists — a check that fails outright
/// or finds nothing newer sends nothing at all, since there's no user-facing state change
/// either way.
enum UpdateCheckEvent {
    NewerVersionAvailable { version: String, html_url: String },
}

/// A GitHub release newer than the running build, surfaced by [`App::pump_update_check`] as
/// [`App::available_update`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub version: String,
    pub html_url: String,
}

/// A message from a background text-to-speech worker thread (see
/// [`App::spawn_generate_tts`]) back to the UI thread.
enum TtsEvent {
    Done { wav_path: PathBuf },
    Failed { message: String },
}

/// A message from a background model-download worker thread (see
/// [`App::spawn_download_whisper_model`]/[`App::spawn_download_reframe_model`]) back to the UI
/// thread. Only one download can run at a time ([`App::cancel_model_download`] gates that), so
/// a single shared channel/progress state covers every downloadable model.
enum ModelDownloadEvent {
    Progress { downloaded: u64, total: u64 },
    Done { kind: ModelKind, path: PathBuf },
    Cancelled,
    Failed { message: String },
}

/// A poster frame extracted on a background thread (see [`App::request_thumbnail`]),
/// ready to upload as an egui texture. No failure variant — a (asset, bucket) that couldn't be
/// extracted just never gets a texture; [`App::requested_thumbnails`] already stops it from
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
/// and calls [`App::ui`](eframe::App::ui) on it every frame.
pub struct App {
    pub screen: Screen,
    pub tool: EditorTool,
    pub locale: Locale,
    pub projects: Vec<Project>,
    pub active_project: usize,
    pub selected_asset_id: Option<u64>,
    pub export_jobs: Vec<ExportJob>,
    pub prefs: PrefsState,
    /// Sends [`avcore::TelemetryEvent`]s to the dedicated background writer thread spawned in
    /// [`App::new`] — see [`App::record_telemetry`]/`telemetry::spawn_telemetry_writer`. No
    /// paired receiver is kept on `App`; that thread owns the only one.
    telemetry_tx: UnboundedSender<avcore::TelemetryEvent>,
    /// Wall-clock time [`App::record_telemetry`] last recorded a `PreviewFrameTime` sample —
    /// throttles sampling to roughly once every [`PREVIEW_FRAME_TELEMETRY_INTERVAL`] rather
    /// than every single frame, which would flood `telemetry.jsonl`.
    last_preview_frame_telemetry: Option<std::time::Instant>,
    render_tx: UnboundedSender<RenderEvent>,
    render_rx: UnboundedReceiver<RenderEvent>,
    /// Cancellation flags for jobs a worker thread is currently rendering, keyed by job id.
    /// A job id present here is the source of truth for "how many workers are busy right
    /// now" — [`App::pump_export_queue`] uses its length against `prefs.export_workers`.
    active_renders: HashMap<u64, Arc<AtomicBool>>,
    /// The GStreamer pipeline for the clip currently covering the active sequence's timeline
    /// playhead, if it could be opened (`None` before any project has a clip at the playhead,
    /// before it's been lazily opened, and when `Preview::open` failed, e.g. a source file
    /// that's since been moved or deleted — see [`App::ensure_preview_loaded`]).
    preview: Option<avcore::preview::Preview>,
    /// The clip id [`App::ensure_preview_loaded`] last attempted to open a pipeline for,
    /// whether or not it succeeded — lets it tell "already tried and failed for this exact
    /// clip, don't retry every frame" apart from "the playhead moved onto a different clip, try
    /// again".
    preview_clip_id: Option<u64>,
    /// Uploaded from the latest [`avcore::preview::Preview::current_frame`] each frame the
    /// Editor screen is shown; `None` until the first frame decodes. Reset whenever
    /// [`App::ensure_preview_loaded`] reopens the pipeline for a different clip so a stale
    /// frame from the previous one never lingers.
    pub preview_texture: Option<egui::TextureHandle>,
    /// Whether the preview pipeline is in `Playing` state. `Preview` has no state getter of
    /// its own, so the Editor's play/pause button and [`App::pump_export_queue`]'s repaint
    /// cadence both rely on this instead.
    pub preview_playing: bool,
    /// Wall-clock playback start `(Instant, timeline playhead at that instant)` for a frozen
    /// clip — set whenever playback begins while the clip covering the playhead has
    /// `ClipInstance::frozen` set. A frozen clip's pipeline is kept `Paused` at
    /// `source_in_secs` (so it always shows the held anchor frame) rather than actually
    /// playing, so [`App::pump_preview_frame`] has no `Preview::position_secs` to derive
    /// the advancing playhead from the way it does for a normal clip — this stands in for it.
    /// `None` when nothing is playing or the current clip isn't frozen.
    preview_frozen_since: Option<(std::time::Instant, f64)>,
    import_tx: UnboundedSender<ImportEvent>,
    import_rx: UnboundedReceiver<ImportEvent>,
    /// How many files a call to [`App::spawn_import`] haven't been probed yet, in the
    /// background. The Mídia screen shows a busy note while this is nonzero. Reaches zero as
    /// soon as each file's cheap probe comes back and it's added to the library — loudness
    /// measurement and proxy generation keep running after that in the background (see
    /// [`App::pending_enrichment`]) without holding this counter up, since the asset is
    /// already usable by then.
    pub pending_imports: usize,
    /// The next id to hand out in [`App::spawn_import`], one per file in the batch —
    /// correlates a file's `ImportEvent::AssetReady` with its later `ImportEvent::Enriched`
    /// once [`App::pump_import_queue`] knows the asset's real (project-assigned) id.
    next_import_token: u64,
    /// Import tokens awaiting their `ImportEvent::Enriched` (loudness + proxy), mapped to the
    /// asset id they were assigned when their `AssetReady` landed — removed once the
    /// enrichment arrives and gets applied, or left dangling harmlessly if the asset is gone
    /// by then (media library has no delete yet, so that can't currently happen).
    pending_enrichment: HashMap<u64, u64>,
    /// Import tokens whose asset should be appended to the timeline the moment its
    /// `ImportEvent::AssetReady` lands (see [`App::pump_import_queue`]) — used by
    /// [`App::add_sound_library_track_to_timeline`]'s one-click "add to timeline" for a Music &
    /// SFX track that hasn't been imported into the active project yet.
    auto_add_to_timeline: HashSet<u64>,
    /// Tracks found by the last scan of `prefs.sound_library_path` (see
    /// [`App::rescan_sound_library`]) — not persisted, recomputed from disk whenever the Music
    /// & SFX screen is opened or the configured folder changes.
    pub sound_library_tracks: Vec<avcore::sound_library::LibraryTrack>,
    sound_library_tx: UnboundedSender<Vec<avcore::sound_library::LibraryTrack>>,
    sound_library_rx: UnboundedReceiver<Vec<avcore::sound_library::LibraryTrack>>,
    transcribe_tx: UnboundedSender<TranscribeEvent>,
    transcribe_rx: UnboundedReceiver<TranscribeEvent>,
    /// The media asset id a background transcription is currently running for, if any — only
    /// one transcription runs at a time (unlike imports, which are per-file parallel). The
    /// Mídia screen shows a busy state on that asset's card while this is `Some`.
    pub transcribing_asset_id: Option<u64>,
    auto_reframe_tx: UnboundedSender<AutoReframeEvent>,
    auto_reframe_rx: UnboundedReceiver<AutoReframeEvent>,
    /// The timeline clip id a background auto-reframe run is currently computing a crop for, if
    /// any — only one runs at a time, same shape as `transcribing_asset_id`.
    pub auto_reframing_clip_id: Option<u64>,
    motion_tracking_tx: UnboundedSender<MotionTrackEvent>,
    motion_tracking_rx: UnboundedReceiver<MotionTrackEvent>,
    /// The timeline clip id a background motion-tracking run is currently tracking, if any —
    /// only one runs at a time, same shape as `auto_reframing_clip_id`.
    pub motion_tracking_clip_id: Option<u64>,
    /// The tracked region's center, as a `0.0..=1.0` fraction of the *source* frame (same
    /// convention as `avcore::track_region`'s `initial_center_x_frac`/`_y`, not canvas/layer
    /// space) — user-editable via the properties panel's region controls next to the "Rastrear
    /// movimento" button, instead of the button always defaulting to a centered region. Plain
    /// UI/session state, not persisted to the project: it's a one-shot tracking job's input, not
    /// a durable clip property (unlike, say, `ClipInstance::crop_x/y`) — nothing else reads it
    /// once a run finishes, only its *output* (`position_keyframes`) is saved.
    pub motion_track_center_x: f32,
    pub motion_track_center_y: f32,
    /// The tracked block's width/height, each independently as a fraction of the frame's
    /// shorter dimension — `avcore::track_region`'s `template_width_frac`/`template_height_frac`.
    /// Same non-persistence rationale as `motion_track_center_x`/`_y`.
    pub motion_track_width: f32,
    pub motion_track_height: f32,
    /// How far the tracked block is allowed to move between consecutive sampled frames, as a
    /// fraction of the frame's shorter dimension — `avcore::track_region`'s
    /// `search_radius_frac`. Same non-persistence rationale as `motion_track_center_x`/`_y`.
    pub motion_track_search_radius: f32,
    matte_generation_tx: UnboundedSender<MatteGenerationEvent>,
    matte_generation_rx: UnboundedReceiver<MatteGenerationEvent>,
    /// The timeline clip id a background AI-background-removal matte-generation run is
    /// currently computing a matte for, if any — only one runs at a time, same shape as
    /// `auto_reframing_clip_id`.
    pub matte_generating_clip_id: Option<u64>,
    tts_tx: UnboundedSender<TtsEvent>,
    tts_rx: UnboundedReceiver<TtsEvent>,
    /// `Some(text)` while the "Texto-pra-fala" modal is open — the text buffer being edited.
    /// `None` when the modal is closed.
    pub tts_modal_text: Option<String>,
    /// `true` while a background TTS synthesis run is in flight — only one at a time, same
    /// shape as `transcribing_asset_id`.
    pub tts_generating: bool,
    model_download_tx: UnboundedSender<ModelDownloadEvent>,
    model_download_rx: UnboundedReceiver<ModelDownloadEvent>,
    /// `Some((downloaded_bytes, total_bytes))` while a model download is running —
    /// `total_bytes` is `0` if the server didn't report a `Content-Length` yet. `None` when no
    /// download is in flight. Preferences shows a progress bar in place of the size picker
    /// while this is `Some`.
    pub model_download_progress: Option<(u64, u64)>,
    /// Which model `model_download_progress` belongs to — since only one download runs at a
    /// time, Preferences uses this to show the progress bar in the right section (Whisper's or
    /// auto-reframe's) instead of both.
    pub(crate) model_download_kind: Option<ModelKind>,
    cancel_model_download: Option<Arc<AtomicBool>>,
    /// The timeline clip currently highlighted in the Editor's timeline strip, if any — a
    /// separate concept from `selected_asset_id` (that's the media-library selection driving
    /// the preview panel; this is a placed [`avcore::timeline::ClipInstance`]). `Delete`
    /// removes whichever clip this points at.
    pub selected_clip_id: Option<u64>,
    /// The text overlay clip currently selected on a text track, if any. Selecting a text clip
    /// clears `selected_clip_id`/`selected_shape_clip_id` and vice versa — only one kind of clip
    /// can be selected at a time. The properties panel shows text-clip controls when this is
    /// `Some`.
    pub selected_text_clip_id: Option<u64>,
    /// The shape overlay clip currently selected on a shape track, if any. Same mutual-exclusion
    /// rule as `selected_text_clip_id`. The properties panel shows shape-clip controls when this
    /// is `Some`.
    pub selected_shape_clip_id: Option<u64>,
    /// Canvas-fraction points clicked so far while drawing a custom shape (`request.md`'s Fase
    /// 4 "forma personalizada"), or `None` when not in drawing mode. Same "pending one-shot
    /// mode" shape as `binding_capture`/`renaming_sequence` below: `Some(vec![])` on
    /// [`App::start_drawing_custom_shape`], grows via [`App::push_drawing_shape_point`],
    /// committed into a new [`avcore::timeline::ShapeClip`] by
    /// [`App::finish_drawing_custom_shape`] (Enter, needs >= 3 points) or discarded by
    /// [`App::cancel_drawing_custom_shape`] (Escape). Read each frame by
    /// `screens::editor::layer_transform_preview`.
    pub drawing_shape_points: Option<Vec<(f32, f32)>>,
    /// Horizontal scale of the timeline strip and its ruler, in pixels per second. Adjusted by
    /// `Ctrl` + scroll over the timeline (per `request.md`'s Fase 3 spec) — more zoom for
    /// frame-accurate edits, less to see the whole project at once.
    pub timeline_px_per_sec: f32,
    /// Width, in points, of the Editor's media-library column — dragged via the divider
    /// between it and the preview column (`editor.rs::resizable_divider`). Clamped to the
    /// window's current size every frame (`editor.rs::show`). Seeded from
    /// `PrefsState::lib_panel_width` at startup and captured back into it by
    /// `App::prefs_snapshot` on save — the per-user half of `request.md`'s "layout salvo por
    /// projeto ou por usuário" (per-project persistence isn't wired up).
    pub lib_panel_width: f32,
    /// Same idea as `lib_panel_width`, for the clip-properties column on the right.
    pub props_panel_width: f32,
    /// Height, in points, of the timeline strip — dragged via the horizontal divider above it.
    /// Same persistence shape as `lib_panel_width`.
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
    /// not — stops [`App::request_thumbnail`] from spawning a new extraction thread every
    /// frame for a bucket that's still pending, or that already failed once.
    requested_thumbnails: HashSet<(u64, i64)>,
    /// Set by the media library panel on the frame a dragged asset is released (screen-space
    /// pointer position), consumed by the timeline panel later in the same frame to place it —
    /// how dragging an asset out of the library and dropping it on the timeline works. Always
    /// `None` between frames.
    pub pending_asset_drop: Option<(u64, egui::Pos2)>,
    /// The last clip(s) copied or cut via `Ctrl+C`/`Ctrl+X`/the timeline context menu, and the
    /// track kind they came from (so a paste lands on a matching-kind track — same rule as a
    /// drag-move). More than one clip only when the copied clip was a composite block member —
    /// every clip sharing its `composite_id` is captured too, so `App::paste_clip_at_playhead`
    /// can paste the whole block back as one unit. Not scoped to a project or sequence: pasting
    /// into a different tab, or even a different project, is what makes "copiar e colar entre
    /// abas" (`request.md`'s Fase 3 spec) work for free, rather than needing separate cross-tab
    /// plumbing.
    clipboard_clip: Option<(
        Vec<avcore::timeline::ClipInstance>,
        avcore::timeline::TrackKind,
    )>,
    /// The last formatting (gain/freeze settings, not the clip itself) copied via
    /// `Ctrl+Shift+C`/the timeline context menu — [`App::paste_selected_clip_formatting`]
    /// applies it onto a different block, per `request.md`'s Fase 4 "copiar formatação" spec.
    formatting_clipboard: Option<ClipFormatting>,
    /// Clip ids picked (via `Ctrl+click`) as candidates for [`App::merge_into_composite`] —
    /// separate from `selected_clip_id`, which stays single-target for every other clip
    /// operation (trim, delete, copy, split). Cleared after a successful merge; not otherwise
    /// tied to `selected_clip_id`, so the last-clicked clip can be a multi-select member
    /// without also being "the" selection.
    pub multi_selected_clip_ids: HashSet<u64>,
    /// Set whenever [`App::active_project_mut`] is called; cleared after each autosave
    /// write. Guards [`App::pump_autosave`] from writing unchanged state to disk.
    project_dirty: bool,
    /// Timestamp of the most recent call to [`App::active_project_mut`] — the debounce
    /// start for the 2-second idle window in [`App::pump_autosave`].
    last_edit_instant: Option<Instant>,
    /// Timestamp of the last successful autosave write — used to enforce the 30-second
    /// ceiling that forces a save even during continuous editing.
    last_autosave_instant: Option<Instant>,
    /// Short-lived error messages shown as floating overlays at the bottom-right of the window.
    /// Each entry is `(message, born_at)`; [`App::show_toasts`] removes entries older than
    /// 4 seconds each frame. Use [`App::push_toast`] to add one.
    toasts: Vec<(String, Instant)>,
    /// Whether the preferences modal is currently open — toggled by the nav rail's ⚙ button.
    /// Kept separate from `screen` so the modal overlays whatever screen is currently active
    /// rather than replacing it with a dedicated route.
    pub prefs_open: bool,
    /// The value of `prefs_open` on the previous frame — lets [`App::ui`] detect the
    /// closing edge (true → false) and trigger a prefs save exactly once.
    prev_prefs_open: bool,
    /// Set to the autosave file path when opening a project that has a newer autosave on disk.
    /// [`App::pump_autosave_restore`] consumes it to show the restore/discard modal.
    autosave_restore_pending: Option<PathBuf>,
    /// `true` when a crash sentinel from a previous session was found at startup — consumed
    /// by [`App::ui`] to show a one-time toast, then cleared.
    crash_detected: bool,
    /// Target aspect ratio selected in the export queue's "Add Export" row. Defaults to
    /// `Original` (source dimensions). Persists between export invocations so the user doesn't
    /// have to re-select it every time.
    pub export_aspect_ratio: avcore::ExportAspectRatio,
    /// When `Some((index, name_buf, summary_buf))`, a project-settings modal is shown for
    /// `projects[index]` with editable name and summary fields. Committed on confirm, discarded
    /// on Escape/cancel.
    pub renaming_project: Option<(usize, String, String)>,
    /// When `Some((seq_index, buf))`, a rename modal is shown for the active project's
    /// `sequences[seq_index]`. Committed on Enter/confirm, discarded on Escape/cancel.
    pub renaming_sequence: Option<(usize, String)>,
    /// A snapshot of `multi_selected_clip_ids`' per-layer `(TrackKind, ClipFormatting)`, plus a
    /// name buffer, staged while the "save as template" naming modal is open — captured at
    /// click time (`App::begin_save_layer_template`) so a selection change while the modal is
    /// open can't retroactively change what gets saved. `None` when the modal is closed.
    pub saving_layer_template: Option<(
        Vec<(avcore::timeline::TrackKind, avcore::ClipFormatting)>,
        String,
    )>,
    /// `Some((template_index, layer_asset_ids))` while the "apply template" modal is open —
    /// `layer_asset_ids[i]` is the media-library asset id chosen for
    /// `prefs.saved_layer_templates[template_index].layers[i]`, `None` until the user picks one
    /// from that layer's dropdown. `None` when the modal is closed.
    pub applying_layer_template: Option<(usize, Vec<Option<u64>>)>,
    /// Whether the toolbar's "Templates" list popup (pick one to apply, or delete it) is open.
    pub layer_templates_menu_open: bool,
    /// When `Some(action)`, the prefs modal is waiting for the next key press to set that
    /// action's binding. Pressing Escape clears it without changing the binding.
    pub binding_capture: Option<BindableAction>,
    update_check_tx: UnboundedSender<UpdateCheckEvent>,
    update_check_rx: UnboundedReceiver<UpdateCheckEvent>,
    /// Set once [`App::spawn_update_check`]'s background check finds a GitHub release newer
    /// than `CARGO_PKG_VERSION` — `None` otherwise, including while the check is still in
    /// flight or failed outright (offline, no releases published yet). The Home screen shows a
    /// small banner linking to `html_url` when this is `Some`. Fase 8's "Versão e auto-update"
    /// scoped down to check-and-notify — see `avcore::update_check`'s module doc comment for
    /// why downloading/applying the update itself isn't covered.
    pub available_update: Option<AvailableUpdate>,
    /// Set when "Adicionar exportação" picked an output path that already exists — holds
    /// everything needed to queue the export once the user resolves the conflict via
    /// [`App::show_export_conflict_modal`] (Overwrite / Rename / Cancel).
    pub pending_export_conflict: Option<export::PendingExportConflict>,
}

impl App {
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
        // Captured before `prefs` itself is moved into the struct literal below.
        let lib_panel_width = prefs.lib_panel_width;
        let props_panel_width = prefs.props_panel_width;
        let timeline_height = prefs.timeline_height;
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
        let (transcribe_tx, transcribe_rx) = mpsc::unbounded_channel();
        let (auto_reframe_tx, auto_reframe_rx) = mpsc::unbounded_channel();
        let (motion_tracking_tx, motion_tracking_rx) = mpsc::unbounded_channel();
        let (matte_generation_tx, matte_generation_rx) = mpsc::unbounded_channel();
        let (tts_tx, tts_rx) = mpsc::unbounded_channel();
        let (model_download_tx, model_download_rx) = mpsc::unbounded_channel();
        let (sound_library_tx, sound_library_rx) = mpsc::unbounded_channel();
        let (telemetry_tx, telemetry_rx) = mpsc::unbounded_channel();
        telemetry::spawn_telemetry_writer(telemetry_rx, telemetry::telemetry_path());
        let (update_check_tx, update_check_rx) = mpsc::unbounded_channel();
        let mut app = Self {
            screen: Screen::Home,
            tool: EditorTool::Select,
            locale: prefs.locale,
            projects,
            active_project: 0,
            selected_asset_id: None,
            export_jobs: export::load_queue(),
            prefs,
            telemetry_tx,
            last_preview_frame_telemetry: None,
            render_tx,
            render_rx,
            active_renders: HashMap::new(),
            preview: None,
            preview_clip_id: None,
            preview_texture: None,
            preview_playing: false,
            preview_frozen_since: None,
            import_tx,
            import_rx,
            pending_imports: 0,
            next_import_token: 0,
            pending_enrichment: HashMap::new(),
            auto_add_to_timeline: HashSet::new(),
            sound_library_tracks: Vec::new(),
            sound_library_tx,
            sound_library_rx,
            transcribe_tx,
            transcribe_rx,
            transcribing_asset_id: None,
            auto_reframe_tx,
            auto_reframe_rx,
            auto_reframing_clip_id: None,
            motion_tracking_tx,
            motion_tracking_rx,
            motion_tracking_clip_id: None,
            motion_track_center_x: 0.5,
            motion_track_center_y: 0.5,
            motion_track_width: 0.2,
            motion_track_height: 0.2,
            motion_track_search_radius: 0.08,
            matte_generation_tx,
            matte_generation_rx,
            matte_generating_clip_id: None,
            tts_tx,
            tts_rx,
            tts_modal_text: None,
            tts_generating: false,
            model_download_tx,
            model_download_rx,
            model_download_progress: None,
            model_download_kind: None,
            cancel_model_download: None,
            selected_clip_id: None,
            selected_text_clip_id: None,
            selected_shape_clip_id: None,
            drawing_shape_points: None,
            timeline_px_per_sec: 4.0,
            lib_panel_width,
            props_panel_width,
            timeline_height,
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
            pending_export_conflict: None,
            renaming_project: None,
            renaming_sequence: None,
            saving_layer_template: None,
            applying_layer_template: None,
            layer_templates_menu_open: false,
            binding_capture: None,
            update_check_tx,
            update_check_rx,
            available_update: None,
        };
        if !app.prefs.sound_library_path.is_empty() {
            app.rescan_sound_library();
        }
        app.spawn_update_check();
        app
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
    /// autosave dirty flag so [`App::pump_autosave`] knows to write the recovery file.
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
    /// library panel does, and what [`App::open_project`] uses to select the newly-opened
    /// project's first asset. `None` clears the selection (empty media library). Does *not*
    /// itself open a pipeline for the new selection — [`App::ensure_preview_loaded`] does
    /// that lazily, the next time the Editor's preview panel actually draws, so switching
    /// projects or asking for a project at startup never pays GStreamer's open cost for an
    /// asset nobody's looking at yet.
    pub fn select_asset(&mut self, id: Option<u64>) {
        self.selected_asset_id = id;
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

    /// Builds the snapshot [`App::save_prefs`]/[`App::save_prefs_sync`] persist — clones
    /// `self.prefs` and copies in whatever live `App` state is meant to survive a restart but
    /// isn't edited through the Preferences modal itself (locale, Editor panel/timeline
    /// layout), then prunes `recent_project_paths` entries whose files no longer exist.
    fn prefs_snapshot(&self) -> PrefsState {
        let mut prefs_snapshot = self.prefs.clone();
        // Always capture the live locale (app.locale may differ from prefs.locale if the user
        // changed it this session without having previously saved).
        prefs_snapshot.locale = self.locale;
        // Editor panel/timeline layout (request.md's Fase 3 "layout salvo por... usuário") —
        // dragged live via the resizable dividers, with no save trigger of their own short of
        // this snapshot being taken.
        prefs_snapshot.lib_panel_width = self.lib_panel_width;
        prefs_snapshot.props_panel_width = self.props_panel_width;
        prefs_snapshot.timeline_height = self.timeline_height;
        // Prune stale recents (moved/deleted files) so the list stays clean.
        prefs_snapshot
            .recent_project_paths
            .retain(|p| std::path::Path::new(p).exists());
        prefs_snapshot
    }

    /// Serializes `prefs` to the platform config file on a background thread. Called whenever
    /// the preferences modal closes or a project is opened/removed.
    pub fn save_prefs(&self) {
        let prefs_snapshot = self.prefs_snapshot();
        let Ok(bytes) = avcore::to_ocproj_bytes(&prefs_snapshot) else {
            return;
        };
        let path = prefs_path();
        std::thread::spawn(move || {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = std::fs::write(&path, &bytes) {
                tracing::error!(path = %path.display(), error = %e, "failed to write prefs");
            } else {
                tracing::debug!(path = %path.display(), "prefs saved");
            }
        });
    }

    /// Synchronous twin of [`App::save_prefs`], only for [`eframe::App::on_exit`] — a
    /// background thread's write has no guarantee of completing before the process actually
    /// exits right after `on_exit` returns (nothing joins it), so panel-size/layout changes
    /// made this session, which have no other save trigger short of opening Preferences, would
    /// otherwise silently fail to persist on a normal quit. The prefs file is small (a gzip+
    /// MessagePack blob, not project media), so blocking briefly during an already-in-progress
    /// shutdown is an acceptable tradeoff for actually guaranteeing the write happens.
    fn save_prefs_sync(&self) {
        let prefs_snapshot = self.prefs_snapshot();
        let Ok(bytes) = avcore::to_ocproj_bytes(&prefs_snapshot) else {
            return;
        };
        let path = prefs_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&path, &bytes) {
            tracing::error!(path = %path.display(), error = %e, "failed to write prefs (sync, on exit)");
        }
    }
}

/// Returns the path of the crash sentinel file. Its presence at startup means the previous
/// session exited uncleanly (crash, kill, power loss). Cleared by [`App::on_exit`].
fn sentinel_path() -> PathBuf {
    prefs_path()
        .parent()
        .map(|d| d.join("oca.running"))
        .unwrap_or_else(|| PathBuf::from("oca.running"))
}

/// Directory downloaded Whisper models are saved into — next to `prefs.oc`, same convention
/// as [`sentinel_path`]/`queue_path`.
pub(self) fn models_dir() -> PathBuf {
    prefs_path()
        .parent()
        .map(|d| d.join("models"))
        .unwrap_or_else(|| PathBuf::from("models"))
}

/// Where [`App::spawn_generate_tts`] writes its synthesized WAV files before importing them —
/// same platform-config-dir shape as [`models_dir`], a sibling `tts_output/` folder rather than
/// a per-project location, since the text that produced a given clip has no other home either.
pub(self) fn tts_output_dir() -> PathBuf {
    prefs_path()
        .parent()
        .map(|d| d.join("tts_output"))
        .unwrap_or_else(|| PathBuf::from("tts_output"))
}

/// Loads [`PrefsState`] from the platform config file, falling back to the default if the file
/// is absent or cannot be parsed.
pub fn load_prefs() -> PrefsState {
    let path = prefs_path();
    let Ok(bytes) = std::fs::read(&path) else {
        return PrefsState::default();
    };
    avcore::from_ocproj_bytes(&bytes).unwrap_or_default()
}

/// Returns the platform-appropriate path for the oca preferences file — `.oc`, the same
/// gzip-compressed MessagePack framing as a `.ocproj` project (see
/// [`avcore::persistence::to_ocproj_bytes`]/[`avcore::persistence::from_ocproj_bytes`]), not
/// JSON.
///
/// - macOS:   `~/Library/Application Support/oca/prefs.oc`
/// - Windows: `%APPDATA%\oca\prefs.oc`
/// - Linux:   `~/.config/oca/prefs.oc`
pub(self) fn prefs_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join("Library/Application Support/oca/prefs.oc");
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("oca\\prefs.oc");
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".config/oca/prefs.oc");
        }
    }
    PathBuf::from("prefs.oc")
}

/// Source-time width, in seconds, of one filmstrip thumbnail bucket (see
/// [`App::thumbnail_textures`]). Fixed rather than zoom-dependent — simpler cache
/// invalidation (a bucket's key never changes as the user zooms) at the cost of some tiles
/// repeating when zoomed in past roughly one tile per this many seconds.
pub const THUMBNAIL_BUCKET_SECS: f64 = 1.0;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump_export_queue();
        self.pump_import_queue();
        self.pump_sound_library_queue();
        self.pump_transcribe();
        self.pump_auto_reframe();
        self.pump_motion_tracking();
        self.pump_matte_generation();
        self.pump_text_to_speech();
        self.pump_model_download();
        self.pump_update_check();
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
            self.sample_preview_frame_telemetry(ui.ctx());
        } else {
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }

        // Must run before any panel narrows `ui`'s rect — see its own doc comment.
        screens::breadcrumb::handle_resize_borders(ui);

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
            Screen::SoundLibrary => screens::sound_library::show(self, ui),
            Screen::Queue => screens::queue::show(self, ui),
        });
        self.show_prefs_modal(ui.ctx());
        self.show_rename_project_modal(ui.ctx());
        self.show_rename_sequence_modal(ui.ctx());
        self.show_export_conflict_modal(ui.ctx());
        self.show_save_layer_template_modal(ui.ctx());
        self.show_layer_templates_menu(ui.ctx());
        self.show_apply_layer_template_modal(ui.ctx());
        self.show_tts_modal(ui.ctx());
        self.show_toasts(ui.ctx());
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Persist synchronously — see save_prefs_sync's doc comment on why the normal
        // background-thread save_prefs can't be trusted to finish before the process exits.
        self.save_prefs_sync();
        // Clean exit — remove the crash sentinel so the next launch doesn't think we crashed.
        let _ = std::fs::remove_file(sentinel_path());
        tracing::info!("clean exit — crash sentinel removed");
    }
}

#[cfg(test)]
mod app_test;
