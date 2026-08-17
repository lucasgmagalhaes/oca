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

use serde::{Deserialize, Serialize};

use crate::{AudioSegment, Canvas, ClipSegment, ShapeSegment, TextSegment};

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
    /// Single-track segment list kept for backwards compatibility with jobs serialized before
    /// multi-track support was added.  When [`ExportJob::track_segments`] is non-empty,
    /// it takes precedence and this field is ignored by the render path.
    pub segments: Vec<ClipSegment>,
    /// Text overlay clips snapshotted from the sequence's text tracks at queue time.
    /// `#[serde(default)]` keeps existing saved queues (without this field) loading correctly.
    #[serde(default)]
    pub text_segments: Vec<TextSegment>,
    /// Shape overlay clips snapshotted from the sequence's shape tracks at queue time — same
    /// reasoning as `text_segments`. `#[serde(default)]` keeps existing saved queues (without
    /// this field) loading correctly.
    #[serde(default)]
    pub shape_segments: Vec<ShapeSegment>,
    /// One inner `Vec<ClipSegment>` per visible video track, as returned by
    /// [`crate::render::resolve_timeline_segments_multi`].  Empty when the job was queued
    /// before multi-track support (those jobs fall back to [`ExportJob::segments`]).
    #[serde(default)]
    pub track_segments: Vec<Vec<ClipSegment>>,
    /// Complete audio mix snapshot when a track beyond the background contributes audio.
    /// Empty keeps persisted pre-mix jobs on the established background-only path.
    #[serde(default)]
    pub audio_segments: Vec<AudioSegment>,
    pub canvas: Canvas,
    pub target_lufs: f32,
    pub output_path: String,
    pub status: ExportJobStatus,
}
