//! `core` — the UI-agnostic engine crate for oca.
//!
//! Holds the project/timeline/media data model (see [`project`], [`timeline`], [`media`],
//! [`export`]), the general keyframe-animation system ([`keyframe`]), the `ffprobe`/`ffmpeg`
//! wrappers that populate that model from real files ([`probe`], [`loudness`]), lightweight
//! editing proxies ([`proxy`]), the normalized-export renderer ([`render`]), a GStreamer-based
//! playback pipeline ([`preview`]), and `.ocproj` save/load — gzip-compressed MessagePack
//! ([`persistence`]).
//!
//! Nothing in this crate depends on `egui` or any GUI toolkit — `ui` is the only
//! consumer, and it owns all presentation/formatting concerns (see its `i18n` module).

pub mod export;
pub mod keyframe;
pub mod loudness;
pub mod media;
pub mod model_download;
pub mod persistence;
pub mod preview;
pub mod probe;
pub mod project;
pub mod proxy;
pub mod render;
pub mod text_metrics;
pub mod timeline;
pub mod transcribe;
pub mod waveform;

pub use avbridge::{Canvas, ClipSegment, GpuEncoderPreference, TextSegment};
pub use export::{ExportJob, ExportJobStatus};
pub use keyframe::{Keyframe, Position};
pub use loudness::{measure_loudness, LoudnessError};
pub use media::{LoudnessMetrics, MediaAsset, MediaKind};
pub use model_download::{
    download_whisper_model, DownloadError, DownloadOutcome, WhisperModelSize,
};
pub use persistence::{
    from_ocproj_bytes, load_project_from_file, save_project_to_file, to_ocproj_bytes, PersistError,
};
pub use preview::{Preview, PreviewError};
pub use probe::{probe_media, ProbeError, ProbedMedia};
pub use project::{Project, Recency, Sequence};
pub use proxy::{ensure_proxy, ProxyError};
pub use render::{
    apply_export_aspect_ratio, render_export, render_export_job, render_export_job_multi,
    render_timeline_export, resolve_text_segments, resolve_timeline_segments,
    resolve_timeline_segments_multi, ExportAspectRatio, RenderError, RenderOutcome,
};
pub use timeline::{ClipFormatting, ClipInstance, TextClip, Timeline, Track, TrackKind};
pub use transcribe::{transcribe, TranscribeError, TranscribeOutcome, TranscribeSegment};
pub use waveform::{generate_waveform, WaveformError, WAVEFORM_BUCKET_COUNT};
