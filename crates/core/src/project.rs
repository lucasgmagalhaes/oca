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

/// One editable cut within a project — its own timeline, independently zoomable/playable and
/// shown as its own tab in the Editor (per `request.md`'s Fase 3 "abas de projeto" spec, e.g.
/// one sequence for trimmed highlights, another for the full unedited recording). Every
/// project has at least one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sequence {
    pub id: u64,
    pub name: String,
    pub timeline: Timeline,
}

/// A single edit project: its imported media, its sequences, and display metadata for the
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
    /// Every sequence (tab) in this project — always non-empty; [`Project::new_sequence`]
    /// and every construction site guarantee at least one.
    pub sequences: Vec<Sequence>,
    /// Index into `sequences` of the tab currently shown in the Editor.
    pub active_sequence: usize,
    /// Where this project was last saved to/loaded from, if anywhere. Not serialized — the
    /// file *contains* this data, it doesn't need to know its own path, and a copied/renamed
    /// project file shouldn't carry a stale path forward.
    #[serde(skip)]
    pub file_path: Option<PathBuf>,
}

impl Project {
    /// The active tab's timeline — what the Editor screen and every clip-editing `OcaApp`
    /// method actually read. Panics if `active_sequence` is out of bounds, which shouldn't
    /// happen given `sequences` is never empty and every mutation keeps the index in range —
    /// same trust-the-invariant style as `OcaApp::active_project`'s own indexing.
    pub fn timeline(&self) -> &Timeline {
        &self.sequences[self.active_sequence].timeline
    }

    /// Mutable counterpart of [`Project::timeline`].
    pub fn timeline_mut(&mut self) -> &mut Timeline {
        &mut self.sequences[self.active_sequence].timeline
    }

    /// Appends a new, empty sequence named `name` and switches `active_sequence` to it —
    /// what the Editor's tab bar "+" button does. Returns the new sequence's id.
    pub fn new_sequence(&mut self, name: String) -> u64 {
        let id = self.sequences.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        self.sequences.push(Sequence {
            id,
            name,
            timeline: Timeline {
                tracks: Vec::new(),
                playhead_secs: 0.0,
            },
        });
        self.active_sequence = self.sequences.len() - 1;
        id
    }
}
