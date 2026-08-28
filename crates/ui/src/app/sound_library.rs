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

use avcore::sound_library::LibraryTrack;

use super::App;

impl App {
    /// Rescans `prefs.sound_library_path` on a background thread — same reasoning as
    /// [`App::spawn_import`]: probing every file in the folder is FFI/decode work that must not
    /// block the UI thread. Called once at startup if a path is already configured, and again
    /// whenever the Music & SFX screen's "Rescan" button is pressed or the folder is changed in
    /// Preferences. A no-op (empty result) when the configured path is blank.
    pub fn rescan_sound_library(&mut self) {
        let dir = self.prefs.sound_library_path.clone();
        if dir.is_empty() {
            self.sound_library_tracks.clear();
            return;
        }
        let tx = self.sound_library_tx.clone();
        std::thread::spawn(move || {
            let tracks = avcore::scan_library_dir(std::path::Path::new(&dir));
            let _ = tx.send(tracks);
        });
    }

    /// Applies the latest completed [`App::rescan_sound_library`] scan, if one has landed.
    /// Called once per frame from [`eframe::App::ui`], same as [`App::pump_import_queue`].
    pub(super) fn pump_sound_library_queue(&mut self) {
        // A rescan can be triggered again before the previous one lands (e.g. the folder path
        // changed twice in a row) — draining to the last message rather than the first keeps
        // only the most recent scan's result, discarding any stale one still in the channel.
        let mut latest = None;
        while let Ok(tracks) = self.sound_library_rx.try_recv() {
            latest = Some(tracks);
        }
        if let Some(tracks) = latest {
            self.sound_library_tracks = tracks;
        }
    }

    /// Places `track` onto the timeline — what clicking "Adicionar à timeline" on the Music &
    /// SFX screen does. If `track.path` is already in the active project's media library (it
    /// was added before, from this screen or a regular import), reuses that asset id directly;
    /// otherwise imports it first (same probe/loudness/waveform pipeline as any other file) and
    /// marks its import token in [`App::auto_add_to_timeline`] so [`App::pump_import_queue`]
    /// appends it to the timeline as soon as the probe (not the slower enrichment pass) lands.
    pub fn add_sound_library_track_to_timeline(&mut self, track: &LibraryTrack) {
        if let Some(existing) = self
            .active_project()
            .media_library
            .iter()
            .find(|a| a.source_path == track.path)
        {
            let asset_id = existing.id;
            self.add_asset_to_timeline(asset_id);
            return;
        }

        self.ensure_active_project();
        let project_id = self.active_project().id;
        let import_token = self.import_state.next_import_token;
        self.import_state.next_import_token += 1;
        self.import_state.pending_imports += 1;
        self.import_state.auto_add_to_timeline.insert(import_token);

        let tx = self.import_state.import_tx.clone();
        let path = track.path.clone();
        // Audio never needs a proxy (see `import::import_one`'s `MediaKind::Video` gate), so an
        // empty proxy dir is safe here — it's simply never touched for this file.
        let proxy_dir = std::path::PathBuf::new();
        std::thread::spawn(move || {
            super::import::import_one(
                &path,
                project_id,
                import_token,
                &proxy_dir,
                avcore::PreviewQuality::default(),
                &tx,
            );
        });
    }
}
