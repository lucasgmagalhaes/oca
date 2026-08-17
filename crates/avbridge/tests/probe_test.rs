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

use std::path::Path;

use avbridge::{probe, ProbeError, StreamKind};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn probes_a_video_file() {
    let info = probe(&fixture("video.mp4")).unwrap();
    assert_eq!(info.kind, StreamKind::Video);
    assert_eq!(info.codec_name, "mpeg4");
    assert_eq!(info.resolution, Some((320, 240)));
    assert!((info.fps.unwrap() - 30.0).abs() < 0.01);
    assert!((info.duration_secs - 1.0).abs() < 0.1);
    assert!(info.bit_rate.unwrap() > 0);
    assert_eq!(info.sample_rate_hz, None);
}

#[test]
fn falls_back_to_audio_when_there_is_no_video_stream() {
    let info = probe(&fixture("audio.m4a")).unwrap();
    assert_eq!(info.kind, StreamKind::Audio);
    assert_eq!(info.codec_name, "aac");
    assert_eq!(info.resolution, None);
    assert_eq!(info.fps, None);
    assert_eq!(info.sample_rate_hz, Some(44100));
}

#[test]
fn missing_file_returns_open_error() {
    let err = probe(&fixture("does_not_exist.mp4")).unwrap_err();
    assert!(matches!(err, ProbeError::Open));
}

#[test]
fn non_media_file_returns_open_error() {
    let err = probe(&fixture("garbage.bin")).unwrap_err();
    assert!(matches!(err, ProbeError::Open));
}
