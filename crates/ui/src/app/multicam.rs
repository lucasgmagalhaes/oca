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

//! P2 item 10, "Multicam editing" (`spec/ROADMAP.md`): syncs every `Video` track in the active
//! sequence into one [`avcore::timeline::MulticamGroup`] (by cross-correlating each track's
//! first clip's audio against a reference track, via [`avcore::compute_sync_offset_secs`]), then
//! lets number-key presses at the playhead retarget the program track to a different angle at
//! that exact moment ([`avcore::timeline::Timeline::switch_multicam_angle`] does the actual
//! split+retarget). See the roadmap item's own scoping note for what's deliberately out: no
//! live multi-feed preview during scrub/playback (still plays one decoded source, same as every
//! other effect without live preview), no timecode-based sync (waveform only).

use std::collections::HashMap;

use super::{timeline_ops::next_clip_id, App};
use crate::i18n::Text;

impl App {
    /// Syncs every `Video` track in the active sequence's timeline into one new
    /// [`avcore::timeline::MulticamGroup`], first track becomes the program (visible/exported)
    /// angle, sets [`App::active_multicam_group_id`] to it, and toasts a summary. Each other
    /// track's sync offset is computed by cross-correlating its first clip's underlying asset
    /// audio (via [`avcore::extract_pcm_16k_mono`]) against the program track's — best-effort:
    /// a track whose PCM extraction fails (no audio stream, decode error) still joins the group,
    /// just with a `0.0` offset (equivalent to "assume already aligned") rather than aborting
    /// the whole sync over one bad source. Toasts and does nothing if fewer than two `Video`
    /// tracks exist, or if any track has no clip at all to sync from.
    pub fn create_multicam_group_from_video_tracks(&mut self) {
        enum Failure {
            NeedsTwoTracks,
            NeedsAudio,
        }

        // Scoped so the immutable `project` borrow ends before any `push_toast` (`&mut self`)
        // call below -- can't hold both at once.
        let gathered: Result<(Vec<u64>, HashMap<u64, std::path::PathBuf>), Failure> = {
            let project = self.active_project();
            let video_track_ids: Vec<u64> = project
                .timeline()
                .tracks
                .iter()
                .filter(|t| t.kind == avcore::timeline::TrackKind::Video)
                .map(|t| t.id)
                .collect();
            if video_track_ids.len() < 2 {
                Err(Failure::NeedsTwoTracks)
            } else {
                let mut source_paths = HashMap::new();
                let mut every_track_has_a_clip = true;
                for &track_id in &video_track_ids {
                    let Some(track) = project.timeline().tracks.iter().find(|t| t.id == track_id)
                    else {
                        continue;
                    };
                    let Some(first_clip) = track.clips.first() else {
                        every_track_has_a_clip = false;
                        break;
                    };
                    if let Some(asset) = project
                        .media_library
                        .iter()
                        .find(|a| a.id == first_clip.asset_id)
                    {
                        source_paths.insert(track_id, asset.source_path.clone());
                    }
                }
                if every_track_has_a_clip {
                    Ok((video_track_ids, source_paths))
                } else {
                    Err(Failure::NeedsAudio)
                }
            }
        };

        let (video_track_ids, source_paths) = match gathered {
            Ok(gathered) => gathered,
            Err(Failure::NeedsTwoTracks) => {
                self.push_toast(
                    Text::MulticamGroupNeedsTwoVideoTracks
                        .tr(self.locale)
                        .to_string(),
                );
                return;
            }
            Err(Failure::NeedsAudio) => {
                self.push_toast(Text::MulticamGroupNeedsAudio.tr(self.locale).to_string());
                return;
            }
        };

        let program_track_id = video_track_ids[0];
        let Some(reference_path) = source_paths.get(&program_track_id).cloned() else {
            self.push_toast(Text::MulticamGroupNeedsAudio.tr(self.locale).to_string());
            return;
        };
        let reference_pcm = avcore::extract_pcm_16k_mono(&reference_path).unwrap_or_default();

        let mut sync_offsets_secs = HashMap::new();
        for &track_id in &video_track_ids {
            if track_id == program_track_id {
                continue;
            }
            let offset = source_paths
                .get(&track_id)
                .and_then(|path| avcore::extract_pcm_16k_mono(path).ok())
                .filter(|pcm| !reference_pcm.is_empty() && !pcm.is_empty())
                .and_then(|other_pcm| {
                    avcore::compute_sync_offset_secs(
                        &reference_pcm,
                        &other_pcm,
                        16_000.0,
                        avcore::DEFAULT_ENVELOPE_WINDOW_SECS,
                        avcore::DEFAULT_MAX_SYNC_OFFSET_SECS,
                    )
                })
                .unwrap_or(0.0);
            sync_offsets_secs.insert(track_id, offset);
        }

        self.push_undo_snapshot();
        let angle_count = video_track_ids.len();
        let timeline = self.active_project_mut().timeline_mut();
        let group_id = timeline.add_multicam_group(
            format!("Multicam {}", timeline.multicam_groups.len() + 1),
            video_track_ids,
            program_track_id,
            sync_offsets_secs,
        );
        let Some(group_id) = group_id else {
            return;
        };
        self.active_multicam_group_id = Some(group_id);
        self.push_toast(
            Text::MulticamGroupSynced
                .tr(self.locale)
                .replace("{n}", &angle_count.to_string()),
        );
    }

    /// Switches [`App::active_multicam_group_id`]'s program track to angle `angle_index` (0-
    /// based -- number key `1` maps to `0`, `2` to `1`, etc.) at the current playhead position,
    /// what pressing a number key on the Editor screen does. Toasts instead of a no-op silently
    /// failing: [`Text::MulticamNoActiveGroup`] if no group has been synced yet,
    /// [`Text::MulticamSwitchFailed`] if the switch itself couldn't be performed (e.g. that
    /// angle has no footage at this exact moment).
    pub fn switch_multicam_angle_at_playhead(&mut self, angle_index: usize) {
        let Some(group_id) = self.active_multicam_group_id else {
            self.push_toast(Text::MulticamNoActiveGroup.tr(self.locale).to_string());
            return;
        };

        self.push_undo_snapshot();
        let at_secs = self.active_project().timeline().playhead_secs;
        let new_clip_id = next_clip_id(self.active_project().timeline());
        let timeline = self.active_project_mut().timeline_mut();
        let switched = timeline.switch_multicam_angle(group_id, angle_index, at_secs, new_clip_id);

        if !switched {
            self.push_toast(
                Text::MulticamSwitchFailed
                    .tr(self.locale)
                    .replace("{n}", &(angle_index + 1).to_string()),
            );
        }
    }
}
