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

//! Application state ([`App`]) and the top-level `eframe::App` implementation that
//! drives one frame: pump the background export queue, draw the nav rail and breadcrumb,
//! then delegate to whichever [`Screen`] is currently active (see [`crate::screens`]).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
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
mod collab_bundle;
mod color;
mod crash_review;
pub(crate) mod error_reporting;
pub mod export;
mod gameplay_events;
mod highlight_detection;
mod import;
mod layer_templates;
mod markers;
mod modals;
mod motion_tracking;
mod multicam;
mod preview;
mod scene_detection;
mod shorts_pack;
mod silence_review;
mod smart_bins;
mod sound_library;
mod telemetry;
mod text_to_speech;
mod timeline_clipboard_ops;
mod timeline_ops;
mod timeline_track_ops;
mod timeline_trim_ops;
mod transcribe;
mod transcript_panel;
mod transcript_proposals;
mod update_check;
mod watch_folder;
mod youtube_download;

pub(crate) use color::{format_color_hex, TextColorEdit, TextColorTarget};

/// Which of the app's five top-level views is currently showing. Drives both the central
/// panel content and which nav-rail button is highlighted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Editor,
    Library,
    SoundLibrary,
    Queue,
    WatchFolder,
}

/// The editor toolbar's active tool. `Select`/`Trim` are just tracked for the toolbar's
/// highlight state — dragging a clip's body/edge behaves the same regardless of which of
/// those two is active (Fase 3 never ended up gating that on the tool selection). `Ripple`/
/// `Roll`/`Slip`/`Slide` (`ROADMAP.md` P2 item 11 — Premiere/DaVinci/FCP's named trim modes)
/// *do* change what a drag does — `screens::editor::timeline_panel` branches on `app.tool` when
/// committing a trim-edge or clip-body drag. Splitting ("Cortar") isn't a persistent mode like
/// any of these — it's a one-shot action, performed directly by [`App::split_at_playhead`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTool {
    Select,
    Trim,
    /// Trim without leaving a gap — later clips on the same track shift to fill it.
    Ripple,
    /// Move the cut point between two adjacent clips; their combined timeline span is
    /// unchanged, just reallocated between them.
    Roll,
    /// Change which part of the source media a clip shows, without moving it on the timeline
    /// or changing its duration.
    Slip,
    /// Move a clip along the timeline; its immediate neighbors' in/out points adjust to absorb
    /// the move, nothing else shifts.
    Slide,
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
    AddOpacityMarker,
    Undo,
    Redo,
}

/// User-configurable key bindings for the five main editor shortcuts. Persisted as part of
/// [`PrefsState`] so changes survive restarts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyBindings {
    pub play_pause: KeyCombo,
    pub split_at_playhead: KeyCombo,
    pub copy_formatting: KeyCombo,
    pub paste_formatting: KeyCombo,
    /// `Ctrl+O` by default, per `request.md`'s Fase 6 key binding spec ("adicionar marcador de
    /// opacidade") — see [`App::add_opacity_marker_at_playhead`]. Added after the other four,
    /// so `#[serde(default)]` keeps an older saved `prefs.oc` (with no such key at all in its
    /// serialized `KeyBindings`) loading correctly instead of failing outright.
    #[serde(default = "default_add_opacity_marker_binding")]
    pub add_opacity_marker: KeyCombo,
    /// `Ctrl+Z` by default — see [`crate::app::App::undo`]. Added after the other five, so
    /// `#[serde(default)]` keeps an older saved `prefs.oc` loading correctly.
    #[serde(default = "default_undo_binding")]
    pub undo: KeyCombo,
    /// `Ctrl+Y` by default — see [`crate::app::App::redo`].
    #[serde(default = "default_redo_binding")]
    pub redo: KeyCombo,
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
            add_opacity_marker: default_add_opacity_marker_binding(),
            undo: default_undo_binding(),
            redo: default_redo_binding(),
        }
    }
}

fn default_add_opacity_marker_binding() -> KeyCombo {
    KeyCombo {
        ctrl: true,
        shift: false,
        key_name: "O".to_string(),
    }
}

fn default_undo_binding() -> KeyCombo {
    KeyCombo {
        ctrl: true,
        shift: false,
        key_name: "Z".to_string(),
    }
}

fn default_redo_binding() -> KeyCombo {
    KeyCombo {
        ctrl: true,
        shift: false,
        key_name: "Y".to_string(),
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
    /// Preferred video encoder for exports — hardware-accelerated
    /// (NVENC/Quick Sync/AMF/VAAPI) with an automatic CPU (libopenh264) fallback, or a specific
    /// choice. See
    /// [`avcore::GpuEncoderPreference`].
    #[serde(default)]
    pub gpu_encoder: avcore::GpuEncoderPreference,
    /// Path to the bundled GGML Whisper Base model, or an optional user override.
    #[serde(default)]
    pub whisper_model_path: String,
    /// Path to a local UltraFace ONNX model file for auto-reframe
    /// ([`App::spawn_auto_reframe_selected_clip`]), or an optional user override.
    #[serde(default)]
    pub reframe_model_path: String,
    /// Path to a local MODNet ONNX model file for AI background removal
    /// (`avcore::background_removal::segment_person`), or an optional user override.
    #[serde(default)]
    pub background_removal_model_path: String,
    /// Path to a local Piper voice `.onnx` model file for text-to-speech
    /// (`avcore::text_to_speech::synthesize`). Its `.onnx.json` config sidecar is expected right
    /// next to it (`<this path>.json`). Defaults to the bundled pt-BR voice and may be
    /// overridden with a compatible local voice.
    #[serde(default)]
    pub tts_model_path: String,
    /// ER-01B consent for remote error reporting — separate from `telemetry_enabled` (local-only,
    /// never sent). Defaults to disabled; the user must explicitly opt in.
    #[serde(default)]
    pub error_reporting_consent: error_reporting::ErrorReportingConsent,
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
    /// Prefer GStreamer's hardware video decoders for Editor preview and scrubbing. Hardware
    /// is enabled by default when available and falls back to software automatically; turning
    /// this off forces software decoding for every preview pipeline.
    #[serde(default = "default_preview_hardware_decode")]
    pub preview_hardware_decode: bool,
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
    /// Which half of the "layout salvo por projeto ou por usuário" spec is active — whether
    /// [`App::lib_panel_width`]/`props_panel_width`/`timeline_height` are captured back into
    /// this per-user `PrefsState` (the default, matching this codebase's pre-existing
    /// behavior) or into the active project's own `avcore::Project::panel_layout` instead
    /// (`App::sync_panel_layout_into_active_project`), so different projects can each remember
    /// their own layout. `#[serde(default)]` so an older saved `prefs.oc` loads as `PerUser`,
    /// unchanged behavior.
    #[serde(default)]
    pub layout_scope: LayoutScope,
    /// Unix timestamp of the most recent `crash_<unix>.txt` the user has already responded to
    /// through ER-01B's post-crash review modal (`crash_review.rs`) — `App::new` only stages a
    /// crash file newer than this for review, so the same crash is never re-prompted on a later
    /// launch. `0` (the default for an older `prefs.oc` with no such crash yet reviewed) means
    /// every crash file found is still unreviewed.
    #[serde(default)]
    pub last_reviewed_crash_unix: u64,
}

/// See [`PrefsState::layout_scope`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum LayoutScope {
    #[default]
    PerUser,
    PerProject,
}

fn default_telemetry_enabled() -> bool {
    true
}

fn default_preview_hardware_decode() -> bool {
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
            preview_hardware_decode: true,
            telemetry_enabled: true,
            error_reporting_consent: error_reporting::ErrorReportingConsent::default(),
            lib_panel_width: default_lib_panel_width(),
            props_panel_width: default_props_panel_width(),
            timeline_height: default_timeline_height(),
            layout_scope: LayoutScope::default(),
            last_reviewed_crash_unix: 0,
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

/// Gap between `ResourceUsage` (CPU/RAM) telemetry samples — far coarser than
/// [`PREVIEW_FRAME_TELEMETRY_INTERVAL`] since system-wide resource usage doesn't need to be
/// tracked at anywhere near frame granularity to be useful for `request.md`'s "uso de RAM numa
/// sessão de 2h" target metric.
pub const RESOURCE_TELEMETRY_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

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

/// A message from a background scene-cut-detection worker thread (see
/// [`App::spawn_detect_scene_cuts_for_selected_clip`]) back to the UI thread.
enum SceneCutEvent {
    Done {
        clip_id: u64,
        cuts: Vec<avcore::SceneCut>,
    },
}

/// A message from a background AI-background-removal matte-generation worker thread (see
/// [`App::spawn_generate_matte_for_selected_clip`]) back to the UI thread.
enum MatteGenerationEvent {
    Done { clip_id: u64, mask_path: PathBuf },
    Failed { message: String },
}

/// A message from the background update-check thread (see [`App::spawn_update_check`]) back to
/// the UI thread. Every terminal result is sent because the About modal distinguishes a
/// successful current-version result from a failed network request.
enum UpdateCheckEvent {
    NewerVersionAvailable {
        version: String,
        html_url: String,
        auto_update_available: bool,
    },
    UpToDate,
    Failed,
    Installed {
        update: AvailableUpdate,
    },
    InstallFailed {
        update: AvailableUpdate,
    },
}

/// A GitHub release newer than the running build, carried by
/// [`UpdateCheckStatus::Available`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub version: String,
    pub html_url: String,
    pub auto_update_available: bool,
}

/// User-facing state of the one-shot GitHub Releases check. Keeping failure distinct from
/// `UpToDate` prevents the About modal from claiming the running version is current when the
/// network request never completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheckStatus {
    Checking,
    UpToDate,
    Failed,
    Available(AvailableUpdate),
    Installing(AvailableUpdate),
    RestartRequired(AvailableUpdate),
    InstallFailed(AvailableUpdate),
}

/// A message from a background text-to-speech worker thread (see
/// [`App::spawn_generate_tts`]) back to the UI thread.
enum TtsEvent {
    Done { wav_path: PathBuf },
    Failed { message: String },
}

/// Which format the "Baixar do YouTube" modal is currently set to — picks which of
/// [`App::youtube_modal_mp4_quality`]/[`App::youtube_modal_mp3_bitrate`]
/// [`App::spawn_youtube_download`] reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YoutubeFormatChoice {
    Mp4,
    Mp3,
}

/// A message from a background YouTube-download worker thread (see
/// [`App::spawn_youtube_download`]) back to the UI thread.
enum YoutubeDownloadEvent {
    Progress(f32),
    Done { path: PathBuf },
    Failed { message: String },
}

/// Where one file tracked by the watch-folder worker thread (see
/// [`App::start_watching_folder`]) currently sits — mirrors `Watch-Gameplay.ps1`'s own per-file
/// pipeline stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WatchFolderFileStatus {
    Stabilizing,
    Processing,
    Done,
    Error,
}

/// One file the watch-folder worker thread has seen this run, as shown in the Limpeza screen's
/// list — newest-first, same order [`App::pump_watch_folder`] inserts into
/// `WatchFolderState::files`.
pub(crate) struct WatchedFileRow {
    pub(crate) path: PathBuf,
    pub(crate) status: WatchFolderFileStatus,
    pub(crate) percent: u8,
    pub(crate) error: Option<String>,
    pub(crate) before: Option<avcore::LoudnessMetrics>,
    pub(crate) after: Option<avcore::LoudnessMetrics>,
    /// CF-02 slice 1's tractable follow-up (bringing a cleaned-up recording into a project's
    /// media library, reusing this watcher's own [`avcore::StabilityTracker`]): `true` once
    /// [`App::add_watched_file_to_project`] has queued this row's cleaned-up output for import,
    /// so the Limpeza screen shows "Added" instead of a re-clickable button.
    pub(crate) added_to_project: bool,
}

/// A message from the watch-folder worker thread (see [`App::start_watching_folder`]) back to
/// the UI thread.
enum WatchFolderEvent {
    Detected(PathBuf),
    Stabilizing(PathBuf),
    Processing(PathBuf),
    Progress(PathBuf, u8),
    Done {
        path: PathBuf,
        before: avcore::LoudnessMetrics,
        after: avcore::LoudnessMetrics,
    },
    Failed {
        path: PathBuf,
        message: String,
    },
}

/// Result of one background poster-frame extraction (see [`App::request_thumbnail`]). The key
/// is `(asset_id, frame_index)` rather than a fixed-width seconds bucket: timeline zoom chooses
/// a source frame for each visible filmstrip tile, while quantizing to the source frame rate
/// keeps nearby zoom levels able to share cached textures.
enum ThumbnailReady {
    Ready {
        asset_id: u64,
        frame_index: i64,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    Failed {
        asset_id: u64,
        frame_index: i64,
    },
}

type ThumbnailKey = (u64, i64);

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
    /// Telemetry channel/flag/throttle-timestamp state, grouped the same way
    /// [`PreviewState`] was — see that struct's doc comment for why. Field names, visibility,
    /// and invariants unchanged from when they lived directly on `App`.
    pub(crate) telemetry_state: TelemetryState,
    /// ER-01A remote-error reporter slot: `Some` only once consent UI and a provider adapter
    /// land (ER-01B). `None` is the consent-disabled state — [`App::report_error`] short-circuits
    /// and nothing is built or written. See [`crate::app::error_reporting`].
    pub(crate) error_reporter: Option<Arc<dyn avcore::ErrorReporter>>,
    render_tx: UnboundedSender<RenderEvent>,
    render_rx: UnboundedReceiver<RenderEvent>,
    /// Cooperative pause/cancel controls for jobs a worker thread is currently rendering,
    /// keyed by job id.
    /// A job id present here is the source of truth for "how many workers are busy right
    /// now" — [`App::pump_export_queue`] uses its length against `prefs.export_workers`.
    active_renders: HashMap<u64, Arc<export::RenderControl>>,
    /// Cached result of resolving the active sequence's video tracks and media library into
    /// export segments — see [`App::resolved_active_sequence_export_preview`]. `None` before
    /// the Fila (export queue) screen's header has ever been drawn.
    export_preview_cache: Option<export::ExportPreviewCache>,
    /// Rendered-file cache for compound clips (nested sequences), keyed by
    /// `ClipInstance::nested_sequence_id` — see
    /// [`avcore::nested_sequence::materialize_nested_sequences`]'s own doc comment. Shared by
    /// both export resolution and preview (each pays the render cost once per edit to the
    /// nested sequence, not once per caller).
    pub(super) nested_sequence_render_cache:
        HashMap<u64, avcore::nested_sequence::NestedSequenceCache>,
    /// Every field around the live preview pipeline — GStreamer pipeline handle, loaded-clip-id
    /// tracking, uploaded textures, playback/fullscreen state — grouped into its own struct
    /// rather than left flat on `App` (an internal-audit finding: `App` had grown to 122 flat
    /// fields with no substructure at all). Field names and visibility are unchanged from when
    /// they lived directly on `App`; only the access path grew one `.preview_state` hop. See
    /// [`PreviewState`]'s own doc comment.
    pub(crate) preview_state: PreviewState,
    /// Import background-job channel/asset-tracking state — same pattern as
    /// [`App::auto_reframe_state`].
    pub(crate) import_state: ImportState,
    /// Tracks found by the last scan of `prefs.sound_library_path` (see
    /// [`App::rescan_sound_library`]) — not persisted, recomputed from disk whenever the Music
    /// & SFX screen is opened or the configured folder changes.
    pub sound_library_tracks: Vec<avcore::sound_library::LibraryTrack>,
    sound_library_tx: UnboundedSender<Vec<avcore::sound_library::LibraryTrack>>,
    sound_library_rx: UnboundedReceiver<Vec<avcore::sound_library::LibraryTrack>>,
    /// Transcription background-job channel/asset-tracking state — same pattern as
    /// [`App::auto_reframe_state`].
    pub(crate) transcribe_state: TranscribeState,
    /// Auto-reframe background-job channel/clip-tracking state, grouped the same way
    /// [`PreviewState`] was — see that struct's doc comment for why.
    pub(crate) auto_reframe_state: AutoReframeState,
    /// Motion-tracking background-job channel/clip-tracking state — same pattern.
    pub(crate) motion_tracking_state: MotionTrackingState,
    /// Scene-cut-detection background-job channel/clip-tracking state — same pattern.
    pub(crate) scene_cut_detection_state: SceneCutDetectionState,
    /// Motion-tracking region-picker session state — see [`MotionTrackRegionState`]'s own doc
    /// comment.
    pub motion_track_region: MotionTrackRegionState,
    /// Matte-generation background-job channel/clip-tracking state — same pattern as
    /// [`App::auto_reframe_state`].
    pub(crate) matte_generation_state: MatteGenerationState,
    /// Text-to-speech modal/background-job state, grouped the same way [`PreviewState`] was.
    pub(crate) tts_state: TtsState,
    /// YouTube-download modal/background-job state, grouped the same way [`PreviewState`] was.
    pub(crate) youtube_download_state: YoutubeDownloadState,
    /// Watch-folder screen state, grouped the same way [`PreviewState`] was.
    pub(crate) watch_folder_state: WatchFolderState,
    /// The timeline clip currently highlighted in the Editor's timeline strip, if any — a
    /// separate concept from `selected_asset_id` (that's the media-library selection driving
    /// the preview panel; this is a placed [`avcore::timeline::ClipInstance`]). `Delete`
    /// removes whichever clip this points at.
    pub selected_clip_id: Option<u64>,
    /// Snapshot-based undo/redo history for the active sequence's timeline (`spec/ROADMAP.md`
    /// P0 item 1, `spec/architecture/undo-redo.md`) — scoped to one sequence, cleared on every
    /// sequence/project switch since history from one tab is meaningless applied to another.
    pub undo_stack: avcore::undo::UndoStack,
    /// Whether an effect-property drag (a slider/`DragValue` held down in the properties
    /// panel) is currently pushing its *first* undo snapshot — see
    /// [`App::push_undo_snapshot_for_drag`]. Reset to `false` once per frame in
    /// `screens::editor::show` whenever no pointer button is held, so the next drag (or
    /// instant click) starts a fresh snapshot instead of reusing this one.
    undo_drag_active: bool,
    /// The text overlay clip currently selected on a text track, if any. Selecting a text clip
    /// clears `selected_clip_id`/`selected_shape_clip_id` and vice versa — only one kind of clip
    /// can be selected at a time. The properties panel shows text-clip controls when this is
    /// `Some`.
    pub selected_text_clip_id: Option<u64>,
    /// Transactional state for the text color modal. The project is only mutated when the
    /// user confirms, so closing or cancelling the modal leaves the original color untouched.
    text_color_edit: Option<TextColorEdit>,
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
    /// Thumbnail background-job channel/cache state — same pattern as
    /// [`App::auto_reframe_state`].
    pub(crate) thumbnail_state: ThumbnailState,
    /// Set by the media library panel on the frame a dragged asset is released (screen-space
    /// pointer position), consumed by the timeline panel later in the same frame to place it —
    /// how dragging an asset out of the library and dropping it on the timeline works. Always
    /// `None` between frames.
    pub pending_asset_drop: Option<(u64, egui::Pos2)>,
    /// Live text typed into the Editor's media-library search box (`editor.rs::
    /// media_library_panel`) — filters the visible asset list by substring match on file name,
    /// same as the search field in `oca-editor-mock.html`'s `.search-input`. Transient UI state,
    /// not persisted: like `selected_clip_id`, a search-in-progress has no meaning across a
    /// save/reload.
    pub media_search: String,
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
    /// The multicam group (P2 item 10, "Multicam editing") number-key angle switching applies
    /// to — the one most recently created via [`App::create_multicam_group_from_video_tracks`],
    /// or `None` before any group exists in the active sequence yet. Not persisted: re-derived
    /// (first group found, if any) the same way `selected_asset_id` resets on project switch,
    /// see [`App::open_project`].
    pub active_multicam_group_id: Option<u64>,
    /// The smart bin (P4 item 22, "Smart bins") currently filtering the Library panel's asset
    /// list — `None` shows every asset in `media_library`, unfiltered. Not persisted: resets to
    /// `None` on project switch, same as `active_multicam_group_id`.
    pub active_smart_bin_id: Option<u64>,
    /// A draft [`avcore::SmartBin`] being created/edited, shown as a modal by
    /// [`App::show_smart_bin_modal`] when `Some`. `id == 0` (never a real assigned id, which
    /// starts at 1 — see [`avcore::Project::add_smart_bin`]) means "new bin, not yet created";
    /// any other id means "editing that existing bin's rules in place." `None` when the modal
    /// is closed.
    pub editing_smart_bin: Option<avcore::SmartBin>,
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
    /// Whether the Fase 8 About modal is open. It is separate from `prefs_open` so opening it
    /// from Preferences closes that larger modal instead of stacking two modal layers.
    pub about_open: bool,
    /// Set to the autosave file path when opening a project that has a newer autosave on disk.
    /// [`App::pump_autosave_restore`] consumes it to show the restore/discard modal.
    autosave_restore_pending: Option<PathBuf>,
    /// `true` when a crash sentinel from a previous session was found at startup — consumed
    /// by [`App::ui`] to show a one-time toast, then cleared. Independent of
    /// [`App::pending_crash_review`]: the sentinel just means the previous process didn't exit
    /// cleanly (crash, kill, power loss), while the pending review is only set when that crash
    /// was specifically a captured Rust panic with a matching `crash_<unix>.txt` file.
    crash_detected: bool,
    /// ER-01B's post-crash review offer (see `crash_review.rs`): `Some` when a `crash_<unix>.txt`
    /// file newer than `prefs.last_reviewed_crash_unix` was found at startup, consumed by
    /// [`App::show_crash_review_modal`].
    pending_crash_review: Option<crash_review::PendingCrashReview>,
    /// When `Some((index, name_buf, summary_buf))`, a project-settings modal is shown for
    /// `projects[index]` with editable name and summary fields. Committed on confirm, discarded
    /// on Escape/cancel.
    pub renaming_project: Option<(usize, String, String)>,
    /// When `Some((seq_index, buf))`, a rename modal is shown for the active project's
    /// `sequences[seq_index]`. Committed on Enter/confirm, discarded on Escape/cancel.
    pub renaming_sequence: Option<(usize, String)>,
    /// `(sequence_id, name)` staged while the destructive sequence-delete confirmation modal
    /// is open. Stored by id rather than index so a reordered tab cannot make confirmation
    /// delete a different sequence.
    pub deleting_sequence: Option<(u64, String)>,
    /// `(clip_id, start_speed_buf, end_speed_buf, steps_buf, smooth)` staged while the custom
    /// speed-ramp dialog is open — the fixed-preset "Rampa de velocidade" submenu entries call
    /// [`App::apply_speed_ramp_to_selected_clip`] directly with no dialog, but a custom start/
    /// end speed and step count needs editable buffers staged somewhere across frames, same
    /// shape `renaming_project`'s name/summary buffers have. `steps_buf` is a `String` (not a
    /// `usize`) so the field can sit empty/mid-edit rather than snapping to some fallback on
    /// every keystroke; parsed back to `usize` only on confirm, and ignored when `smooth` is
    /// `true` — [`App::apply_smooth_speed_ramp_to_selected_clip`] needs no step count at all
    /// (`spec/ROADMAP.md` item 29's "smooth continuous curve" follow-up). `None` when the
    /// dialog is closed.
    pub speed_ramp_dialog: Option<(u64, f32, f32, String, bool)>,
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
    /// Whether the Timeline Index panel (searchable review/comment marker list — `ROADMAP.md`
    /// P2 item 9) is open.
    pub timeline_index_open: bool,
    /// Live text of the Timeline Index panel's search box — kept on `App` rather than as a
    /// local in the modal-drawing function so it survives being closed and reopened.
    pub marker_search: String,
    /// Whether the Transcript panel (CF-01 slice 2,
    /// `spec/architecture/competitive-feature-plan.md`) is open.
    pub transcript_panel_open: bool,
    /// Live text of the Transcript panel's search box — same "survives close/reopen" reasoning
    /// `marker_search` has.
    pub transcript_search: String,
    /// The transcript document currently shown in the Transcript panel, lazily loaded (and
    /// re-loaded whenever the previewed clip's own asset changes) by
    /// `App::ensure_transcript_loaded_for_preview` — see that method's own doc comment.
    /// `loaded_asset_id` is `None` before anything has ever been loaded; `document` is `None`
    /// either before that first load or when the loaded asset genuinely has no transcript yet
    /// (not the same as "still loading" — this crate has no async load, so there's no such
    /// state to represent).
    pub transcript_panel_state: TranscriptPanelState,
    /// Staged result of `App::begin_silence_review` (D1, `ROADMAP.md` P3 item 13) — `Some`
    /// while the silence-gap review modal is open, `None` otherwise. Nothing here is applied to
    /// the timeline until `App::apply_silence_review`.
    pub silence_review: Option<silence_review::SilenceReview>,
    /// Staged result of `App::begin_transcript_proposals` (CF-01 slice 4/5) — `Some` while the
    /// speech-edit-review modal is open, `None` otherwise. Nothing here is applied to the
    /// timeline until `App::apply_transcript_proposals`, and even then only the accepted
    /// proposals are.
    pub transcript_review: Option<transcript_proposals::TranscriptReview>,
    /// Whether the Transcript panel's search box also matches the *whole project media library*
    /// (CF-01 slice 3) rather than only the previewed clip's own transcript.
    pub transcript_search_project: bool,
    /// When `Some(action)`, the prefs modal is waiting for the next key press to set that
    /// action's binding. Pressing Escape clears it without changing the binding.
    pub binding_capture: Option<BindableAction>,
    update_check_tx: UnboundedSender<UpdateCheckEvent>,
    update_check_rx: UnboundedReceiver<UpdateCheckEvent>,
    /// Result of the startup GitHub Releases check and any user-requested install. The Home
    /// screen shows a banner only for [`UpdateCheckStatus::Available`]; the About modal owns
    /// download/install/retry/restart presentation.
    pub update_check_status: UpdateCheckStatus,
    /// Set when "Adicionar exportação" picked an output path that already exists — holds
    /// everything needed to queue the export once the user resolves the conflict via
    /// [`App::show_export_conflict_modal`] (Overwrite / Rename / Cancel).
    pub pending_export_conflict: Option<export::PendingExportConflict>,
}

/// [`App::transcript_panel_state`]'s own fields.
#[derive(Default)]
pub(crate) struct TranscriptPanelState {
    pub(crate) loaded_asset_id: Option<u64>,
    pub(crate) document: Option<avcore::TranscriptDocument>,
}

/// Live preview pipeline state, extracted from `App`'s own field list (see
/// [`App::preview_state`]'s doc comment for why). Every field here behaves exactly as it did as
/// a flat `App` field before this extraction — same name, same visibility, same invariants,
/// documented on the field itself as before.
#[derive(Default)]
pub(crate) struct PreviewState {
    /// The GStreamer pipeline for the clip currently covering the active sequence's timeline
    /// playhead, if it could be opened (`None` before any project has a clip at the playhead,
    /// before it's been lazily opened, and when `Preview::open` failed, e.g. a source file
    /// that's since been moved or deleted — see [`App::ensure_preview_loaded`]).
    pub(crate) preview: Option<avcore::preview::Preview>,
    /// The clip id [`App::ensure_preview_loaded`] last attempted to open a pipeline for,
    /// whether or not it succeeded — lets it tell "already tried and failed for this exact
    /// clip, don't retry every frame" apart from "the playhead moved onto a different clip, try
    /// again".
    pub(crate) preview_clip_id: Option<u64>,
    /// Clip ids of the overlay-track branches [`App::ensure_preview_loaded`] last opened a
    /// composited pipeline for, in the same order [`avcore::preview::Preview::open_composited`]
    /// was given them (and the same order [`App::seek_preview`]/[`App::pump_preview_frame`]
    /// must pass offsets to [`avcore::preview::Preview::seek_composited`] in). Empty when the
    /// playhead's background clip has no overlay-track clips over it — `preview` is then a
    /// plain [`avcore::preview::Preview::open`] single-clip pipeline instead, same as before
    /// composited preview existed.
    pub(crate) preview_overlay_clip_ids: Vec<u64>,
    /// Audio-only timeline clips currently opened as independent `audiomixer` branches.
    pub(crate) preview_audio_clip_ids: Vec<u64>,
    /// Ids of the [`TrackKind::Text`] clips covering the playhead the last time
    /// [`App::ensure_preview_loaded`] opened a pipeline, in the same order passed as
    /// [`avcore::preview::Preview::open_composited`]'s `text_overlays` — same reopen-detection
    /// role `preview_overlay_clip_ids` has for video overlay branches. Text clips have no
    /// source to seek, but this ordering also guards live word-highlight buffer replacements;
    /// a different id set is left for the next full pipeline rebuild.
    pub(crate) preview_text_clip_ids: Vec<u64>,
    /// Same role as `preview_text_clip_ids`, for [`TrackKind::Shape`] clips.
    pub(crate) preview_shape_clip_ids: Vec<u64>,
    /// Uploaded from the latest [`avcore::preview::Preview::current_frame`] each frame the
    /// Editor screen is shown; `None` until the first frame decodes. Reset whenever
    /// [`App::ensure_preview_loaded`] reopens the pipeline for a different clip so a stale
    /// frame from the previous one never lingers.
    pub(crate) preview_texture: Option<egui::TextureHandle>,
    /// Cache for [`App::pump_preview_frame`]'s CPU-side 3D LUT preview approximation (P4 item
    /// 21, "Preview support for vignette/glitch/deflicker/3D-LUT/stabilization" —
    /// `avcore::preview_effects`): `Some((path, parsed))` once `path` has been attempted, so a
    /// static LUT selection doesn't reparse (or re-fail to parse) the `.cube` file every single
    /// frame. Keyed by path (not clip id) since the same LUT file can be shared across clips;
    /// invalidated by comparing `path` against the currently previewed clip's `lut_path` each
    /// frame. `parsed` is `None` when `path` failed to parse — cached as a failure too, not
    /// retried every frame. `None` (the outer `Option`) before any clip with a LUT has been
    /// previewed yet.
    pub(crate) preview_lut_cache: Option<(String, Option<avcore::Lut3D>)>,
    /// Whether the Editor preview panel's waveform/vectorscope color scopes are shown — off by
    /// default, since computing both is a full pass over every pixel of every decoded frame
    /// (see [`App::pump_preview_frame`]) and most edits don't need it.
    pub(crate) scopes_enabled: bool,
    /// Uploaded from [`avcore::luma_waveform_rgba`] alongside `preview_texture`, only while
    /// `scopes_enabled` is set. `None` until the first frame decodes with scopes on, same
    /// lazily-populated shape as `preview_texture` itself.
    pub(crate) waveform_texture: Option<egui::TextureHandle>,
    /// Same role as `waveform_texture`, for [`avcore::vectorscope_rgba`].
    pub(crate) vectorscope_texture: Option<egui::TextureHandle>,
    /// Whether the preview pipeline is in `Playing` state. `Preview` has no state getter of
    /// its own, so the Editor's play/pause button and [`App::pump_export_queue`]'s repaint
    /// cadence both rely on this instead.
    pub(crate) preview_playing: bool,
    /// Wall-clock playback start `(Instant, timeline playhead at that instant)` for a frozen
    /// clip — set whenever playback begins while the clip covering the playhead has
    /// `ClipInstance::frozen` set. A frozen clip's pipeline is kept `Paused` at
    /// `source_in_secs` (so it always shows the held anchor frame) rather than actually
    /// playing, so [`App::pump_preview_frame`] has no `Preview::position_secs` to derive
    /// the advancing playhead from the way it does for a normal clip — this stands in for it.
    /// `None` when nothing is playing or the current clip isn't frozen.
    pub(crate) preview_frozen_since: Option<(std::time::Instant, f64)>,
    /// Whether the Editor's preview panel is currently rendered as a fullscreen overlay
    /// (covers the whole window, replacing the nav rail/breadcrumb/normal screen for that
    /// frame — see `impl eframe::App for App`'s early-return branch). Toggled by the preview
    /// panel's fullscreen button and cleared by Esc or the overlay's own exit button.
    pub(crate) fullscreen_preview: bool,
    /// Wall-clock time the pointer last moved (or a click/drag occurred) while
    /// `fullscreen_preview` is active — the fullscreen overlay's playback controls
    /// fade out [`FULLSCREEN_CONTROLS_IDLE_SECS`] after this and reappear immediately on the
    /// next pointer movement. `None` right after entering fullscreen so controls start visible.
    pub(crate) fullscreen_controls_last_moved: Option<std::time::Instant>,
}

/// Telemetry channel/flag/throttle state, extracted from `App`'s own field list — see
/// [`PreviewState`]'s doc comment for why. Every field behaves exactly as it did as a flat
/// `App` field before this extraction.
pub(crate) struct TelemetryState {
    /// Sends [`avcore::TelemetryEvent`]s to the dedicated background writer thread spawned in
    /// [`App::new`] — see [`App::record_telemetry`]/`telemetry::spawn_telemetry_writer`. No
    /// paired receiver is kept on `App`; that thread owns the only one.
    pub(crate) telemetry_tx: UnboundedSender<avcore::TelemetryEvent>,
    /// Live mirror of `prefs.telemetry_enabled`, checked by the background resource- and
    /// GPU-sampling threads spawned in [`App::new`] (`telemetry::spawn_resource_sampler`/
    /// `telemetry::spawn_gpu_sampler`) — neither thread has access to `App`/`prefs` directly, so
    /// this shared `Arc<AtomicBool>` is the one piece of state each reads every tick. Kept in
    /// sync with `prefs.telemetry_enabled` wherever the Preferences screen's checkbox mutates it.
    pub(crate) telemetry_enabled_flag: Arc<AtomicBool>,
    /// Wall-clock time [`App::record_telemetry`] last recorded a `PreviewFrameTime` sample —
    /// throttles sampling to roughly once every [`PREVIEW_FRAME_TELEMETRY_INTERVAL`] rather
    /// than every single frame, which would flood `telemetry.jsonl`.
    pub(crate) last_preview_frame_telemetry: Option<std::time::Instant>,
}

/// Motion-tracking region-picker session state, extracted from `App`'s own field list — see
/// [`PreviewState`]'s doc comment for why. Plain UI/session state, not persisted to the
/// project: it's a one-shot tracking job's input, not a durable clip property (unlike, say,
/// `ClipInstance::crop_x/y`) — nothing else reads it once a run finishes, only its *output*
/// (`position_keyframes`) is saved.
pub(crate) struct MotionTrackRegionState {
    /// The tracked region's center, as a `0.0..=1.0` fraction of the *source* frame (same
    /// convention as `avcore::track_region`'s `initial_center_x_frac`/`_y`, not canvas/layer
    /// space) — user-editable via the properties panel's region controls next to the "Rastrear
    /// movimento" button, instead of the button always defaulting to a centered region.
    pub(crate) motion_track_center_x: f32,
    pub(crate) motion_track_center_y: f32,
    /// The tracked block's width/height, each independently as a fraction of the frame's
    /// shorter dimension — `avcore::track_region`'s `template_width_frac`/`template_height_frac`.
    /// Same non-persistence rationale as `motion_track_center_x`/`_y`.
    pub(crate) motion_track_width: f32,
    pub(crate) motion_track_height: f32,
    /// How far the tracked block is allowed to move between consecutive sampled frames, as a
    /// fraction of the frame's shorter dimension — `avcore::track_region`'s
    /// `search_radius_frac`. Same non-persistence rationale as `motion_track_center_x`/`_y`.
    pub(crate) motion_track_search_radius: f32,
    /// Whether the Editor preview panel is currently showing the motion-tracking region
    /// picker (drag the tracked block's body to move it, its corner handle to resize it,
    /// directly on the preview frame) instead of its usual layer position/resize handling —
    /// see [`screens::editor::draw_motion_track_region_picker`]. Toggled by the properties
    /// panel's "pick in preview" button, next to the numeric region controls; Escape exits.
    /// Session-only, like the `motion_track_*` fields it edits.
    pub(crate) picking_motion_track_region: bool,
}

/// Auto-reframe background-job state, extracted from `App`'s own field list — see
/// [`PreviewState`]'s doc comment for why.
pub(crate) struct AutoReframeState {
    pub(crate) auto_reframe_tx: UnboundedSender<AutoReframeEvent>,
    pub(crate) auto_reframe_rx: UnboundedReceiver<AutoReframeEvent>,
    /// The timeline clip id a background auto-reframe run is currently computing a crop for, if
    /// any — only one runs at a time, same shape as `App::transcribing_asset_id`.
    pub(crate) auto_reframing_clip_id: Option<u64>,
}

/// Motion-tracking background-job state — same pattern as [`AutoReframeState`]. Distinct from
/// `App`'s `motion_track_*`/`picking_motion_track_region` fields, which are the region-picker
/// UI's own session state, not this one-shot background job's channel/clip-tracking state.
pub(crate) struct MotionTrackingState {
    pub(crate) motion_tracking_tx: UnboundedSender<MotionTrackEvent>,
    pub(crate) motion_tracking_rx: UnboundedReceiver<MotionTrackEvent>,
    /// The timeline clip id a background motion-tracking run is currently tracking, if any —
    /// only one runs at a time, same shape as `AutoReframeState::auto_reframing_clip_id`.
    pub(crate) motion_tracking_clip_id: Option<u64>,
}

/// Scene-cut-detection (D4) background-job state — same pattern as [`AutoReframeState`].
pub(crate) struct SceneCutDetectionState {
    pub(crate) scene_cut_detection_tx: UnboundedSender<SceneCutEvent>,
    pub(crate) scene_cut_detection_rx: UnboundedReceiver<SceneCutEvent>,
    /// The timeline clip id a background scene-cut-detection run is currently scanning, if
    /// any — only one runs at a time, same shape as `AutoReframeState::auto_reframing_clip_id`.
    pub(crate) scene_cut_detection_clip_id: Option<u64>,
}

/// AI-background-removal matte-generation background-job state — same pattern as
/// [`AutoReframeState`].
pub(crate) struct MatteGenerationState {
    pub(crate) matte_generation_tx: UnboundedSender<MatteGenerationEvent>,
    pub(crate) matte_generation_rx: UnboundedReceiver<MatteGenerationEvent>,
    /// The timeline clip id a background AI-background-removal matte-generation run is
    /// currently computing a matte for, if any — only one runs at a time, same shape as
    /// `AutoReframeState::auto_reframing_clip_id`.
    pub(crate) matte_generating_clip_id: Option<u64>,
}

/// Import background-job channel/asset-tracking state — one batch of files imported via
/// [`App::spawn_import`] tracked at a time (unlike transcription, imports run per-file
/// parallel).
pub(crate) struct ImportState {
    pub(crate) import_tx: UnboundedSender<ImportEvent>,
    pub(crate) import_rx: UnboundedReceiver<ImportEvent>,
    /// How many files a call to [`App::spawn_import`] haven't been probed yet, in the
    /// background. The Mídia screen shows a busy note while this is nonzero. Reaches zero as
    /// soon as each file's cheap probe comes back and it's added to the library — loudness
    /// measurement and proxy generation keep running after that in the background (see
    /// [`ImportState::pending_enrichment`]) without holding this counter up, since the asset is
    /// already usable by then.
    pub(crate) pending_imports: usize,
    /// The next id to hand out in [`App::spawn_import`], one per file in the batch —
    /// correlates a file's `ImportEvent::AssetReady` with its later `ImportEvent::Enriched`
    /// once [`App::pump_import_queue`] knows the asset's real (project-assigned) id.
    pub(crate) next_import_token: u64,
    /// Import tokens awaiting their `ImportEvent::Enriched` (loudness + proxy), mapped to the
    /// asset id they were assigned when their `AssetReady` landed — removed once the
    /// enrichment arrives and gets applied, or left dangling harmlessly if the asset is gone
    /// by then (media library has no delete yet, so that can't currently happen).
    pub(crate) pending_enrichment: HashMap<u64, u64>,
    /// Import tokens whose asset should be appended to the timeline the moment its
    /// `ImportEvent::AssetReady` lands (see [`App::pump_import_queue`]) — used by
    /// [`App::add_sound_library_track_to_timeline`]'s one-click "add to timeline" for a Music &
    /// SFX track that hasn't been imported into the active project yet.
    pub(crate) auto_add_to_timeline: HashSet<u64>,
}

/// Thumbnail background-job channel/cache state. Filmstrip tile textures keyed by
/// `(asset_id, source_frame_index)` — only visible tiles request frames; [`App::touch_thumbnails`]
/// and [`App::pump_thumbnail_queue`] keep the cache bounded (LRU) so browsing/zooming through a
/// long recording cannot grow GPU memory for the rest of the session.
pub(crate) struct ThumbnailState {
    pub(crate) thumbnail_tx: UnboundedSender<ThumbnailReady>,
    pub(crate) thumbnail_rx: UnboundedReceiver<ThumbnailReady>,
    pub(crate) thumbnail_textures: HashMap<ThumbnailKey, egui::TextureHandle>,
    /// Extractions currently running. Separate from cached/failed keys so the hard concurrency
    /// cap doesn't also make an evicted texture permanently non-requestable.
    pub(crate) pending_thumbnails: HashSet<ThumbnailKey>,
    /// Last-use ticks for cached textures, used by the LRU eviction pass.
    pub(crate) thumbnail_last_used: HashMap<ThumbnailKey, u64>,
    /// Recently failed keys and their last-use ticks. Bounded independently so a missing or
    /// corrupt source is not retried every frame but also cannot grow this set forever.
    pub(crate) failed_thumbnails: HashMap<ThumbnailKey, u64>,
    pub(crate) thumbnail_usage_clock: u64,
}

/// Transcription background-job state — same pattern as [`AutoReframeState`].
pub(crate) struct TranscribeState {
    pub(crate) transcribe_tx: UnboundedSender<TranscribeEvent>,
    pub(crate) transcribe_rx: UnboundedReceiver<TranscribeEvent>,
    /// The media asset id a background transcription is currently running for, if any — only
    /// one transcription runs at a time (unlike imports, which are per-file parallel). The
    /// Mídia screen shows a busy state on that asset's card while this is `Some`.
    pub(crate) transcribing_asset_id: Option<u64>,
}

/// Text-to-speech modal/background-job state, extracted from `App`'s own field list — see
/// [`PreviewState`]'s doc comment for why.
pub(crate) struct TtsState {
    pub(crate) tts_tx: UnboundedSender<TtsEvent>,
    pub(crate) tts_rx: UnboundedReceiver<TtsEvent>,
    /// `Some(text)` while the "Texto-pra-fala" modal is open — the text buffer being edited.
    /// `None` when the modal is closed.
    pub(crate) tts_modal_text: Option<String>,
    /// `true` while a background TTS synthesis run is in flight — only one at a time, same
    /// shape as `TranscribeState::transcribing_asset_id`.
    pub(crate) tts_generating: bool,
}

/// YouTube-download modal/background-job state, extracted from `App`'s own field list — see
/// [`PreviewState`]'s doc comment for why.
pub(crate) struct YoutubeDownloadState {
    pub(crate) youtube_download_tx: UnboundedSender<YoutubeDownloadEvent>,
    pub(crate) youtube_download_rx: UnboundedReceiver<YoutubeDownloadEvent>,
    /// `Some(url)` while the "Baixar do YouTube" modal is open — the URL text buffer being
    /// edited. Unlike `TtsState::tts_modal_text`, stays `Some` (rather than being taken) once a
    /// download starts, so the modal can keep showing the URL alongside progress and an error
    /// message stays actionable (retry without retyping the URL) instead of the modal just
    /// closing on submit the way the TTS one does.
    pub(crate) youtube_modal_url: Option<String>,
    pub(crate) youtube_modal_format: YoutubeFormatChoice,
    pub(crate) youtube_modal_mp4_quality: avcore::Mp4Quality,
    pub(crate) youtube_modal_mp3_bitrate: avcore::Mp3Bitrate,
    /// `true` while a background `yt-dlp` download is in flight — only one at a time.
    pub(crate) youtube_downloading: bool,
    /// `0.0..=1.0` fraction reported by `yt-dlp`'s own progress output — meaningless while
    /// `youtube_downloading` is `false`.
    pub(crate) youtube_download_progress: f32,
    /// Set after a failed/cancelled download; cleared on the next successful submit. Shown
    /// inline in the modal rather than as a toast, since the modal stays open for a retry.
    pub(crate) youtube_download_error: Option<String>,
    pub(crate) youtube_download_cancel: Option<Arc<AtomicBool>>,
}

/// Watch-folder screen state, extracted from `App`'s own field list — see [`PreviewState`]'s
/// doc comment for why.
pub(crate) struct WatchFolderState {
    tx: UnboundedSender<WatchFolderEvent>,
    rx: UnboundedReceiver<WatchFolderEvent>,
    pub(crate) watch_path: Option<PathBuf>,
    /// `true` while the background polling thread is running — only one watch session at a
    /// time.
    pub(crate) running: bool,
    /// Set when `running` starts, cleared when it stops; the thread checks this every poll and
    /// mid-render (it's the same `cancel: &AtomicBool` `avcore::process_watched_file` already
    /// accepts), same shape as `YoutubeDownloadState::youtube_download_cancel`.
    stop: Option<Arc<AtomicBool>>,
    /// Newest-first, same convention `Watch-Gameplay.ps1`'s own `$FileOrder` (reversed for
    /// display) uses.
    pub(crate) files: Vec<WatchedFileRow>,
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
        let mut prefs = load_prefs();
        apply_bundled_model_defaults(&mut prefs);
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
        let (scene_cut_detection_tx, scene_cut_detection_rx) = mpsc::unbounded_channel();
        let (matte_generation_tx, matte_generation_rx) = mpsc::unbounded_channel();
        let (tts_tx, tts_rx) = mpsc::unbounded_channel();
        let (youtube_download_tx, youtube_download_rx) = mpsc::unbounded_channel();
        let (watch_folder_tx, watch_folder_rx) = mpsc::unbounded_channel();
        let (sound_library_tx, sound_library_rx) = mpsc::unbounded_channel();
        let (telemetry_tx, telemetry_rx) = mpsc::unbounded_channel();
        telemetry::spawn_telemetry_writer(telemetry_rx, telemetry::telemetry_path());
        let telemetry_enabled_flag = Arc::new(AtomicBool::new(prefs.telemetry_enabled));
        telemetry::spawn_resource_sampler(
            telemetry_tx.clone(),
            Arc::clone(&telemetry_enabled_flag),
        );
        telemetry::spawn_gpu_sampler(telemetry_tx.clone(), Arc::clone(&telemetry_enabled_flag));
        let (update_check_tx, update_check_rx) = mpsc::unbounded_channel();
        // ER-01B: seeds the process-wide report builder and, only when the user has opted in
        // (`prefs.error_reporting_consent`, default disabled), spawns the delivery worker.
        let error_reporter =
            error_reporting::seed_error_reporting(prefs.locale, prefs.error_reporting_consent);
        // ER-01B's post-crash review offer: only set when a previous launch's panic hook left a
        // `crash_<unix>.txt` newer than the last one the user already reviewed.
        let pending_crash_review = crash_review::find_latest_unreviewed_crash(
            &crate::platform_log_dir(),
            prefs.last_reviewed_crash_unix,
        );
        let mut app = Self {
            screen: Screen::Home,
            tool: EditorTool::Select,
            locale: prefs.locale,
            projects,
            active_project: 0,
            selected_asset_id: None,
            export_jobs: export::load_queue(),
            prefs,
            error_reporter,
            telemetry_state: TelemetryState {
                telemetry_tx,
                telemetry_enabled_flag,
                last_preview_frame_telemetry: None,
            },
            render_tx,
            render_rx,
            active_renders: HashMap::new(),
            export_preview_cache: None,
            nested_sequence_render_cache: HashMap::new(),
            preview_state: PreviewState::default(),
            import_state: ImportState {
                import_tx,
                import_rx,
                pending_imports: 0,
                next_import_token: 0,
                pending_enrichment: HashMap::new(),
                auto_add_to_timeline: HashSet::new(),
            },
            sound_library_tracks: Vec::new(),
            sound_library_tx,
            sound_library_rx,
            transcribe_state: TranscribeState {
                transcribe_tx,
                transcribe_rx,
                transcribing_asset_id: None,
            },
            auto_reframe_state: AutoReframeState {
                auto_reframe_tx,
                auto_reframe_rx,
                auto_reframing_clip_id: None,
            },
            motion_tracking_state: MotionTrackingState {
                motion_tracking_tx,
                motion_tracking_rx,
                motion_tracking_clip_id: None,
            },
            scene_cut_detection_state: SceneCutDetectionState {
                scene_cut_detection_tx,
                scene_cut_detection_rx,
                scene_cut_detection_clip_id: None,
            },
            motion_track_region: MotionTrackRegionState {
                motion_track_center_x: 0.5,
                motion_track_center_y: 0.5,
                motion_track_width: 0.2,
                motion_track_height: 0.2,
                motion_track_search_radius: 0.08,
                picking_motion_track_region: false,
            },
            matte_generation_state: MatteGenerationState {
                matte_generation_tx,
                matte_generation_rx,
                matte_generating_clip_id: None,
            },
            tts_state: TtsState {
                tts_tx,
                tts_rx,
                tts_modal_text: None,
                tts_generating: false,
            },
            youtube_download_state: YoutubeDownloadState {
                youtube_download_tx,
                youtube_download_rx,
                youtube_modal_url: None,
                youtube_modal_format: YoutubeFormatChoice::Mp4,
                youtube_modal_mp4_quality: avcore::Mp4Quality::P720,
                youtube_modal_mp3_bitrate: avcore::Mp3Bitrate::K192,
                youtube_downloading: false,
                youtube_download_progress: 0.0,
                youtube_download_error: None,
                youtube_download_cancel: None,
            },
            watch_folder_state: WatchFolderState {
                tx: watch_folder_tx,
                rx: watch_folder_rx,
                watch_path: None,
                running: false,
                stop: None,
                files: Vec::new(),
            },
            selected_clip_id: None,
            undo_stack: avcore::undo::UndoStack::new(),
            undo_drag_active: false,
            selected_text_clip_id: None,
            text_color_edit: None,
            selected_shape_clip_id: None,
            drawing_shape_points: None,
            timeline_px_per_sec: 4.0,
            lib_panel_width,
            props_panel_width,
            timeline_height,
            thumbnail_state: ThumbnailState {
                thumbnail_tx,
                thumbnail_rx,
                thumbnail_textures: HashMap::new(),
                pending_thumbnails: HashSet::new(),
                thumbnail_last_used: HashMap::new(),
                failed_thumbnails: HashMap::new(),
                thumbnail_usage_clock: 0,
            },
            pending_asset_drop: None,
            media_search: String::new(),
            clipboard_clip: None,
            formatting_clipboard: None,
            multi_selected_clip_ids: HashSet::new(),
            active_multicam_group_id: None,
            active_smart_bin_id: None,
            editing_smart_bin: None,
            toasts: Vec::new(),
            prefs_open: false,
            prev_prefs_open: false,
            about_open: false,
            project_dirty: false,
            last_edit_instant: None,
            last_autosave_instant: None,
            autosave_restore_pending: None,
            crash_detected,
            pending_crash_review,
            pending_export_conflict: None,
            renaming_project: None,
            renaming_sequence: None,
            deleting_sequence: None,
            speed_ramp_dialog: None,
            saving_layer_template: None,
            applying_layer_template: None,
            layer_templates_menu_open: false,
            timeline_index_open: false,
            marker_search: String::new(),
            transcript_panel_open: false,
            transcript_search: String::new(),
            transcript_panel_state: TranscriptPanelState::default(),
            silence_review: None,
            transcript_review: None,
            transcript_search_project: false,
            binding_capture: None,
            update_check_tx,
            update_check_rx,
            update_check_status: UpdateCheckStatus::Checking,
        };
        if !app.prefs.sound_library_path.is_empty() {
            app.rescan_sound_library();
        }
        if !app.projects.is_empty() {
            app.load_panel_layout_for_active_project();
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
        self.undo_stack.clear();
        let project = self.active_project();
        info!(
            project_id = project.id,
            name = %project.name,
            path = ?project.file_path,
            "project opened"
        );
        let asset_id = project.media_library.first().map(|a| a.id);
        let multicam_group_id = project.timeline().multicam_groups.first().map(|g| g.id);
        self.select_asset(asset_id);
        self.active_multicam_group_id = multicam_group_id;
        self.active_smart_bin_id = None;
        self.load_panel_layout_for_active_project();
        self.screen = Screen::Editor;
    }

    /// Loads the Editor's live panel-layout fields (`lib_panel_width`/`props_panel_width`/
    /// `timeline_height`) for whichever project is now active — called from [`Self::open_project`]
    /// so switching projects doesn't leave the previous project's dragged layout on screen.
    /// Under [`LayoutScope::PerUser`] this is a no-op (the per-user `prefs` values already
    /// apply to every project uniformly). Under [`LayoutScope::PerProject`], falls back to the
    /// current live values (effectively the per-user defaults from `App::new`, unchanged) when
    /// the newly active project has never saved its own layout yet, rather than resetting to
    /// some arbitrary size.
    pub(crate) fn load_panel_layout_for_active_project(&mut self) {
        if self.prefs.layout_scope != LayoutScope::PerProject {
            return;
        }
        if let Some(layout) = self.active_project().panel_layout {
            self.lib_panel_width = layout.lib_panel_width;
            self.props_panel_width = layout.props_panel_width;
            self.timeline_height = layout.timeline_height;
        }
    }

    /// Captures the Editor's live panel-layout fields back into the active project's own
    /// `avcore::Project::panel_layout`, under [`LayoutScope::PerProject`] — called right before
    /// serializing a project (explicit save and autosave both), mirroring how the per-user
    /// scope's `App::save_prefs` already captures the same three fields into `prefs` at its own
    /// save points. A no-op under [`LayoutScope::PerUser`] (nothing to capture — the per-user
    /// scope never touches `Project::panel_layout` at all) or once nothing has changed.
    pub(crate) fn sync_panel_layout_into_active_project(&mut self) {
        if self.prefs.layout_scope != LayoutScope::PerProject {
            return;
        }
        let layout = avcore::PanelLayout {
            lib_panel_width: self.lib_panel_width,
            props_panel_width: self.props_panel_width,
            timeline_height: self.timeline_height,
        };
        if self.active_project().panel_layout != Some(layout) {
            self.active_project_mut().panel_layout = Some(layout);
        }
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
        let target_lufs = LUFS_PROFILES
            .get(self.prefs.lufs_profile)
            .map(|(_, target_lufs)| *target_lufs)
            .unwrap_or_else(|| avcore::SequenceExportSettings::default().target_lufs);
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
                    markers: Vec::new(),
                    multicam_groups: Vec::new(),
                },
                export_settings: avcore::SequenceExportSettings {
                    aspect_ratio: avcore::ExportAspectRatio::Original,
                    target_lufs,
                },
            }],
            active_sequence: 0,
            file_path: None,
            panel_layout: None,
            smart_bins: Vec::new(),
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
        self.reset_sequence_context();
    }

    /// Switches the active project's tab to `index` — what clicking a tab in the Editor's tab
    /// bar does. A no-op if `index` is out of range.
    pub fn select_sequence(&mut self, index: usize) {
        if index >= self.active_project().sequences.len()
            || index == self.active_project().active_sequence
        {
            return;
        }
        self.active_project_mut().active_sequence = index;
        self.reset_sequence_context();
    }

    /// Duplicates a sequence, including its complete timeline and export defaults, immediately
    /// after the source tab and switches to the duplicate. The localized copy name stays in
    /// the UI layer while [`Project::duplicate_sequence`] owns the data invariants.
    pub fn duplicate_sequence(&mut self, index: usize) {
        let Some(source_name) = self
            .active_project()
            .sequences
            .get(index)
            .map(|sequence| sequence.name.clone())
        else {
            return;
        };
        let name = crate::i18n::sequence_copy_name(self.locale, &source_name);
        if self
            .active_project_mut()
            .duplicate_sequence(index, name)
            .is_some()
        {
            self.reset_sequence_context();
        }
    }

    /// Deletes a sequence by stable id. The core model refuses to remove the project's last
    /// tab; a successful deletion switches to the nearest surviving tab and clears state tied
    /// to the removed/previous sequence.
    pub fn delete_sequence(&mut self, sequence_id: u64) {
        let active_id_before =
            self.active_project().sequences[self.active_project().active_sequence].id;
        let Some(index) = self
            .active_project()
            .sequences
            .iter()
            .position(|sequence| sequence.id == sequence_id)
        else {
            return;
        };
        if self.active_project().sequences.len() <= 1 {
            return;
        }
        if self.active_project_mut().remove_sequence(index) {
            let active_id_after =
                self.active_project().sequences[self.active_project().active_sequence].id;
            if active_id_after != active_id_before {
                self.reset_sequence_context();
            }
        }
    }

    /// Reorders tabs by index while preserving the active sequence's identity. Unlike a tab
    /// switch this deliberately keeps selections/preview alive because their owning sequence
    /// did not change, only its visual position did.
    pub fn move_sequence(&mut self, from_index: usize, target_index: usize) {
        let sequence_count = self.active_project().sequences.len();
        if from_index >= sequence_count
            || target_index >= sequence_count
            || from_index == target_index
        {
            return;
        }
        self.active_project_mut()
            .move_sequence(from_index, target_index);
    }

    /// Clears state whose ids/frames are scoped to the active sequence. Clip ids restart from
    /// one in each tab, so retaining any of these across a switch could target an unrelated
    /// clip with the same numeric id. Reordering does not call this because identity is stable.
    fn reset_sequence_context(&mut self) {
        self.selected_clip_id = None;
        self.selected_text_clip_id = None;
        self.text_color_edit = None;
        self.selected_shape_clip_id = None;
        self.multi_selected_clip_ids.clear();
        self.drawing_shape_points = None;
        self.motion_track_region.picking_motion_track_region = false;
        self.preview_state.preview_playing = false;
        self.preview_state.preview_frozen_since = None;
        self.undo_stack.clear();
        self.invalidate_preview_rendering();
    }

    /// Records the active sequence's current state as an undo point — call this immediately
    /// *before* applying a timeline-mutating edit (move/trim/split/effect-change/track-add/...),
    /// never after. See `spec/architecture/undo-redo.md` for which call sites need this.
    pub(crate) fn push_undo_snapshot(&mut self) {
        let sequence = self.active_project().active_sequence().clone();
        self.undo_stack.push(sequence);
    }

    /// Like [`App::push_undo_snapshot`], but coalesces a continuous drag (a slider/`DragValue`
    /// held down in the properties panel, which re-fires its setter every single frame while
    /// dragged) into exactly one undo step instead of one per frame. Pushes only the first time
    /// it's called since [`App::end_undo_drag_tracking_if_pointer_released`] last reset the
    /// flag — call sites are the shared per-clip-kind mutation dispatch points
    /// (`with_selected_clip_mut`, `text_clip_properties`'s and `shape_clip_properties`'s
    /// write-back), not each individual slider, so every effect-property setter gets this for
    /// free. Mirrors the `drag_started()`-gated push the timeline strip's trim/move drags use,
    /// but via a stateful flag rather than an `egui::Response` — the property setters are
    /// called through several layers of `bool`-returning helpers
    /// (`components::property_section` etc.) that don't thread a `Response` back to the caller.
    pub(crate) fn push_undo_snapshot_for_drag(&mut self) {
        if !self.undo_drag_active {
            self.push_undo_snapshot();
            self.undo_drag_active = true;
        }
    }

    /// Resets [`App::undo_drag_active`] once no pointer button is held — call once per frame
    /// from the Editor screen. Until the pointer is released, [`App::push_undo_snapshot_for_drag`]
    /// keeps treating further setter calls as the same in-progress drag.
    pub(crate) fn end_undo_drag_tracking_if_pointer_released(&mut self, pointer_down: bool) {
        if !pointer_down {
            self.undo_drag_active = false;
        }
    }

    /// Whether [`App::undo`] would do anything — drives the toolbar undo button's enabled state.
    pub fn can_undo(&self) -> bool {
        self.undo_stack.can_undo()
    }

    /// Whether [`App::redo`] would do anything — drives the toolbar redo button's enabled state.
    pub fn can_redo(&self) -> bool {
        self.undo_stack.can_redo()
    }

    /// Restores the active sequence to its state before the last recorded edit — what `Ctrl+Z`/
    /// the toolbar's undo button do. A no-op if there's nothing to undo. Clears clip selection
    /// since the restored timeline may not contain the currently selected clip id, and
    /// invalidates the preview pipeline so it reopens against the restored timeline.
    pub fn undo(&mut self) {
        let current = self.active_project().active_sequence().clone();
        let Some(previous) = self.undo_stack.undo(current) else {
            return;
        };
        *self.active_project_mut().active_sequence_mut() = previous;
        self.selected_clip_id = None;
        self.selected_text_clip_id = None;
        self.selected_shape_clip_id = None;
        self.multi_selected_clip_ids.clear();
        self.preview_state.preview_playing = false;
        self.preview_state.preview_frozen_since = None;
        self.invalidate_preview_rendering();
    }

    /// The inverse of [`App::undo`] — what `Ctrl+Y`/the toolbar's redo button do.
    pub fn redo(&mut self) {
        let current = self.active_project().active_sequence().clone();
        let Some(next) = self.undo_stack.redo(current) else {
            return;
        };
        *self.active_project_mut().active_sequence_mut() = next;
        self.selected_clip_id = None;
        self.selected_text_clip_id = None;
        self.selected_shape_clip_id = None;
        self.multi_selected_clip_ids.clear();
        self.preview_state.preview_playing = false;
        self.preview_state.preview_frozen_since = None;
        self.invalidate_preview_rendering();
    }

    /// Returns the active tab's persisted export defaults. Keeping this as a copied value
    /// avoids extending a project borrow through egui closures that may mutate the same app.
    pub fn active_sequence_export_settings(&self) -> avcore::SequenceExportSettings {
        self.active_project().sequences[self.active_project().active_sequence].export_settings
    }

    /// Updates the active tab's target aspect ratio and marks the project dirty for autosave.
    pub fn set_active_sequence_export_aspect_ratio(
        &mut self,
        aspect_ratio: avcore::ExportAspectRatio,
    ) {
        let active_sequence = self.active_project().active_sequence;
        self.active_project_mut().sequences[active_sequence]
            .export_settings
            .aspect_ratio = aspect_ratio;
    }

    /// Updates the active tab's normalization target and marks the project dirty for autosave.
    pub fn set_active_sequence_target_lufs(&mut self, target_lufs: f32) {
        let active_sequence = self.active_project().active_sequence;
        self.active_project_mut().sequences[active_sequence]
            .export_settings
            .target_lufs = target_lufs;
    }

    /// Applies `preset`'s aspect ratio + loudness target to the active tab in one action — the
    /// Fila (export queue) screen's platform-preset picker, so choosing "TikTok" sets both
    /// fields correctly instead of the user needing to know the right combination themselves.
    /// Still just calls the same two setters a manual pick would — the aspect-ratio and LUFS
    /// pickers stay live afterward for fine-tuning, nothing about picking a preset locks them.
    pub fn apply_platform_export_preset(&mut self, preset: avcore::PlatformExportPreset) {
        let (aspect_ratio, target_lufs) = preset.settings();
        self.set_active_sequence_export_aspect_ratio(aspect_ratio);
        self.set_active_sequence_target_lufs(target_lufs);
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

fn apply_bundled_model_defaults(prefs: &mut PrefsState) {
    fn use_bundled(path: &mut String, resource: avcore::BundledResource) {
        if !path.trim().is_empty() && Path::new(path).is_file() {
            return;
        }
        if let Some(bundled) = avcore::bundled_resource_path(resource) {
            *path = bundled.display().to_string();
        }
    }

    use_bundled(
        &mut prefs.whisper_model_path,
        avcore::BundledResource::WhisperBase,
    );
    use_bundled(
        &mut prefs.reframe_model_path,
        avcore::BundledResource::Reframe,
    );
    use_bundled(
        &mut prefs.background_removal_model_path,
        avcore::BundledResource::BackgroundRemoval,
    );
    use_bundled(&mut prefs.tts_model_path, avcore::BundledResource::TtsVoice);
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

/// Where [`App::spawn_youtube_download`] tells `yt-dlp` to save downloaded files before
/// importing them — same platform-config-dir shape as [`tts_output_dir`], a sibling
/// `youtube_downloads/` folder.
pub(self) fn youtube_downloads_dir() -> PathBuf {
    prefs_path()
        .parent()
        .map(|d| d.join("youtube_downloads"))
        .unwrap_or_else(|| PathBuf::from("youtube_downloads"))
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
pub(crate) fn prefs_path() -> PathBuf {
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

/// Upper bounds for filmstrip resources. At [`THUMBNAIL_MAX_DIM`] a worst-case square RGBA
/// texture is 36 KiB, so 512 entries cap decoded texture data around 18 MiB (usually lower for
/// landscape video). Extraction is deliberately much tighter: every request opens and seeks a
/// real GStreamer pipeline on its own worker thread.
pub(crate) const THUMBNAIL_CACHE_CAPACITY: usize = 512;
pub(crate) const THUMBNAIL_FAILURE_CAPACITY: usize = 128;
pub(crate) const THUMBNAIL_MAX_PENDING: usize = 16;
pub(crate) const THUMBNAIL_MAX_DIM: u32 = 96;
const THUMBNAIL_FALLBACK_FPS: f64 = 30.0;

pub(crate) fn thumbnail_fps(fps: Option<f32>) -> f64 {
    match fps {
        Some(value) if value.is_finite() && value > 0.0 => value as f64,
        _ => THUMBNAIL_FALLBACK_FPS,
    }
}

/// Quantizes a source timestamp to a stable frame key. This is adaptive without making the
/// cache zoom-specific: zoom changes which tile-center timestamps are requested, while the
/// same underlying source frame always maps to the same key.
pub(crate) fn thumbnail_frame_index(at_secs: f64, fps: Option<f32>) -> i64 {
    let at_secs = if at_secs.is_finite() {
        at_secs.max(0.0)
    } else {
        0.0
    };
    (at_secs * thumbnail_fps(fps)).floor() as i64
}

pub(crate) fn thumbnail_frame_time(frame_index: i64, fps: Option<f32>) -> f64 {
    frame_index.max(0) as f64 / thumbnail_fps(fps)
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump_export_queue();
        self.pump_import_queue();
        self.pump_sound_library_queue();
        self.pump_transcribe();
        self.pump_auto_reframe();
        self.pump_motion_tracking();
        self.pump_scene_cut_detection();
        self.pump_matte_generation();
        self.pump_text_to_speech();
        self.pump_youtube_download();
        self.pump_watch_folder();
        self.pump_update_check();
        self.pump_thumbnail_queue(ui.ctx());
        self.pump_preview_frame(ui.ctx());
        self.pump_autosave();
        self.pump_autosave_restore(ui.ctx());
        self.handle_dropped_files(ui.ctx());
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
        if self.preview_state.preview_playing {
            // Smooth video needs every-frame repaints; the 200ms throttle below would show
            // it as a slideshow.
            ui.ctx().request_repaint();
            self.sample_preview_frame_telemetry(ui.ctx());
        } else {
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }

        if self.preview_state.fullscreen_preview {
            screens::editor::fullscreen_preview_overlay(self, ui);
            return;
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
            Screen::WatchFolder => screens::watch_folder::show(self, ui),
        });
        self.show_prefs_modal(ui.ctx());
        self.show_about_modal(ui.ctx());
        self.show_rename_project_modal(ui.ctx());
        self.show_rename_sequence_modal(ui.ctx());
        self.show_speed_ramp_modal(ui.ctx());
        self.show_delete_sequence_modal(ui.ctx());
        self.show_text_color_modal(ui.ctx());
        self.show_export_conflict_modal(ui.ctx());
        self.show_crash_review_modal(ui.ctx());
        self.show_save_layer_template_modal(ui.ctx());
        self.show_layer_templates_menu(ui.ctx());
        self.show_apply_layer_template_modal(ui.ctx());
        self.show_tts_modal(ui.ctx());
        self.show_youtube_download_modal(ui.ctx());
        self.show_timeline_index_panel(ui.ctx());
        self.show_transcript_panel(ui.ctx());
        self.show_silence_review_modal(ui.ctx());
        self.show_transcript_proposals_modal(ui.ctx());
        self.show_smart_bin_modal(ui.ctx());
        self.show_toasts(ui.ctx());
        self.show_drop_hint_overlay(ui.ctx());
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
