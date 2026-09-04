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

use super::*;
use crate::interchange::{
    InterchangeClip, InterchangeMarker, InterchangeTimeline, InterchangeTrack,
    InterchangeTrackItem, InterchangeTrackKind, InterchangeTransitionKind, MediaReference,
    TimeRange,
};
use crate::timeline::MarkerKind;

fn sample_timeline() -> InterchangeTimeline {
    InterchangeTimeline {
        name: "Sequence 1".to_string(),
        tracks: vec![InterchangeTrack {
            name: "V1".to_string(),
            kind: InterchangeTrackKind::Video,
            items: vec![
                InterchangeTrackItem::Gap(TimeRange::from_seconds(0.0, 1.0, 1_000_000.0)),
                InterchangeTrackItem::Clip(InterchangeClip {
                    source_id: 42,
                    source_range: TimeRange::from_seconds(2.0, 5.0, 1_000_000.0),
                    media_reference: MediaReference::External {
                        target_url: "file:///tmp/clip.mp4".to_string(),
                    },
                    speed_factor: 1.5,
                    transition_in: Some(InterchangeTransitionKind::Fade),
                }),
                InterchangeTrackItem::Clip(InterchangeClip {
                    source_id: 43,
                    source_range: TimeRange::from_seconds(0.0, 3.0, 1_000_000.0),
                    media_reference: MediaReference::Missing,
                    speed_factor: 1.0,
                    transition_in: None,
                }),
            ],
        }],
        markers: vec![InterchangeMarker {
            name: "Highlight".to_string(),
            marked_range: TimeRange::from_seconds(1.5, 0.0, 1_000_000.0),
            kind: MarkerKind::Highlight,
        }],
    }
}

fn schema_of(value: &Value) -> &str {
    value["OTIO_SCHEMA"].as_str().expect("OTIO_SCHEMA present")
}

#[test]
fn top_level_schema_tags_match_the_real_otio_wire_format() {
    let doc = serialize_interchange_timeline(&sample_timeline());
    assert_eq!(schema_of(&doc), "Timeline.1");
    assert_eq!(doc["name"], "Sequence 1");
    assert_eq!(schema_of(&doc["tracks"]), "Stack.1");
    assert_eq!(schema_of(&doc["tracks"]["children"][0]), "Track.1");
    assert_eq!(doc["tracks"]["children"][0]["kind"], "Video");
}

#[test]
fn a_gap_serializes_as_gap_1_with_a_source_range() {
    let doc = serialize_interchange_timeline(&sample_timeline());
    let gap = &doc["tracks"]["children"][0]["children"][0];
    assert_eq!(schema_of(gap), "Gap.1");
    assert_eq!(schema_of(&gap["source_range"]), "TimeRange.1");
    assert_eq!(
        schema_of(&gap["source_range"]["start_time"]),
        "RationalTime.1"
    );
    assert_eq!(gap["source_range"]["start_time"]["value"], 0.0);
    assert_eq!(gap["source_range"]["duration"]["value"], 1_000_000.0);
}

#[test]
fn a_clip_with_an_external_reference_serializes_as_clip_1_with_external_reference_1() {
    let doc = serialize_interchange_timeline(&sample_timeline());
    let clip = &doc["tracks"]["children"][0]["children"][1];
    assert_eq!(schema_of(clip), "Clip.1");
    assert_eq!(schema_of(&clip["media_reference"]), "ExternalReference.1");
    assert_eq!(
        clip["media_reference"]["target_url"],
        "file:///tmp/clip.mp4"
    );
    // Source range: 5 real seconds at the 1_000_000 interchange rate.
    assert_eq!(clip["source_range"]["duration"]["value"], 5_000_000.0);
    assert_eq!(clip["metadata"]["oca"]["speed_factor"], 1.5);
}

#[test]
fn a_clip_with_a_missing_reference_serializes_as_missing_reference_1() {
    let doc = serialize_interchange_timeline(&sample_timeline());
    let clip = &doc["tracks"]["children"][0]["children"][2];
    assert_eq!(schema_of(clip), "Clip.1");
    assert_eq!(schema_of(&clip["media_reference"]), "MissingReference.1");
}

#[test]
fn a_marker_serializes_as_marker_2_with_a_plain_string_color() {
    let doc = serialize_interchange_timeline(&sample_timeline());
    let marker = &doc["tracks"]["markers"][0];
    assert_eq!(schema_of(marker), "Marker.2");
    assert_eq!(marker["name"], "Highlight");
    assert_eq!(marker["color"], "YELLOW");
    assert_eq!(schema_of(&marker["marked_range"]), "TimeRange.1");
}

#[test]
fn an_empty_timeline_still_produces_a_well_formed_document() {
    let empty = InterchangeTimeline {
        name: "Empty".to_string(),
        tracks: vec![],
        markers: vec![],
    };
    let doc = serialize_interchange_timeline(&empty);
    assert_eq!(schema_of(&doc), "Timeline.1");
    assert_eq!(doc["tracks"]["children"].as_array().unwrap().len(), 0);
    assert_eq!(doc["tracks"]["markers"].as_array().unwrap().len(), 0);
}

#[test]
fn the_document_round_trips_through_a_real_json_parser() {
    // Not a claim of round-tripping *back* into an InterchangeTimeline (import is a separate,
    // later slice) -- just confirms the emitted document is valid, parseable JSON, the way any
    // real consumer (the OTIO Python library, another NLE) would first check it.
    let pretty = serialize_interchange_timeline_pretty(&sample_timeline());
    let reparsed: Value = serde_json::from_str(&pretty).expect("valid JSON");
    assert_eq!(schema_of(&reparsed), "Timeline.1");
}

#[test]
fn audio_tracks_carry_the_audio_kind() {
    let timeline = InterchangeTimeline {
        name: "Audio only".to_string(),
        tracks: vec![InterchangeTrack {
            name: "A1".to_string(),
            kind: InterchangeTrackKind::Audio,
            items: vec![],
        }],
        markers: vec![],
    };
    let doc = serialize_interchange_timeline(&timeline);
    assert_eq!(doc["tracks"]["children"][0]["kind"], "Audio");
}
