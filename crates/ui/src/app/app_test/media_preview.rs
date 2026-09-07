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

//! Media selection, preview lifecycle, preferences, and transcript-panel tests.

use super::support::*;
use super::*;

#[test]
fn selected_asset_is_none_when_no_asset_id_is_selected() {
    let app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    assert!(app.selected_asset().is_none());
}

#[test]
fn selected_asset_finds_the_asset_in_the_active_project() {
    let mut app = test_app(
        vec![test_project(1, vec![test_asset(1), test_asset(2)])],
        Vec::new(),
    );
    app.selected_asset_id = Some(2);

    assert_eq!(app.selected_asset().unwrap().id, 2);
}

#[test]
fn select_asset_updates_the_selection() {
    let mut app = test_app(
        vec![test_project(1, vec![test_asset(1), test_asset(2)])],
        Vec::new(),
    );

    app.select_asset(Some(2));

    assert_eq!(app.selected_asset_id, Some(2));
}

#[test]
fn select_asset_with_none_clears_the_selection() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());
    app.selected_asset_id = Some(1);

    app.select_asset(None);

    assert_eq!(app.selected_asset_id, None);
}

#[test]
fn select_asset_only_updates_selected_asset_id() {
    // Selecting a media-library asset no longer touches preview state — preview follows the
    // timeline playhead now, a separate concept from the library selection (see
    // `current_preview_clip`).
    let mut app = test_app(
        vec![test_project(1, vec![test_asset(1), test_asset(2)])],
        Vec::new(),
    );
    app.selected_asset_id = Some(1);
    app.preview_state.preview_playing = true;
    let ctx = egui::Context::default();
    let image = egui::ColorImage::new([1, 1], vec![egui::Color32::BLACK]);
    app.preview_state.preview_texture =
        Some(ctx.load_texture("test", image, egui::TextureOptions::default()));

    app.select_asset(Some(2));

    assert_eq!(app.selected_asset_id, Some(2));
    assert!(app.preview_state.preview_playing);
    assert!(app.preview_state.preview_texture.is_some());
}

#[test]
fn toggle_asset_favorite_flips_the_flag_on_the_matching_asset() {
    let mut app = test_app(
        vec![test_project(1, vec![test_asset(1), test_asset(2)])],
        Vec::new(),
    );

    app.toggle_asset_favorite(1);

    assert!(app.active_project().media_library[0].favorited);
    assert!(!app.active_project().media_library[1].favorited);

    app.toggle_asset_favorite(1);

    assert!(!app.active_project().media_library[0].favorited);
}

#[test]
fn toggle_asset_favorite_is_a_no_op_for_an_unknown_asset_id() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.toggle_asset_favorite(999);

    assert!(!app.active_project().media_library[0].favorited);
}

// test_asset()'s source_path is a relative, nonexistent file, so `ensure_preview_loaded`
// always bails out before actually touching GStreamer here (see the `path.exists()` guard in
// app.rs) — these exercise the no-pipeline branches of the preview API, not real playback.
#[test]
fn pump_preview_frame_ignores_a_fresh_session_without_a_project() {
    let mut app = test_app(Vec::new(), Vec::new());
    let ctx = egui::Context::default();

    app.pump_preview_frame(&ctx);

    assert!(app.open_projects.is_empty());
}

#[test]
fn preview_available_is_false_without_a_live_pipeline() {
    let app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    assert!(!app.preview_available());
}

#[test]
fn toggle_preview_playback_is_a_no_op_without_a_live_pipeline() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.toggle_preview_playback();

    assert!(!app.preview_state.preview_playing);
}

#[test]
fn seek_preview_updates_the_timeline_playhead_without_a_live_pipeline() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.seek_preview(5.0);

    assert_eq!(app.active_project().timeline().playhead_secs, 5.0);
}

#[test]
fn frozen_playhead_advances_by_elapsed_wall_clock_time() {
    use super::preview::frozen_playhead;

    assert_eq!(frozen_playhead(2.0, 1.5, 0.0, 10.0), 3.5);
}

#[test]
fn frozen_playhead_clamps_to_the_clip_end() {
    use super::preview::frozen_playhead;

    // clip spans [5.0, 5.0+2.0) = [5.0, 7.0); way more than 2s elapsed should still land
    // exactly on the clip's end, not run past it.
    assert_eq!(frozen_playhead(5.0, 100.0, 5.0, 2.0), 7.0);
}

#[test]
fn preview_hardware_decode_is_enabled_by_default() {
    assert!(PrefsState::default().preview_hardware_decode);
}

#[test]
fn gpu_encoder_defaults_to_automatic_hardware_detection() {
    assert_eq!(
        PrefsState::default().gpu_encoder,
        avcore::GpuEncoderPreference::Auto
    );
}

#[test]
fn vaapi_gpu_encoder_preference_roundtrips_through_preferences() {
    let prefs = PrefsState {
        gpu_encoder: avcore::GpuEncoderPreference::Vaapi,
        ..PrefsState::default()
    };

    let encoded = serde_json::to_vec(&prefs).unwrap();
    let decoded: PrefsState = serde_json::from_slice(&encoded).unwrap();

    assert_eq!(decoded.gpu_encoder, avcore::GpuEncoderPreference::Vaapi);
}

#[test]
fn videotoolbox_gpu_encoder_preference_roundtrips_through_preferences() {
    let prefs = PrefsState {
        gpu_encoder: avcore::GpuEncoderPreference::VideoToolbox,
        ..PrefsState::default()
    };

    let encoded = serde_json::to_vec(&prefs).unwrap();
    let decoded: PrefsState = serde_json::from_slice(&encoded).unwrap();

    assert_eq!(
        decoded.gpu_encoder,
        avcore::GpuEncoderPreference::VideoToolbox
    );
}

#[test]
fn prefs_without_preview_hardware_decode_migrate_to_enabled() {
    let mut legacy = serde_json::to_value(PrefsState::default()).unwrap();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("preview_hardware_decode");

    let migrated: PrefsState = serde_json::from_value(legacy).unwrap();

    assert!(migrated.preview_hardware_decode);
}

#[test]
fn changing_preview_hardware_decode_invalidates_preview_state() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.preview_state.preview_clip_id = Some(7);
    app.preview_state.preview_overlay_clip_ids = vec![8];
    app.preview_state.preview_audio_clip_ids = vec![9];

    app.set_preview_hardware_decode(false);

    assert!(!app.prefs.preview_hardware_decode);
    assert_eq!(app.preview_state.preview_clip_id, None);
    assert!(app.preview_state.preview_overlay_clip_ids.is_empty());
    assert!(app.preview_state.preview_audio_clip_ids.is_empty());
}

#[test]
fn invalidate_preview_rendering_drops_the_scope_textures_too() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let ctx = egui::Context::default();
    let image = egui::ColorImage::new([1, 1], vec![egui::Color32::BLACK]);
    app.preview_state.waveform_texture = Some(ctx.load_texture(
        "waveform-test",
        image.clone(),
        egui::TextureOptions::default(),
    ));
    app.preview_state.vectorscope_texture =
        Some(ctx.load_texture("vectorscope-test", image, egui::TextureOptions::default()));

    app.invalidate_preview_rendering();

    assert!(app.preview_state.waveform_texture.is_none());
    assert!(app.preview_state.vectorscope_texture.is_none());
}

#[test]
fn ensure_preview_loaded_is_a_no_op_with_no_video_track() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.ensure_preview_loaded();

    assert!(!app.preview_clip_present());
    assert!(!app.preview_available());
}

#[test]
fn ensure_preview_loaded_does_not_mark_a_clip_present_when_its_asset_is_missing() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 10.0)],
            )],
        )],
        Vec::new(),
    );

    app.ensure_preview_loaded();

    // test_clip's asset_id (1) isn't in this project's (empty) media_library, so
    // current_preview_clip() actually resolves to None here — covers the "clip exists on the
    // track but its asset can't be found" branch distinctly from "no clip at all".
    assert!(!app.preview_clip_present());
}

#[test]
fn ensure_preview_loaded_resolves_the_clip_covering_the_playhead() {
    let mut project = test_project_with_tracks(
        1,
        vec![test_track(
            1,
            TrackKind::Video,
            vec![test_clip(1, 0.0, 0.0, 10.0)],
        )],
    );
    project.media_library.push(test_asset(1));
    project.timeline_mut().playhead_secs = 3.0;
    let mut app = test_app(vec![project], Vec::new());

    app.ensure_preview_loaded();

    // The clip covers the playhead and its asset resolves, so a pipeline open was attempted
    // (and failed only because test_asset's source_path doesn't exist on disk).
    assert!(app.preview_clip_present());
    assert!(!app.preview_available());
}

#[test]
fn ensure_preview_loaded_clears_state_once_the_playhead_moves_past_every_clip() {
    let mut project = test_project_with_tracks(
        1,
        vec![test_track(
            1,
            TrackKind::Video,
            vec![test_clip(1, 0.0, 0.0, 10.0)],
        )],
    );
    project.media_library.push(test_asset(1));
    project.timeline_mut().playhead_secs = 3.0;
    let mut app = test_app(vec![project], Vec::new());
    app.ensure_preview_loaded();
    assert!(app.preview_clip_present());

    app.active_project_mut().timeline_mut().playhead_secs = 20.0;
    app.preview_state.preview_playing = true;
    app.ensure_preview_loaded();

    assert!(!app.preview_clip_present());
    assert!(!app.preview_state.preview_playing);
}

#[test]
fn toggle_transcript_panel_flips_the_open_flag() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    assert!(!app.transcript_panel_open);
    app.toggle_transcript_panel();
    assert!(app.transcript_panel_open);
    app.toggle_transcript_panel();
    assert!(!app.transcript_panel_open);
}

#[test]
fn ensure_transcript_loaded_for_preview_clears_state_with_no_clip() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.transcript_panel_state.loaded_asset_id = Some(99);
    app.ensure_transcript_loaded_for_preview();
    assert_eq!(app.transcript_panel_state.loaded_asset_id, None);
    assert!(app.transcript_panel_state.document.is_none());
}

#[test]
fn ensure_transcript_loaded_for_preview_finds_no_document_when_none_saved() {
    let dir = tempfile::tempdir().unwrap();
    let mut project = test_project_with_tracks_and_assets(
        1,
        vec![test_track(
            1,
            TrackKind::Video,
            vec![test_clip(1, 0.0, 0.0, 10.0)],
        )],
        vec![test_asset(1)],
    );
    project.file_path = Some(dir.path().join("proj.ocproj"));
    project.timeline_mut().playhead_secs = 3.0;
    let mut app = test_app(vec![project], Vec::new());

    app.ensure_transcript_loaded_for_preview();

    assert_eq!(app.transcript_panel_state.loaded_asset_id, Some(1));
    assert!(app.transcript_panel_state.document.is_none());
}

#[test]
fn ensure_transcript_loaded_for_preview_loads_a_saved_document() {
    let dir = tempfile::tempdir().unwrap();
    let mut project = test_project_with_tracks_and_assets(
        1,
        vec![test_track(
            1,
            TrackKind::Video,
            vec![test_clip(1, 0.0, 0.0, 10.0)],
        )],
        vec![test_asset(1)],
    );
    project.file_path = Some(dir.path().join("proj.ocproj"));
    project.timeline_mut().playhead_secs = 3.0;

    let cache_dir = avcore::transcript_cache_dir_for_project(&project);
    let segments = vec![avcore::transcribe::TranscribeSegment {
        start_secs: 0.0,
        end_secs: 1.0,
        text: "hi".to_string(),
        words: vec![avcore::transcribe::TranscribeWord {
            text: "hi".to_string(),
            start_secs: 0.0,
            end_secs: 0.5,
            confidence: 0.9,
        }],
    }];
    let document = avcore::TranscriptDocument::from_transcribe_segments(1, None, &segments);
    avcore::save_transcript_document(&cache_dir, &document).unwrap();

    let mut app = test_app(vec![project], Vec::new());
    app.ensure_transcript_loaded_for_preview();

    assert_eq!(app.transcript_panel_state.loaded_asset_id, Some(1));
    assert_eq!(app.transcript_panel_state.document, Some(document));
}

#[test]
fn save_transcript_document_for_refreshes_an_already_open_panel() {
    let dir = tempfile::tempdir().unwrap();
    let mut project = test_project(1, vec![test_asset(1)]);
    project.file_path = Some(dir.path().join("proj.ocproj"));
    let mut app = test_app(vec![project], Vec::new());
    app.transcript_panel_state.loaded_asset_id = Some(1);
    assert!(app.transcript_panel_state.document.is_none());

    let segments = vec![avcore::transcribe::TranscribeSegment {
        start_secs: 0.0,
        end_secs: 1.0,
        text: "hi".to_string(),
        words: vec![avcore::transcribe::TranscribeWord {
            text: "hi".to_string(),
            start_secs: 0.0,
            end_secs: 0.5,
            confidence: 0.9,
        }],
    }];
    app.save_transcript_document_for(1, &segments);

    let document = app
        .transcript_panel_state
        .document
        .as_ref()
        .expect("panel already showed asset 1, so saving its transcript should refresh it");
    assert_eq!(document.asset_id, 1);
    assert_eq!(document.words.len(), 1);
}
