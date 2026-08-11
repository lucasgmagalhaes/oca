//! `core` — the UI-agnostic engine crate for oca.
//!
//! Holds the project/timeline/media data model (see [`project`], [`timeline`], [`media`],
//! [`export`]), the `ffprobe`/`ffmpeg` wrappers that populate that model from real files
//! ([`probe`], [`loudness`]), lightweight editing proxies ([`proxy`]), the normalized-export
//! renderer ([`render`]), a GStreamer-based playback pipeline ([`preview`]), JSON save/load
//! ([`persistence`]), and mock data for exercising the UI before those wrappers are wired into
//! it end-to-end ([`sample`]).
//!
//! Nothing in this crate depends on `egui` or any GUI toolkit — `ui` is the only
//! consumer, and it owns all presentation/formatting concerns (see its `i18n` module).

pub mod export;
pub mod loudness;
pub mod media;
pub mod persistence;
pub mod preview;
pub mod probe;
pub mod project;
pub mod proxy;
pub mod render;
pub mod sample;
pub mod timeline;
pub mod waveform;

pub use export::{ExportJob, ExportJobStatus};
pub use loudness::{measure_loudness, LoudnessError};
pub use media::{LoudnessMetrics, MediaAsset, MediaKind};
pub use persistence::{load_project_from_file, save_project_to_file, PersistError};
pub use preview::{Preview, PreviewError};
pub use probe::{probe_media, ProbeError, ProbedMedia};
pub use project::{Project, Recency, Sequence};
pub use proxy::{ensure_proxy, ProxyError};
pub use render::{render_export, RenderError, RenderOutcome};
pub use timeline::{ClipInstance, Timeline, Track, TrackKind};
pub use waveform::{generate_waveform, WaveformError, WAVEFORM_BUCKET_COUNT};
