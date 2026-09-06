// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Highlight detection and gameplay-event import tests.

use super::support::*;
use super::*;

fn spiky_peaks(spike_range: std::ops::Range<usize>) -> Vec<(f32, f32)> {
    let mut peaks = vec![(-0.1, 0.1); 20];
    for p in &mut peaks[spike_range] {
        *p = (-0.9, 0.9);
    }
    peaks
}

fn highlight_test_project() -> Project {
    let game_asset = MediaAsset {
        waveform_peaks: Some(spiky_peaks(6..12)),
        favorited: false,
        duration_secs: 10.0,
        ..test_asset(1)
    };
    let mic_asset = MediaAsset {
        id: 2,
        waveform_peaks: Some(spiky_peaks(6..12)),
        favorited: false,
        duration_secs: 10.0,
        ..test_asset(2)
    };
    let mut game_track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    game_track.audio_role = avcore::AudioRole::GameAudio;

    let mic_clip = ClipInstance {
        asset_id: 2,
        ..test_clip(2, 0.0, 0.0, 10.0)
    };
    let mut mic_track = test_track(2, TrackKind::Audio, vec![mic_clip]);
    mic_track.audio_role = avcore::AudioRole::Mic;

    test_project_with_tracks_and_assets(1, vec![game_track, mic_track], vec![game_asset, mic_asset])
}

#[test]
fn detect_highlights_adds_a_marker_at_the_simultaneous_spike() {
    let mut app = test_app(vec![highlight_test_project()], Vec::new());

    app.detect_highlights();

    let markers = app.active_project().timeline().markers_sorted();
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].kind, avcore::MarkerKind::Highlight);
    assert_eq!(markers[0].position_secs, 3.0);
}

#[test]
fn detect_highlights_toasts_when_a_role_is_missing() {
    // Only a game-audio track, no mic track tagged.
    let track = {
        let mut t = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
        t.audio_role = avcore::AudioRole::GameAudio;
        t
    };
    let asset = MediaAsset {
        waveform_peaks: Some(spiky_peaks(3..6)),
        favorited: false,
        duration_secs: 10.0,
        ..test_asset(1)
    };
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![asset],
        )],
        Vec::new(),
    );

    app.detect_highlights();

    assert!(app.active_project().timeline().markers.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn detect_highlights_toasts_when_nothing_spikes_together() {
    let project = {
        let mut p = highlight_test_project();
        // Mic never spikes.
        p.media_library[1].waveform_peaks = Some(vec![(-0.1, 0.1); 20]);
        p
    };
    let mut app = test_app(vec![project], Vec::new());

    app.detect_highlights();

    assert!(app.active_project().timeline().markers.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

fn sidecar_json(source_media_filename: &str, event_source_timestamp_secs: f64) -> String {
    format!(
        r#"{{"schema_version":1,"source_media_filename":"{source_media_filename}","events":[{{"kind":"kill","source_timestamp_secs":{event_source_timestamp_secs},"confidence":1.0}}]}}"#
    )
}

fn write_sidecar(dir: &std::path::Path, name: &str, json: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, json).unwrap();
    path
}

#[test]
fn import_gameplay_events_adds_a_marker_for_a_matching_clip() {
    let asset = test_asset(1); // file_name: "asset-1.mp4"
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![asset],
        )],
        Vec::new(),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = write_sidecar(dir.path(), "events.json", &sidecar_json("asset-1.mp4", 4.0));

    app.import_gameplay_events(path);

    let markers = app.active_project().timeline().markers_sorted();
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].kind, avcore::MarkerKind::Highlight);
    assert_eq!(markers[0].position_secs, 4.0);
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn import_gameplay_events_is_idempotent() {
    let asset = test_asset(1);
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![asset],
        )],
        Vec::new(),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = write_sidecar(dir.path(), "events.json", &sidecar_json("asset-1.mp4", 4.0));

    app.import_gameplay_events(path.clone());
    app.import_gameplay_events(path);

    let markers = app.active_project().timeline().markers_sorted();
    assert_eq!(markers.len(), 1, "re-importing must not duplicate markers");
}

#[test]
fn import_gameplay_events_toasts_when_no_clip_uses_the_recording() {
    let asset = test_asset(1); // file_name: "asset-1.mp4"
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![asset],
        )],
        Vec::new(),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = write_sidecar(dir.path(), "events.json", &sidecar_json("other.mp4", 4.0));

    app.import_gameplay_events(path);

    assert!(app.active_project().timeline().markers.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn import_gameplay_events_toasts_when_the_event_falls_outside_the_trimmed_clip_range() {
    let asset = test_asset(1);
    // Clip only covers source seconds [2.0, 5.0).
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 2.0, 5.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![asset],
        )],
        Vec::new(),
    );
    let dir = tempfile::tempdir().unwrap();
    // Event at source second 20 — well outside the clip's trimmed range.
    let path = write_sidecar(
        dir.path(),
        "events.json",
        &sidecar_json("asset-1.mp4", 20.0),
    );

    app.import_gameplay_events(path);

    assert!(app.active_project().timeline().markers.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn import_gameplay_events_toasts_on_invalid_json() {
    let mut app = test_app(Vec::new(), Vec::new());
    let dir = tempfile::tempdir().unwrap();
    let path = write_sidecar(dir.path(), "events.json", "not valid json");

    app.import_gameplay_events(path);

    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn import_gameplay_events_toasts_when_the_file_cannot_be_read() {
    let mut app = test_app(Vec::new(), Vec::new());

    app.import_gameplay_events(PathBuf::from("/does/not/exist.json"));

    assert_eq!(app.toasts.len(), 1);
}
