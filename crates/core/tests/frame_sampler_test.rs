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

use std::path::{Path, PathBuf};
use std::time::Duration;

use avcore::FrameSampler;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn sample_decodes_a_frame_at_the_requested_time() {
    let sampler = FrameSampler::open(&fixture("video.mp4"), Duration::from_millis(20)).unwrap();
    let frame = sampler
        .sample(0.0, Duration::from_millis(2000))
        .expect("a real fixture should decode within the deadline");
    assert!(frame.width > 0 && frame.height > 0);
    assert_eq!(frame.rgba.len(), (frame.width * frame.height * 4) as usize);
}

#[test]
fn sample_can_be_called_repeatedly_against_the_same_open_pipeline() {
    let sampler = FrameSampler::open(&fixture("video.mp4"), Duration::from_millis(20)).unwrap();
    let first = sampler
        .sample(0.0, Duration::from_millis(2000))
        .expect("first sample should decode");
    let second = sampler
        .sample(0.2, Duration::from_millis(2000))
        .expect("second sample, same sampler, should also decode");
    assert_eq!(first.width, second.width);
    assert_eq!(first.height, second.height);
}

#[test]
fn open_fails_for_a_nonexistent_source() {
    assert!(FrameSampler::open(&fixture("does_not_exist.mp4"), Duration::from_millis(20)).is_err());
}
