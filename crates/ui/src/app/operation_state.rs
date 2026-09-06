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

//! Session and background-operation state owned by [`super::App`].

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use eframe::egui;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::{
    AutoReframeEvent, DynamicReframeEvent, ImportEvent, MatteGenerationEvent, MotionTrackEvent,
    NestedSequenceEvent, PreviewZoom, PrivacyBlurGenerationEvent, SceneCutEvent, ThumbnailKey,
    ThumbnailReady, TranscribeEvent, TtsEvent, VoiceCleanupPreviewEvent, WatchFolderEvent,
    WatchedFileRow, YoutubeDownloadEvent, YoutubeFormatChoice,
};

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
    /// A clone of the same decoded, effects-applied frame just uploaded into `preview_texture`
    /// — kept around only because the texture upload doesn't hand the pixels back, and the
    /// preview transport row's snapshot button (`App::save_preview_snapshot`) needs a real CPU
    /// buffer to write out as a PNG. Reset alongside `preview_texture` for the same reasons.
    pub(crate) last_frame: Option<avcore::preview::VideoFrame>,
    /// The preview panel's zoom level — see [`PreviewZoom`].
    pub(crate) zoom: PreviewZoom,
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
    /// The currently previewed clip's id, plus its rolling [`avcore::DeflickerHistory`] — kept
    /// separate from `preview_clip_id` (which tracks the pipeline's own open/reopen state, not
    /// this effect specifically) so [`App::pump_preview_frame`] can reset the history to empty
    /// the moment the previewed clip changes, the same "one clip's temporal state must never
    /// leak into the next" reasoning [`avcore::DeflickerHistory`]'s own doc comment gives.
    pub(crate) preview_deflicker_history: (Option<u64>, avcore::DeflickerHistory),
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
    /// Whether playback should restart from the beginning instead of stopping when it runs off
    /// the end of the timeline — the OCA mockup's transport-row loop toggle
    /// (`spec/architecture/editor-ui-visual-redesign.md`'s Program monitor mapping). `false` by
    /// default, like every other transport setting here. Checked in
    /// [`App::ensure_preview_loaded`]'s "nothing covers the new playhead" branch.
    pub(crate) loop_enabled: bool,
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

/// Dynamic-reframe (CF-04) background-job state — same pattern as [`AutoReframeState`], kept
/// separate since a static and a dynamic run could otherwise race each other's `Option<u64>`.
pub(crate) struct DynamicReframeState {
    pub(crate) dynamic_reframe_tx: UnboundedSender<DynamicReframeEvent>,
    pub(crate) dynamic_reframe_rx: UnboundedReceiver<DynamicReframeEvent>,
    /// The timeline clip id a background dynamic-reframe run is currently computing crop
    /// keyframes for, if any — only one runs at a time, same shape as
    /// `AutoReframeState::auto_reframing_clip_id`.
    pub(crate) dynamic_reframing_clip_id: Option<u64>,
}

/// Tracks [`App::spawn_shorts_pack`]'s own sequential auto-reframe pre-pass — one background
/// dynamic-reframe run per un-reframed video clip a highlight window touches, processed one at a
/// time (reusing [`DynamicReframeState`]'s existing single-in-flight guard rather than a second
/// one) before the actual per-short export queueing runs. `pending_clip_ids[0]` is always the
/// clip currently in flight; `App::pump_dynamic_reframe` pops it on completion/failure and moves
/// on to the next, or — once empty — proceeds straight to queueing the exports.
pub(crate) struct ShortsPackReframeState {
    pub(crate) pending_clip_ids: Vec<u64>,
    pub(crate) output_dir: std::path::PathBuf,
}

/// A [`avcore::motion_template::GraphicTemplate`] loaded via [`App::load_graphic_template_from_
/// file`] that declares at least one [`avcore::motion_template::TemplateParameter`] — staged
/// here for [`App::show_apply_graphic_template_modal`] instead of applying immediately (a
/// parameterless template still applies right away, no modal involved). `text_values`/
/// `color_values` are pre-seeded with one entry per declared parameter of the matching kind (an
/// empty string / opaque white) so the modal always has something bound to edit; confirming
/// builds a `HashMap<String, ParameterValue>` from these two maps for [`App::
/// apply_graphic_template`].
pub(crate) struct PendingGraphicTemplateApply {
    pub(crate) template: avcore::motion_template::GraphicTemplate,
    pub(crate) text_values: std::collections::HashMap<String, String>,
    pub(crate) color_values: std::collections::HashMap<String, [u8; 4]>,
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

/// CF-03 slice 3 (`spec/architecture/competitive-feature-plan.md`) background-job state for the
/// voice-cleanup A/B preview — same channel/one-in-flight pattern as [`MotionTrackingState`].
/// `result` is the last completed render (if any), tagged with the clip id it belongs to so
/// switching the selected clip doesn't show a stale A/B comparison for a different clip's own
/// parameters. `player` is a playback pipeline dedicated to the two rendered sample files —
/// deliberately separate from [`PreviewState::preview`] (the main timeline's own pipeline) so
/// auditioning a sample never disturbs the timeline's playhead/pipeline state.
pub(crate) struct VoiceCleanupPreviewState {
    pub(crate) tx: UnboundedSender<VoiceCleanupPreviewEvent>,
    pub(crate) rx: UnboundedReceiver<VoiceCleanupPreviewEvent>,
    pub(crate) rendering_clip_id: Option<u64>,
    pub(crate) result: Option<(
        u64,
        avcore::voice_cleanup_preview::VoiceCleanupPreviewResult,
    )>,
    pub(crate) player: Option<avcore::preview::Preview>,
    /// Whether `player` currently holds the processed sample (`true`) or the bypassed one
    /// (`false`) — purely so the properties panel can highlight whichever of the two "▶" buttons
    /// is the one actually playing right now.
    pub(crate) player_is_processed: bool,
}

/// Background-job state for rendering compound clips (nested sequences) off the UI thread — see
/// [`App::materialize_nested_sequences_for_active_sequence`]. Deliberately separate from
/// `App::nested_sequence_render_cache` (the actual rendered-file cache, keyed by nested
/// sequence id and shared by every recursive caller of
/// [`avcore::nested_sequence::materialize_nested_sequences`]) — this struct only tracks the
/// *dispatch* bookkeeping (what's in flight, what was last returned, what input produced it),
/// same "one-shot background job state, not the underlying domain cache" split
/// `MotionTrackingState` already keeps from `MotionTrackRegionState`. Closes ROADMAP.md P4 item
/// 35's one remaining honest gap: materializing a nested sequence used to block the whole UI
/// thread on the first hit after an edit (a real, if bounded, FFmpeg re-encode).
pub(crate) struct NestedSequenceRenderState {
    pub(crate) nested_sequence_tx: UnboundedSender<NestedSequenceEvent>,
    pub(crate) nested_sequence_rx: UnboundedReceiver<NestedSequenceEvent>,
    /// Active-sequence ids a background render is currently running for — a `HashSet`, not a
    /// single `Option<u64>` like `MotionTrackingState::motion_tracking_clip_id`, because
    /// switching Editor tabs mid-render is a real case here (motion-tracking has no equivalent:
    /// it's always scoped to one already-selected clip, never "whichever sequence is active
    /// right now").
    pub(crate) nested_sequence_rendering_ids: HashSet<u64>,
    /// The last successfully (or unsuccessfully) materialized synthetic asset list per active
    /// sequence id — what [`App::materialize_nested_sequences_for_active_sequence`] returns
    /// immediately while a fresh render (if one was just dispatched) is still in flight. Empty
    /// for a sequence that failed to materialize, matching the old synchronous function's own
    /// "failure degrades to no nested clips resolved" behavior.
    pub(crate) nested_sequence_last_result: HashMap<u64, Vec<avcore::MediaAsset>>,
    /// The `Project::sequences` snapshot that produced `nested_sequence_last_result`'s entry for
    /// the same key — compared wholesale against the *current* `Project::sequences`, not just
    /// the active sequence's own timeline, since a compound clip's rendered content depends on
    /// whatever *other* `Sequence` it points at (the exact reasoning `ExportPreviewCache`'s own
    /// doc comment already gives for the same "every sequence, not just the active one" choice).
    /// Latched on both success *and* failure, so a persistently broken nested-sequence reference
    /// doesn't get re-dispatched to a new background thread every single frame forever.
    pub(crate) nested_sequence_last_input: HashMap<u64, Vec<avcore::project::Sequence>>,
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

/// CF-09 privacy-blur matte-generation background-job state — same pattern as
/// [`MatteGenerationState`].
pub(crate) struct PrivacyBlurGenerationState {
    pub(crate) privacy_blur_generation_tx: UnboundedSender<PrivacyBlurGenerationEvent>,
    pub(crate) privacy_blur_generation_rx: UnboundedReceiver<PrivacyBlurGenerationEvent>,
    /// The timeline clip id a background privacy-blur matte-generation run is currently
    /// computing a matte for, if any — only one runs at a time, same shape as
    /// [`MatteGenerationState::matte_generating_clip_id`].
    pub(crate) privacy_blur_generating_clip_id: Option<u64>,
}

/// CF-09 privacy-blur seed-rectangle session state — the region [`App::
/// spawn_apply_privacy_blur_for_selected_clip`] seeds `avcore::mask_propagation` from, entered
/// numerically (no preview click-and-drag picker yet — a real, separate follow-up, same
/// deliberate v1 scope cut `spec/architecture/competitive-feature-plan.md`'s CF-09 "UI
/// integration design" section documents). `center_x`/`center_y` are a `0.0..=1.0` fraction of
/// the *source* frame (same convention `MotionTrackRegionState::motion_track_center_x`/`_y`
/// already uses); `width`/`height` are each independently a fraction of the frame's shorter
/// dimension (`avcore::mask_propagation::propagate_mask_by_translation`'s own
/// `template_width_frac`/`template_height_frac`, which are literally `avcore::track_region`'s
/// own parameters it tracks with). Not persisted — a fresh session/clip selection starts back at
/// the centered default, same non-persistence rationale `MotionTrackRegionState`'s own doc
/// comment gives.
pub(crate) struct PrivacyBlurRegionState {
    pub(crate) privacy_blur_center_x: f32,
    pub(crate) privacy_blur_center_y: f32,
    pub(crate) privacy_blur_width: f32,
    pub(crate) privacy_blur_height: f32,
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
    /// Import tokens whose asset should land on a brand-new track (never an existing one) the
    /// moment its `ImportEvent::AssetReady` lands — used for a multi-file drop/drag onto the
    /// Editor ([`App::handle_dropped_files`]) so simultaneous files land on parallel tracks
    /// (all starting at `0.0`) instead of being silently concatenated one after another onto a
    /// single track, which is what [`App::add_asset_to_timeline`]'s always-reuse-first-track
    /// resolution would otherwise do for every file in the batch.
    pub(crate) auto_add_to_new_track: HashSet<u64>,
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
    pub(crate) tx: UnboundedSender<WatchFolderEvent>,
    pub(crate) rx: UnboundedReceiver<WatchFolderEvent>,
    pub(crate) watch_path: Option<PathBuf>,
    /// `true` while the background polling thread is running — only one watch session at a
    /// time.
    pub(crate) running: bool,
    /// Set when `running` starts, cleared when it stops; the thread checks this every poll and
    /// mid-render (it's the same `cancel: &AtomicBool` `avcore::process_watched_file` already
    /// accepts), same shape as `YoutubeDownloadState::youtube_download_cancel`.
    pub(crate) stop: Option<Arc<AtomicBool>>,
    /// Newest-first, same convention `Watch-Gameplay.ps1`'s own `$FileOrder` (reversed for
    /// display) uses.
    pub(crate) files: Vec<WatchedFileRow>,
}
