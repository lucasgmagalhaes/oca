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

use avcore::project::{Project, Sequence};
use avcore::timeline::Timeline;
use avcore::{MediaAsset, Recency};

fn test_project() -> Project {
    Project {
        id: 1,
        name: "Test".to_string(),
        last_edited: Recency::HoursAgo(0),
        summary: String::new(),
        media_library: Vec::<MediaAsset>::new(),
        sequences: vec![Sequence {
            id: 1,
            name: "Main".to_string(),
            timeline: Timeline {
                tracks: Vec::new(),
                playhead_secs: 5.0,
            },
        }],
        active_sequence: 0,
        file_path: None,
    }
}

#[test]
fn timeline_reads_the_active_sequence() {
    let project = test_project();
    assert_eq!(project.timeline().playhead_secs, 5.0);
}

#[test]
fn timeline_mut_writes_the_active_sequence() {
    let mut project = test_project();
    project.timeline_mut().playhead_secs = 42.0;
    assert_eq!(project.sequences[0].timeline.playhead_secs, 42.0);
}

#[test]
fn new_sequence_appends_and_switches_to_it() {
    let mut project = test_project();
    let id = project.new_sequence("Second".to_string());

    assert_eq!(project.sequences.len(), 2);
    assert_eq!(id, 2);
    assert_eq!(project.sequences[1].id, 2);
    assert_eq!(project.sequences[1].name, "Second");
    assert_eq!(project.active_sequence, 1);
    assert!(project.timeline().tracks.is_empty());
}

#[test]
fn new_sequence_ids_keep_increasing_after_multiple_calls() {
    let mut project = test_project();
    project.new_sequence("A".to_string());
    let third_id = project.new_sequence("B".to_string());

    assert_eq!(third_id, 3);
}
