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

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

use avcore::{
    AudioSegment, Canvas, ClipSegment, ExportJob, ExportJobStatus, RenderOutcome, ShapeSegment,
    TextSegment,
};
use tracing::{debug, error, info};

use super::{App, RenderEvent};

/// Cooperative controls shared by the UI and one render worker. The native encoder already
/// checks `cancel` between packets; pause is implemented at its per-frame progress callback,
/// where the worker can safely wait without blocking egui's UI thread.
pub(super) struct RenderControl {
    cancel: AtomicBool,
    paused: AtomicBool,
    wait_lock: Mutex<()>,
    wake: Condvar,
}

impl RenderControl {
    pub(super) fn new() -> Self {
        Self {
            cancel: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            wait_lock: Mutex::new(()),
            wake: Condvar::new(),
        }
    }

    fn lock_wait_state(&self) -> std::sync::MutexGuard<'_, ()> {
        self.wait_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(super) fn pause(&self) {
        let _guard = self.lock_wait_state();
        self.paused.store(true, Ordering::Release);
    }

    pub(super) fn resume(&self) {
        let _guard = self.lock_wait_state();
        self.paused.store(false, Ordering::Release);
        self.wake.notify_all();
    }

    pub(super) fn cancel(&self) {
        let _guard = self.lock_wait_state();
        self.cancel.store(true, Ordering::Release);
        self.paused.store(false, Ordering::Release);
        self.wake.notify_all();
    }

    pub(super) fn cancel_flag(&self) -> &AtomicBool {
        &self.cancel
    }

    #[cfg(test)]
    pub(super) fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(super) fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }

    /// Waits until Resume or Cancel when a pause was requested. Returns `false` after
    /// cancellation so the caller can avoid doing any more callback-side work.
    pub(super) fn wait_if_paused(&self) -> bool {
        let mut guard = self.lock_wait_state();
        while self.paused.load(Ordering::Acquire) && !self.cancel.load(Ordering::Acquire) {
            guard = self
                .wake
                .wait(guard)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        !self.cancel.load(Ordering::Acquire)
    }
}

/// A ready-to-queue export whose output path collided with an existing file — held until the
/// user picks Overwrite, Rename, or Cancel in [`App::show_export_conflict_modal`]. Everything
/// [`App::queue_export`] needs is captured here so resolving the conflict is just a matter of
/// picking (or rewriting) `output_path` and calling it.
pub struct PendingExportConflict {
    pub title: String,
    pub track_segments: Vec<Vec<ClipSegment>>,
    pub audio_segments: Vec<AudioSegment>,
    pub text_segments: Vec<TextSegment>,
    pub shape_segments: Vec<ShapeSegment>,
    pub canvas: Canvas,
    pub target_lufs: f32,
    pub output_path: PathBuf,
}

impl App {
    /// Appends a new `Queued` job — what "Adicionar exportação" does, given video, audio, text,
    /// and shape segments already resolved from the active sequence so this job renders the
    /// timeline as it was at the moment it entered the queue, not whatever is edited later.
    /// Picked up by [`App::pump_export_queue`] once a worker slot ([`App::queue_workers`])
    /// frees up.
    pub fn queue_export(
        &mut self,
        title: String,
        track_segments: Vec<Vec<avcore::ClipSegment>>,
        audio_segments: Vec<avcore::AudioSegment>,
        text_segments: Vec<avcore::TextSegment>,
        shape_segments: Vec<avcore::ShapeSegment>,
        canvas: avcore::Canvas,
        target_lufs: f32,
        output_path: String,
    ) {
        let id = self.export_jobs.iter().map(|j| j.id).max().unwrap_or(0) + 1;
        info!(job_id = id, title = %title, output = %output_path, "export job queued");
        // `segments` is kept as the first track for backwards-compatible JSON; `track_segments`
        // carries the full multi-track snapshot that the render worker actually uses.
        let segments = track_segments.first().cloned().unwrap_or_default();
        self.export_jobs.push(ExportJob {
            id,
            title,
            segments,
            text_segments,
            shape_segments,
            track_segments,
            audio_segments,
            canvas,
            target_lufs,
            output_path,
            status: ExportJobStatus::Queued,
        });
        save_queue(&self.export_jobs);
    }

    /// Stops a job: kills its `ffmpeg` process if it's actively rendering, or just removes it
    /// from the list if it hadn't started yet. There's no such thing as cancelling a `Done`/
    /// `Failed` job — callers only wire this to the buttons where it's meaningful.
    pub fn cancel_export_job(&mut self, job_id: u64) {
        match self.active_renders.get(&job_id) {
            Some(control) => control.cancel(),
            None => self.export_jobs.retain(|j| j.id != job_id),
        }
        save_queue(&self.export_jobs);
    }

    /// Pauses a queued job before it starts, or requests a cooperative pause for an active
    /// render at its next frame-progress checkpoint. Completed and failed jobs are unchanged.
    pub fn pause_export_job(&mut self, job_id: u64) {
        let Some(job) = self.export_jobs.iter_mut().find(|job| job.id == job_id) else {
            return;
        };
        let percent = match job.status {
            ExportJobStatus::Queued => 0,
            ExportJobStatus::Rendering { percent } => percent,
            _ => return,
        };
        if let Some(control) = self.active_renders.get(&job_id) {
            control.pause();
        }
        job.status = ExportJobStatus::Paused { percent };
        save_queue(&self.export_jobs);
    }

    /// Resumes an active paused worker, or moves a persisted/pre-start paused job back to the
    /// queue so [`App::pump_export_queue`] can dispatch it normally.
    pub fn resume_export_job(&mut self, job_id: u64) {
        let Some(job) = self.export_jobs.iter_mut().find(|job| job.id == job_id) else {
            return;
        };
        let ExportJobStatus::Paused { percent } = job.status else {
            return;
        };
        if let Some(control) = self.active_renders.get(&job_id) {
            control.resume();
            job.status = ExportJobStatus::Rendering { percent };
        } else {
            job.status = ExportJobStatus::Queued;
        }
        save_queue(&self.export_jobs);
    }

    /// Applies events from render worker threads to `export_jobs`, then — if there's a free
    /// worker slot under `queue_workers` — dispatches the next `Queued` job to a new
    /// background thread. Called once per frame; this is the entire "background export
    /// queue" from the execution plan's Fase 4.
    pub(super) fn pump_export_queue(&mut self) {
        while let Ok(event) = self.render_rx.try_recv() {
            match event {
                RenderEvent::Progress { job_id, percent } => {
                    if let Some(job) = self.export_jobs.iter_mut().find(|j| j.id == job_id) {
                        match &mut job.status {
                            ExportJobStatus::Rendering { percent: current }
                            | ExportJobStatus::Paused { percent: current } => {
                                *current = percent;
                            }
                            _ => {}
                        }
                    }
                }
                RenderEvent::Done {
                    job_id,
                    duration_ms,
                    output_duration_secs,
                } => {
                    if let Some(job) = self.export_jobs.iter_mut().find(|j| j.id == job_id) {
                        info!(job_id, output = %job.output_path, "export job completed");
                        job.status = ExportJobStatus::Done;
                    }
                    self.active_renders.remove(&job_id);
                    save_queue(&self.export_jobs);
                    self.record_telemetry(avcore::TelemetryEvent::ExportCompleted {
                        duration_ms,
                        output_duration_secs,
                        success: true,
                    });
                }
                RenderEvent::Failed {
                    job_id,
                    message,
                    duration_ms,
                    output_duration_secs,
                } => {
                    if let Some(job) = self.export_jobs.iter_mut().find(|j| j.id == job_id) {
                        error!(job_id, output = %job.output_path, error = %message, "export job failed");
                        job.status = ExportJobStatus::Failed {
                            message: message.clone(),
                        };
                    }
                    self.active_renders.remove(&job_id);
                    save_queue(&self.export_jobs);
                    self.record_telemetry(avcore::TelemetryEvent::ExportCompleted {
                        duration_ms,
                        output_duration_secs,
                        success: false,
                    });
                    self.record_telemetry(avcore::TelemetryEvent::Error {
                        context: "export".to_string(),
                        message,
                    });
                }
                RenderEvent::Cancelled { job_id } => {
                    debug!(job_id, "export job cancelled");
                    self.export_jobs.retain(|j| j.id != job_id);
                    self.active_renders.remove(&job_id);
                    save_queue(&self.export_jobs);
                }
            }
        }

        if self.active_renders.len() >= self.prefs.export_workers as usize {
            return;
        }
        let Some(job) = self
            .export_jobs
            .iter_mut()
            .find(|j| j.status == ExportJobStatus::Queued)
        else {
            return;
        };

        let job_id = job.id;
        let text_segments = job.text_segments.clone();
        let shape_segments = job.shape_segments.clone();
        let audio_segments = job.audio_segments.clone();
        // Prefer multi-track snapshot; fall back to legacy single-track `segments` field for
        // jobs persisted before multi-track support was added.
        let track_segments: Vec<Vec<avcore::ClipSegment>> = if !job.track_segments.is_empty() {
            job.track_segments.clone()
        } else {
            vec![job.segments.clone()]
        };
        let canvas = job.canvas;
        let output_path = PathBuf::from(&job.output_path);
        let target_lufs = job.target_lufs;
        let gpu_encoder = self.prefs.gpu_encoder;
        job.status = ExportJobStatus::Rendering { percent: 0 };

        // The exported timeline's own length (footage time, not encode wall-clock time) —
        // track 0 defines a multi-track export's overall duration, same as
        // `resolve_timeline_segments`'s `total_duration_secs`, just re-derived here from the
        // already-resolved `ClipSegment`s rather than the original `ClipInstance`s.
        let output_duration_secs: f64 = track_segments
            .first()
            .map(|track| {
                track
                    .iter()
                    .map(|seg| {
                        (seg.source_out_secs - seg.source_in_secs)
                            / (seg.speed_factor as f64).max(0.0001)
                    })
                    .sum()
            })
            .unwrap_or(0.0);

        let control = Arc::new(RenderControl::new());
        self.active_renders.insert(job_id, Arc::clone(&control));

        info!(job_id, output = %output_path.display(), "export render worker dispatched");
        let tx = self.render_tx.clone();
        std::thread::spawn(move || {
            let started = Instant::now();
            let outcome = avcore::render_export_job_multi_with_audio(
                &track_segments,
                &audio_segments,
                canvas,
                &output_path,
                target_lufs,
                gpu_encoder,
                &text_segments,
                &shape_segments,
                control.cancel_flag(),
                |percent| {
                    let _ = tx.send(RenderEvent::Progress { job_id, percent });
                    control.wait_if_paused();
                },
            );
            let duration_ms = started.elapsed().as_millis() as u64;

            let event = match outcome {
                Ok(RenderOutcome::Completed) => RenderEvent::Done {
                    job_id,
                    duration_ms,
                    output_duration_secs,
                },
                Ok(RenderOutcome::Cancelled) => RenderEvent::Cancelled { job_id },
                Err(e) => RenderEvent::Failed {
                    job_id,
                    message: e.to_string(),
                    duration_ms,
                    output_duration_secs,
                },
            };
            let _ = tx.send(event);
        });
    }
}

/// Finds the first `<stem> (2)<ext>`, `<stem> (3)<ext>`, ... sibling of `path` that doesn't
/// already exist. `path` itself is assumed to exist (that's why the caller is renaming).
pub(super) fn next_available_path(path: &std::path::Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new(""));
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned());
    for n in 2.. {
        let candidate_name = match &ext {
            Some(ext) => format!("{stem} ({n}).{ext}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = parent.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("infinite range always yields a free name eventually")
}

/// Returns the platform-appropriate path for the compressed oca export queue, next to
/// `prefs.oc`.
fn queue_path() -> std::path::PathBuf {
    super::prefs_path()
        .parent()
        .map(|d| d.join("queue.ocqueue"))
        .unwrap_or_else(|| std::path::PathBuf::from("queue.ocqueue"))
}

/// Returns the old JSON queue path used before `.ocqueue`. It is read only when the binary file
/// does not exist, then removed after a successful one-time migration.
fn legacy_queue_path() -> std::path::PathBuf {
    super::prefs_path()
        .parent()
        .map(|d| d.join("queue.json"))
        .unwrap_or_else(|| std::path::PathBuf::from("queue.json"))
}

/// Saves `jobs` to `queue.ocqueue`. Queue mutations are infrequent and the metadata is tiny,
/// so this completes synchronously: unlike the old detached writer thread, a newer snapshot
/// cannot be overwritten by an older thread that happens to finish later. The temporary file
/// is created beside the destination and atomically persisted over it.
pub(super) fn save_queue(jobs: &[ExportJob]) {
    let path = queue_path();
    if let Err(storage_error) = save_queue_to_path(jobs, &path) {
        error!(path = %path.display(), error = %storage_error, "failed to save export queue");
    }
}

pub(super) fn save_queue_to_path(
    jobs: &[ExportJob],
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let snapshot = persisted_queue_snapshot(jobs);
    let bytes = avcore::to_ocqueue_bytes(&snapshot)?;
    atomic_write(path, &bytes)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    Ok(())
}

pub(super) fn persisted_queue_snapshot(jobs: &[ExportJob]) -> Vec<ExportJob> {
    let mut snapshot = jobs.to_vec();
    for job in &mut snapshot {
        match job.status {
            ExportJobStatus::Rendering { .. } => job.status = ExportJobStatus::Queued,
            ExportJobStatus::Paused { .. } => {
                job.status = ExportJobStatus::Paused { percent: 0 };
            }
            _ => {}
        }
    }
    snapshot
}

/// Loads `queue.ocqueue`, falling back to a one-time migration from the old `queue.json` only
/// when the binary file does not exist. A corrupt binary file never silently resurrects an
/// older JSON snapshot. `Rendering` jobs are reset to `Queued`; `Paused` jobs stay paused at
/// 0% until explicitly resumed, because workers and partial output do not survive the process.
pub(super) fn load_queue() -> Vec<ExportJob> {
    load_queue_from_paths(&queue_path(), &legacy_queue_path())
}

pub(super) fn load_queue_from_paths(path: &Path, legacy_path: &Path) -> Vec<ExportJob> {
    match std::fs::read(path) {
        Ok(bytes) => match avcore::from_ocqueue_bytes::<Vec<ExportJob>>(&bytes) {
            Ok(jobs) => normalize_loaded_queue(jobs),
            Err(storage_error) => {
                error!(path = %path.display(), error = %storage_error, "failed to load export queue");
                Vec::new()
            }
        },
        Err(read_error) if read_error.kind() == std::io::ErrorKind::NotFound => {
            migrate_legacy_queue(path, legacy_path)
        }
        Err(read_error) => {
            error!(path = %path.display(), error = %read_error, "failed to read export queue");
            Vec::new()
        }
    }
}

fn migrate_legacy_queue(path: &Path, legacy_path: &Path) -> Vec<ExportJob> {
    let Ok(bytes) = std::fs::read(legacy_path) else {
        return Vec::new();
    };
    let Ok(jobs) = serde_json::from_slice::<Vec<ExportJob>>(&bytes) else {
        error!(path = %legacy_path.display(), "failed to parse legacy export queue");
        return Vec::new();
    };
    let jobs = normalize_loaded_queue(jobs);
    match save_queue_to_path(&jobs, path) {
        Ok(()) => {
            if let Err(remove_error) = std::fs::remove_file(legacy_path) {
                error!(path = %legacy_path.display(), error = %remove_error, "failed to remove migrated legacy export queue");
            }
            info!(legacy = %legacy_path.display(), path = %path.display(), "export queue migrated to .ocqueue");
        }
        Err(storage_error) => {
            error!(path = %path.display(), error = %storage_error, "failed to migrate export queue");
        }
    }
    jobs
}

pub(super) fn normalize_loaded_queue(mut jobs: Vec<ExportJob>) -> Vec<ExportJob> {
    for job in &mut jobs {
        if matches!(job.status, ExportJobStatus::Rendering { .. }) {
            job.status = ExportJobStatus::Queued;
        } else if matches!(job.status, ExportJobStatus::Paused { .. }) {
            job.status = ExportJobStatus::Paused { percent: 0 };
        }
    }
    jobs
}
