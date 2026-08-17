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

use super::{App, UpdateCheckEvent};

impl App {
    /// Checks GitHub Releases for a newer published version on a background thread — called
    /// once from [`App::new`] on startup, not on a repeating timer. Silent on any failure
    /// (offline, rate-limited, no releases published yet — see
    /// `avcore::update_check::UpdateCheckError`) and silent when the latest release isn't
    /// actually newer: this is a best-effort courtesy notice, not something that should ever
    /// interrupt or alarm the user, so nothing is sent back at all unless there's real news.
    pub(super) fn spawn_update_check(&self) {
        let tx = self.update_check_tx.clone();
        std::thread::spawn(move || {
            let Ok(release) = avcore::fetch_latest_release() else {
                return;
            };
            if avcore::is_newer(env!("CARGO_PKG_VERSION"), &release.version) {
                let _ = tx.send(UpdateCheckEvent::NewerVersionAvailable {
                    version: release.version,
                    html_url: release.html_url,
                });
            }
        });
    }

    /// Applies a finished update check to [`App::available_update`]. Called once per frame
    /// from [`eframe::App::ui`], same as every other `pump_*` method.
    pub(super) fn pump_update_check(&mut self) {
        while let Ok(event) = self.update_check_rx.try_recv() {
            match event {
                UpdateCheckEvent::NewerVersionAvailable { version, html_url } => {
                    self.available_update = Some(super::AvailableUpdate { version, html_url });
                }
            }
        }
    }
}
