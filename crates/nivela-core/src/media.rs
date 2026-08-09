use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaKind {
    Video,
    Audio,
}

/// Result of a `loudnorm`-style analysis pass on a clip, before or after processing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LoudnessMetrics {
    pub integrated_lufs: f32,
    pub true_peak_dbtp: f32,
    pub loudness_range_lu: f32,
}

/// A source file imported into the project's media library. Built either from
/// [`crate::sample`]'s mock data or, for real imports, from [`crate::probe::probe_media`]
/// (via [`crate::probe::ProbedMedia::into_media_asset`]) plus a [`crate::loudness::measure_loudness`]
/// pass for the `loudness` field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: u64,
    pub file_name: String,
    pub kind: MediaKind,
    pub duration_secs: f64,
    pub codec: String,
    pub source_bitrate_mbps: f32,
    /// `None` for audio-only assets.
    pub resolution: Option<(u32, u32)>,
    /// `None` for audio-only assets.
    pub fps: Option<f32>,
    /// `None` for video assets without a separate sample rate to show.
    pub sample_rate_khz: Option<f32>,
    pub loudness: Option<LoudnessMetrics>,
}

impl MediaAsset {
    /// This asset's duration as a display-ready timecode (e.g. `"02:14"`).
    pub fn duration_label(&self) -> String {
        format_timecode(self.duration_secs)
    }
}

/// Formats seconds as `H:MM:SS` (or `MM:SS` under an hour), matching the mockup's timecodes.
pub fn format_timecode(total_secs: f64) -> String {
    let total = total_secs.round() as i64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}
