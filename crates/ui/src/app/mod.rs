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
    AudioRole, ClipInstance, ExportJob, ExportJobStatus, MediaAsset, Project, RenderOutcome,
    TrackKind,
};
use eframe::egui;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use tracing::{debug, error, info, warn};

use crate::i18n::{Locale, Text};
use crate::screens;
use crate::theme;

mod auto_reframe;
mod autosave;
mod background_removal;
mod clip_modals;
mod clip_props;
mod collab_bundle;
mod color;
mod crash_review;
mod dynamic_reframe;
pub mod effects_panel;
pub(crate) mod error_reporting;
pub mod export;
mod gameplay_events;
mod highlight_detection;
mod import;
mod interchange_export;
mod layer_templates;
mod lifecycle;
mod markers;
mod modals;
mod motion_template;
mod motion_tracking;
mod multicam;
mod operation_state;
mod preview;
mod project_timeline_modals;
mod scene_detection;
mod shorts_pack;
mod silence_review;
mod smart_bins;
mod sound_library;
mod state;
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
mod voice_cleanup_preview;
mod watch_folder;
mod worker_events;
mod youtube_download;

pub(crate) use color::{format_color_hex, TextColorEdit, TextColorTarget};
pub(crate) use operation_state::{
    AutoReframeState, DynamicReframeState, ImportState, MatteGenerationState,
    MotionTrackRegionState, MotionTrackingState, NestedSequenceRenderState,
    PendingGraphicTemplateApply, PreviewState, SceneCutDetectionState, ShortsPackReframeState,
    TelemetryState, ThumbnailState, TranscribeState, TranscriptPanelState, TtsState,
    VoiceCleanupPreviewState, WatchFolderState, YoutubeDownloadState,
};
pub use state::{
    BindableAction, EditorTool, KeyBindings, KeyCombo, LayoutScope, MediaLibraryFilter,
    MediaViewMode, PrefsState, PreviewZoom, PropertiesTab, Screen,
};
use worker_events::{
    AutoReframeEvent, DynamicReframeEvent, ImportEvent, MatteGenerationEvent, MotionTrackEvent,
    NestedSequenceEvent, RenderEvent, SceneCutEvent, ThumbnailKey, ThumbnailReady, TranscribeEvent,
    TtsEvent, UpdateCheckEvent, VoiceCleanupPreviewEvent, WatchFolderEvent, YoutubeDownloadEvent,
};
pub use worker_events::{AvailableUpdate, UpdateCheckStatus, YoutubeFormatChoice};
pub(crate) use worker_events::{WatchFolderFileStatus, WatchedFileRow};

/// Loudness normalization presets offered in Preferences and shown on the Editor's "Ao
/// exportar" panel. `(label, target LUFS)` — the label is a loanword-heavy string
/// (`"YouTube"`/`"Podcast"`/`"Broadcast"`) that reads the same in both locales, so unlike
/// most UI text it isn't routed through `i18n::Text`.
pub const LUFS_PROFILES: [(&str, f32); 3] = [
    ("-14 LUFS | YouTube", -14.0),
    ("-16 LUFS | Podcast", -16.0),
    ("-23 LUFS | Broadcast", -23.0),
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

/// Slider bounds for the properties panel's CF-03 voice-cleanup "noise floor" control
/// ([`avcore::timeline::ClipInstance::voice_cleanup_noise_floor_db`], `afftdn`'s `nf`) — wide
/// enough either side of the proven `-30.0` script default to cover a noisier or cleaner mic
/// input without reaching values `afftdn` itself would reject.
pub const VOICE_CLEANUP_NOISE_FLOOR_RANGE: std::ops::RangeInclusive<f32> = -80.0..=-10.0;

/// Slider bounds for the properties panel's voice-cleanup compressor threshold control
/// ([`avcore::timeline::ClipInstance::voice_cleanup_compressor_threshold_db`], `acompressor`'s
/// `threshold`) — dB, around the proven `-18.0` script default.
pub const VOICE_CLEANUP_COMPRESSOR_THRESHOLD_RANGE: std::ops::RangeInclusive<f32> = -40.0..=0.0;

/// Slider bounds for the properties panel's voice-cleanup compressor ratio control
/// ([`avcore::timeline::ClipInstance::voice_cleanup_compressor_ratio`], `acompressor`'s `ratio`)
/// — `1.0` is no compression, the proven script default is `3.0`, and `acompressor` itself caps
/// at `20.0`.
pub const VOICE_CLEANUP_COMPRESSOR_RATIO_RANGE: std::ops::RangeInclusive<f32> = 1.0..=20.0;

/// Slider bounds for the properties panel's voice-cleanup ceiling control
/// ([`avcore::timeline::ClipInstance::voice_cleanup_ceiling_linear`], `alimiter`'s `limit`) —
/// linear (not dB), matching that field's own `0.0..=1.0` convention; floored well above `0.0`
/// since a limiter ceiling near silence isn't a meaningful setting.
pub const VOICE_CLEANUP_CEILING_RANGE: std::ops::RangeInclusive<f32> = 0.5..=1.0;

/// Slider bounds for the properties panel's motion-tracking region width/height controls
/// (`App::motion_track_width`/`_height`, `avcore::track_region`'s `template_width_frac`/
/// `template_height_frac`) — kept well under `1.0` so the template can always slide within the
/// frame during search, and above a few percent so it still covers enough texture to match
/// against. Shared by both the width and height sliders.
pub const MOTION_TRACK_SIZE_RANGE: std::ops::RangeInclusive<f32> = 0.05..=0.6;

/// Slider bounds for the properties panel's motion-tracking search-radius control
/// (`App::motion_track_search_radius`, `avcore::track_region`'s `search_radius_frac`).
pub const MOTION_TRACK_SEARCH_RADIUS_RANGE: std::ops::RangeInclusive<f32> = 0.02..=0.3;

// ClipFormatting is defined in core::timeline and re-exported as avcore::ClipFormatting;
// the type alias below is kept for in-module readability only.
use avcore::ClipFormatting;

/// The whole application's state: which screen is showing, the loaded projects, the export
/// queue, and user preferences. `eframe` owns one instance of this for the app's lifetime
/// and calls [`App::ui`](eframe::App::ui) on it every frame.
pub struct App {
    pub screen: Screen,
    pub tool: EditorTool,
    /// The properties panel's active tab (Inspector/Effects/Audio) — see [`PropertiesTab`].
    pub properties_tab: PropertiesTab,
    /// The media library panel's asset layout (list rows vs. a thumbnail grid) — see
    /// [`MediaViewMode`].
    pub media_view_mode: MediaViewMode,
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
    /// Wall-clock start time of each currently-rendering job, keyed by job id — set when a job
    /// dispatches to a worker ([`App::pump_export_queue`]), cleared alongside `active_renders`
    /// at Done/Failed/Cancelled. Transient UI state, not persisted with the job queue. Used by
    /// the Fila (export queue) screen to render "elapsed" (`CINECUT_PRODUCT_DECISIONS_v1.0.md`
    /// section 17) — explicitly no ETA, per that doc's section 18.
    pub export_job_started_at: HashMap<u64, std::time::Instant>,
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
    /// Dispatch bookkeeping for the background thread(s) that actually populate
    /// `nested_sequence_render_cache` — see [`NestedSequenceRenderState`]'s own doc comment.
    pub(crate) nested_sequence_render_state: NestedSequenceRenderState,
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
    /// Dynamic-reframe (CF-04) background-job channel/clip-tracking state — same pattern.
    pub(crate) dynamic_reframe_state: DynamicReframeState,
    /// Shorts Pack's own sequential auto-reframe queue (CF-04's "Shorts Pack integration" slice)
    /// — `Some` only while [`App::spawn_shorts_pack`] is waiting on un-reframed clips one at a
    /// time before it queues the actual exports. See [`ShortsPackReframeState`]'s own doc
    /// comment.
    pub(crate) shorts_pack_reframe_state: Option<ShortsPackReframeState>,
    /// A loaded graphic template awaiting its parameter values — see
    /// [`PendingGraphicTemplateApply`]'s own doc comment. `None` when no "🖼 Load graphic
    /// template" flow is in progress, including the common case of a parameterless template,
    /// which never sets this at all.
    pub(crate) pending_graphic_template_apply: Option<PendingGraphicTemplateApply>,
    /// Motion-tracking background-job channel/clip-tracking state — same pattern.
    pub(crate) motion_tracking_state: MotionTrackingState,
    /// CF-03 slice 3 voice-cleanup A/B preview background-job/playback state — see
    /// [`VoiceCleanupPreviewState`]'s own doc comment.
    pub(crate) voice_cleanup_preview_state: VoiceCleanupPreviewState,
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
    /// `timeline_px_per_sec` as of the last frame a thumbnail request was actually allowed to
    /// fire — compared each frame against the current value to detect an in-progress zoom.
    /// Every filmstrip tile's source frame index is a function of `timeline_px_per_sec`
    /// ([`crate::screens::editor::timeline_panel::filmstrip_frame_index_for_tile`]), so a
    /// continuous zoom drag changes nearly every visible tile's cache key on every single
    /// frame — without this debounce, that fires one background extraction thread per tile
    /// per frame throughout the whole gesture, almost all of them wasted on a tile whose key
    /// is already stale again a frame later, and paints blank gaps instead of a thumbnail for
    /// the whole gesture. Not persisted — same transient-UI category as `timeline_px_per_sec`
    /// itself.
    pub timeline_thumbnail_zoom_settled_px_per_sec: f32,
    /// When `timeline_px_per_sec` last differed from
    /// `timeline_thumbnail_zoom_settled_px_per_sec` — i.e. when the in-progress zoom was last
    /// still moving. `screens::editor::timeline_panel` only lets new thumbnail requests through
    /// once `Instant::now()` is `TIMELINE_THUMBNAIL_ZOOM_DEBOUNCE` past this, so requests resume
    /// shortly after the zoom gesture settles rather than firing continuously while it's still
    /// moving. `None` means settled (no pending debounce).
    pub timeline_zoom_changed_at: Option<std::time::Instant>,
    /// Horizontal pan of the timeline canvas (ruler + clip area, not the track-label gutter),
    /// in pixels — how far the shared content origin has scrolled right. Mirrored every frame
    /// across the ruler's and every track's own small `ScrollArea::horizontal()` (forced via
    /// `.horizontal_scroll_offset`, then read back from whichever one the user actually
    /// scrolled/dragged this frame — see `screens::editor::timeline_panel`'s own comment on the
    /// mechanism) so they all stay in lockstep one frame behind the interacted one, imperceptible
    /// at normal frame rates. The [`EditorTool::Hand`] tool is what makes a plain drag (not just
    /// scroll-wheel/trackpad) pan these areas — see [`EditorTool`]'s own doc comment. Not
    /// persisted, resets to `0.0` on project switch/launch, same as `timeline_px_per_sec`.
    pub timeline_pan_px: f32,
    /// Track ids currently collapsed to `COLLAPSED_TRACK_ROW_HEIGHT` in the timeline strip —
    /// toggled by the track header's own collapse/expand button (`screens::editor::
    /// timeline_panel`). Transient UI presentation state, same category as `selected_clip_id`
    /// (see `ARCHITECTURE.md`'s persistent-vs-transient split) — never serialized, and not tied
    /// to a specific project/sequence, so it just naturally resets if a collapsed track's id
    /// never recurs.
    pub collapsed_track_ids: std::collections::HashSet<u64>,
    /// Timeline-level magnetic-snap toggle — Section 56's own Snapping spec ("a timeline-level
    /// toggle... click magnet icon"). Defaults to `true` (matches this codebase's own
    /// pre-existing default snap behavior). Alt still temporarily inverts whichever way this is
    /// set (`screens::editor::timeline_panel`'s own `snap_enabled` computation) rather than
    /// replacing this toggle outright. Transient, not serialized — same category as
    /// `collapsed_track_ids` above.
    pub snap_enabled: bool,
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
    /// The Media library panel's active asset filter — see [`MediaLibraryFilter`].
    pub media_filter: MediaLibraryFilter,
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
    /// When `Some((track_id, buf))`, a rename modal is shown for that timeline track — Section
    /// 49's Track More Menu "Rename Track". Committed on Enter/confirm, discarded on Escape/
    /// cancel. Same shape as `renaming_sequence`/`renaming_project`.
    pub renaming_track: Option<(u64, String)>,
    /// `(track_id, name)` staged while the destructive track-delete confirmation modal is open
    /// — Section 49's own "Deleting a track containing clips requires confirmation". Stored by
    /// id, same reasoning as `deleting_sequence`.
    pub deleting_track: Option<(u64, String)>,
    /// Set to the clip id right after the Text Tool creates a new, still-empty Text Graphic
    /// (`App::add_text_clip_at`) — an Escape press while this is `Some` and that clip's text is
    /// still blank deletes it outright (`CINECUT_PRODUCT_DECISIONS_v1.0.md` Section 6's "empty
    /// text" rule); any other case (text typed, different clip selected, tool switched) just
    /// clears this without deleting anything. Not serialized — transient UI interaction state.
    pub text_tool_pending_empty_clip_id: Option<u64>,
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

impl App {
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

    /// The [`AudioRole`] of the track `selected_clip_id` lives on, if any — lets the properties
    /// panel suggest enabling CF-03 voice cleanup when a clip sits on a `Mic`-role track but
    /// wasn't defaulted to it at creation time (e.g. the track's role was reassigned after the
    /// clip already existed — `App::add_asset_to_timeline`'s own "Mic by default" only applies
    /// at creation, never retroactively).
    pub fn selected_clip_track_audio_role(&self) -> Option<AudioRole> {
        let id = self.selected_clip_id?;
        self.active_project()
            .timeline()
            .tracks
            .iter()
            .find(|t| t.clips.iter().any(|c| c.id == id))
            .map(|t| t.audio_role)
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
        self.media_filter = MediaLibraryFilter::All;
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

    /// Flips `asset_id`'s [`avcore::media::MediaAsset::favorited`] flag — what clicking the
    /// star button on a media library row/tile does. A no-op if `asset_id` isn't in the active
    /// project's media library.
    pub fn toggle_asset_favorite(&mut self, asset_id: u64) {
        if let Some(asset) = self
            .active_project_mut()
            .media_library
            .iter_mut()
            .find(|a| a.id == asset_id)
        {
            asset.favorited = !asset.favorited;
        }
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
            recent_asset_ids: Vec::new(),
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
        self.pump_nested_sequence_renders();
        self.pump_import_queue();
        self.pump_sound_library_queue();
        self.pump_transcribe();
        self.pump_auto_reframe();
        self.pump_dynamic_reframe();
        self.pump_motion_tracking();
        self.pump_voice_cleanup_preview();
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
        self.show_rename_track_modal(ui.ctx());
        self.show_delete_track_modal(ui.ctx());
        self.show_speed_ramp_modal(ui.ctx());
        self.show_delete_sequence_modal(ui.ctx());
        self.show_text_color_modal(ui.ctx());
        self.show_export_conflict_modal(ui.ctx());
        self.show_crash_review_modal(ui.ctx());
        self.show_save_layer_template_modal(ui.ctx());
        self.show_layer_templates_menu(ui.ctx());
        self.show_apply_layer_template_modal(ui.ctx());
        self.show_apply_graphic_template_modal(ui.ctx());
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
