use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use avcore::{ExportJob, ExportJobStatus, RenderOutcome};
use tracing::{debug, error, info};

use super::{OcaApp, RenderEvent};

impl OcaApp {
    /// Appends a new `Queued` job — what "Adicionar exportação" does, given `track_segments`
    /// and `text_segments` already resolved from the active sequence so this job renders the
    /// timeline as it was at the moment it entered the queue, not whatever it's edited to later.
    /// Picked up by [`OcaApp::pump_export_queue`] once a worker slot ([`OcaApp::queue_workers`])
    /// frees up.
    pub fn queue_export(
        &mut self,
        title: String,
        track_segments: Vec<Vec<avcore::ClipSegment>>,
        text_segments: Vec<avcore::TextSegment>,
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
            track_segments,
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
            Some(cancel_flag) => cancel_flag.store(true, Ordering::Relaxed),
            None => self.export_jobs.retain(|j| j.id != job_id),
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
                        if let ExportJobStatus::Rendering { percent: p } = &mut job.status {
                            *p = percent;
                        }
                    }
                }
                RenderEvent::Done { job_id } => {
                    if let Some(job) = self.export_jobs.iter_mut().find(|j| j.id == job_id) {
                        info!(job_id, output = %job.output_path, "export job completed");
                        job.status = ExportJobStatus::Done;
                    }
                    self.active_renders.remove(&job_id);
                    save_queue(&self.export_jobs);
                }
                RenderEvent::Failed { job_id, message } => {
                    if let Some(job) = self.export_jobs.iter_mut().find(|j| j.id == job_id) {
                        error!(job_id, output = %job.output_path, error = %message, "export job failed");
                        job.status = ExportJobStatus::Failed { message };
                    }
                    self.active_renders.remove(&job_id);
                    save_queue(&self.export_jobs);
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
        job.status = ExportJobStatus::Rendering { percent: 0 };

        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.active_renders.insert(job_id, Arc::clone(&cancel_flag));

        info!(job_id, output = %output_path.display(), "export render worker dispatched");
        let tx = self.render_tx.clone();
        std::thread::spawn(move || {
            let outcome = avcore::render_export_job_multi(
                &track_segments,
                canvas,
                &output_path,
                target_lufs,
                &text_segments,
                &cancel_flag,
                |percent| {
                    let _ = tx.send(RenderEvent::Progress { job_id, percent });
                },
            );

            let event = match outcome {
                Ok(RenderOutcome::Completed) => RenderEvent::Done { job_id },
                Ok(RenderOutcome::Cancelled) => RenderEvent::Cancelled { job_id },
                Err(e) => RenderEvent::Failed {
                    job_id,
                    message: e.to_string(),
                },
            };
            let _ = tx.send(event);
        });
    }
}

/// Returns the platform-appropriate path for the oca export queue file, next to `prefs.json`.
fn queue_path() -> std::path::PathBuf {
    super::prefs_path()
        .parent()
        .map(|d| d.join("queue.json"))
        .unwrap_or_else(|| std::path::PathBuf::from("queue.json"))
}

/// Saves `jobs` to `queue.json` on a background thread. `Rendering` jobs are written as
/// `Queued` so they restart properly if the app is reopened mid-queue. `Done` and `Failed`
/// jobs are included for history display.
pub(super) fn save_queue(jobs: &[ExportJob]) {
    let mut snapshot: Vec<ExportJob> = jobs.to_vec();
    for job in &mut snapshot {
        if matches!(job.status, ExportJobStatus::Rendering { .. }) {
            job.status = ExportJobStatus::Queued;
        }
    }
    let path = queue_path();
    std::thread::spawn(move || {
        if let Ok(json) = serde_json::to_string_pretty(&snapshot) {
            let _ = std::fs::write(&path, json.as_bytes());
        }
    });
}

/// Loads the persisted queue from `queue.json`. Returns an empty vec if absent or unparseable.
/// `Rendering`/`Paused` jobs are reset to `Queued` — the render worker died when the app
/// closed.
pub(super) fn load_queue() -> Vec<ExportJob> {
    let path = queue_path();
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    let mut jobs: Vec<ExportJob> = serde_json::from_slice(&bytes).unwrap_or_default();
    for job in &mut jobs {
        if matches!(
            job.status,
            ExportJobStatus::Rendering { .. } | ExportJobStatus::Paused { .. }
        ) {
            job.status = ExportJobStatus::Queued;
        }
    }
    jobs
}
