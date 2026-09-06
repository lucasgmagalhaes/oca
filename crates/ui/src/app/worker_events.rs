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

//! Messages exchanged between background workers and the UI thread.

use std::collections::HashMap;
use std::path::PathBuf;

use avcore::MediaAsset;

/// A message from a background render worker thread (see [`App::pump_export_queue`])
/// back to the UI thread, sent over a plain `tokio::sync::mpsc` channel used purely
/// synchronously (`try_recv` on the UI side, `send` on the worker side) — no async runtime
/// needed, matching the execution plan's "tokio + canais assíncronos" without pulling egui's
/// synchronous frame loop into async code.
pub(crate) enum RenderEvent {
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
pub(crate) enum ImportEvent {
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
pub(crate) enum TranscribeEvent {
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
pub(crate) enum AutoReframeEvent {
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

/// A message from a background dynamic-reframe worker thread (see
/// [`App::spawn_dynamic_reframe_selected_clip`]) back to the UI thread.
pub(crate) enum DynamicReframeEvent {
    Done {
        clip_id: u64,
        crop_x_keyframes: Vec<avcore::Keyframe<f32>>,
        crop_y_keyframes: Vec<avcore::Keyframe<f32>>,
        crop_w_keyframes: Vec<avcore::Keyframe<f32>>,
        crop_h_keyframes: Vec<avcore::Keyframe<f32>>,
        /// Whether a subject was detected at any sample — `false` means every sample fell back
        /// to a centered crop, worth telling the user about (same as [`AutoReframeEvent::Done`]'s
        /// `subject_found`).
        subject_found: bool,
    },
    Failed {
        clip_id: u64,
        message: String,
    },
}

/// A message from a background motion-tracking worker thread (see
/// [`App::spawn_motion_track_selected_clip`]) back to the UI thread.
pub(crate) enum MotionTrackEvent {
    Done {
        clip_id: u64,
        keyframes: Vec<avcore::Keyframe<avcore::Position>>,
    },
}

/// A message from a background voice-cleanup A/B preview render (see
/// [`App::spawn_voice_cleanup_preview`]) back to the UI thread. `Err` carries a `Display`-
/// formatted message rather than the original `VoiceCleanupPreviewError`, since `avcore`
/// errors aren't required to be `Send`/`'static` across the channel boundary and `ui` only ever
/// shows the message as a toast anyway.
pub(crate) enum VoiceCleanupPreviewEvent {
    Done {
        clip_id: u64,
        result: Result<avcore::voice_cleanup_preview::VoiceCleanupPreviewResult, String>,
    },
}

/// A message from a background nested-sequence-materialization worker thread (see
/// [`App::materialize_nested_sequences_for_active_sequence`]) back to the UI thread.
pub(crate) enum NestedSequenceEvent {
    Ready {
        sequence_id: u64,
        cache: HashMap<u64, avcore::nested_sequence::NestedSequenceCache>,
        assets: Vec<avcore::MediaAsset>,
        /// `Project::sequences` snapshot the render was dispatched against — latched into
        /// [`NestedSequenceRenderState::nested_sequence_last_input`] on completion so an
        /// unrelated frame doesn't re-dispatch a render whose inputs haven't actually changed.
        sequences_input: Vec<avcore::project::Sequence>,
    },
    Failed {
        sequence_id: u64,
        error: String,
        sequences_input: Vec<avcore::project::Sequence>,
    },
}

/// A message from a background scene-cut-detection worker thread (see
/// [`App::spawn_detect_scene_cuts_for_selected_clip`]) back to the UI thread.
pub(crate) enum SceneCutEvent {
    Done {
        clip_id: u64,
        cuts: Vec<avcore::SceneCut>,
    },
}

/// A message from a background AI-background-removal matte-generation worker thread (see
/// [`App::spawn_generate_matte_for_selected_clip`]) back to the UI thread.
pub(crate) enum MatteGenerationEvent {
    Done { clip_id: u64, mask_path: PathBuf },
    Failed { message: String },
}

/// A message from a background CF-09 privacy-blur matte-generation worker thread (see
/// [`App::spawn_apply_privacy_blur_for_selected_clip`]) back to the UI thread. Same shape as
/// [`MatteGenerationEvent`] plus the seed vertices/center the run actually tracked from (so the
/// UI thread can persist them onto the clip alongside the mask path — the background thread has
/// no direct `App` access to write them itself).
pub(crate) enum PrivacyBlurGenerationEvent {
    Done {
        clip_id: u64,
        mask_path: PathBuf,
        seed_vertices: Vec<(f32, f32)>,
    },
    Failed {
        message: String,
    },
}

/// A message from the background update-check thread (see [`App::spawn_update_check`]) back to
/// the UI thread. Every terminal result is sent because the About modal distinguishes a
/// successful current-version result from a failed network request.
pub(crate) enum UpdateCheckEvent {
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
pub(crate) enum TtsEvent {
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
pub(crate) enum YoutubeDownloadEvent {
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
pub(crate) enum WatchFolderEvent {
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
pub(crate) enum ThumbnailReady {
    Ready {
        project_id: u64,
        asset_id: u64,
        frame_index: i64,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    Failed {
        project_id: u64,
        asset_id: u64,
        frame_index: i64,
    },
}

/// `(project_id, asset_id, frame_index)` — `project_id` is part of the key because
/// `MediaAsset::id` is only unique *within* one project (assigned per-project, starting from 1
/// in each), not globally; without it, two different projects' assets sharing an id would
/// collide in the shared `thumbnail_textures` cache. Needed since [`App::request_thumbnail`]
/// serves both the active project's timeline/library (`screens::editor::timeline_panel`,
/// `screens::library`) and, for the Início screen's project cards, any project in `App::projects`
/// regardless of which one is active.
pub(super) type ThumbnailKey = (u64, u64, i64);
