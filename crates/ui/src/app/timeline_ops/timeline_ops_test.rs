use avcore::timeline::{ShapeClip, ShapeKind, TextClip, Track};

use super::*;

fn empty_timeline() -> avcore::timeline::Timeline {
    avcore::timeline::Timeline {
        tracks: Vec::new(),
        playhead_secs: 0.0,
        markers: Vec::new(),
        multicam_groups: Vec::new(),
    }
}

fn track(id: u64, kind: TrackKind) -> Track {
    Track {
        id,
        name: "T".to_string(),
        kind,
        clips: Vec::new(),
        text_clips: Vec::new(),
        shape_clips: Vec::new(),
        visible: true,
        audio_role: AudioRole::Unspecified,
        locked: false,
        color_label: None,
    }
}

fn text_clip(id: u64) -> TextClip {
    TextClip {
        id,
        start_secs: 0.0,
        duration_secs: 3.0,
        text: String::new(),
        font_size: 48.0,
        font_family: Default::default(),
        font_style: Default::default(),
        font_weight: None,
        color_rgba: [255, 255, 255, 255],
        background_rgba: [0, 0, 0, 0],
        background_padding: 8.0,
        background_corner_radius: 8.0,
        pos_x: 0.1,
        pos_y: 0.85,
        words: Vec::new(),
        highlight_enabled: false,
        highlight_color_rgba: [255, 220, 0, 255],
        opacity_keyframes: vec![],
        pos_x_keyframes: vec![],
        pos_y_keyframes: vec![],
        scale_keyframes: vec![],
        rotation_keyframes: vec![],
        direction: Default::default(),
        language: None,
        text_align: Default::default(),
    }
}

fn shape_clip(id: u64) -> ShapeClip {
    ShapeClip {
        id,
        start_secs: 0.0,
        duration_secs: 3.0,
        shape_kind: ShapeKind::rectangle(),
        center_x: 0.5,
        center_y: 0.5,
        center_x_keyframes: vec![],
        center_y_keyframes: vec![],
        width_keyframes: vec![],
        height_keyframes: vec![],
        rotation_keyframes: vec![],
        width: 0.3,
        height: 0.3,
        rotation_deg: 0.0,
        color_rgba: [255, 255, 255, 255],
        stroke_thickness_px: 0.0,
    }
}

#[test]
fn next_clip_id_starts_at_one_for_an_empty_timeline() {
    let timeline = empty_timeline();
    assert_eq!(next_clip_id(&timeline), 1);
}

#[test]
fn next_clip_id_is_one_past_the_highest_video_or_audio_clip_id() {
    let mut timeline = empty_timeline();
    let mut t = track(1, TrackKind::Video);
    t.clips
        .push(default_clip_instance(5, 1, 0.0, 0.0, 1.0, false));
    t.clips
        .push(default_clip_instance(2, 1, 0.0, 0.0, 1.0, false));
    timeline.tracks.push(t);

    assert_eq!(next_clip_id(&timeline), 6);
}

#[test]
fn next_clip_id_accounts_for_text_clips_too() {
    let mut timeline = empty_timeline();
    let mut video = track(1, TrackKind::Video);
    video
        .clips
        .push(default_clip_instance(1, 1, 0.0, 0.0, 1.0, false));
    let mut text = track(2, TrackKind::Text);
    text.text_clips.push(text_clip(9));
    timeline.tracks.push(video);
    timeline.tracks.push(text);

    assert_eq!(next_clip_id(&timeline), 10);
}

#[test]
fn next_clip_id_accounts_for_shape_clips_too() {
    let mut timeline = empty_timeline();
    let mut shape = track(1, TrackKind::Shape);
    shape.shape_clips.push(shape_clip(7));
    timeline.tracks.push(shape);

    assert_eq!(next_clip_id(&timeline), 8);
}

#[test]
fn next_clip_id_ids_are_unique_across_all_three_clip_kinds() {
    let mut timeline = empty_timeline();
    let mut video = track(1, TrackKind::Video);
    video
        .clips
        .push(default_clip_instance(3, 1, 0.0, 0.0, 1.0, false));
    let mut text = track(2, TrackKind::Text);
    text.text_clips.push(text_clip(10));
    let mut shape = track(3, TrackKind::Shape);
    shape.shape_clips.push(shape_clip(4));
    timeline.tracks.push(video);
    timeline.tracks.push(text);
    timeline.tracks.push(shape);

    assert_eq!(next_clip_id(&timeline), 11);
}

#[test]
fn create_new_track_appends_and_numbers_by_existing_count_of_that_kind() {
    let mut timeline = empty_timeline();
    timeline.tracks.push(track(1, TrackKind::Video));

    let index = create_new_track(&mut timeline, TrackKind::Video);

    assert_eq!(index, 1);
    assert_eq!(timeline.tracks[1].name, "V2");
    assert_eq!(timeline.tracks[1].kind, TrackKind::Video);
}

#[test]
fn create_new_track_never_reuses_an_existing_track() {
    let mut timeline = empty_timeline();
    timeline.tracks.push(track(1, TrackKind::Video));
    let before = timeline.tracks.len();

    create_new_track(&mut timeline, TrackKind::Video);

    assert_eq!(timeline.tracks.len(), before + 1);
}

#[test]
fn create_new_track_assigns_the_first_number_for_a_brand_new_kind() {
    let mut timeline = empty_timeline();
    timeline.tracks.push(track(1, TrackKind::Video));

    create_new_track(&mut timeline, TrackKind::Audio);

    assert_eq!(timeline.tracks[1].name, "A1");
}

#[test]
fn create_new_track_ids_never_collide_with_an_existing_track() {
    let mut timeline = empty_timeline();
    timeline.tracks.push(track(5, TrackKind::Video));

    create_new_track(&mut timeline, TrackKind::Video);

    assert_eq!(timeline.tracks[1].id, 6);
}

#[test]
fn resolve_or_create_track_prefers_the_given_id_when_it_matches_the_kind() {
    let mut timeline = empty_timeline();
    timeline.tracks.push(track(1, TrackKind::Video));
    timeline.tracks.push(track(2, TrackKind::Video));

    let index = resolve_or_create_track(&mut timeline, TrackKind::Video, Some(2));

    assert_eq!(index, 1);
    assert_eq!(timeline.tracks.len(), 2);
}

#[test]
fn resolve_or_create_track_ignores_a_preferred_id_of_the_wrong_kind() {
    let mut timeline = empty_timeline();
    timeline.tracks.push(track(1, TrackKind::Video));
    timeline.tracks.push(track(2, TrackKind::Audio));

    let index = resolve_or_create_track(&mut timeline, TrackKind::Video, Some(2));

    assert_eq!(index, 0);
}

#[test]
fn resolve_or_create_track_falls_back_to_the_first_existing_track_of_that_kind() {
    let mut timeline = empty_timeline();
    timeline.tracks.push(track(1, TrackKind::Audio));
    timeline.tracks.push(track(2, TrackKind::Video));
    timeline.tracks.push(track(3, TrackKind::Video));

    let index = resolve_or_create_track(&mut timeline, TrackKind::Video, None);

    assert_eq!(index, 1);
    assert_eq!(timeline.tracks.len(), 3);
}

#[test]
fn resolve_or_create_track_creates_a_first_track_when_none_of_that_kind_exist() {
    let mut timeline = empty_timeline();

    let index = resolve_or_create_track(&mut timeline, TrackKind::Video, None);

    assert_eq!(index, 0);
    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].name, "V1");
    assert_eq!(timeline.tracks[0].id, 1);
}

#[test]
fn resolve_or_create_track_created_track_ids_never_collide() {
    let mut timeline = empty_timeline();
    timeline.tracks.push(track(9, TrackKind::Audio));

    let index = resolve_or_create_track(&mut timeline, TrackKind::Video, None);

    assert_eq!(timeline.tracks[index].id, 10);
}

#[test]
fn default_clip_instance_uses_the_given_ids_and_trim_range() {
    let clip = default_clip_instance(3, 7, 1.5, 0.5, 2.5, true);
    assert_eq!(clip.id, 3);
    assert_eq!(clip.asset_id, 7);
    assert_eq!(clip.start_secs, 1.5);
    assert_eq!(clip.source_in_secs, 0.5);
    assert_eq!(clip.source_out_secs, 2.5);
    assert!(clip.voice_cleanup_enabled);
}

#[test]
fn default_clip_instance_has_neutral_effect_defaults() {
    let clip = default_clip_instance(1, 1, 0.0, 0.0, 1.0, false);
    assert_eq!(clip.speed_factor, 1.0);
    assert_eq!(clip.gain_db, 0.0);
    assert!(!clip.frozen);
    assert_eq!(clip.color_filter, avcore::timeline::ColorFilter::None);
    assert_eq!(clip.blend_mode, avcore::timeline::BlendMode::Normal);
    assert!(clip.position_keyframes.is_empty());
}
