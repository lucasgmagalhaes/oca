// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::fs;
use std::path::PathBuf;

use avcore::collab_bundle::{export_collab_bundle, import_collab_bundle};
use avcore::proxy::{cache_dir_for_project, proxy_path_for, PreviewQuality};
use avcore::timeline::{AudioRole, Timeline, Track, TrackKind};
use avcore::{MediaAsset, MediaKind, Project, Recency, Sequence};

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oca_collab_bundle_test_{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn asset(id: u64, source_path: PathBuf) -> MediaAsset {
    MediaAsset {
        id,
        file_name: source_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        source_path,
        kind: MediaKind::Video,
        has_audio: true,
        duration_secs: 10.0,
        codec: "H.264".to_string(),
        source_bitrate_mbps: 40.0,
        resolution: Some((1920, 1080)),
        fps: Some(60.0),
        sample_rate_khz: None,
        loudness: None,
        proxy_path: None,
        waveform_peaks: None,
    }
}

fn project_at(project_path: &PathBuf, assets: Vec<MediaAsset>) -> Project {
    Project {
        id: 1,
        name: "Collab bundle fixture".to_string(),
        last_edited: Recency::HoursAgo(1),
        summary: String::new(),
        media_library: assets,
        sequences: vec![Sequence {
            id: 1,
            name: "Sequência principal".to_string(),
            timeline: Timeline {
                tracks: vec![Track {
                    id: 1,
                    name: "V1".to_string(),
                    kind: TrackKind::Video,
                    clips: vec![],
                    text_clips: vec![],
                    shape_clips: vec![],
                    visible: true,
                    audio_role: AudioRole::Unspecified,
                }],
                playhead_secs: 0.0,
                markers: vec![],
                multicam_groups: Vec::new(),
            },
            export_settings: Default::default(),
        }],
        active_sequence: 0,
        file_path: Some(project_path.clone()),
        panel_layout: None,
    }
}

#[test]
fn round_trips_the_project_and_its_proxies() {
    let dir = scratch_dir("round_trip");
    let sender_project_path = dir.join("sender/project.ocproj");
    fs::create_dir_all(sender_project_path.parent().unwrap()).unwrap();
    let source_path = dir.join("original_media/clip.mp4");

    let project = project_at(&sender_project_path, vec![asset(1, source_path.clone())]);
    let proxy_dir = cache_dir_for_project(&project);
    fs::create_dir_all(&proxy_dir).unwrap();
    let proxy_file = proxy_path_for(&source_path, &proxy_dir, PreviewQuality::Medium);
    fs::write(&proxy_file, b"fake proxy bytes").unwrap();

    let zip_path = dir.join("handoff.zip");
    export_collab_bundle(&project, &zip_path).unwrap();
    assert!(zip_path.exists());

    let recipient_project_path = dir.join("recipient/project.ocproj");
    let imported = import_collab_bundle(&zip_path, &recipient_project_path).unwrap();

    assert_eq!(imported.id, project.id);
    assert_eq!(imported.name, project.name);
    assert_eq!(imported.file_path, Some(recipient_project_path.clone()));
    assert_eq!(imported.media_library.len(), 1);

    // The recipient never has `source_path` on disk -- proxy_path still resolves because it's
    // unpacked into the recipient's own proxy cache dir under the same filename.
    let recipient_proxy = imported.media_library[0].proxy_path.as_ref().unwrap();
    assert!(
        recipient_proxy.exists(),
        "unpacked proxy file should exist on disk"
    );
    assert_eq!(
        fs::read(recipient_proxy).unwrap(),
        b"fake proxy bytes",
        "proxy content should survive the round trip"
    );
    assert_ne!(
        recipient_proxy, &proxy_file,
        "recipient's proxy lives under their own project's cache dir, not the sender's"
    );
}

#[test]
fn skips_assets_with_no_proxy_generated_yet() {
    let dir = scratch_dir("no_proxy");
    let sender_project_path = dir.join("project.ocproj");
    let source_path = dir.join("clip.mp4");
    let project = project_at(&sender_project_path, vec![asset(1, source_path)]);
    // No proxy_dir/proxy file created at all -- background enrichment hasn't reached it yet.

    let zip_path = dir.join("handoff.zip");
    export_collab_bundle(&project, &zip_path).unwrap();

    let recipient_project_path = dir.join("recipient.ocproj");
    let imported = import_collab_bundle(&zip_path, &recipient_project_path).unwrap();

    assert_eq!(imported.media_library.len(), 1);
    assert!(
        imported.media_library[0].proxy_path.is_none(),
        "nothing was bundled for this asset, so no proxy_path should resolve"
    );
}

#[test]
fn works_for_a_project_with_an_empty_media_library() {
    let dir = scratch_dir("empty_library");
    let sender_project_path = dir.join("project.ocproj");
    let project = project_at(&sender_project_path, vec![]);

    let zip_path = dir.join("handoff.zip");
    export_collab_bundle(&project, &zip_path).unwrap();

    let recipient_project_path = dir.join("recipient.ocproj");
    let imported = import_collab_bundle(&zip_path, &recipient_project_path).unwrap();

    assert!(imported.media_library.is_empty());
    assert_eq!(imported.name, project.name);
}
