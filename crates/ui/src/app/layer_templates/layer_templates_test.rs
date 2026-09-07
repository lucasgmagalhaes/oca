use super::*;

#[test]
fn video_media_matches_a_video_track() {
    assert!(media_kind_matches(
        avcore::MediaKind::Video,
        avcore::timeline::TrackKind::Video
    ));
}

#[test]
fn audio_media_matches_an_audio_track() {
    assert!(media_kind_matches(
        avcore::MediaKind::Audio,
        avcore::timeline::TrackKind::Audio
    ));
}

#[test]
fn video_media_does_not_match_an_audio_track() {
    assert!(!media_kind_matches(
        avcore::MediaKind::Video,
        avcore::timeline::TrackKind::Audio
    ));
}

#[test]
fn audio_media_does_not_match_a_video_track() {
    assert!(!media_kind_matches(
        avcore::MediaKind::Audio,
        avcore::timeline::TrackKind::Video
    ));
}

#[test]
fn no_media_kind_matches_a_text_or_shape_track() {
    for kind in [avcore::MediaKind::Video, avcore::MediaKind::Audio] {
        assert!(!media_kind_matches(kind, avcore::timeline::TrackKind::Text));
        assert!(!media_kind_matches(
            kind,
            avcore::timeline::TrackKind::Shape
        ));
    }
}
