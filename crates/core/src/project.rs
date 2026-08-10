use std::path::PathBuf;

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

/// A single edit project: its imported media, its timeline, and display metadata for the
/// Início screen's project list. This is the root of everything a user works on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,
    pub name: String,
    pub last_edited: Recency,
    /// Short user-facing description shown on the project card (e.g. "Cortes dos boss
    /// fights, um vídeo por chefe.") — free text, not a translatable UI label.
    pub summary: String,
    pub media_library: Vec<MediaAsset>,
    pub timeline: Timeline,
    /// Where this project was last saved to/loaded from, if anywhere. Not serialized — the
    /// file *contains* this data, it doesn't need to know its own path, and a copied/renamed
    /// project file shouldn't carry a stale path forward.
    #[serde(skip)]
    pub file_path: Option<PathBuf>,
}
