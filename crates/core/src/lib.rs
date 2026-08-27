// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! `core` — the UI-agnostic engine crate for oca.
//!
//! Holds the project/timeline/media data model (see [`project`], [`timeline`], [`media`],
//! [`export`]), the general keyframe-animation system ([`keyframe`]), the `ffprobe`/`ffmpeg`
//! wrappers that populate that model from real files ([`probe`], [`loudness`]), lightweight
//! editing proxies ([`proxy`]), the normalized-export renderer ([`render`]), a GStreamer-based
//! playback pipeline ([`preview`]), a local music/SFX catalog ([`sound_library`]), and
//! `.ocproj`/`.ocqueue` save/load — gzip-compressed MessagePack ([`persistence`]).
//!
//! Nothing in this crate depends on `egui` or any GUI toolkit — `ui` is the only
//! consumer, and it owns all presentation/formatting concerns (see its `i18n` module).

pub mod auto_reframe;
pub mod background_removal;
pub mod bundle;
pub mod export;
pub mod frame_sampler;
pub mod keyframe;
pub mod loudness;
pub mod media;
pub mod motion_tracking;
pub mod overlay_render;
pub mod persistence;
pub mod preview;
pub mod probe;
pub mod project;
pub mod proxy;
pub mod render;
pub mod shape_render;
pub mod sound_library;
pub mod subtitles;
pub mod telemetry;
pub mod text_metrics;
pub mod text_to_speech;
pub mod timeline;
pub mod transcribe;
pub mod undo;
pub mod update_check;
pub mod waveform;
pub mod youtube_download;

pub use auto_reframe::{
    compute_reframe_crop, detect_faces, main_subject_center, CropRect, FaceBox, ReframeError,
};
pub use avbridge::{AudioSegment, Canvas, ClipSegment, GpuEncoderPreference, ShapeSegment};
pub use background_removal::{encode_matte_video, segment_person, MatteEncodeError, SegmentError};
pub use bundle::{
    bundled_resource_path, bundled_resources_dir, configure_bundled_runtime, resource_path_in,
    validate_bundled_resources, BundledResource, MissingBundleResources,
};
pub use export::{ExportAspectRatio, ExportJob, ExportJobStatus, PlatformExportPreset};
pub use frame_sampler::FrameSampler;
pub use keyframe::{Keyframe, Position};
pub use loudness::{measure_loudness, LoudnessError};
pub use media::{LoudnessMetrics, MediaAsset, MediaKind};
pub use motion_tracking::{
    rgba_to_gray, track_region, tracked_positions_to_keyframes, GrayFrame, TrackedPosition,
};
pub use persistence::{
    from_ocproj_bytes, from_ocqueue_bytes, load_project_from_file, save_project_to_file,
    to_ocproj_bytes, to_ocqueue_bytes, PersistError,
};
pub use preview::{Preview, PreviewError};
pub use probe::{probe_media, ProbeError, ProbedMedia};
pub use project::{PanelLayout, Project, Recency, Sequence, SequenceExportSettings};
pub use proxy::{ensure_proxy, PreviewQuality, ProxyError};
pub use render::{
    apply_export_aspect_ratio, render_export, render_export_job, render_export_job_multi,
    render_export_job_multi_with_audio, render_timeline_export, resolve_audio_segments,
    resolve_shape_segments, resolve_text_segments, resolve_timeline_segments,
    resolve_timeline_segments_multi, RenderError, RenderOutcome, TextSegment,
};
pub use shape_render::{build_shape_filter_desc, point_in_polygon, ShapeRenderInput};
pub use sound_library::{scan_library_dir, LibraryTrack, SoundCategory};
pub use subtitles::export_srt;
pub use telemetry::{record_event, ResourceSampler, TelemetryError, TelemetryEvent};
pub use text_to_speech::{
    load_voice_config, phonemes_to_ids, synthesize, write_wav, PiperVoiceConfig, TtsError,
};
pub use timeline::{
    ClipFormatting, ClipInstance, LayerTemplate, ShapeClip, ShapeKind, TextClip, TextFontFamily,
    TextFontStyle, Timeline, Track, TrackKind,
};
pub use transcribe::{transcribe, TranscribeError, TranscribeOutcome, TranscribeSegment};
pub use update_check::{
    apply_update, auto_update_supported, expected_update_asset_name, fetch_latest_release,
    is_newer, package_supports_atomic_update, release_supports_auto_update, restart_application,
    ApplyUpdateError, ApplyUpdateOutcome, LatestRelease, UpdateCheckError, UpdatePackage,
};
pub use waveform::{generate_waveform, WaveformError, WAVEFORM_BUCKET_COUNT};
pub use youtube_download::{
    download_youtube, is_yt_dlp_available, Mp3Bitrate, Mp4Quality, YoutubeDownloadError,
    YoutubeDownloadTarget,
};
