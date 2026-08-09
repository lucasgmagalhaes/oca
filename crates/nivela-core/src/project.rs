use serde::{Deserialize, Serialize};

use crate::media::MediaAsset;
use crate::timeline::Timeline;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,
    pub name: String,
    pub last_edited_label: String,
    pub summary: String,
    pub media_library: Vec<MediaAsset>,
    pub timeline: Timeline,
}

impl Project {
    pub fn track_summary(&self) -> String {
        let clip_count: usize = self.media_library.len();
        let track_names: Vec<&str> = self
            .timeline
            .tracks
            .iter()
            .map(|t| t.name.as_str())
            .collect();
        format!("{clip_count} clipes · {}", track_names.join("/"))
    }
}
