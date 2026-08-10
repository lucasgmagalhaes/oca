use avcore::timeline::{ClipInstance, Timeline, Track, TrackKind};

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

fn track_with(clips: Vec<ClipInstance>) -> Track {
    Track {
        id: 1,
        name: "V1".to_string(),
        kind: TrackKind::Video,
        clips,
    }
}

#[test]
fn split_clip_at_divides_the_covering_clip_into_two() {
    let mut track = track_with(vec![clip(1, 10.0, 0.0, 20.0)]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    assert_eq!(
        track.clips,
        vec![clip(1, 10.0, 0.0, 10.0), clip(99, 20.0, 10.0, 20.0)]
    );
}

#[test]
fn split_clip_at_only_splits_the_clip_that_covers_the_position() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0), clip(2, 10.0, 0.0, 20.0)]);

    let split = track.split_clip_at(15.0, 99);

    assert!(split);
    assert_eq!(track.clips[0], clip(1, 0.0, 0.0, 10.0));
    assert_eq!(
        track.clips[1..],
        [clip(2, 10.0, 0.0, 5.0), clip(99, 15.0, 5.0, 20.0)]
    );
}

#[test]
fn split_clip_at_is_a_no_op_when_nothing_covers_the_position() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0)]);

    let split = track.split_clip_at(50.0, 99);

    assert!(!split);
    assert_eq!(track.clips, vec![clip(1, 0.0, 0.0, 10.0)]);
}

#[test]
fn split_clip_at_is_a_no_op_exactly_on_a_clip_boundary() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0), clip(2, 10.0, 0.0, 20.0)]);

    let split = track.split_clip_at(10.0, 99);

    assert!(!split);
    assert_eq!(
        track.clips,
        vec![clip(1, 0.0, 0.0, 10.0), clip(2, 10.0, 0.0, 20.0)]
    );
}

#[test]
fn trim_start_shifts_start_and_source_in_by_the_same_delta() {
    let mut c = clip(1, 10.0, 5.0, 30.0);

    let trimmed = c.trim_start(15.0, 1.0);

    assert!(trimmed);
    assert_eq!(c, clip(1, 15.0, 10.0, 30.0));
}

#[test]
fn trim_start_is_a_no_op_when_it_would_go_negative() {
    let mut c = clip(1, 10.0, 5.0, 30.0);

    // Would push start_secs to -1.0.
    let trimmed = c.trim_start(-1.0, 1.0);

    assert!(!trimmed);
    assert_eq!(c, clip(1, 10.0, 5.0, 30.0));
}

#[test]
fn trim_start_is_a_no_op_when_it_would_shrink_below_the_minimum_duration() {
    let mut c = clip(1, 10.0, 5.0, 30.0);

    // delta = 24.5 -> source_in becomes 29.5 -> duration becomes 0.5, under the 1.0 minimum.
    let trimmed = c.trim_start(34.5, 1.0);

    assert!(!trimmed);
    assert_eq!(c, clip(1, 10.0, 5.0, 30.0));
}

#[test]
fn trim_end_extends_source_out_and_leaves_start_untouched() {
    let mut c = clip(1, 10.0, 5.0, 30.0);

    let trimmed = c.trim_end(25.0, 1.0, None);

    assert!(trimmed);
    // start=10, source_in=5, new duration = 25-10 = 15, so source_out = 5+15 = 20.
    assert_eq!(c, clip(1, 10.0, 5.0, 20.0));
}

#[test]
fn trim_end_is_a_no_op_past_the_source_medias_own_duration() {
    let mut c = clip(1, 10.0, 5.0, 30.0);

    // Would need source_out_secs = 35.0, past the 32.0 source duration.
    let trimmed = c.trim_end(40.0, 1.0, Some(32.0));

    assert!(!trimmed);
    assert_eq!(c, clip(1, 10.0, 5.0, 30.0));
}

#[test]
fn trim_end_is_a_no_op_when_it_would_shrink_below_the_minimum_duration() {
    let mut c = clip(1, 10.0, 5.0, 30.0);

    let trimmed = c.trim_end(10.5, 1.0, None);

    assert!(!trimmed);
    assert_eq!(c, clip(1, 10.0, 5.0, 30.0));
}

#[test]
fn clip_mut_finds_a_clip_by_id() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0), clip(2, 10.0, 0.0, 20.0)]);

    let found = track.clip_mut(2).unwrap();

    assert_eq!(found.start_secs, 10.0);
}

#[test]
fn clip_mut_returns_none_for_an_unknown_id() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0)]);

    assert!(track.clip_mut(99).is_none());
}

#[test]
fn move_clip_repositions_start_secs_and_leaves_the_source_range_untouched() {
    let mut track = track_with(vec![clip(1, 0.0, 5.0, 15.0)]);

    let moved = track.move_clip(1, 40.0);

    assert!(moved);
    assert_eq!(track.clips[0], clip(1, 40.0, 5.0, 15.0));
}

#[test]
fn move_clip_is_a_no_op_for_a_negative_position() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0)]);

    let moved = track.move_clip(1, -5.0);

    assert!(!moved);
    assert_eq!(track.clips[0], clip(1, 0.0, 0.0, 10.0));
}

#[test]
fn move_clip_is_a_no_op_for_an_unknown_clip_id() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0)]);

    let moved = track.move_clip(99, 5.0);

    assert!(!moved);
    assert_eq!(track.clips[0], clip(1, 0.0, 0.0, 10.0));
}
