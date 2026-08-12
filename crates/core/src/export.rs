use serde::{Deserialize, Serialize};

use crate::{Canvas, ClipSegment};

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

/// One export job: a snapshot of a sequence's video-track clips, resolved to
/// [`ClipSegment`]s, taken when the job entered the queue — later edits to the source
/// project must not retroactively change a queued job. Rendered via
/// [`crate::render::render_timeline_export`]'s underlying `avbridge::encode_timeline_export`.
#[derive(Debug, Clone)]
pub struct ExportJob {
    pub id: u64,
    pub title: String,
    pub segments: Vec<ClipSegment>,
    pub canvas: Canvas,
    pub target_lufs: f32,
    pub output_path: String,
    pub status: ExportJobStatus,
}
