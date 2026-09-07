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

//! Persisted preferences and UI state value types.

use eframe::egui;

use super::error_reporting;

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
/// `Hand` also changes drag behavior: `timeline_panel` gates the ruler's scrub-drag and every
/// clip's click/drag `Sense` down to `hover()` while it's active, so a drag anywhere in the
/// timeline canvas falls through to the horizontal-pan `ScrollArea`s instead
/// (`spec/architecture/editor-ui-visual-redesign.md`'s Left icon rail section).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTool {
    Select,
    /// Click a timeline clip to split it at that click's timeline position — `CINECUT_UI_UX_
    /// SPEC_v1.0.md`'s product-decisions addendum, Section 4. Distinct from the existing
    /// playhead-position `App::split_at_playhead` (still reachable via its own shortcut/menu
    /// item unchanged); this one splits wherever the click landed, independent of the playhead.
    Razor,
    Trim,
    /// Trim without leaving a gap — later clips on the same track shift to fill it. Kept as a
    /// real tool mode (not removed) even though the product-decisions addendum's "final toolbar"
    /// list (Section 2) doesn't include it as a *button* — that section is about what the
    /// toolbar exposes, not a mandate to delete working Ripple-trim behavior; there was no
    /// separate keyboard/menu path to it before, so the mode itself stays reachable via
    /// `App::tool` even without its own toolbar button. See `screens::editor::toolbar`'s own
    /// note on why Ripple/Roll/Slip/Slide lost their buttons.
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
    /// Click the Program Monitor to create a new Text Graphic at that position, immediately
    /// selected and ready to type — Section 6 of the product-decisions addendum. Does not
    /// require click-and-drag (explicitly out of scope for this pass).
    Text,
    /// Section 8's Effects Tool: activates Effects mode and focuses the Effects Panel
    /// (`PropertiesTab::Effects`) — explicitly *not* a paint/place-in-monitor interaction
    /// (Section 12's own non-goals).
    Effects,
    /// Pans the timeline canvas horizontally by dragging anywhere in it — clips/ruler stop
    /// reacting to drags while this is active (see [`EditorTool`]'s own doc comment).
    Hand,
    /// Section 13's Zoom Tool: controls the Program Monitor *viewport* zoom only (never clip
    /// Transform/Scale — Section 13's own "critical distinction"). Click zooms in one step
    /// around the cursor; Alt/Option+click zooms out; wheel zooms continuously; double-click
    /// resets to Fit. Does not pan (the Hand tool's job).
    Zoom,
}

/// Which group of clip properties the properties panel's tab strip is currently showing —
/// a pure UI grouping of the same [`avcore::timeline::ClipInstance`] fields the panel already
/// edits (`spec/architecture/editor-ui-visual-redesign.md`'s Inspector mapping), not a new
/// data model. Not persisted: resets to `Inspector` on every app launch like `tool` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PropertiesTab {
    #[default]
    Inspector,
    Effects,
    Audio,
}

/// How the media library panel lays out its asset list — a pure UI presentation toggle over
/// the same [`avcore::media::MediaAsset`] entries (`spec/architecture/editor-ui-visual-
/// redesign.md`'s Media library mapping: "Grid/list view toggle ... pure UI, no new `App`/
/// `avcore` state beyond a `bool`/enum view-mode field"), not a new data model. Not persisted:
/// resets to `List` on every app launch, same as `tool`/`properties_tab`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaViewMode {
    #[default]
    List,
    Grid,
}

/// The Editor preview panel's zoom level (`spec/architecture/editor-ui-visual-redesign.md`'s
/// Program monitor mapping: "50%/Fit/100%" controls) — a pure display-size choice, not a new
/// `avcore` concept. `Fit` (the default, and the only behavior that existed before this field)
/// always fills the available panel space at the sequence's own aspect ratio, exactly as
/// `layer_transform_preview` always has. `Percent50`/`Percent100` instead size the canvas off
/// the sequence's real pixel dimensions, clamped back down to the available panel space when
/// the canvas is bigger than the panel — this preview has no scrollable viewport, so an
/// oversized zoom for a large canvas resolution silently caps back to whatever `Fit` would have
/// produced rather than overflowing the panel or needing scroll support (a real, separate
/// follow-up if true 1:1-pixel scrolling is ever wanted). Not persisted: resets to `Fit` on
/// every app launch, same as `tool`/`media_view_mode`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PreviewZoom {
    #[default]
    Fit,
    Percent50,
    Percent100,
    /// An arbitrary zoom multiplier (`1.0` = 100%) — what the Zoom Tool's click/wheel
    /// interaction sets (`screens::editor::layer_transform_preview`), distinct from the three
    /// fixed presets above the quick-select row still offers. No `Eq` derive here (a `f32`
    /// field can't implement it); nothing in this codebase used `PreviewZoom` as a map/set key.
    Custom(f32),
}

/// Which filter the Media library panel's asset list is currently showing — a single-selection
/// model unifying the pre-existing smart-bin chip row with the two new Favorites/Recent chips
/// (`spec/architecture/editor-ui-visual-redesign.md`'s Media panel mapping), replacing the old
/// bare `Option<u64>` (`None`/`Some(bin_id)`) it grew out of. `All` shows every asset,
/// unfiltered. Not persisted: resets to `All` on project switch, same as
/// `active_multicam_group_id` before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaLibraryFilter {
    #[default]
    All,
    SmartBin(u64),
    Favorites,
    Recent,
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
    /// (NVENC/Quick Sync/AMF/VAAPI/VideoToolbox) with an automatic CPU (libopenh264) fallback,
    /// or a specific choice. See
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
    /// CF-02 slice 5's per-game event allowlists — app-wide, not per-project, since the whole
    /// point is reusing the same profile (which event kinds to import, default pre/post-roll
    /// seconds) across every sidecar for the same game. Applied by
    /// [`App::import_gameplay_events`] via [`avcore::gameplay_events::apply_game_event_allowlist`]
    /// when a sidecar's own `game_id` matches one of these entries; an unconfigured game (no
    /// match) is imported unrestricted, same as before this field existed.
    #[serde(default)]
    pub game_event_allowlists: Vec<avcore::gameplay_events::GameEventAllowlist>,
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
            game_event_allowlists: Vec::new(),
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

#[cfg(test)]
#[path = "state/state_test.rs"]
mod tests;
