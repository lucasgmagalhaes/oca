use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use tokio::sync::mpsc::UnboundedSender;

use super::{App, ImportEvent, ThumbnailReady, THUMBNAIL_BUCKET_SECS};

impl App {
    /// Probes each of `paths` on its own background thread — what "Importar arquivos" does.
    /// Each file becomes usable in the media library as soon as its probe (cheap: container/
    /// stream metadata only, no decode) comes back, the same way other NLEs show an imported
    /// file instantly and refine it afterward; loudness measurement and (for video) proxy
    /// generation, both full decode passes that can take minutes on a multi-GB capture, keep
    /// running in the background past that point and update the asset in place once they
    /// finish (see [`ImportEvent`]/[`App::pump_import_queue`]). One thread per file rather
    /// than one thread for the whole batch, so a multi-file import isn't serialized behind
    /// its slowest file either. Applied to the target project by id rather than the active
    /// project index, which could change before a slow import finishes.
    pub fn spawn_import(&mut self, paths: Vec<PathBuf>) {
        let project_id = self.active_project().id;
        let proxy_dir = avcore::proxy::cache_dir_for_project(self.active_project());
        let preview_quality = self.prefs.preview_quality;
        self.pending_imports += paths.len();

        for path in paths {
            let import_token = self.next_import_token;
            self.next_import_token += 1;
            let tx = self.import_tx.clone();
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
    }

    /// Applies import progress to its target project's media library as it arrives. Called
    /// once per frame from [`eframe::App::ui`], same as [`App::pump_export_queue`].
    pub(super) fn pump_import_queue(&mut self) {
        while let Ok(event) = self.import_rx.try_recv() {
            match event {
                ImportEvent::AssetReady {
                    project_id,
                    import_token,
                    mut asset,
                } => {
                    self.pending_imports = self.pending_imports.saturating_sub(1);
                    let Some(project) = self.projects.iter_mut().find(|p| p.id == project_id)
                    else {
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
                    self.pending_enrichment.insert(import_token, next_id);
                    // Set by `App::add_sound_library_track_to_timeline` for a Music & SFX
                    // track that wasn't already in the library — only honored if the active
                    // project is still the one this asset landed in, in case it changed while
                    // the import was in flight.
                    if self.auto_add_to_timeline.remove(&import_token)
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
                } => {
                    let Some(asset_id) = self.pending_enrichment.remove(&import_token) else {
                        continue;
                    };
                    let Some(project) = self.projects.iter_mut().find(|p| p.id == project_id)
                    else {
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
                    self.pending_imports = self.pending_imports.saturating_sub(1);
                    tracing::error!(path = %path.display(), error = %message, "asset import failed");
                    self.push_toast(format!(
                        "Import failed — {}: {message}",
                        path.file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.display().to_string())
                    ));
                }
            }
        }
    }

    /// Requests a poster-frame thumbnail for a timeline clip — what `timeline_panel` calls for
    /// every visible filmstrip tile that doesn't have a texture cached yet (see
    /// `editor.rs::draw_filmstrip`). A no-op if `(asset_id, bucket)` was already requested
    /// (successfully or not; see [`App::requested_thumbnails`]). Extraction (open the
    /// asset's proxy-or-source file, seek to the bucket's representative time, grab a frame,
    /// downscale) runs on a background thread — the same reasoning as
    /// [`App::spawn_import`]: this is FFI/decode work that must not run on the UI thread.
    pub fn request_thumbnail(&mut self, asset_id: u64, bucket: i64) {
        let key = (asset_id, bucket);
        if self.requested_thumbnails.contains(&key) {
            return;
        }
        self.requested_thumbnails.insert(key);

        let Some(asset) = self
            .active_project()
            .media_library
            .iter()
            .find(|a| a.id == asset_id)
        else {
            return;
        };
        let path = asset
            .proxy_path
            .clone()
            .unwrap_or_else(|| asset.source_path.clone());
        let at_secs = bucket as f64 * THUMBNAIL_BUCKET_SECS;

        let tx = self.thumbnail_tx.clone();
        std::thread::spawn(move || {
            if let Some((width, height, rgba)) = extract_thumbnail(&path, at_secs) {
                let _ = tx.send(ThumbnailReady {
                    asset_id,
                    bucket,
                    width,
                    height,
                    rgba,
                });
            }
        });
    }

    /// Uploads finished thumbnail extractions as egui textures. Called once per frame from
    /// [`eframe::App::ui`], same as [`App::pump_import_queue`].
    pub(super) fn pump_thumbnail_queue(&mut self, ctx: &egui::Context) {
        while let Ok(ready) = self.thumbnail_rx.try_recv() {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [ready.width as usize, ready.height as usize],
                &ready.rgba,
            );
            let texture = ctx.load_texture(
                format!("thumb-{}-{}", ready.asset_id, ready.bucket),
                image,
                egui::TextureOptions::LINEAR,
            );
            self.thumbnail_textures
                .insert((ready.asset_id, ready.bucket), texture);
        }
    }
}

/// Longest side, in pixels, a generated thumbnail is downscaled to — tiny on purpose, these
/// are drawn small and tiled, not viewed full-size.
const THUMBNAIL_MAX_DIM: u32 = 96;

/// Runs on one of [`App::request_thumbnail`]'s background threads — opens `path`, seeks to
/// `at_secs`, and grabs the first frame that decodes, downscaled. `None` if the file doesn't
/// exist, fails to open, or no frame arrives within the poll deadline.
fn extract_thumbnail(path: &Path, at_secs: f64) -> Option<(u32, u32, Vec<u8>)> {
    if !path.exists() {
        return None;
    }
    let preview = avcore::preview::Preview::open(path, None).ok()?;
    let _ = preview.seek(at_secs.max(0.0));

    // current_frame() is non-blocking — a frame isn't necessarily ready the instant seek()
    // returns, so poll briefly for one.
    let deadline = Instant::now() + Duration::from_millis(800);
    let frame = loop {
        if let Some(frame) = preview.current_frame() {
            break frame;
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };

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
            eprintln!("failed to measure loudness for {}: {e}", path.display());
            None
        }
    };
    let proxy_path = if kind == avcore::MediaKind::Video {
        match avcore::ensure_proxy(path, proxy_dir, preview_quality) {
            Ok(proxy_path) => Some(proxy_path),
            Err(e) => {
                eprintln!("failed to generate proxy for {}: {e}", path.display());
                None
            }
        }
    } else {
        None
    };
    let waveform_peaks = match avcore::generate_waveform(path) {
        Ok(peaks) => Some(peaks),
        Err(e) => {
            eprintln!("failed to compute waveform for {}: {e}", path.display());
            None
        }
    };
    let _ = tx.send(ImportEvent::Enriched {
        project_id,
        import_token,
        loudness,
        proxy_path,
        waveform_peaks,
    });
}
