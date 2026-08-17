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
    /// once from [`App::new`] on startup, not on a repeating timer. Failure and current-version
    /// results are sent back as quiet About-modal status rather than toasts; only a genuinely
    /// newer release gets the prominent Home banner.
    pub(super) fn spawn_update_check(&self) {
        let tx = self.update_check_tx.clone();
        std::thread::spawn(move || {
            let release = match avcore::fetch_latest_release() {
                Ok(release) => release,
                Err(_) => {
                    let _ = tx.send(UpdateCheckEvent::Failed);
                    return;
                }
            };
            if avcore::is_newer(env!("CARGO_PKG_VERSION"), &release.version) {
                let _ = tx.send(UpdateCheckEvent::NewerVersionAvailable {
                    version: release.version,
                    html_url: release.html_url,
                });
            } else {
                let _ = tx.send(UpdateCheckEvent::UpToDate);
            }
        });
    }

    /// Applies a finished update check to [`App::update_check_status`]. Called once per frame
    /// from [`eframe::App::ui`], same as every other `pump_*` method.
    pub(super) fn pump_update_check(&mut self) {
        while let Ok(event) = self.update_check_rx.try_recv() {
            match event {
                UpdateCheckEvent::NewerVersionAvailable { version, html_url } => {
                    self.update_check_status =
                        super::UpdateCheckStatus::Available(super::AvailableUpdate {
                            version,
                            html_url,
                        });
                }
                UpdateCheckEvent::UpToDate => {
                    self.update_check_status = super::UpdateCheckStatus::UpToDate;
                }
                UpdateCheckEvent::Failed => {
                    self.update_check_status = super::UpdateCheckStatus::Failed;
                }
            }
        }
    }
}
