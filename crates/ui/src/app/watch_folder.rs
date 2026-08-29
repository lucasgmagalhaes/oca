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

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{App, WatchFolderEvent, WatchFolderFileStatus, WatchedFileRow};

impl App {
    /// Opens a folder picker and sets it as the watch path — what the Limpeza screen's "Choose
    /// folder..." button does. A no-op (keeps the current path) if the dialog is cancelled.
    pub fn choose_watch_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.watch_folder_state.watch_path = Some(folder);
        }
    }

    /// Starts the background watch loop against `App::watch_folder_state.watch_path` — what the
    /// screen's "Start watching" button does. A no-op if already running or no path is set.
    pub fn start_watching_folder(&mut self) {
        if self.watch_folder_state.running {
            return;
        }
        let Some(watch_path) = self.watch_folder_state.watch_path.clone() else {
            return;
        };

        self.watch_folder_state.running = true;
        self.watch_folder_state.files.clear();
        let stop = Arc::new(AtomicBool::new(false));
        self.watch_folder_state.stop = Some(Arc::clone(&stop));
        let tx = self.watch_folder_state.tx.clone();

        std::thread::spawn(move || watch_loop(watch_path, stop, tx));
    }

    /// Stops the background watch loop, if running — what the screen's "Stop watching" button
    /// does. The thread checks the shared flag on its own schedule (between poll iterations, and
    /// mid-render via the same `cancel` parameter `avcore::process_watched_file` already takes),
    /// so this returns immediately rather than blocking for the thread to actually exit.
    pub fn stop_watching_folder(&mut self) {
        if let Some(stop) = self.watch_folder_state.stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
        self.watch_folder_state.running = false;
    }

    /// Applies watch-folder events to app state. Called once per frame from
    /// [`eframe::App::ui`], same as every other `pump_*` method.
    pub(super) fn pump_watch_folder(&mut self) {
        while let Ok(event) = self.watch_folder_state.rx.try_recv() {
            match event {
                WatchFolderEvent::Detected(path) => {
                    self.watch_folder_state.files.insert(
                        0,
                        WatchedFileRow {
                            path,
                            status: WatchFolderFileStatus::Stabilizing,
                            percent: 0,
                            error: None,
                            before: None,
                            after: None,
                            added_to_project: false,
                        },
                    );
                }
                WatchFolderEvent::Stabilizing(path) => {
                    if let Some(row) = self.find_watched_row_mut(&path) {
                        row.status = WatchFolderFileStatus::Stabilizing;
                    }
                }
                WatchFolderEvent::Processing(path) => {
                    if let Some(row) = self.find_watched_row_mut(&path) {
                        row.status = WatchFolderFileStatus::Processing;
                        row.percent = 0;
                    }
                }
                WatchFolderEvent::Progress(path, percent) => {
                    if let Some(row) = self.find_watched_row_mut(&path) {
                        row.percent = percent;
                    }
                }
                WatchFolderEvent::Done {
                    path,
                    before,
                    after,
                } => {
                    if let Some(row) = self.find_watched_row_mut(&path) {
                        row.status = WatchFolderFileStatus::Done;
                        row.percent = 100;
                        row.before = Some(before);
                        row.after = Some(after);
                    }
                }
                WatchFolderEvent::Failed { path, message } => {
                    tracing::warn!(path = %path.display(), error = %message, "watch-folder processing failed");
                    if let Some(row) = self.find_watched_row_mut(&path) {
                        row.status = WatchFolderFileStatus::Error;
                        row.error = Some(message);
                    }
                }
            }
        }
    }

    /// Queues a completed row's cleaned-up output (`avcore::output_path_for`'s deterministic
    /// `processed/` path — recomputed here rather than stored on the event, since it's a pure
    /// function of `watch_path`/`row.path` that never changes after the fact) for import into
    /// the active project's media library, through the same [`App::spawn_import`] pipeline
    /// every other import path uses — what the Limpeza screen's "Add to project" button does on
    /// a `Done` row. CF-02 slice 1's tractable follow-up: this watcher already reuses
    /// [`avcore::StabilityTracker`] to detect a finished recording; this closes the loop by
    /// bringing its cleaned-up result into a project, rather than leaving that as a manual
    /// "Import..." step. A no-op if there's no active project open, or if this row was already
    /// queued (guards a double-click, since [`App::spawn_import`] has no dedup of its own).
    pub fn add_watched_file_to_project(&mut self, source_path: std::path::PathBuf) {
        if self.projects.is_empty() {
            self.push_toast(
                crate::i18n::Text::WatchFolderNeedsOpenProject
                    .tr(self.locale)
                    .to_string(),
            );
            return;
        }
        let Some(row) = self.find_watched_row_mut(&source_path) else {
            return;
        };
        if row.added_to_project {
            return;
        }
        row.added_to_project = true;
        let Some(watch_path) = self.watch_folder_state.watch_path.clone() else {
            return;
        };
        let output =
            avcore::output_path_for(&watch_path, avcore::DEFAULT_OUTPUT_SUBFOLDER, &source_path);
        self.spawn_import(vec![output]);
    }

    fn find_watched_row_mut(&mut self, path: &std::path::Path) -> Option<&mut WatchedFileRow> {
        self.watch_folder_state
            .files
            .iter_mut()
            .find(|row| row.path == path)
    }
}

/// The background thread's own loop: poll `watch_path` for new/changed video files every
/// [`POLL_INTERVAL`], wait for each to stabilize, then process it — mirrors
/// `Watch-Gameplay.ps1`'s main `while ($true)` loop exactly, one file worth of state
/// (`seen`/`tracker`/`errored`) per watch session.
const POLL_INTERVAL: Duration = Duration::from_secs(2);

fn watch_loop(
    watch_path: PathBuf,
    stop: Arc<AtomicBool>,
    tx: tokio::sync::mpsc::UnboundedSender<WatchFolderEvent>,
) {
    let stable_for = Duration::from_secs(avcore::DEFAULT_STABLE_SECS);
    let mut tracker = avcore::StabilityTracker::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut errored: HashSet<PathBuf> = HashSet::new();

    while !stop.load(Ordering::Relaxed) {
        let Ok(entries) = std::fs::read_dir(&watch_path) else {
            std::thread::sleep(POLL_INTERVAL);
            continue;
        };

        for entry in entries.flatten() {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let path = entry.path();
            if !path.is_file() || !avcore::is_video_file(&path) {
                continue;
            }
            let output =
                avcore::output_path_for(&watch_path, avcore::DEFAULT_OUTPUT_SUBFOLDER, &path);
            if output.exists() {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };

            if seen.insert(path.clone()) {
                let _ = tx.send(WatchFolderEvent::Detected(path.clone()));
            }

            let stable = tracker.poll(&path, metadata.len(), Instant::now(), stable_for);
            if !stable {
                // A file that changed size after erroring gets a fresh chance once it
                // re-stabilizes — matches the script's own "Errored is cleared when the size
                // changes" rule.
                errored.remove(&path);
                let _ = tx.send(WatchFolderEvent::Stabilizing(path.clone()));
                continue;
            }
            if errored.contains(&path) {
                continue;
            }

            let _ = tx.send(WatchFolderEvent::Processing(path.clone()));
            let progress_tx = tx.clone();
            let progress_path = path.clone();
            let result = avcore::process_watched_file(
                &path,
                &output,
                avcore::DEFAULT_TARGET_LUFS,
                &stop,
                move |percent| {
                    let _ = progress_tx
                        .send(WatchFolderEvent::Progress(progress_path.clone(), percent));
                },
            );

            match result {
                Ok(Some((before, after))) => {
                    tracker.forget(&path);
                    let _ = tx.send(WatchFolderEvent::Done {
                        path: path.clone(),
                        before,
                        after,
                    });
                }
                Ok(None) => {
                    // Cancelled mid-render (stop was set) — the outer while-condition will end
                    // the loop on its own next check, nothing to report as an error.
                }
                Err(e) => {
                    errored.insert(path.clone());
                    let _ = tx.send(WatchFolderEvent::Failed {
                        path: path.clone(),
                        message: e.to_string(),
                    });
                }
            }
        }

        std::thread::sleep(POLL_INTERVAL);
    }
}
