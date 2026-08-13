use serde::{Deserialize, Serialize};

use crate::{Canvas, ClipSegment, TextSegment};

/// A queued render's lifecycle. `Rendering`/`Paused` carry a snapshot progress percentage;
/// the queue itself (Fase 4) will drive these via the background worker channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExportJobStatus {
    Queued,
    Rendering { percent: u8 },
    Paused { percent: u8 },
    Done,
    Failed { message: String },
}

/// One export job: a snapshot of a sequence's video-track clips (and text overlays), resolved
/// to [`ClipSegment`]s and [`TextSegment`]s at the moment the job entered the queue — later
/// edits to the source project must not retroactively change a queued job. Rendered via
/// [`crate::render::render_export_job`] (main encode) followed by
/// [`crate::render::apply_text_overlay_pass`]-equivalent (drawtext post-processing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    pub id: u64,
    pub title: String,
    pub segments: Vec<ClipSegment>,
    /// Text overlay clips snapshotted from the sequence's text tracks at queue time.
    /// `#[serde(default)]` keeps existing saved queues (without this field) loading correctly.
    #[serde(default)]
    pub text_segments: Vec<TextSegment>,
    pub canvas: Canvas,
    pub target_lufs: f32,
    pub output_path: String,
    pub status: ExportJobStatus,
}
