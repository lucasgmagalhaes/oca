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

use super::timeline_ops::{next_clip_id, resolve_or_create_track};
use super::App;

impl App {
    /// Whether a clip is waiting in the clipboard for [`App::paste_clip_at_playhead`] — lets
    /// the timeline context menu grey out "Colar" instead of pasting nothing.
    pub fn has_clipboard_clip(&self) -> bool {
        self.clipboard_clip.is_some()
    }

    /// Copies `selected_clip_id` (and the track kind it's on) to [`App::clipboard_clip`] —
    /// what `Ctrl+C`/the timeline context menu's "Copiar" do. If the selected clip is a
    /// composite block member, every clip sharing its `composite_id` is captured too (not just
    /// the one clicked), so [`App::paste_clip_at_playhead`] can paste the whole block back as
    /// one unit — per `request.md`'s Fase 3 "reutilizado ... como se fosse um clipe só" spec.
    /// A no-op if nothing is selected.
    pub fn copy_selected_clip(&mut self) {
        let Some(clip_id) = self.selected_clip_id else {
            return;
        };
        let found = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .find_map(|t| {
                let selected = t.clips.iter().find(|c| c.id == clip_id)?;
                let group: Vec<_> = match selected.composite_id {
                    Some(composite_id) => t
                        .clips
                        .iter()
                        .filter(|c| c.composite_id == Some(composite_id))
                        .cloned()
                        .collect(),
                    None => vec![selected.clone()],
                };
                Some((group, t.kind))
            });
        if let Some(copied) = found {
            self.clipboard_clip = Some(copied);
        }
    }

    /// [`App::copy_selected_clip`] followed by [`App::delete_selected_clip`] — what
    /// `Ctrl+X`/the context menu's "Recortar" do.
    pub fn cut_selected_clip(&mut self) {
        self.copy_selected_clip();
        self.delete_selected_clip();
    }

    /// Pastes [`App::clipboard_clip`] as new, freshly-id'd clip(s) at the playhead's current
    /// position on the active sequence — what `Ctrl+V`/the context menu's "Colar" do. Lands on
    /// a matching-kind track the same way [`App::add_asset_to_timeline`] does (first existing
    /// track of that kind, auto-created if none exists); the earliest copied clip always lands
    /// exactly on the playhead, not wherever the context menu happened to be opened — a known
    /// simplification. A no-op if the clipboard is empty. Works across sequence tabs and even
    /// across projects, since `clipboard_clip` isn't scoped to either.
    ///
    /// A copied composite block (more than one clip in [`App::clipboard_clip`]) pastes back as
    /// one block: every other copied clip keeps its original offset relative to the earliest
    /// one, and all pasted clips share one fresh `composite_id` — a single copied clip (not a
    /// composite member) stays standalone, same as before.
    pub fn paste_clip_at_playhead(&mut self) {
        let Some((copied, kind)) = self.clipboard_clip.clone() else {
            return;
        };
        self.push_undo_snapshot();
        let playhead_secs = self.active_project().timeline().playhead_secs;
        let timeline = self.active_project_mut().timeline_mut();
        let track_index = resolve_or_create_track(timeline, kind, None);

        let group_min_start = copied
            .iter()
            .map(|c| c.start_secs)
            .fold(f64::INFINITY, f64::min);
        let new_composite_id = (copied.len() > 1).then(|| {
            timeline.tracks[track_index]
                .clips
                .iter()
                .filter_map(|c| c.composite_id)
                .max()
                .unwrap_or(0)
                + 1
        });

        for source in copied {
            let clip_id = next_clip_id(timeline);
            let start_secs = playhead_secs + (source.start_secs - group_min_start);
            timeline.tracks[track_index]
                .clips
                .push(avcore::timeline::ClipInstance {
                    id: clip_id,
                    asset_id: source.asset_id,
                    start_secs,
                    source_in_secs: source.source_in_secs,
                    source_out_secs: source.source_out_secs,
                    composite_id: new_composite_id,
                    color_label: None,
                    gain_db: source.gain_db,
                    frozen: source.frozen,
                    speed_factor: source.speed_factor,
                    crop_x: source.crop_x,
                    crop_y: source.crop_y,
                    crop_w: source.crop_w,
                    crop_h: source.crop_h,
                    mask_shape: source.mask_shape,
                    mask_corner_radius: source.mask_corner_radius,
                    flipped_h: source.flipped_h,
                    color_filter: source.color_filter,
                    vignette_intensity: source.vignette_intensity,
                    brightness: source.brightness,
                    contrast: source.contrast,
                    saturation: source.saturation,
                    sharpen: source.sharpen,
                    chroma_key_enabled: source.chroma_key_enabled,
                    chroma_key_color: source.chroma_key_color,
                    chroma_key_tolerance: source.chroma_key_tolerance,
                    blur_intensity: source.blur_intensity,
                    shake_intensity: source.shake_intensity,
                    glitch_intensity: source.glitch_intensity,
                    pixelize_intensity: source.pixelize_intensity,
                    transition_in: source.transition_in,
                    transition_duration_secs: source.transition_duration_secs,
                    position_keyframes: source.position_keyframes,
                    scale_keyframes: source.scale_keyframes,
                    rotation_keyframes: source.rotation_keyframes,
                    opacity_keyframes: source.opacity_keyframes,
                    gain_keyframes: source.gain_keyframes,
                    brightness_keyframes: source.brightness_keyframes,
                    contrast_keyframes: source.contrast_keyframes,
                    saturation_keyframes: source.saturation_keyframes,
                    crop_x_keyframes: source.crop_x_keyframes,
                    crop_y_keyframes: source.crop_y_keyframes,
                    crop_w_keyframes: source.crop_w_keyframes,
                    crop_h_keyframes: source.crop_h_keyframes,
                    deflicker_enabled: source.deflicker_enabled,
                    lut_path: source.lut_path,
                    layer_scale_x: source.layer_scale_x,
                    layer_scale_y: source.layer_scale_y,
                    stabilization_intensity: source.stabilization_intensity,
                    // A pasted clip keeps the same source_in_secs/source_out_secs as the copied
                    // original, so a matte generated for that exact range (unlike a split's
                    // halves, whose ranges change) is still valid to carry over.
                    background_removal_enabled: source.background_removal_enabled,
                    background_removal_mask_path: source.background_removal_mask_path,
                });
        }
    }

    /// Adds/removes `clip_id` from [`App::multi_selected_clip_ids`] — what `Ctrl+click`ing
    /// a timeline clip does, building up a set of candidates for
    /// [`App::merge_into_composite`].
    pub fn toggle_multi_select(&mut self, clip_id: u64) {
        if !self.multi_selected_clip_ids.remove(&clip_id) {
            self.multi_selected_clip_ids.insert(clip_id);
        }
    }

    /// Merges every clip in [`App::multi_selected_clip_ids`] into one composite block — what
    /// the toolbar's "Mesclar em bloco composto" button and the timeline context menu's
    /// matching entry both do (per `request.md`'s Fase 3 "blocos compostos" spec, and its
    /// context-menu spec listing this among the actions it should offer too). Assigns them all
    /// a fresh `composite_id` and clears the
    /// multi-selection. A no-op, leaving the multi-selection untouched so the user can fix
    /// their pick, if fewer than two ids were selected or they aren't all on the same track —
    /// composite blocks don't span tracks yet.
    pub fn merge_into_composite(&mut self) {
        if self.multi_selected_clip_ids.len() < 2 {
            return;
        }
        self.push_undo_snapshot();
        let ids = self.multi_selected_clip_ids.clone();
        let timeline = self.active_project_mut().timeline_mut();
        let Some(track) = timeline
            .tracks
            .iter_mut()
            .find(|t| ids.iter().all(|id| t.clips.iter().any(|c| c.id == *id)))
        else {
            return;
        };
        let group_id = track
            .clips
            .iter()
            .filter_map(|c| c.composite_id)
            .max()
            .unwrap_or(0)
            + 1;
        for clip in &mut track.clips {
            if ids.contains(&clip.id) {
                clip.composite_id = Some(group_id);
            }
        }
        self.multi_selected_clip_ids.clear();
    }

    /// Whether formatting is waiting in the clipboard for
    /// [`App::paste_selected_clip_formatting`] — lets the timeline context menu grey out
    /// "Colar formatação" otherwise.
    pub fn has_formatting_clipboard(&self) -> bool {
        self.formatting_clipboard.is_some()
    }

    /// Copies `selected_clip_id`'s gain/freeze/speed/crop/mask/flip/color-filter/vignette/
    /// color-adjustment settings to [`App::formatting_clipboard`] — what `Ctrl+Shift+C`/the
    /// context menu's "Copiar formatação" do. A no-op if nothing is selected.
    pub fn copy_selected_clip_formatting(&mut self) {
        let Some(clip) = self.selected_clip() else {
            return;
        };
        self.formatting_clipboard = Some(clip.formatting());
    }

    /// Applies [`App::formatting_clipboard`] onto `selected_clip_id`, without touching any
    /// other field (position, trim, composite membership) — what `Ctrl+Shift+V`/the context
    /// menu's "Colar formatação" do. A no-op if nothing is selected or the clipboard is empty.
    pub fn paste_selected_clip_formatting(&mut self) {
        let Some(formatting) = self.formatting_clipboard.clone() else {
            return;
        };
        self.with_selected_clip_mut(|clip| clip.apply_formatting(&formatting));
    }
}
