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

//! YouTube download state-machine tests.

use super::support::*;

#[test]
fn open_youtube_modal_starts_with_an_empty_url() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.open_youtube_modal();

    assert_eq!(
        app.youtube_download_state.youtube_modal_url,
        Some(String::new())
    );
}

#[test]
fn open_youtube_modal_is_a_no_op_while_already_open() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.youtube_download_state.youtube_modal_url = Some("https://example.com/x".to_string());

    app.open_youtube_modal();

    assert_eq!(
        app.youtube_download_state.youtube_modal_url,
        Some("https://example.com/x".to_string())
    );
}

#[test]
fn open_youtube_modal_is_a_no_op_while_downloading() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.youtube_download_state.youtube_downloading = true;

    app.open_youtube_modal();

    assert_eq!(app.youtube_download_state.youtube_modal_url, None);
}

#[test]
fn spawn_youtube_download_is_a_no_op_with_a_blank_url() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.youtube_download_state.youtube_modal_url = Some("   ".to_string());

    app.spawn_youtube_download();

    assert!(!app.youtube_download_state.youtube_downloading);
}

#[test]
fn spawn_youtube_download_is_a_no_op_while_already_downloading() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.youtube_download_state.youtube_modal_url = Some("https://example.com/x".to_string());
    app.youtube_download_state.youtube_downloading = true;
    app.youtube_download_state.youtube_download_progress = 0.4;

    app.spawn_youtube_download();

    // Progress isn't reset by this second, ignored call.
    assert_eq!(app.youtube_download_state.youtube_download_progress, 0.4);
}

#[test]
fn close_youtube_modal_is_a_no_op_while_downloading() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.youtube_download_state.youtube_modal_url = Some("https://example.com/x".to_string());
    app.youtube_download_state.youtube_downloading = true;

    app.close_youtube_modal();

    assert!(app.youtube_download_state.youtube_modal_url.is_some());
}

#[test]
fn close_youtube_modal_clears_the_url_when_idle() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.youtube_download_state.youtube_modal_url = Some("https://example.com/x".to_string());

    app.close_youtube_modal();

    assert_eq!(app.youtube_download_state.youtube_modal_url, None);
}

#[test]
fn request_cancel_youtube_download_is_a_no_op_when_nothing_is_downloading() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    // Just needs to not panic without a live download.
    app.request_cancel_youtube_download();
}
