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

//! CF-05's deferred slice, now unblocked: real OpenTimelineIO `.otio` JSON serialization.
//! `interchange.rs`'s own doc comment explained why this didn't happen in slice 1 — no network
//! path in that sandbox to fetch or verify OTIO's actual wire format, and writing one from
//! memory alone would be exactly the kind of guessing `CLAUDE.md`'s "do not guess APIs,
//! versions, flags, or package names" rule forbids. This session has real network access, so the
//! schema below is verified against real files from the OpenTimelineIO project's own
//! `tests/sample_data/` directory (fetched via `curl`, not summarized through a lossy
//! intermediary) — `simple_cut.otio`, `screening_example.otio`, `transition.otio`,
//! `multitrack.otio`, `nested_example.otio` — not reconstructed from documentation prose.
//!
//! **Schema versions used, and why** — verified not just against sample files but against the
//! real `opentimelineio` 0.18.1 Python reference implementation itself (`pip install
//! opentimelineio`, this session had real network access): `Timeline.1`/`Stack.1`/`Track.1`/
//! `Gap.1`/`Transition.1`/`TimeRange.1`/`RationalTime.1`/`ExternalReference.1`/
//! `MissingReference.1` are unambiguous. `Clip` has two live versions in the real ecosystem:
//! `Clip.1` (a single `media_reference` field) and `Clip.2` (a `media_references` map +
//! `active_media_reference_key`, for multi-reference workflows like stereo or proxy/full-res
//! pairs — confirmed as what 0.18.1 itself constructs by default via `otio.schema.Clip()`).
//! This module writes `Clip.1` deliberately, not because it's current but because
//! [`super::MediaReference`] only ever models one reference per clip, the same shape `Clip.1`
//! has — writing `Clip.2` would mean inventing a fake single-entry map to satisfy a field oca
//! has no real multi-reference data for. Confirmed for real, not assumed: `opentimelineio`
//! 0.18.1's own `read_from_file` successfully loads `Clip.1` (tested directly against
//! `simple_cut.otio`, a `Clip.1` file from the project's own `tests/sample_data/`) — backward
//! compatibility, not a guess. `Marker` genuinely needed a real-verification catch: an initial
//! attempt used `Marker.3` with a nested `Color.1` object, matching a sample fetched from the
//! OpenTimelineIO GitHub repo's `main` branch — but 0.18.1's own `otio.schema.Marker()` emits
//! `Marker.2` with a *plain string* `color` field and a `comment` field, and rejected the
//! `Marker.3` version outright (`UnsupportedSchemaError`) when the generated file was loaded
//! back through the real library. `main`'s sample data was ahead of the actual released/
//! installable library version — exactly the kind of gap only a real load-it-back-and-see check
//! (not just eyeballing a sample file) catches. Fixed to `Marker.2`, then re-verified: the real
//! library loads every object this module emits without error.
//!
//! **Only the write direction is implemented in this pass.** Reading an arbitrary real-world
//! `.otio` file (from Resolve, Premiere, or the reference Python implementation) needs to
//! tolerate a much wider variety of shapes than this module's own output — unknown schema
//! versions, `Clip.2`'s multi-reference form, nested `Stack`s (compound clips), audio-only
//! tracks with no video, vendor-specific metadata — a genuinely larger and riskier scope than
//! verifying export against known-good samples. Real `.otio` import is left as CF-05's next
//! slice, same "one honest vertical slice at a time" discipline this whole roadmap item has
//! followed so far.

use serde_json::{json, Value};

use super::{
    InterchangeClip, InterchangeMarker, InterchangeTimeline, InterchangeTrack,
    InterchangeTrackItem, InterchangeTrackKind, MediaReference, RationalTime, TimeRange,
};
use crate::timeline::MarkerKind;

fn rational_time_json(t: RationalTime) -> Value {
    json!({
        "OTIO_SCHEMA": "RationalTime.1",
        "rate": t.rate,
        "value": t.value,
    })
}

fn time_range_json(r: TimeRange) -> Value {
    json!({
        "OTIO_SCHEMA": "TimeRange.1",
        "start_time": rational_time_json(r.start_time),
        "duration": rational_time_json(r.duration),
    })
}

fn media_reference_json(reference: &MediaReference) -> Value {
    match reference {
        MediaReference::External { target_url } => json!({
            "OTIO_SCHEMA": "ExternalReference.1",
            "name": "",
            "metadata": {},
            "available_range": null,
            "available_image_bounds": null,
            "target_url": target_url,
        }),
        MediaReference::Missing => json!({
            "OTIO_SCHEMA": "MissingReference.1",
            "name": "",
            "metadata": {},
            "available_range": null,
            "available_image_bounds": null,
        }),
    }
}

/// OTIO's `Marker.2` convention doesn't mandate any particular color-per-marker-kind meaning —
/// this mapping is oca's own judgment call for readability when a marker is opened in another
/// NLE, not a spec requirement. Named colors match the standard string constants
/// `opentimelineio.schema.MarkerColor` defines (`RED`/`GREEN`/`BLUE`/`YELLOW`).
fn marker_color_name(kind: MarkerKind) -> &'static str {
    match kind {
        MarkerKind::Standard => "GREEN",
        MarkerKind::ToDo => "RED",
        MarkerKind::Chapter => "BLUE",
        MarkerKind::Highlight => "YELLOW",
    }
}

fn marker_json(marker: &InterchangeMarker) -> Value {
    json!({
        "OTIO_SCHEMA": "Marker.2",
        "name": marker.name,
        "metadata": {},
        "color": marker_color_name(marker.kind),
        "marked_range": time_range_json(marker.marked_range),
        "comment": "",
    })
}

fn clip_json(clip: &InterchangeClip) -> Value {
    json!({
        "OTIO_SCHEMA": "Clip.1",
        "name": format!("Clip-{}", clip.source_id),
        "metadata": {
            // `speed_factor`/`transition_in` have no OTIO-native field at this schema version
            // (see this module's doc comment on what's carried vs. approximated) -- kept in
            // metadata rather than silently dropped, so a round-trip back into oca (once import
            // lands) or a careful human reader in another tool can still recover them.
            "oca": {
                "speed_factor": clip.speed_factor,
                "transition_in": clip.transition_in.map(|t| format!("{t:?}")),
            }
        },
        "effects": [],
        "markers": [],
        "enabled": true,
        "source_range": time_range_json(clip.source_range),
        "media_reference": media_reference_json(&clip.media_reference),
    })
}

fn gap_json(range: TimeRange) -> Value {
    json!({
        "OTIO_SCHEMA": "Gap.1",
        "name": "Gap",
        "metadata": {},
        "effects": [],
        "markers": [],
        "enabled": true,
        "source_range": time_range_json(range),
    })
}

fn track_item_json(item: &InterchangeTrackItem) -> Value {
    match item {
        InterchangeTrackItem::Clip(clip) => clip_json(clip),
        InterchangeTrackItem::Gap(range) => gap_json(*range),
    }
}

fn track_json(track: &InterchangeTrack) -> Value {
    let kind = match track.kind {
        InterchangeTrackKind::Video => "Video",
        InterchangeTrackKind::Audio => "Audio",
    };
    json!({
        "OTIO_SCHEMA": "Track.1",
        "name": track.name,
        "metadata": {},
        "source_range": null,
        "effects": [],
        "markers": [],
        "enabled": true,
        "kind": kind,
        "children": track.items.iter().map(track_item_json).collect::<Vec<_>>(),
    })
}

/// Serializes `timeline` as a real, schema-verified OpenTimelineIO `.otio` JSON document — see
/// this module's own doc comment for exactly which schema versions and why. The top-level
/// `Timeline` carries `timeline.markers` on its `tracks` `Stack` (OTIO has no separate
/// timeline-level marker list distinct from the stack's own) rather than dropping them.
pub fn serialize_interchange_timeline(timeline: &InterchangeTimeline) -> Value {
    json!({
        "OTIO_SCHEMA": "Timeline.1",
        "name": timeline.name,
        "metadata": {},
        "global_start_time": null,
        "tracks": {
            "OTIO_SCHEMA": "Stack.1",
            "name": "tracks",
            "metadata": {},
            "source_range": null,
            "effects": [],
            "enabled": true,
            "markers": timeline.markers.iter().map(marker_json).collect::<Vec<_>>(),
            "children": timeline.tracks.iter().map(track_json).collect::<Vec<_>>(),
        },
    })
}

/// [`serialize_interchange_timeline`], pretty-printed — what a caller writing a `.otio` file to
/// disk actually wants (OTIO's own Python library writes indented JSON too, and a hand-readable
/// file is one of interchange's own practical benefits over a binary format).
pub fn serialize_interchange_timeline_pretty(timeline: &InterchangeTimeline) -> String {
    serde_json::to_string_pretty(&serialize_interchange_timeline(timeline))
        .expect("serde_json::Value serialization is infallible")
}

#[cfg(test)]
#[path = "otio_json_test.rs"]
mod tests;
