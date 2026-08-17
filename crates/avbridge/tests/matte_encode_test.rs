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

use avbridge::{encode_matte_video, probe, MatteError, StreamKind};

#[test]
fn encodes_a_real_probeable_matte_video() {
    let out = std::env::temp_dir().join("avbridge_test_matte_ok.mp4");
    // 8 frames of a 64x64 luma gradient — content doesn't matter here, just that it's a real
    // per-frame buffer of the declared size.
    let frames: Vec<Vec<u8>> = (0..8).map(|i| vec![(i * 30) as u8; 64 * 64]).collect();

    encode_matte_video(&frames, 64, 64, 16, 1, &out).unwrap();

    let info = probe(&out).unwrap();
    assert_eq!(info.kind, StreamKind::Video);
    assert_eq!(info.resolution, Some((64, 64)));
    // 8 frames at 16fps = 0.5s.
    assert!((info.duration_secs - 0.5).abs() < 0.1);

    let _ = std::fs::remove_file(&out);
}

#[test]
fn fails_with_empty_on_no_frames() {
    let out = std::env::temp_dir().join("avbridge_test_matte_empty.mp4");

    let err = encode_matte_video(&[], 64, 64, 16, 1, &out).unwrap_err();

    assert!(matches!(err, MatteError::Empty));
    assert!(!out.exists());
}

#[test]
fn fails_with_odd_dimensions_on_an_odd_width() {
    let out = std::env::temp_dir().join("avbridge_test_matte_odd.mp4");
    let frames: Vec<Vec<u8>> = vec![vec![0u8; 63 * 64]];

    let err = encode_matte_video(&frames, 63, 64, 16, 1, &out).unwrap_err();

    assert!(matches!(err, MatteError::OddDimensions));
    assert!(!out.exists());
}
