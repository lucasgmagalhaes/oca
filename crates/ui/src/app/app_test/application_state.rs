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

//! Application update, voice preview, and preference state tests.

use super::support::*;
use super::*;

#[test]
fn pump_update_check_sets_available_status_on_a_newer_version_event() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.update_check_tx
        .send(UpdateCheckEvent::NewerVersionAvailable {
            version: "99.0.0".to_string(),
            html_url: "https://github.com/lucasgmagalhaes/oca/releases/tag/v99.0.0".to_string(),
            auto_update_available: true,
        })
        .unwrap();

    app.pump_update_check();

    assert_eq!(
        app.update_check_status,
        UpdateCheckStatus::Available(AvailableUpdate {
            version: "99.0.0".to_string(),
            html_url: "https://github.com/lucasgmagalhaes/oca/releases/tag/v99.0.0".to_string(),
            auto_update_available: true,
        })
    );
}

#[test]
fn pump_update_check_is_a_no_op_with_no_pending_events() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.pump_update_check();

    assert_eq!(app.update_check_status, UpdateCheckStatus::Checking);
}

fn test_voice_cleanup_preview_result(
    integrated_lufs_bypassed: f32,
    integrated_lufs_processed: f32,
) -> avcore::voice_cleanup_preview::VoiceCleanupPreviewResult {
    let metrics = |integrated_lufs: f32| avcore::media::LoudnessMetrics {
        integrated_lufs,
        true_peak_dbtp: -1.0,
        loudness_range_lu: 5.0,
    };
    avcore::voice_cleanup_preview::VoiceCleanupPreviewResult {
        bypassed: avcore::voice_cleanup_preview::VoiceCleanupPreviewSample {
            path: PathBuf::from("bypassed.m4a"),
            metrics: metrics(integrated_lufs_bypassed),
        },
        processed: avcore::voice_cleanup_preview::VoiceCleanupPreviewSample {
            path: PathBuf::from("processed.m4a"),
            metrics: metrics(integrated_lufs_processed),
        },
    }
}

#[test]
fn spawn_voice_cleanup_preview_marks_the_selected_clip_as_rendering() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Audio,
        vec![test_clip(1, 0.0, 0.0, 5.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());
    app.selected_clip_id = Some(1);

    app.spawn_voice_cleanup_preview();

    assert_eq!(app.voice_cleanup_preview_state.rendering_clip_id, Some(1));
}

#[test]
fn spawn_voice_cleanup_preview_is_a_no_op_while_a_render_is_already_in_flight() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Audio,
        vec![test_clip(1, 0.0, 0.0, 5.0), test_clip(2, 5.0, 0.0, 5.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());
    app.voice_cleanup_preview_state.rendering_clip_id = Some(1);
    app.selected_clip_id = Some(2);

    app.spawn_voice_cleanup_preview();

    // Still tagged as rendering clip 1 -- a second clip's render was never dispatched while
    // one was already in flight.
    assert_eq!(app.voice_cleanup_preview_state.rendering_clip_id, Some(1));
}

#[test]
fn pump_voice_cleanup_preview_stores_a_successful_result_and_clears_rendering() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.voice_cleanup_preview_state.rendering_clip_id = Some(1);
    let result = test_voice_cleanup_preview_result(-16.0, -14.0);
    app.voice_cleanup_preview_state
        .tx
        .send(VoiceCleanupPreviewEvent::Done {
            clip_id: 1,
            result: Ok(result),
        })
        .unwrap();

    app.pump_voice_cleanup_preview();

    assert_eq!(app.voice_cleanup_preview_state.rendering_clip_id, None);
    let (clip_id, result) = app.voice_cleanup_preview_state.result.as_ref().unwrap();
    assert_eq!(*clip_id, 1);
    assert_eq!(result.bypassed.metrics.integrated_lufs, -16.0);
    assert_eq!(result.processed.metrics.integrated_lufs, -14.0);
}

#[test]
fn pump_voice_cleanup_preview_toasts_on_failure_without_storing_a_result() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.voice_cleanup_preview_state.rendering_clip_id = Some(1);
    app.voice_cleanup_preview_state
        .tx
        .send(VoiceCleanupPreviewEvent::Done {
            clip_id: 1,
            result: Err("source file not found".to_string()),
        })
        .unwrap();

    app.pump_voice_cleanup_preview();

    assert_eq!(app.voice_cleanup_preview_state.rendering_clip_id, None);
    assert!(app.voice_cleanup_preview_state.result.is_none());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn pump_update_check_distinguishes_up_to_date_from_failure() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.update_check_tx
        .send(UpdateCheckEvent::UpToDate)
        .unwrap();

    app.pump_update_check();

    assert_eq!(app.update_check_status, UpdateCheckStatus::UpToDate);

    app.update_check_tx.send(UpdateCheckEvent::Failed).unwrap();
    app.pump_update_check();

    assert_eq!(app.update_check_status, UpdateCheckStatus::Failed);
}

#[test]
fn pump_update_check_marks_an_installed_update_ready_to_restart() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let update = AvailableUpdate {
        version: "99.0.0".to_string(),
        html_url: "https://github.com/lucasgmagalhaes/oca/releases/tag/v99.0.0".to_string(),
        auto_update_available: true,
    };
    app.update_check_tx
        .send(UpdateCheckEvent::Installed {
            update: update.clone(),
        })
        .unwrap();

    app.pump_update_check();

    assert_eq!(
        app.update_check_status,
        UpdateCheckStatus::RestartRequired(update)
    );
}

#[test]
fn pump_update_check_preserves_release_details_after_install_failure() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let update = AvailableUpdate {
        version: "99.0.0".to_string(),
        html_url: "https://github.com/lucasgmagalhaes/oca/releases/tag/v99.0.0".to_string(),
        auto_update_available: true,
    };
    app.update_check_tx
        .send(UpdateCheckEvent::InstallFailed {
            update: update.clone(),
        })
        .unwrap();

    app.pump_update_check();

    assert_eq!(
        app.update_check_status,
        UpdateCheckStatus::InstallFailed(update)
    );
}

#[test]
fn open_about_closes_preferences_and_close_about_clears_the_modal() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs_open = true;

    app.open_about();

    assert!(!app.prefs_open);
    assert!(app.about_open);

    app.close_about();

    assert!(!app.about_open);
}

#[test]
fn prefs_snapshot_captures_live_locale_and_panel_layout() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.locale = Locale::En;
    app.lib_panel_width = 321.0;
    app.props_panel_width = 456.0;
    app.timeline_height = 111.0;
    // Deliberately different from the live values above, so the snapshot can only match by
    // actually reading the live App fields, not by coincidentally already matching prefs.
    app.prefs.locale = Locale::PtBr;
    app.prefs.lib_panel_width = 1.0;
    app.prefs.props_panel_width = 2.0;
    app.prefs.timeline_height = 3.0;

    let snapshot = app.prefs_snapshot();

    assert_eq!(snapshot.locale, Locale::En);
    assert_eq!(snapshot.lib_panel_width, 321.0);
    assert_eq!(snapshot.props_panel_width, 456.0);
    assert_eq!(snapshot.timeline_height, 111.0);
}

#[test]
fn prefs_snapshot_prunes_recent_paths_that_no_longer_exist() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.recent_project_paths = vec![
        "/definitely/does/not/exist/project.ocproj".to_string(),
        env!("CARGO_MANIFEST_DIR").to_string(), // this directory does exist.
    ];

    let snapshot = app.prefs_snapshot();

    assert_eq!(
        snapshot.recent_project_paths,
        vec![env!("CARGO_MANIFEST_DIR").to_string()]
    );
}

#[test]
fn sync_panel_layout_is_a_no_op_under_per_user_scope() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.layout_scope = LayoutScope::PerUser;
    app.lib_panel_width = 999.0;

    app.sync_panel_layout_into_active_project();

    assert_eq!(app.active_project().panel_layout, None);
}

#[test]
fn sync_panel_layout_captures_live_values_under_per_project_scope() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.layout_scope = LayoutScope::PerProject;
    app.lib_panel_width = 321.0;
    app.props_panel_width = 456.0;
    app.timeline_height = 111.0;

    app.sync_panel_layout_into_active_project();

    assert_eq!(
        app.active_project().panel_layout,
        Some(avcore::PanelLayout {
            lib_panel_width: 321.0,
            props_panel_width: 456.0,
            timeline_height: 111.0,
        })
    );
}

#[test]
fn load_panel_layout_is_a_no_op_under_per_user_scope() {
    let mut project = test_project(1, Vec::new());
    project.panel_layout = Some(avcore::PanelLayout {
        lib_panel_width: 10.0,
        props_panel_width: 20.0,
        timeline_height: 30.0,
    });
    let mut app = test_app(vec![project], Vec::new());
    app.prefs.layout_scope = LayoutScope::PerUser;
    app.lib_panel_width = 999.0;

    app.load_panel_layout_for_active_project();

    assert_eq!(app.lib_panel_width, 999.0);
}

#[test]
fn load_panel_layout_applies_the_active_projects_saved_layout() {
    let mut project = test_project(1, Vec::new());
    project.panel_layout = Some(avcore::PanelLayout {
        lib_panel_width: 10.0,
        props_panel_width: 20.0,
        timeline_height: 30.0,
    });
    let mut app = test_app(vec![project], Vec::new());
    app.prefs.layout_scope = LayoutScope::PerProject;
    app.lib_panel_width = 999.0;

    app.load_panel_layout_for_active_project();

    assert_eq!(app.lib_panel_width, 10.0);
    assert_eq!(app.props_panel_width, 20.0);
    assert_eq!(app.timeline_height, 30.0);
}

#[test]
fn load_panel_layout_keeps_live_values_when_project_has_no_saved_layout() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.layout_scope = LayoutScope::PerProject;
    app.lib_panel_width = 999.0;

    app.load_panel_layout_for_active_project();

    assert_eq!(app.lib_panel_width, 999.0);
}
