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

//! Project, sequence, timeline, and thumbnail-cache tests.

use super::support::*;
use super::*;

#[test]
fn open_project_switches_active_project_and_selects_its_first_asset() {
    let mut app = test_app(
        vec![
            test_project(1, Vec::new()),
            test_project(2, vec![test_asset(42), test_asset(43)]),
        ],
        Vec::new(),
    );

    app.open_project(1);

    assert_eq!(app.open_projects.active_index(), Some(1));
    assert_eq!(app.selected_asset_id, Some(42));
    assert_eq!(app.screen, Screen::Editor);
}

#[test]
fn open_project_with_an_empty_media_library_selects_nothing() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.open_project(0);

    assert_eq!(app.selected_asset_id, None);
}

#[test]
fn add_and_open_project_appends_and_opens_it() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_and_open_project(test_project(99, vec![test_asset(7)]));

    assert_eq!(app.open_projects.len(), 2);
    assert_eq!(app.open_projects.active_index(), Some(1));
    assert_eq!(app.active_project().id, 99);
    assert_eq!(app.selected_asset_id, Some(7));
}

#[test]
fn create_new_project_assigns_the_next_id_after_the_highest_existing_one() {
    let mut app = test_app(
        vec![test_project(1, Vec::new()), test_project(5, Vec::new())],
        Vec::new(),
    );

    app.create_new_project("New".to_string());

    assert_eq!(app.active_project().id, 6);
    assert_eq!(app.active_project().name, "New");
    assert_eq!(app.screen, Screen::Editor);
}

#[test]
fn create_new_project_starts_at_one_when_no_projects_exist() {
    let mut app = test_app(Vec::new(), Vec::new());

    app.create_new_project("First".to_string());

    assert_eq!(app.active_project().id, 1);
}

#[test]
fn create_new_project_uses_the_preferred_loudness_as_its_sequence_default() {
    let mut app = test_app(Vec::new(), Vec::new());
    app.prefs.lufs_profile = 2;

    app.create_new_project("Broadcast".to_string());

    assert_eq!(
        app.active_sequence_export_settings().aspect_ratio,
        avcore::ExportAspectRatio::Original
    );
    assert_eq!(app.active_sequence_export_settings().target_lufs, -23.0);
}

#[test]
fn add_sequence_appends_a_named_tab_and_switches_to_it() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_sequence();

    assert_eq!(app.active_project().sequences.len(), 2);
    assert_eq!(app.active_project().sequences[1].name, "Sequência 2");
    assert_eq!(app.active_project().active_sequence, 1);
    assert!(app.active_project().timeline().tracks.is_empty());
}

#[test]
fn add_sequence_clears_a_stale_clip_selection() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.selected_clip_id = Some(1);

    app.add_sequence();

    assert_eq!(app.selected_clip_id, None);
}

#[test]
fn duplicate_sequence_copies_the_tab_switches_to_it_and_clears_sequence_state() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.active_project_mut().timeline_mut().playhead_secs = 17.0;
    app.selected_clip_id = Some(1);
    app.selected_text_clip_id = Some(2);
    app.selected_shape_clip_id = Some(3);
    app.multi_selected_clip_ids.extend([1, 4]);
    app.preview_state.preview_clip_id = Some(1);
    app.preview_state.preview_overlay_clip_ids.push(2);
    app.preview_state.preview_playing = true;

    app.duplicate_sequence(0);

    assert_eq!(app.active_project().sequences.len(), 2);
    assert_eq!(app.active_project().active_sequence, 1);
    assert_eq!(app.active_project().sequences[1].id, 2);
    assert_eq!(
        app.active_project().sequences[1].name,
        "Cópia de Sequence 1"
    );
    assert_eq!(app.active_project().timeline().playhead_secs, 17.0);
    assert_eq!(app.selected_clip_id, None);
    assert_eq!(app.selected_text_clip_id, None);
    assert_eq!(app.selected_shape_clip_id, None);
    assert!(app.multi_selected_clip_ids.is_empty());
    assert_eq!(app.preview_state.preview_clip_id, None);
    assert!(app.preview_state.preview_overlay_clip_ids.is_empty());
    assert!(!app.preview_state.preview_playing);
}

#[test]
fn delete_sequence_removes_the_target_and_selects_a_surviving_neighbor() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_sequence();
    let second_id = app.active_project().sequences[1].id;
    app.selected_clip_id = Some(9);
    app.selected_text_clip_id = Some(8);
    app.selected_shape_clip_id = Some(7);

    app.delete_sequence(second_id);

    assert_eq!(app.active_project().sequences.len(), 1);
    assert_eq!(app.active_project().active_sequence, 0);
    assert_eq!(app.active_project().sequences[0].name, "Sequence 1");
    assert_eq!(app.selected_clip_id, None);
    assert_eq!(app.selected_text_clip_id, None);
    assert_eq!(app.selected_shape_clip_id, None);
}

#[test]
fn create_compound_clip_from_selected_clip_wraps_a_single_clip() {
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 2.0, 0.0, 5.0)]);
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());
    app.select_timeline_clip(1);

    app.create_compound_clip_from_selected_clip();

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks[0].clips.len(), 1);
    let wrapper = &timeline.tracks[0].clips[0];
    assert_eq!(
        wrapper.id, 1,
        "the wrapper keeps the original clip's own id"
    );
    assert!(wrapper.nested_sequence_id.is_some());
    assert_eq!(wrapper.start_secs, 2.0);
    assert!((wrapper.source_out_secs - 5.0).abs() < 1e-9);
    let nested_id = wrapper.nested_sequence_id.unwrap();
    let nested = app
        .active_project()
        .sequences
        .iter()
        .find(|s| s.id == nested_id)
        .unwrap();
    assert_eq!(nested.timeline.tracks[0].clips.len(), 1);
    assert_eq!(
        nested.timeline.tracks[0].clips[0].start_secs, 0.0,
        "a lone clip rebases to 0.0, same as before multi-clip compounding existed"
    );
}

#[test]
fn create_compound_clip_from_selected_clip_wraps_the_whole_multi_selection_on_the_same_track() {
    let track = test_track(
        1,
        TrackKind::Video,
        vec![
            test_clip(1, 0.0, 0.0, 4.0),
            test_clip(2, 6.0, 0.0, 3.0), // a 2s gap after clip 1, deliberately non-contiguous
        ],
    );
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());
    app.select_timeline_clip(1);
    app.multi_selected_clip_ids.insert(1);
    app.multi_selected_clip_ids.insert(2);

    app.create_compound_clip_from_selected_clip();

    let timeline = app.active_project().timeline();
    assert_eq!(
        timeline.tracks[0].clips.len(),
        1,
        "both members collapse into one compound clip"
    );
    let wrapper = &timeline.tracks[0].clips[0];
    assert_eq!(
        wrapper.id, 1,
        "the wrapper keeps the right-clicked clip's own id"
    );
    assert_eq!(
        wrapper.start_secs, 0.0,
        "spans from the earliest member's start"
    );
    assert!(
        (wrapper.duration_secs() - 9.0).abs() < 1e-9,
        "spans through the latest member's own end (clip 2 ends at 9.0)"
    );
    let nested_id = wrapper.nested_sequence_id.unwrap();
    let nested = app
        .active_project()
        .sequences
        .iter()
        .find(|s| s.id == nested_id)
        .unwrap();
    let mut nested_clips = nested.timeline.tracks[0].clips.clone();
    nested_clips.sort_by(|a, b| a.start_secs.total_cmp(&b.start_secs));
    assert_eq!(nested_clips.len(), 2);
    assert_eq!(
        nested_clips[0].start_secs, 0.0,
        "the earliest member rebases to 0.0"
    );
    assert_eq!(
        nested_clips[1].start_secs, 6.0,
        "the later member keeps its 6s gap relative to the earliest one, not rebased to 0.0"
    );
}

#[test]
fn create_compound_clip_from_selected_clip_falls_back_to_single_clip_across_tracks() {
    let video_track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 4.0)]);
    let audio_track = test_track(2, TrackKind::Audio, vec![test_clip(2, 0.0, 0.0, 4.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks(1, vec![video_track, audio_track])],
        Vec::new(),
    );
    app.select_timeline_clip(1);
    app.multi_selected_clip_ids.insert(1);
    app.multi_selected_clip_ids.insert(2);

    app.create_compound_clip_from_selected_clip();

    let timeline = app.active_project().timeline();
    let video_track = timeline
        .tracks
        .iter()
        .find(|t| t.kind == TrackKind::Video)
        .unwrap();
    let audio_track = timeline
        .tracks
        .iter()
        .find(|t| t.kind == TrackKind::Audio)
        .unwrap();
    assert_eq!(
        video_track.clips.len(),
        1,
        "only the selected clip's own track is touched"
    );
    assert!(video_track.clips[0].nested_sequence_id.is_some());
    assert_eq!(
        audio_track.clips.len(),
        1,
        "a multi-selection spanning tracks never touches the other track's clip"
    );
    assert!(audio_track.clips[0].nested_sequence_id.is_none());
}

#[test]
fn insert_sequence_as_compound_clip_appends_a_nested_clip_on_the_active_timeline() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let nested_id = app.active_project().sequences[0].id + 1;
    app.active_project_mut().sequences.push(Sequence {
        id: nested_id,
        name: "Other Sequence".to_string(),
        timeline: Timeline {
            tracks: vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 8.0)],
            )],
            playhead_secs: 0.0,
            markers: Vec::new(),
            multicam_groups: Vec::new(),
        },
        export_settings: Default::default(),
    });

    app.insert_sequence_as_compound_clip(nested_id);

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].kind, TrackKind::Video);
    assert_eq!(timeline.tracks[0].clips.len(), 1);
    let clip = &timeline.tracks[0].clips[0];
    assert_eq!(clip.nested_sequence_id, Some(nested_id));
    assert_eq!(clip.start_secs, 0.0);
    assert!((clip.source_out_secs - 8.0).abs() < 1e-9);
}

#[test]
fn insert_sequence_as_compound_clip_appends_after_existing_clips_on_the_first_video_track() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 5.0)],
            )],
        )],
        Vec::new(),
    );
    let nested_id = 99;
    app.active_project_mut().sequences.push(Sequence {
        id: nested_id,
        name: "Other Sequence".to_string(),
        timeline: Timeline {
            tracks: vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 3.0)],
            )],
            playhead_secs: 0.0,
            markers: Vec::new(),
            multicam_groups: Vec::new(),
        },
        export_settings: Default::default(),
    });

    app.insert_sequence_as_compound_clip(nested_id);

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks[0].clips.len(), 2);
    let inserted = &timeline.tracks[0].clips[1];
    assert_eq!(inserted.nested_sequence_id, Some(nested_id));
    assert_eq!(
        inserted.start_secs, 5.0,
        "must append after the existing clip"
    );
}

#[test]
fn insert_sequence_as_compound_clip_creates_a_video_track_when_none_exists() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let nested_id = 42;
    app.active_project_mut().sequences.push(Sequence {
        id: nested_id,
        name: "Other Sequence".to_string(),
        timeline: Timeline {
            tracks: vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 4.0)],
            )],
            playhead_secs: 0.0,
            markers: Vec::new(),
            multicam_groups: Vec::new(),
        },
        export_settings: Default::default(),
    });
    assert!(app.active_project().timeline().tracks.is_empty());

    app.insert_sequence_as_compound_clip(nested_id);

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].kind, TrackKind::Video);
}

#[test]
fn insert_sequence_as_compound_clip_refuses_the_active_sequence_itself() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let active_id = app.active_project().sequences[0].id;

    app.insert_sequence_as_compound_clip(active_id);

    assert!(app.active_project().timeline().tracks.is_empty());
}

#[test]
fn insert_sequence_as_compound_clip_refuses_a_nonexistent_sequence_id() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.insert_sequence_as_compound_clip(999);

    assert!(app.active_project().timeline().tracks.is_empty());
}

#[test]
fn insert_sequence_as_compound_clip_refuses_an_empty_nested_sequence() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let nested_id = 7;
    app.active_project_mut().sequences.push(Sequence {
        id: nested_id,
        name: "Empty Sequence".to_string(),
        timeline: Timeline {
            tracks: Vec::new(),
            playhead_secs: 0.0,
            markers: Vec::new(),
            multicam_groups: Vec::new(),
        },
        export_settings: Default::default(),
    });

    app.insert_sequence_as_compound_clip(nested_id);

    assert!(app.active_project().timeline().tracks.is_empty());
}

#[test]
fn insert_sequence_as_compound_clip_pushes_one_undo_snapshot() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let nested_id = 5;
    app.active_project_mut().sequences.push(Sequence {
        id: nested_id,
        name: "Other Sequence".to_string(),
        timeline: Timeline {
            tracks: vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 4.0)],
            )],
            playhead_secs: 0.0,
            markers: Vec::new(),
            multicam_groups: Vec::new(),
        },
        export_settings: Default::default(),
    });

    app.insert_sequence_as_compound_clip(nested_id);
    assert!(app.can_undo());
    app.undo();

    assert!(app.active_project().timeline().tracks.is_empty());
}

#[test]
fn delete_sequence_refuses_to_remove_the_last_tab() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let only_id = app.active_project().sequences[0].id;

    app.delete_sequence(only_id);

    assert_eq!(app.active_project().sequences.len(), 1);
    assert_eq!(app.active_project().active_sequence, 0);
}

#[test]
fn deleting_an_inactive_sequence_preserves_the_active_sequence_state() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_sequence();
    let inactive_id = app.active_project().sequences[0].id;
    let active_id = app.active_project().sequences[1].id;
    app.selected_clip_id = Some(42);
    app.preview_state.preview_clip_id = Some(42);

    app.delete_sequence(inactive_id);

    assert_eq!(app.active_project().sequences.len(), 1);
    assert_eq!(app.active_project().sequences[0].id, active_id);
    assert_eq!(app.active_project().active_sequence, 0);
    assert_eq!(app.selected_clip_id, Some(42));
    assert_eq!(app.preview_state.preview_clip_id, Some(42));
}

#[test]
fn move_sequence_reorders_tabs_while_preserving_active_identity_and_selection() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_sequence();
    app.add_sequence();
    app.select_sequence(0);
    let active_id = app.active_project().sequences[0].id;
    app.selected_clip_id = Some(42);

    app.move_sequence(0, 2);

    let names: Vec<&str> = app
        .active_project()
        .sequences
        .iter()
        .map(|sequence| sequence.name.as_str())
        .collect();
    assert_eq!(names, vec!["Sequência 2", "Sequência 3", "Sequence 1"]);
    assert_eq!(app.active_project().active_sequence, 2);
    assert_eq!(
        app.active_project().sequences[app.active_project().active_sequence].id,
        active_id
    );
    assert_eq!(app.selected_clip_id, Some(42));
}

#[test]
fn select_sequence_switches_the_active_index() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_sequence();
    app.select_sequence(0);

    assert_eq!(app.active_project().active_sequence, 0);
}

#[test]
fn select_sequence_is_a_no_op_for_an_out_of_range_index() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.select_sequence(5);

    assert_eq!(app.active_project().active_sequence, 0);
}

#[test]
fn export_settings_follow_the_active_sequence_without_leaking_between_tabs() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.set_active_sequence_export_aspect_ratio(avcore::ExportAspectRatio::Landscape);
    app.set_active_sequence_target_lufs(-16.0);
    app.add_sequence();
    assert_eq!(
        app.active_sequence_export_settings().aspect_ratio,
        avcore::ExportAspectRatio::Landscape
    );
    assert_eq!(app.active_sequence_export_settings().target_lufs, -16.0);

    app.set_active_sequence_export_aspect_ratio(avcore::ExportAspectRatio::Portrait);
    app.set_active_sequence_target_lufs(-23.0);
    app.select_sequence(0);

    assert_eq!(
        app.active_sequence_export_settings().aspect_ratio,
        avcore::ExportAspectRatio::Landscape
    );
    assert_eq!(app.active_sequence_export_settings().target_lufs, -16.0);
    assert_eq!(
        app.active_project().sequences[1]
            .export_settings
            .aspect_ratio,
        avcore::ExportAspectRatio::Portrait
    );
    assert_eq!(
        app.active_project().sequences[1]
            .export_settings
            .target_lufs,
        -23.0
    );
}

#[test]
fn add_asset_to_timeline_creates_a_track_and_appends_a_clip_when_none_exists() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.add_asset_to_timeline(1);

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].name, "V1");
    assert_eq!(tracks[0].kind, TrackKind::Video);
    assert_eq!(tracks[0].clips.len(), 1);
    let clip = &tracks[0].clips[0];
    assert_eq!(clip.asset_id, 1);
    assert_eq!(clip.start_secs, 0.0);
    assert_eq!(clip.source_in_secs, 0.0);
    assert_eq!(clip.source_out_secs, 10.0); // test_asset's fixed duration_secs.
}

#[test]
fn add_asset_to_timeline_creates_an_audio_track_for_an_audio_asset() {
    let mut app = test_app(
        vec![test_project(
            1,
            vec![test_asset_with_kind(1, MediaKind::Audio)],
        )],
        Vec::new(),
    );

    app.add_asset_to_timeline(1);

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks[0].name, "A1");
    assert_eq!(tracks[0].kind, TrackKind::Audio);
}

#[test]
fn add_asset_to_timeline_appends_after_whatever_is_already_on_the_matching_track() {
    let mut project = test_project(1, vec![test_asset(2)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 20.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.add_asset_to_timeline(2);

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1); // Reused the existing Video track, no second one created.
    assert_eq!(tracks[0].clips.len(), 2);
    assert_eq!(tracks[0].clips[1].start_secs, 20.0);
    assert_eq!(tracks[0].clips[1].asset_id, 2);
}

#[test]
fn add_asset_to_timeline_defaults_voice_cleanup_on_for_a_mic_role_track() {
    // CF-03's "Mic by default, explicit override elsewhere" acceptance criterion.
    let mut project = test_project(1, vec![test_asset_with_kind(2, MediaKind::Audio)]);
    let mut mic_track = test_track(1, TrackKind::Audio, vec![]);
    mic_track.audio_role = AudioRole::Mic;
    project.timeline_mut().tracks = vec![mic_track];
    let mut app = test_app(vec![project], Vec::new());

    app.add_asset_to_timeline(2);

    let tracks = &app.active_project().timeline().tracks;
    assert!(tracks[0].clips[0].voice_cleanup_enabled);
}

#[test]
fn add_asset_to_timeline_leaves_voice_cleanup_off_for_a_non_mic_role_track() {
    let mut app = test_app(
        vec![test_project(
            1,
            vec![test_asset_with_kind(1, MediaKind::Audio)],
        )],
        Vec::new(),
    );

    app.add_asset_to_timeline(1);

    let tracks = &app.active_project().timeline().tracks;
    assert!(!tracks[0].clips[0].voice_cleanup_enabled);
}

#[test]
fn add_asset_to_timeline_is_a_no_op_for_an_unknown_asset_id() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_asset_to_timeline(99);

    assert!(app.active_project().timeline().tracks.is_empty());
}

#[test]
fn add_asset_to_new_track_always_creates_a_fresh_track_even_when_a_matching_one_exists() {
    let mut project = test_project(1, vec![test_asset(2)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 20.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.add_asset_to_new_track(2);

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 2, "existing V1 must be left untouched");
    assert_eq!(tracks[0].clips.len(), 1);
    assert_eq!(tracks[1].name, "V2");
    assert_eq!(tracks[1].clips.len(), 1);
    // A brand-new track is always empty, so the clip starts at 0 — this is what makes a batch
    // of simultaneously-dropped files land on parallel (time-aligned) tracks.
    assert_eq!(tracks[1].clips[0].start_secs, 0.0);
    assert_eq!(tracks[1].clips[0].asset_id, 2);
}

#[test]
fn add_asset_to_new_track_gives_each_call_its_own_track() {
    let mut app = test_app(
        vec![test_project(1, vec![test_asset(1), test_asset(2)])],
        Vec::new(),
    );

    app.add_asset_to_new_track(1);
    app.add_asset_to_new_track(2);

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].name, "V1");
    assert_eq!(tracks[1].name, "V2");
    assert_eq!(tracks[0].clips[0].asset_id, 1);
    assert_eq!(tracks[1].clips[0].asset_id, 2);
    assert_eq!(tracks[0].clips[0].start_secs, 0.0);
    assert_eq!(tracks[1].clips[0].start_secs, 0.0);
}

#[test]
fn add_asset_to_new_track_is_a_no_op_for_an_unknown_asset_id() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_asset_to_new_track(99);

    assert!(app.active_project().timeline().tracks.is_empty());
}

#[test]
fn request_thumbnail_marks_the_frame_pending_and_does_not_duplicate_on_a_second_call() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    app.request_thumbnail(1, 1, 0);
    assert!(app.thumbnail_state.pending_thumbnails.contains(&(1, 1, 0)));

    app.request_thumbnail(1, 1, 0);
    assert_eq!(app.thumbnail_state.pending_thumbnails.len(), 1);
}

#[test]
fn request_thumbnail_remembers_an_unknown_asset_as_a_bounded_failure() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.request_thumbnail(1, 99, 0);

    assert!(app
        .thumbnail_state
        .failed_thumbnails
        .contains_key(&(1, 99, 0)));
    assert!(app.thumbnail_state.pending_thumbnails.is_empty());
}

#[test]
fn thumbnail_frame_keys_follow_source_fps_and_have_a_safe_fallback() {
    assert_eq!(thumbnail_frame_index(1.5, Some(60.0)), 90);
    assert_eq!(thumbnail_frame_index(1.5, None), 45);
    assert_eq!(thumbnail_frame_index(-2.0, Some(30.0)), 0);
    assert_eq!(thumbnail_frame_time(90, Some(60.0)), 1.5);
}

#[test]
fn thumbnail_fps_uses_a_finite_positive_source_fps_as_is() {
    assert_eq!(thumbnail_fps(Some(24.0)), 24.0);
}

#[test]
fn thumbnail_fps_falls_back_to_30_for_a_missing_source_fps() {
    assert_eq!(thumbnail_fps(None), 30.0);
}

#[test]
fn thumbnail_fps_falls_back_to_30_for_a_zero_or_negative_source_fps() {
    assert_eq!(thumbnail_fps(Some(0.0)), 30.0);
    assert_eq!(thumbnail_fps(Some(-5.0)), 30.0);
}

#[test]
fn thumbnail_fps_falls_back_to_30_for_a_non_finite_source_fps() {
    assert_eq!(thumbnail_fps(Some(f32::NAN)), 30.0);
    assert_eq!(thumbnail_fps(Some(f32::INFINITY)), 30.0);
}

#[test]
fn thumbnail_requests_cap_concurrent_extractions() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(1)])], Vec::new());

    for frame_index in 0..(THUMBNAIL_MAX_PENDING as i64 + 10) {
        app.request_thumbnail(1, 1, frame_index);
    }

    assert_eq!(
        app.thumbnail_state.pending_thumbnails.len(),
        THUMBNAIL_MAX_PENDING
    );
}

#[test]
fn failed_thumbnail_suppression_is_bounded() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    for frame_index in 0..(THUMBNAIL_FAILURE_CAPACITY as i64 + 1) {
        app.request_thumbnail(1, 99, frame_index);
    }

    assert_eq!(
        app.thumbnail_state.failed_thumbnails.len(),
        THUMBNAIL_FAILURE_CAPACITY
    );
    assert!(!app
        .thumbnail_state
        .failed_thumbnails
        .contains_key(&(1, 99, 0)));
    assert!(app.thumbnail_state.failed_thumbnails.contains_key(&(
        1,
        99,
        THUMBNAIL_FAILURE_CAPACITY as i64
    )));
}

#[test]
fn thumbnail_texture_cache_evicts_the_least_recently_used_entry() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let ctx = egui::Context::default();
    for frame_index in 0..THUMBNAIL_CACHE_CAPACITY as i64 {
        app.thumbnail_state
            .thumbnail_tx
            .send(ThumbnailReady::Ready {
                project_id: 1,
                asset_id: 1,
                frame_index,
                width: 1,
                height: 1,
                rgba: vec![0, 0, 0, 255],
            })
            .unwrap();
    }
    app.pump_thumbnail_queue(&ctx);
    app.touch_thumbnails(&[(1, 1, 0)]);

    app.thumbnail_state
        .thumbnail_tx
        .send(ThumbnailReady::Ready {
            project_id: 1,
            asset_id: 1,
            frame_index: THUMBNAIL_CACHE_CAPACITY as i64,
            width: 1,
            height: 1,
            rgba: vec![255, 255, 255, 255],
        })
        .unwrap();
    app.pump_thumbnail_queue(&ctx);

    assert_eq!(
        app.thumbnail_state.thumbnail_textures.len(),
        THUMBNAIL_CACHE_CAPACITY
    );
    assert!(app
        .thumbnail_state
        .thumbnail_textures
        .contains_key(&(1, 1, 0)));
    assert!(!app
        .thumbnail_state
        .thumbnail_textures
        .contains_key(&(1, 1, 1)));
    assert!(app.thumbnail_state.thumbnail_textures.contains_key(&(
        1,
        1,
        THUMBNAIL_CACHE_CAPACITY as i64
    )));
}
