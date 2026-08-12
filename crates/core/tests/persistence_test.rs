use std::fs;
use std::path::PathBuf;

use avcore::persistence::{
    from_json, load_project_from_file, save_project_to_file, to_json, PersistError,
};
use avcore::timeline::{
    ClipInstance, ColorFilter, MaskShape, Timeline, Track, TrackKind, TransitionType,
};
use avcore::{LoudnessMetrics, MediaAsset, MediaKind, Project, Recency, Sequence};

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
        zoom_start: 1.0,
        zoom_end: 1.0,
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
fn round_trips_a_project_through_json() {
    let original = fixture_project();
    let json = to_json(&original).unwrap();
    let restored = from_json(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn projects_saved_before_per_block_gain_load_at_unity_gain() {
    let original = fixture_project();
    let mut value: serde_json::Value = serde_json::from_str(&to_json(&original).unwrap()).unwrap();
    for track in value["sequences"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .flat_map(|sequence| sequence["timeline"]["tracks"].as_array_mut().unwrap())
    {
        for clip in track["clips"].as_array_mut().unwrap() {
            clip.as_object_mut().unwrap().remove("gain_db");
        }
    }

    let restored = from_json(&serde_json::to_string(&value).unwrap()).unwrap();

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
    let json = to_json(&original).unwrap();
    let restored = from_json(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn from_json_rejects_malformed_input() {
    assert!(from_json("not json").is_err());
}

#[test]
fn file_path_is_not_part_of_the_serialized_json() {
    let mut project = fixture_project();
    project.file_path = Some("/tmp/whatever.json".into());
    let json = to_json(&project).unwrap();
    assert!(!json.contains("whatever.json"));
    assert!(!json.contains("file_path"));
}

#[test]
fn save_then_load_round_trips_through_a_real_file() {
    let original = fixture_project();
    let path = std::env::temp_dir().join(format!("oca_persist_test_{}.json", original.id));

    save_project_to_file(&original, &path).unwrap();
    let loaded = load_project_from_file(&path).unwrap();
    let _ = fs::remove_file(&path);

    assert_eq!(original, loaded);
}

#[test]
fn load_project_from_file_errors_on_a_missing_file() {
    let path = std::env::temp_dir().join("oca_persist_test_does_not_exist.json");
    assert!(matches!(
        load_project_from_file(&path),
        Err(PersistError::Io(_))
    ));
}
