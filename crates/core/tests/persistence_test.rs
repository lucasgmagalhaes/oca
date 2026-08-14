use std::fs;
use std::path::PathBuf;

use avcore::persistence::{
    from_ocproj_bytes, load_project_from_file, save_project_to_file, to_ocproj_bytes, PersistError,
};
use avcore::timeline::{
    ClipInstance, ColorFilter, MaskShape, Timeline, Track, TrackKind, TransitionType,
};
use avcore::{LoudnessMetrics, MediaAsset, MediaKind, Project, Recency, Sequence};

/// Looks up `key` in a MessagePack struct-map value, panicking if `value` isn't a map or
/// doesn't have that key. `rmpv::Value` only exposes read-only indexing/`as_map`, so mutating
/// a nested field means walking the map ourselves.
fn as_map_field_mut<'a>(value: &'a mut rmpv::Value, key: &str) -> &'a mut rmpv::Value {
    match value {
        rmpv::Value::Map(pairs) => {
            &mut pairs
                .iter_mut()
                .find(|(k, _)| k.as_str() == Some(key))
                .unwrap_or_else(|| panic!("missing key {key:?}"))
                .1
        }
        _ => panic!("expected a map looking up {key:?}"),
    }
}

/// Same as [`as_map_field_mut`], but expects the field's value to be an array.
fn as_array_field_mut<'a>(value: &'a mut rmpv::Value, key: &str) -> &'a mut Vec<rmpv::Value> {
    match as_map_field_mut(value, key) {
        rmpv::Value::Array(items) => items,
        _ => panic!("expected {key:?} to be an array"),
    }
}

fn clip(id: u64, asset_id: u64) -> ClipInstance {
    ClipInstance {
        id,
        asset_id,
        start_secs: 0.0,
        source_in_secs: 0.0,
        source_out_secs: 10.0,
        composite_id: None,
        gain_db: 0.0,
        frozen: false,
        speed_factor: 1.0,
        crop_x: 0.0,
        crop_y: 0.0,
        crop_w: 1.0,
        crop_h: 1.0,
        mask_shape: MaskShape::None,
        mask_corner_radius: 0.0,
        flipped_h: false,
        color_filter: ColorFilter::None,
        vignette_intensity: 0.0,
        brightness: 0.0,
        contrast: 1.0,
        saturation: 1.0,
        sharpen: 0.0,
        chroma_key_enabled: false,
        chroma_key_color: [0, 255, 0],
        chroma_key_tolerance: 0.4,
        blur_intensity: 0.0,
        shake_intensity: 0.0,
        glitch_intensity: 0.0,
        pixelize_intensity: 0.0,
        transition_in: TransitionType::None,
        transition_duration_secs: 0.5,
        position_keyframes: vec![],
        scale_keyframes: vec![],
        rotation_keyframes: vec![],
        opacity_keyframes: vec![],
        deflicker_enabled: false,
        lut_path: String::new(),
    }
}

fn fixture_project() -> Project {
    Project {
        id: 1,
        name: "Fixture project".to_string(),
        last_edited: Recency::HoursAgo(2),
        summary: "Persistence test fixture.".to_string(),
        media_library: vec![MediaAsset {
            id: 1,
            file_name: "clip.mp4".to_string(),
            source_path: PathBuf::from("/media/clip.mp4"),
            kind: MediaKind::Video,
            duration_secs: 10.0,
            codec: "H.264".to_string(),
            source_bitrate_mbps: 40.0,
            resolution: Some((1920, 1080)),
            fps: Some(60.0),
            sample_rate_khz: None,
            loudness: Some(LoudnessMetrics {
                integrated_lufs: -18.0,
                true_peak_dbtp: -3.0,
                loudness_range_lu: 8.0,
            }),
            proxy_path: None,
            waveform_peaks: None,
        }],
        sequences: vec![Sequence {
            id: 1,
            name: "Sequência principal".to_string(),
            timeline: Timeline {
                playhead_secs: 0.0,
                tracks: vec![Track {
                    id: 1,
                    name: "V1".to_string(),
                    kind: TrackKind::Video,
                    clips: vec![clip(1, 1)],

                    text_clips: vec![],

                    visible: true,
                }],
            },
        }],
        active_sequence: 0,
        file_path: None,
    }
}

fn empty_project() -> Project {
    Project {
        id: 2,
        name: "Empty project".to_string(),
        last_edited: Recency::DaysAgo(3),
        summary: String::new(),
        media_library: vec![],
        sequences: vec![Sequence {
            id: 1,
            name: "Sequência principal".to_string(),
            timeline: Timeline {
                tracks: vec![],
                playhead_secs: 0.0,
            },
        }],
        active_sequence: 0,
        file_path: None,
    }
}

#[test]
fn round_trips_a_project_through_ocproj() {
    let original = fixture_project();
    let bytes = to_ocproj_bytes(&original).unwrap();
    let restored = from_ocproj_bytes(&bytes).unwrap();
    assert_eq!(original, restored);
}

/// Proves struct-map mode's field-level defaulting actually works end to end — not just that
/// `gain_db` has a default value in isolation. Decodes the real `.ocproj` MessagePack payload
/// as a generic value, removes the `gain_db` key the same way the old JSON test removed it from
/// a `serde_json::Value`, re-encodes, and confirms `from_ocproj_bytes` still loads it (at unity
/// gain) instead of erroring out on the "missing" field.
#[test]
fn projects_saved_before_per_block_gain_load_at_unity_gain() {
    let original = fixture_project();
    let bytes = to_ocproj_bytes(&original).unwrap();

    // Peel off the [MAGIC][version] header and gunzip to get the raw MessagePack bytes.
    let header_len = 5;
    let mut msgpack = Vec::new();
    std::io::Read::read_to_end(
        &mut flate2::read::GzDecoder::new(&bytes[header_len..]),
        &mut msgpack,
    )
    .unwrap();

    let mut value = rmpv::decode::read_value(&mut &msgpack[..]).unwrap();
    let sequences = as_array_field_mut(&mut value, "sequences");
    for sequence in sequences {
        let timeline = as_map_field_mut(sequence, "timeline");
        let tracks = as_array_field_mut(timeline, "tracks");
        for track in tracks {
            let clips = as_array_field_mut(track, "clips");
            for clip in clips {
                if let rmpv::Value::Map(pairs) = clip {
                    pairs.retain(|(key, _)| key.as_str() != Some("gain_db"));
                }
            }
        }
    }

    let mut edited_msgpack = Vec::new();
    rmpv::encode::write_value(&mut edited_msgpack, &value).unwrap();

    let mut edited_bytes = Vec::new();
    edited_bytes.extend_from_slice(&bytes[..header_len]);
    let mut encoder = flate2::write::GzEncoder::new(&mut edited_bytes, flate2::Compression::fast());
    std::io::Write::write_all(&mut encoder, &edited_msgpack).unwrap();
    encoder.finish().unwrap();

    let restored = from_ocproj_bytes(&edited_bytes).unwrap();

    assert!(restored
        .sequences
        .iter()
        .flat_map(|sequence| &sequence.timeline.tracks)
        .flat_map(|track| &track.clips)
        .all(|clip| clip.gain_db == 0.0));
}

#[test]
fn round_trips_a_project_with_an_empty_timeline_and_library() {
    let original = empty_project();
    assert!(original.media_library.is_empty());
    assert!(original.timeline().tracks.is_empty());
    let bytes = to_ocproj_bytes(&original).unwrap();
    let restored = from_ocproj_bytes(&bytes).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn from_ocproj_bytes_rejects_malformed_input() {
    assert!(matches!(
        from_ocproj_bytes(b"not an ocproj file"),
        Err(PersistError::Corrupt(_))
    ));
}

#[test]
fn from_ocproj_bytes_rejects_an_unsupported_version() {
    let original = fixture_project();
    let mut bytes = to_ocproj_bytes(&original).unwrap();
    bytes[4] = 99;
    assert!(matches!(
        from_ocproj_bytes(&bytes),
        Err(PersistError::Corrupt(_))
    ));
}

#[test]
fn file_path_is_not_part_of_the_serialized_bytes() {
    let mut project = fixture_project();
    project.file_path = Some("/tmp/whatever.ocproj".into());
    let bytes = to_ocproj_bytes(&project).unwrap();
    assert!(!bytes.windows(b"whatever".len()).any(|w| w == b"whatever"));
}

#[test]
fn save_then_load_round_trips_through_a_real_file() {
    let original = fixture_project();
    let path = std::env::temp_dir().join(format!("oca_persist_test_{}.ocproj", original.id));

    save_project_to_file(&original, &path).unwrap();
    let loaded = load_project_from_file(&path).unwrap();
    let _ = fs::remove_file(&path);

    assert_eq!(original, loaded);
}

#[test]
fn load_project_from_file_errors_on_a_missing_file() {
    let path = std::env::temp_dir().join("oca_persist_test_does_not_exist.ocproj");
    assert!(matches!(
        load_project_from_file(&path),
        Err(PersistError::Io(_))
    ));
}
