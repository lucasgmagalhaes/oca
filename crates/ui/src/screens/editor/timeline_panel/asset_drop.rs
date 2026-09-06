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
