use std::path::{Path, PathBuf};

use nivela_core::media::MediaKind;
use nivela_core::probe::{probe_media, ProbeError};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn probes_a_video_stream_as_the_primary_track() {
    let media = probe_media(&fixture("video.mp4")).unwrap();
    assert_eq!(media.kind, MediaKind::Video);
    assert_eq!(media.codec, "mpeg4");
    assert_eq!(media.resolution, Some((320, 240)));
    assert_eq!(media.sample_rate_khz, None);
    assert!((media.duration_secs - 1.0).abs() < 0.1);
}

#[test]
fn converts_container_bitrate_from_bps_to_mbps() {
    let media = probe_media(&fixture("video.mp4")).unwrap();
    assert!(media.bitrate_mbps > 0.0);
}

#[test]
fn parses_frame_rate() {
    let media = probe_media(&fixture("video.mp4")).unwrap();
    assert!((media.fps.unwrap() - 30.0).abs() < 0.01);
}

#[test]
fn falls_back_to_the_audio_stream_when_there_is_no_video() {
    let media = probe_media(&fixture("audio.m4a")).unwrap();
    assert_eq!(media.kind, MediaKind::Audio);
    assert_eq!(media.codec, "aac");
    assert_eq!(media.resolution, None);
    assert_eq!(media.sample_rate_khz, Some(44.1));
}

#[test]
fn rejects_a_missing_file() {
    assert!(matches!(
        probe_media(&fixture("does_not_exist.mp4")),
        Err(ProbeError::Open)
    ));
}

#[test]
fn into_media_asset_carries_the_probed_fields_through() {
    let media = probe_media(&fixture("video.mp4")).unwrap();
    let asset = media.into_media_asset(
        7,
        "video.mp4".to_string(),
        PathBuf::from("/videos/video.mp4"),
    );
    assert_eq!(asset.id, 7);
    assert_eq!(asset.file_name, "video.mp4");
    assert_eq!(asset.source_path, PathBuf::from("/videos/video.mp4"));
    assert_eq!(asset.codec, "mpeg4");
    assert!(asset.loudness.is_none());
}
