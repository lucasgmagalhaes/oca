use eframe::egui;

/// Deferred mutations collected while timeline track widgets are painted.
#[derive(Default)]
pub(super) struct TrackRequests {
    pub(super) toggle_visibility: Vec<u64>,
    pub(super) toggle_lock: Vec<u64>,
    pub(super) toggle_collapsed: Vec<u64>,
    pub(super) audio_roles: Vec<(u64, avcore::AudioRole)>,
    pub(super) color_labels: Vec<(u64, Option<[u8; 3]>)>,
    pub(super) renames: Vec<(u64, String)>,
    pub(super) duplicates: Vec<u64>,
    pub(super) move_up: Vec<u64>,
    pub(super) move_down: Vec<u64>,
    pub(super) deletes: Vec<(u64, String)>,
    pub(super) rows: Vec<(u64, avcore::timeline::TrackKind, egui::Rect)>,
    pub(super) razor_splits: Vec<(u64, f64)>,
}
