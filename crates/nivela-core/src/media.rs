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

/// A source file imported into the project's media library. Populated today with mock
/// values; Fase 1 replaces the mock constructor with a real `ffprobe` + loudnorm wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_seconds_is_zeroed_mm_ss() {
        assert_eq!(format_timecode(0.0), "00:00");
    }

    #[test]
    fn under_a_minute_pads_seconds() {
        assert_eq!(format_timecode(5.0), "00:05");
    }

    #[test]
    fn drops_the_hour_component_under_an_hour() {
        assert_eq!(format_timecode(65.0), "01:05");
        assert_eq!(format_timecode(3599.0), "59:59");
    }

    #[test]
    fn includes_hour_component_at_and_beyond_an_hour() {
        assert_eq!(format_timecode(3600.0), "1:00:00");
        assert_eq!(format_timecode(3661.0), "1:01:01");
        assert_eq!(format_timecode(3.0 * 3600.0 + 20.0 * 60.0), "3:20:00");
    }

    #[test]
    fn rounds_to_the_nearest_second() {
        assert_eq!(format_timecode(59.6), "01:00");
        assert_eq!(format_timecode(0.4), "00:00");
    }

    #[test]
    fn duration_label_delegates_to_format_timecode() {
        let asset = MediaAsset {
            id: 1,
            file_name: "clip.mp4".to_string(),
            kind: MediaKind::Video,
            duration_secs: 134.0,
            codec: "H.264".to_string(),
            source_bitrate_mbps: 42.0,
            resolution: Some((1920, 1080)),
            fps: Some(60.0),
            sample_rate_khz: None,
            loudness: None,
        };
        assert_eq!(asset.duration_label(), "02:14");
    }
}
