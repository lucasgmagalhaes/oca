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

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use tokio::sync::mpsc::UnboundedSender;

use super::{
    thumbnail_frame_time, App, ImportEvent, Screen, ThumbnailKey, ThumbnailReady,
    THUMBNAIL_CACHE_CAPACITY, THUMBNAIL_FAILURE_CAPACITY, THUMBNAIL_MAX_DIM, THUMBNAIL_MAX_PENDING,
};

impl App {
    /// Imports every dropped file that carries a real filesystem path (a plain OS drag-and-drop
    /// always does; a drop coming from inside a web view would not, but this is a native
    /// `eframe`/glow app, not a wasm build, so that case doesn't arise here) through the same
    /// [`App::spawn_import`] pipeline as the "Importar arquivos" button — probe/loudness/proxy/
    /// waveform all run identically regardless of how the file reached the media library.
    /// Called once per frame from [`eframe::App::ui`], same as the other `pump_*`/`handle_*`
    /// per-frame helpers. A no-op when nothing was dropped this frame, or when the active screen
    /// has no project media library to import into (only `Editor`/`Library` do) — dropping a
    /// file onto, say, the export queue screen has nowhere sensible to land.
    ///
    /// Dropped onto the Editor specifically (where the timeline is actually visible), each file
    /// is also placed on the timeline as soon as its probe lands, same as
    /// [`App::add_sound_library_track_to_timeline`]'s own auto-add — dragging a video in is a
    /// direct-manipulation "use this now" gesture, not just a library-stocking one, so it should
    /// end up already in the timeline rather than requiring a second manual step (double-click
    /// in the media panel) to get there. Dropped onto the Library screen instead, files land in
    /// the media library only, matching that screen's own "Importar arquivos" button — there's
    /// no timeline in view there for "add it now" to mean anything.
    ///
    /// Dropping more than one file at once lands each on its own brand-new track
    /// ([`App::add_asset_to_new_track`]) instead of [`App::add_asset_to_timeline`]'s usual
    /// "reuse the first track of matching kind" — a batch of files dragged in together is
    /// treated as simultaneous material (e.g. multiple camera angles or a video-plus-narration
    /// pair), so it lands on parallel tracks rather than being silently concatenated one after
    /// another onto a single track. A single dropped file keeps the original append-to-existing-
    /// track behavior.
    pub(super) fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        if !matches!(self.screen, Screen::Editor | Screen::Library) {
            return;
        }
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }
        let paths: Vec<PathBuf> = dropped
            .into_iter()
            .map(|f| f.path().to_path_buf())
            .filter(|p| !p.as_os_str().is_empty())
            .collect();
        if paths.is_empty() {
            return;
        }
        self.ensure_active_project();
        let add_to_timeline = self.screen == Screen::Editor;
        let is_batch = paths.len() > 1;
        let tokens = self.spawn_import(paths);
        if add_to_timeline {
            if is_batch {
                self.import_state.auto_add_to_new_track.extend(tokens);
            } else {
                self.import_state.auto_add_to_timeline.extend(tokens);
            }
        }
    }

    /// Paints a full-window "Drop files here" overlay while the OS is hovering a file drag over
    /// the app and the active screen is one [`App::handle_dropped_files`] would actually import
    /// into — the interaction has no other visual feedback otherwise, since nothing in
    /// `screens/` opts into `egui`'s drag-and-drop payload API (this is OS-level file dragging,
    /// reported through `egui::RawInput::hovered_files`, not an in-app drag). Drawn in its own
    /// foreground `Area` so it sits above whatever screen is currently painted underneath, and
    /// only for the duration of the hover — it never leaves a residual layer once the drag
    /// leaves the window or is dropped.
    pub(super) fn show_drop_hint_overlay(&self, ctx: &egui::Context) {
        if !matches!(self.screen, Screen::Editor | Screen::Library) {
            return;
        }
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        if !hovering {
            return;
        }
        let screen_rect = ctx.content_rect();
        egui::Area::new(egui::Id::new("drop_files_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let painter = ui.painter();
                painter.rect_filled(
                    screen_rect,
                    0.0,
                    egui::Color32::from_rgba_unmultiplied(0x0a, 0x14, 0x14, 200),
                );
                painter.rect_stroke(
                    screen_rect.shrink(16.0),
                    crate::theme::RADIUS_MD as f32,
                    egui::Stroke::new(2.0, crate::theme::ACCENT),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    screen_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    crate::i18n::Text::DropFilesHint.tr(self.locale),
                    egui::FontId::proportional(20.0),
                    crate::theme::TEXT_PRIMARY,
                );
            });
    }

    /// Probes each of `paths` on its own background thread — what "Importar arquivos" does.
    /// Each file becomes usable in the media library as soon as its probe (cheap: container/
    /// stream metadata only, no decode) comes back, the same way other NLEs show an imported
    /// file instantly and refine it afterward; loudness measurement and (for video) proxy
    /// generation, both full decode passes that can take minutes on a multi-GB capture, keep
    /// running in the background past that point and update the asset in place once they
    /// finish (see [`ImportEvent`]/[`App::pump_import_queue`]). One thread per file rather
    /// than one thread for the whole batch, so a multi-file import isn't serialized behind
    /// its slowest file either. Applied to the target project by id rather than the active
    /// project index, which could change before a slow import finishes. Returns each spawned
    /// import's token, one per path in the same order — callers that want a file placed on the
    /// timeline as soon as its probe lands (rather than just sitting in the media library, this
    /// call's own default) mark the returned token(s) in
    /// `import_state.auto_add_to_timeline`, same as [`App::add_sound_library_track_to_timeline`]
    /// already does for its own manually-assigned token.
    pub fn spawn_import(&mut self, paths: Vec<PathBuf>) -> Vec<u64> {
        let project_id = self.active_project().id;
        let proxy_dir = avcore::proxy::cache_dir_for_project(self.active_project());
        let preview_quality = self.prefs.preview_quality;
        self.import_state.pending_imports += paths.len();
        let mut tokens = Vec::with_capacity(paths.len());

        for path in paths {
            let import_token = self.import_state.next_import_token;
            self.import_state.next_import_token += 1;
            tokens.push(import_token);
            let tx = self.import_state.import_tx.clone();
            let proxy_dir = proxy_dir.clone();
            std::thread::spawn(move || {
                import_one(
                    &path,
                    project_id,
                    import_token,
                    &proxy_dir,
                    preview_quality,
                    &tx,
                );
            });
        }
        tokens
    }

    /// Applies import progress to its target project's media library as it arrives. Called
    /// once per frame from [`eframe::App::ui`], same as [`App::pump_export_queue`].
    pub(super) fn pump_import_queue(&mut self) {
        while let Ok(event) = self.import_state.import_rx.try_recv() {
            match event {
                ImportEvent::AssetReady {
                    project_id,
                    import_token,
                    mut asset,
                } => {
                    self.import_state.pending_imports =
                        self.import_state.pending_imports.saturating_sub(1);
                    let Some(project) = self.open_projects.find_mut_by_id(project_id) else {
                        continue;
                    };
                    let next_id = project
                        .media_library
                        .iter()
                        .map(|a| a.id)
                        .max()
                        .unwrap_or(0)
                        + 1;
                    tracing::info!(
                        asset_id = next_id,
                        path = %asset.source_path.display(),
                        codec = %asset.codec,
                        duration_secs = asset.duration_secs,
                        "asset imported"
                    );
                    asset.id = next_id;
                    project.media_library.push(asset);
                    self.import_state
                        .pending_enrichment
                        .insert(import_token, next_id);
                    // Set by `App::add_sound_library_track_to_timeline` for a Music & SFX
                    // track that wasn't already in the library — only honored if the active
                    // project is still the one this asset landed in, in case it changed while
                    // the import was in flight.
                    if self
                        .import_state
                        .auto_add_to_new_track
                        .remove(&import_token)
                        && self.active_project().id == project_id
                    {
                        self.add_asset_to_new_track(next_id);
                    } else if self.import_state.auto_add_to_timeline.remove(&import_token)
                        && self.active_project().id == project_id
                    {
                        self.add_asset_to_timeline(next_id);
                    }
                }
                ImportEvent::Enriched {
                    project_id,
                    import_token,
                    loudness,
                    proxy_path,
                    waveform_peaks,
                    duration_ms,
                } => {
                    self.record_telemetry(avcore::TelemetryEvent::ImportCompleted { duration_ms });
                    let Some(asset_id) = self.import_state.pending_enrichment.remove(&import_token)
                    else {
                        continue;
                    };
                    let Some(project) = self.open_projects.find_mut_by_id(project_id) else {
                        continue;
                    };
                    if let Some(asset) = project.media_library.iter_mut().find(|a| a.id == asset_id)
                    {
                        tracing::debug!(
                            asset_id,
                            has_loudness = loudness.is_some(),
                            has_proxy = proxy_path.is_some(),
                            "asset enrichment complete"
                        );
                        asset.loudness = loudness;
                        asset.proxy_path = proxy_path;
                        asset.waveform_peaks = waveform_peaks;
                    }
                }
                ImportEvent::Failed { path, message } => {
                    self.import_state.pending_imports =
                        self.import_state.pending_imports.saturating_sub(1);
                    tracing::error!(path = %path.display(), error = %message, "asset import failed");
                    self.record_telemetry(avcore::TelemetryEvent::Error {
                        context: "import".to_string(),
                        message: message.clone(),
                    });
                    self.report_error(
                        avcore::ErrorCode::Import,
                        avcore::ErrorSeverity::Error,
                        avcore::Operation::Import,
                        avcore::RecoveryOutcome::RequiresUserAction,
                        false,
                    );
                    self.push_toast(format!(
                        "Import failed - {}: {message}",
                        path.file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.display().to_string())
                    ));
                }
            }
        }
    }

    /// Requests a source frame for one visible timeline filmstrip tile, or (frame 0) one media
    /// library/project card poster frame. `project_id` need not be the active project's — the
    /// Início screen's project cards request thumbnails for every project's assets, not just
    /// the currently open one. A no-op when the key is cached, pending, recently failed, or
    /// [`THUMBNAIL_MAX_PENDING`] extractions are already running. Skipped-over visible keys are
    /// offered again next frame as slots free up. Extraction still runs on a background thread
    /// because opening/seeking/decoding a real GStreamer pipeline must never block the UI thread.
    pub fn request_thumbnail(&mut self, project_id: u64, asset_id: u64, frame_index: i64) {
        let key = (project_id, asset_id, frame_index);
        if self.thumbnail_state.thumbnail_textures.contains_key(&key)
            || self.thumbnail_state.pending_thumbnails.contains(&key)
            || self.thumbnail_state.failed_thumbnails.contains_key(&key)
            || self.thumbnail_state.pending_thumbnails.len() >= THUMBNAIL_MAX_PENDING
        {
            return;
        }

        let Some(asset) = self
            .open_projects
            .iter()
            .find(|p| p.id == project_id)
            .and_then(|p| p.media_library.iter().find(|a| a.id == asset_id))
        else {
            self.remember_thumbnail_failure(key);
            return;
        };
        let path = asset
            .proxy_path
            .clone()
            .unwrap_or_else(|| asset.source_path.clone());
        let at_secs = thumbnail_frame_time(frame_index, asset.fps);

        self.thumbnail_state.pending_thumbnails.insert(key);
        let tx = self.thumbnail_state.thumbnail_tx.clone();
        std::thread::spawn(move || {
            let result = match extract_thumbnail(&path, at_secs) {
                Some((width, height, rgba)) => ThumbnailReady::Ready {
                    project_id,
                    asset_id,
                    frame_index,
                    width,
                    height,
                    rgba,
                },
                None => ThumbnailReady::Failed {
                    project_id,
                    asset_id,
                    frame_index,
                },
            };
            let _ = tx.send(result);
        });
    }

    /// Marks cached keys used by the just-painted visible filmstrip. One shared tick per frame
    /// is enough for LRU ordering and avoids making duplicate tiles artificially newer than
    /// their neighbors merely because the same frame was drawn more than once.
    pub(crate) fn touch_thumbnails(&mut self, keys: &[ThumbnailKey]) {
        if keys.is_empty() {
            return;
        }
        let tick = self.next_thumbnail_usage_tick();
        for key in keys {
            if self.thumbnail_state.thumbnail_textures.contains_key(key) {
                self.thumbnail_state.thumbnail_last_used.insert(*key, tick);
            }
        }
    }

    /// Uploads finished thumbnail extractions as egui textures. Called once per frame from
    /// [`eframe::App::ui`], same as [`App::pump_import_queue`]. Every completion releases its
    /// pending slot; successful uploads enter the bounded texture LRU and failures enter a
    /// smaller bounded retry-suppression set.
    pub(super) fn pump_thumbnail_queue(&mut self, ctx: &egui::Context) {
        while let Ok(ready) = self.thumbnail_state.thumbnail_rx.try_recv() {
            match ready {
                ThumbnailReady::Ready {
                    project_id,
                    asset_id,
                    frame_index,
                    width,
                    height,
                    rgba,
                } => {
                    let key = (project_id, asset_id, frame_index);
                    self.thumbnail_state.pending_thumbnails.remove(&key);
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [width as usize, height as usize],
                        &rgba,
                    );
                    let texture = ctx.load_texture(
                        format!("thumb-{project_id}-{asset_id}-{frame_index}"),
                        image,
                        egui::TextureOptions::LINEAR,
                    );
                    let tick = self.next_thumbnail_usage_tick();
                    self.thumbnail_state.thumbnail_textures.insert(key, texture);
                    self.thumbnail_state.thumbnail_last_used.insert(key, tick);
                    self.evict_thumbnail_textures();
                }
                ThumbnailReady::Failed {
                    project_id,
                    asset_id,
                    frame_index,
                } => {
                    let key = (project_id, asset_id, frame_index);
                    self.thumbnail_state.pending_thumbnails.remove(&key);
                    self.remember_thumbnail_failure(key);
                }
            }
        }
    }

    fn next_thumbnail_usage_tick(&mut self) -> u64 {
        self.thumbnail_state.thumbnail_usage_clock =
            self.thumbnail_state.thumbnail_usage_clock.saturating_add(1);
        self.thumbnail_state.thumbnail_usage_clock
    }

    fn remember_thumbnail_failure(&mut self, key: ThumbnailKey) {
        let tick = self.next_thumbnail_usage_tick();
        self.thumbnail_state.failed_thumbnails.insert(key, tick);
        while self.thumbnail_state.failed_thumbnails.len() > THUMBNAIL_FAILURE_CAPACITY {
            let Some(oldest) = self
                .thumbnail_state
                .failed_thumbnails
                .iter()
                .min_by_key(|(_, tick)| **tick)
                .map(|(key, _)| *key)
            else {
                break;
            };
            self.thumbnail_state.failed_thumbnails.remove(&oldest);
        }
    }

    fn evict_thumbnail_textures(&mut self) {
        while self.thumbnail_state.thumbnail_textures.len() > THUMBNAIL_CACHE_CAPACITY {
            let Some(oldest) = self
                .thumbnail_state
                .thumbnail_textures
                .keys()
                .min_by_key(|key| {
                    self.thumbnail_state
                        .thumbnail_last_used
                        .get(key)
                        .copied()
                        .unwrap_or(0)
                })
                .copied()
            else {
                break;
            };
            self.thumbnail_state.thumbnail_textures.remove(&oldest);
            self.thumbnail_state.thumbnail_last_used.remove(&oldest);
        }
    }
}

/// Runs on one of [`App::request_thumbnail`]'s background threads — opens `path`, seeks to
/// `at_secs`, and grabs the first frame that decodes, downscaled. `None` if the file doesn't
/// exist, fails to open, or no frame arrives within the poll deadline.
fn extract_thumbnail(path: &Path, at_secs: f64) -> Option<(u32, u32, Vec<u8>)> {
    if !path.exists() {
        return None;
    }
    let sampler = avcore::FrameSampler::open(path, Duration::from_millis(20)).ok()?;
    let frame = sampler.sample(at_secs, Duration::from_millis(800))?;

    Some(downscale_rgba(&frame, THUMBNAIL_MAX_DIM))
}

/// Nearest-neighbor downscale of a decoded frame's RGBA buffer so its longest side is at most
/// `max_dim`. Keeps the uploaded texture tiny regardless of the source's actual resolution —
/// fine for something drawn at thumbnail size, and avoids nearest-neighbor's usual aliasing
/// mattering at that scale.
fn downscale_rgba(frame: &avcore::preview::VideoFrame, max_dim: u32) -> (u32, u32, Vec<u8>) {
    let scale = (max_dim as f32 / frame.width.max(frame.height) as f32).min(1.0);
    let new_width = ((frame.width as f32 * scale) as u32).max(1);
    let new_height = ((frame.height as f32 * scale) as u32).max(1);

    let mut rgba = vec![0u8; (new_width * new_height * 4) as usize];
    for y in 0..new_height {
        let src_y = (y * frame.height / new_height).min(frame.height - 1);
        for x in 0..new_width {
            let src_x = (x * frame.width / new_width).min(frame.width - 1);
            let src = ((src_y * frame.width + src_x) * 4) as usize;
            let dst = ((y * new_width + x) * 4) as usize;
            rgba[dst..dst + 4].copy_from_slice(&frame.rgba[src..src + 4]);
        }
    }
    (new_width, new_height, rgba)
}

/// Runs on one of [`App::spawn_import`]'s per-file background threads, in two phases.
/// Phase one probes `path` and sends [`ImportEvent::AssetReady`] the moment that (cheap)
/// call returns — a file that fails to probe sends [`ImportEvent::Failed`] instead and skips
/// phase two entirely. Phase two measures loudness, (for video) generates an editing proxy,
/// and computes a waveform peak table, then sends [`ImportEvent::Enriched`] with whatever came
/// of it; a step that fails just leaves that one field `None` — the asset was already fully
/// usable from phase one, just not as light to scrub, loudness-tagged, or waveform-drawn yet.
pub(super) fn import_one(
    path: &Path,
    project_id: u64,
    import_token: u64,
    proxy_dir: &Path,
    preview_quality: avcore::PreviewQuality,
    tx: &UnboundedSender<ImportEvent>,
) {
    let started = Instant::now();
    let probed = match avcore::probe_media(path) {
        Ok(probed) => probed,
        Err(e) => {
            let _ = tx.send(ImportEvent::Failed {
                path: path.to_path_buf(),
                message: e.to_string(),
            });
            return;
        }
    };

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let asset = probed.into_media_asset(0, file_name, path.to_path_buf());
    let kind = asset.kind;
    let _ = tx.send(ImportEvent::AssetReady {
        project_id,
        import_token,
        asset,
    });

    let loudness = match avcore::measure_loudness(path) {
        Ok(metrics) => Some(metrics),
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "failed to measure loudness");
            None
        }
    };
    let proxy_path = if kind == avcore::MediaKind::Video {
        match avcore::ensure_proxy(path, proxy_dir, preview_quality) {
            Ok(proxy_path) => Some(proxy_path),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "failed to generate proxy");
                None
            }
        }
    } else {
        None
    };
    let waveform_peaks = match avcore::generate_waveform(path) {
        Ok(peaks) => Some(peaks),
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "failed to compute waveform");
            None
        }
    };
    let _ = tx.send(ImportEvent::Enriched {
        project_id,
        import_token,
        loudness,
        proxy_path,
        waveform_peaks,
        duration_ms: started.elapsed().as_millis() as u64,
    });
}

#[cfg(test)]
#[path = "import/import_test.rs"]
mod tests;
