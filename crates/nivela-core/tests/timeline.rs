use nivela_core::timeline::{ClipInstance, Timeline, Track, TrackKind};

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
