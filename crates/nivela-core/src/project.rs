use serde::{Deserialize, Serialize};

use crate::media::MediaAsset;
use crate::timeline::Timeline;

/// How long ago a project was last edited. Locale-neutral by design — the UI layer is
/// responsible for turning this into a translated label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Recency {
    HoursAgo(u32),
    Yesterday,
    DaysAgo(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,
    pub name: String,
    pub last_edited: Recency,
    pub summary: String,
    pub media_library: Vec<MediaAsset>,
    pub timeline: Timeline,
}
