use eframe::egui;

use crate::app::App;

pub(super) fn apply_pending_asset_drop(
    app: &mut App,
    ruler_top: f32,
    px_per_sec: f32,
    track_rows: &[(u64, avcore::timeline::TrackKind, egui::Rect)],
) {
    if let Some((asset_id, pos)) = app.pending_asset_drop.take() {
        if pos.y >= ruler_top {
            let target = track_rows
                .iter()
                .find(|(_, _, rect)| rect.y_range().contains(pos.y));
            match target {
                Some((track_id, _, rect)) => {
                    let secs = ((pos.x - rect.left()) / px_per_sec).max(0.0) as f64;
                    app.add_asset_to_timeline_at(asset_id, Some(*track_id), secs);
                }
                None => app.add_asset_to_timeline(asset_id),
            }
        }
    }
}
