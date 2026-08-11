//! Computes per-bucket amplitude peaks of an asset's audio track, for drawing a waveform in
//! the timeline's audio clips — via `oca-avbridge`'s FFI, no subprocess. Like [`crate::proxy`],
//! this is a full decode pass, so callers run it on a background thread and cache the result
//! rather than call it from the UI thread (see `ui`'s import enrichment pipeline).

use std::path::Path;

#[derive(Debug)]
pub enum WaveformError {
    /// `oca-avbridge` failed before or during the decode/filter pipeline.
    Bridge(avbridge::WaveformError),
}

impl std::fmt::Display for WaveformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaveformError::Bridge(e) => write!(f, "failed to compute waveform: {e}"),
        }
    }
}

impl std::error::Error for WaveformError {}

/// How many (min, max) buckets [`generate_waveform`] computes across an asset's full duration.
/// Fixed rather than tied to any particular timeline zoom level — the timeline panel
/// subsamples/interpolates from this array to whatever pixel width a clip is drawn at, the
/// same way the fixed-resolution source data behind any other zoomable UI is resampled for
/// display.
pub const WAVEFORM_BUCKET_COUNT: usize = 2000;

/// Computes [`WAVEFORM_BUCKET_COUNT`] (min, max) amplitude peaks — each in `[-1.0, 1.0]` — of
/// `path`'s audio track, downmixed to mono and spread evenly across its full duration.
pub fn generate_waveform(path: &Path) -> Result<Vec<(f32, f32)>, WaveformError> {
    avbridge::generate_waveform(path, WAVEFORM_BUCKET_COUNT).map_err(WaveformError::Bridge)
}
