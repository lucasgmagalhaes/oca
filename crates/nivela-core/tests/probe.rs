use std::path::PathBuf;

use nivela_core::media::MediaKind;
use nivela_core::probe::{parse_probe_json, ProbeError};

const VIDEO_FIXTURE: &str = r#"{
    "streams": [
        {
            "index": 0,
            "codec_name": "h264",
            "codec_type": "video",
            "width": 1920,
            "height": 1080,
            "r_frame_rate": "60000/1001",
            "bit_rate": "42000000",
            "duration": "134.234000"
        },
        {
            "index": 1,
            "codec_name": "aac",
            "codec_type": "audio",
            "sample_rate": "48000",
            "channels": 2,
            "bit_rate": "192000",
            "duration": "134.234000"
        }
    ],
    "format": {
        "filename": "boss03_ribby_croaks.mp4",
        "duration": "134.234000",
        "bit_rate": "42021000"
    }
}"#;

const AUDIO_ONLY_FIXTURE: &str = r#"{
    "streams": [
        {
            "index": 0,
            "codec_name": "pcm_s16le",
            "codec_type": "audio",
            "sample_rate": "48000",
            "channels": 2,
            "duration": "134.234000"
        }
    ],
    "format": {
        "filename": "commentary_mic.wav",
        "duration": "134.234000",
        "bit_rate": "1536000"
    }
}"#;

#[test]
fn parses_a_video_stream_as_the_primary_track() {
    let media = parse_probe_json(VIDEO_FIXTURE).unwrap();
    assert_eq!(media.kind, MediaKind::Video);
    assert_eq!(media.codec, "h264");
    assert_eq!(media.resolution, Some((1920, 1080)));
    assert_eq!(media.sample_rate_khz, None);
    assert!((media.duration_secs - 134.234).abs() < 0.001);
}

#[test]
fn converts_container_bitrate_from_bps_to_mbps() {
    let media = parse_probe_json(VIDEO_FIXTURE).unwrap();
    assert!((media.bitrate_mbps - 42.021).abs() < 0.001);
}

#[test]
fn parses_ntsc_frame_rate_fraction() {
    let media = parse_probe_json(VIDEO_FIXTURE).unwrap();
    // 60000/1001 ~= 59.94
    assert!((media.fps.unwrap() - 59.94).abs() < 0.01);
}

#[test]
fn falls_back_to_the_audio_stream_when_there_is_no_video() {
    let media = parse_probe_json(AUDIO_ONLY_FIXTURE).unwrap();
    assert_eq!(media.kind, MediaKind::Audio);
    assert_eq!(media.codec, "pcm_s16le");
    assert_eq!(media.resolution, None);
    assert_eq!(media.sample_rate_khz, Some(48.0));
}

#[test]
fn rejects_output_with_no_usable_stream() {
    let json =
        r#"{"streams": [{"codec_type": "subtitle", "codec_name": "mov_text"}], "format": {}}"#;
    assert!(matches!(
        parse_probe_json(json),
        Err(ProbeError::NoMediaStream)
    ));
}

#[test]
fn rejects_malformed_json() {
    assert!(matches!(
        parse_probe_json("not json"),
        Err(ProbeError::Json(_))
    ));
}

#[test]
fn into_media_asset_carries_the_probed_fields_through() {
    let media = parse_probe_json(VIDEO_FIXTURE).unwrap();
    let asset = media.into_media_asset(
        7,
        "boss03_ribby_croaks.mp4".to_string(),
        PathBuf::from("/videos/boss03_ribby_croaks.mp4"),
    );
    assert_eq!(asset.id, 7);
    assert_eq!(asset.file_name, "boss03_ribby_croaks.mp4");
    assert_eq!(
        asset.source_path,
        PathBuf::from("/videos/boss03_ribby_croaks.mp4")
    );
    assert_eq!(asset.codec, "h264");
    assert!(asset.loudness.is_none());
}
