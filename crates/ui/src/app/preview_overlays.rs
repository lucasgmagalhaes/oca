// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

//! Live text and shape overlay refreshes for an open composited preview.

use super::{App, LiveUpdateKey};
use avcore::TrackKind;
use tracing::warn;

impl App {
    /// Refreshes a loaded text branch after a content-only edit without reopening the pipeline.
    pub(crate) fn refresh_preview_text_content(&mut self, clip_id: u64) {
        if !self.preview_state.preview_text_clip_ids.contains(&clip_id) {
            return;
        }
        let playhead = self.active_project().timeline().playhead_secs;
        let Some(clip) = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Text)
            .flat_map(|track| &track.text_clips)
            .find(|clip| clip.id == clip_id)
            .cloned()
        else {
            return;
        };
        let local_time = playhead - clip.start_secs;
        if let Some(worker) = &self.preview_state.worker {
            worker.live(LiveUpdateKey::global("text"), move |preview| {
                let refs = [(&clip, local_time)];
                if let Err(error) = preview.update_text_overlays(&refs) {
                    warn!(%error, "failed to refresh preview text overlays");
                }
            });
        }
    }

    /// Refreshes a loaded shape branch after a content-only edit without reopening the pipeline.
    pub(crate) fn refresh_preview_shape_content(&mut self, clip_id: u64) {
        if !self.preview_state.preview_shape_clip_ids.contains(&clip_id) {
            return;
        }
        let Some(clip) = self
            .active_project()
            .timeline()
            .tracks
            .iter()
            .filter(|track| track.kind == TrackKind::Shape)
            .flat_map(|track| &track.shape_clips)
            .find(|clip| clip.id == clip_id)
            .cloned()
        else {
            return;
        };
        if let Some(worker) = &self.preview_state.worker {
            worker.live(LiveUpdateKey::for_clip("shape", clip.id), move |preview| {
                if let Err(error) = preview.refresh_shape_overlay(&clip) {
                    warn!(%error, "failed to refresh preview shape overlay");
                }
            });
        }
    }
}
