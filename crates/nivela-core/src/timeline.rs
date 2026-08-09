use serde::{Deserialize, Serialize};

/// What a [`Track`] carries. Determines how the timeline widget renders its clips
/// (thumbnails for video, waveforms for audio) and which asset kind can be dropped onto it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    Video,
    Audio,
}

/// One placed instance of a `MediaAsset` on the timeline. `source_in_secs`/`source_out_secs`
/// mark the trimmed range within the source asset; `start_secs` is its position on the track.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClipInstance {
    pub id: u64,
    pub asset_id: u64,
    pub start_secs: f64,
    pub source_in_secs: f64,
    pub source_out_secs: f64,
}

impl ClipInstance {
    /// How long this instance plays for, i.e. its trimmed length — not the source asset's
    /// full duration.
    pub fn duration_secs(&self) -> f64 {
        self.source_out_secs - self.source_in_secs
    }
}

/// One row of the timeline (e.g. `V1`, `A1`, `A2` in the mockup), holding an ordered list of
/// clips. Tracks don't overlap-check their own clips — that's an editing-time concern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: u64,
    pub name: String,
    pub kind: TrackKind,
    pub clips: Vec<ClipInstance>,
}

/// A project's full set of tracks plus the current playhead position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    pub tracks: Vec<Track>,
    pub playhead_secs: f64,
}

impl Timeline {
    /// The position, in seconds, where the last clip on any track ends — i.e. how long the
    /// edited sequence runs for. `0.0` for an empty timeline.
    pub fn duration_secs(&self) -> f64 {
        self.tracks
            .iter()
            .flat_map(|t| t.clips.iter())
            .map(|c| c.start_secs + c.duration_secs())
            .fold(0.0, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(id: u64, start_secs: f64, source_in_secs: f64, source_out_secs: f64) -> ClipInstance {
        ClipInstance {
            id,
            asset_id: 1,
            start_secs,
            source_in_secs,
            source_out_secs,
        }
    }

    #[test]
    fn clip_duration_is_out_minus_in() {
        assert_eq!(clip(1, 0.0, 10.0, 30.0).duration_secs(), 20.0);
    }

    #[test]
    fn empty_timeline_has_zero_duration() {
        let timeline = Timeline {
            tracks: vec![],
            playhead_secs: 0.0,
        };
        assert_eq!(timeline.duration_secs(), 0.0);
    }

    #[test]
    fn timeline_duration_is_the_furthest_clip_end_across_all_tracks() {
        let timeline = Timeline {
            tracks: vec![
                Track {
                    id: 1,
                    name: "V1".to_string(),
                    kind: TrackKind::Video,
                    clips: vec![clip(1, 0.0, 0.0, 30.0), clip(2, 30.0, 0.0, 44.0)],
                },
                Track {
                    id: 2,
                    name: "A2".to_string(),
                    kind: TrackKind::Audio,
                    // Shorter overall, so it must not win over the V1 track's later end.
                    clips: vec![clip(3, 0.0, 0.0, 10.0)],
                },
            ],
            playhead_secs: 0.0,
        };
        // Track V1's second clip ends at 30 + (44 - 0) = 74.
        assert_eq!(timeline.duration_secs(), 74.0);
    }
}
