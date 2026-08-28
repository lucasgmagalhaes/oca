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

use tracing::{error, info};

use super::{App, UpdateCheckEvent, UpdateCheckStatus};

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
                    auto_update_available: release.auto_update_available,
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
                UpdateCheckEvent::NewerVersionAvailable {
                    version,
                    html_url,
                    auto_update_available,
                } => {
                    self.update_check_status =
                        super::UpdateCheckStatus::Available(super::AvailableUpdate {
                            version,
                            html_url,
                            auto_update_available,
                        });
                }
                UpdateCheckEvent::UpToDate => {
                    self.update_check_status = super::UpdateCheckStatus::UpToDate;
                }
                UpdateCheckEvent::Failed => {
                    self.update_check_status = super::UpdateCheckStatus::Failed;
                }
                UpdateCheckEvent::Installed { update } => {
                    self.update_check_status = UpdateCheckStatus::RestartRequired(update);
                    self.push_toast(
                        crate::i18n::Text::AboutUpdateReadyToast
                            .tr(self.locale)
                            .to_string(),
                    );
                }
                UpdateCheckEvent::InstallFailed { update } => {
                    self.update_check_status = UpdateCheckStatus::InstallFailed(update);
                    self.push_toast(
                        crate::i18n::Text::AboutInstallFailed
                            .tr(self.locale)
                            .to_string(),
                    );
                }
            }
        }
    }

    /// Starts an explicitly requested update on a worker thread. Repeated clicks while a worker
    /// is active are ignored by the state transition: only Available/InstallFailed can enter
    /// Installing.
    pub fn install_available_update(&mut self) {
        let update = match &self.update_check_status {
            UpdateCheckStatus::Available(update) | UpdateCheckStatus::InstallFailed(update) => {
                update.clone()
            }
            _ => return,
        };
        if !avcore::auto_update_supported() || !update.auto_update_available {
            return;
        }
        self.update_check_status = UpdateCheckStatus::Installing(update.clone());
        let tx = self.update_check_tx.clone();
        std::thread::spawn(move || match avcore::apply_update(&update.version) {
            Ok(avcore::ApplyUpdateOutcome::Updated { version }) => {
                info!(%version, "application update installed");
                let mut installed = update;
                installed.version = version;
                let _ = tx.send(UpdateCheckEvent::Installed { update: installed });
            }
            Ok(avcore::ApplyUpdateOutcome::UpToDate) => {
                info!("application was already up to date when installation started");
                let _ = tx.send(UpdateCheckEvent::UpToDate);
            }
            Err(update_error) => {
                error!(error = %update_error, "application update failed");
                let _ = tx.send(UpdateCheckEvent::InstallFailed { update });
            }
        });
    }

    /// Starts the newly installed executable, then asks eframe to close this process. Closing
    /// through the viewport preserves the normal `on_exit` path (preferences + crash sentinel).
    pub fn restart_after_update(&mut self, ctx: &egui::Context) {
        if !matches!(
            self.update_check_status,
            UpdateCheckStatus::RestartRequired(_)
        ) {
            return;
        }
        match avcore::restart_application() {
            Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Err(restart_error) => {
                error!(error = %restart_error, "application restart failed");
                self.push_toast(
                    crate::i18n::Text::AboutRestartFailed
                        .tr(self.locale)
                        .to_string(),
                );
            }
        }
    }
}
