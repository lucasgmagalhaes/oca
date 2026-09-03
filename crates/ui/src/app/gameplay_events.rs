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

//! CF-02's `ui` wiring, the gap its own `spec/ROADMAP.md` entry flagged as still open —
//! `avcore::gameplay_events` (sidecar schema/validation/idempotent marker import) was `core`-only
//! until now. This module is the one missing piece: a file picker plus the source-media-filename
//! matching and source-to-timeline mapping the sidecar module deliberately leaves to its caller
//! (it "has no opinion on timeline placement" — see its own doc comment).

use avcore::gameplay_events::{EventSidecar, GameplayEvent};

use crate::i18n::Text;

use super::App;

/// Just enough of a matching [`avcore::ClipInstance`] to compute its own
/// source-timestamp-to-timeline-position mapping — captured by value so the read pass over
/// `project.timeline()`/`project.media_library` can end before [`App::active_project_mut`]
/// needs to borrow the project again.
struct MatchingClip {
    start_secs: f64,
    source_in_secs: f64,
    speed_factor: f32,
}

impl App {
    /// Reads and validates the event sidecar at `path` (see [`avcore::gameplay_events`]), then
    /// imports its events as `MarkerKind::Highlight` markers onto every clip in the active
    /// sequence's timeline whose source asset's [`avcore::MediaAsset::file_name`] matches the
    /// sidecar's `source_media_filename` — what the Editor toolbar's "Import Gameplay Events..."
    /// button does once the file picker returns a path. Only events whose
    /// `source_timestamp_secs` actually falls within a given clip's trimmed
    /// `[source_in_secs, source_out_secs]` range are imported onto that clip, so a sidecar
    /// covering a whole recording that's been cut into several clips only places each event
    /// where it's actually visible. Idempotent per [`avcore::gameplay_events::
    /// import_events_as_markers`]'s own dedup rule, so re-importing the same sidecar is a no-op
    /// the second time. Toasts on read/parse/validation failure, when no clip in the timeline
    /// covers any of this sidecar's events (either no clip uses the recording at all, or every
    /// event falls outside the trimmed range of the clips that do), and when nothing new was
    /// imported.
    pub fn import_gameplay_events(&mut self, path: std::path::PathBuf) {
        let locale = self.locale;
        let json = match std::fs::read_to_string(&path) {
            Ok(json) => json,
            Err(e) => {
                self.push_toast(format!(
                    "{}: {e}",
                    Text::GameplayEventsImportReadFailed.tr(locale)
                ));
                return;
            }
        };
        let sidecar = match EventSidecar::parse_and_validate(&json) {
            Ok(sidecar) => sidecar,
            Err(e) => {
                self.push_toast(format!(
                    "{}: {e}",
                    Text::GameplayEventsImportInvalid.tr(locale)
                ));
                return;
            }
        };

        // CF-02 slice 5: a configured per-game profile restricts which event kinds import and
        // supplies pre/post-roll defaults for events that don't specify their own — a no-op when
        // no profile's game_id matches this sidecar's (including when the sidecar has none).
        let allowlist = self
            .prefs
            .game_event_allowlists
            .iter()
            .find(|allowlist| Some(allowlist.game_id.as_str()) == sidecar.game_id.as_deref());
        let events =
            avcore::gameplay_events::apply_game_event_allowlist(&sidecar.events, allowlist);

        // (clip mapping, only the events that actually fall within that clip's trimmed range) —
        // computed up front so an all-empty result never pushes a no-op undo snapshot.
        let per_clip_events: Vec<(MatchingClip, Vec<GameplayEvent>)> = {
            let project = self.active_project();
            project
                .timeline()
                .tracks
                .iter()
                .flat_map(|track| &track.clips)
                .filter(|clip| clip.speed_factor > 0.0)
                .filter_map(|clip| {
                    let matches_source = project.media_library.iter().any(|asset| {
                        asset.id == clip.asset_id
                            && asset.file_name == sidecar.source_media_filename
                    });
                    if !matches_source {
                        return None;
                    }
                    let events: Vec<GameplayEvent> = events
                        .iter()
                        .filter(|e| {
                            e.source_timestamp_secs >= clip.source_in_secs
                                && e.source_timestamp_secs <= clip.source_out_secs
                        })
                        .cloned()
                        .collect();
                    if events.is_empty() {
                        return None;
                    }
                    Some((
                        MatchingClip {
                            start_secs: clip.start_secs,
                            source_in_secs: clip.source_in_secs,
                            speed_factor: clip.speed_factor,
                        },
                        events,
                    ))
                })
                .collect()
        };

        if per_clip_events.is_empty() {
            self.push_toast(
                Text::GameplayEventsImportNoMatchingClip
                    .tr(locale)
                    .to_string(),
            );
            return;
        }

        self.push_undo_snapshot();
        let mut total_added = 0usize;
        for (clip, events) in &per_clip_events {
            let start_secs = clip.start_secs;
            let source_in_secs = clip.source_in_secs;
            let speed_factor = clip.speed_factor as f64;
            let timeline = self.active_project_mut().timeline_mut();
            total_added += avcore::gameplay_events::import_events_as_markers(
                timeline,
                events,
                |source_secs| start_secs + (source_secs - source_in_secs) / speed_factor,
            );
        }

        if total_added == 0 {
            self.push_toast(Text::GameplayEventsImportNothingNew.tr(locale).to_string());
        } else {
            self.push_toast(
                Text::GameplayEventsImportSuccess
                    .tr(locale)
                    .replace("{n}", &total_added.to_string()),
            );
        }
    }
}
