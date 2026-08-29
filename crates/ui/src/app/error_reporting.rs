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

//! ER-01A+ER-01B wiring (the UI side of `avcore::error_reporting`): owns the one process-wide
//! [`avcore::ErrorReportBuilder`], this launch's build identity, and the central
//! [`App::report_error`] path every module routes handled failures through.
//!
//! ER-01A's scope was deliberately tight (see `spec/architecture/client-error-reporting.md`):
//! no provider SDK, no networking, no consent UI. The consent gate is still "does a reporter
//! exist this launch" — [`seed_error_reporting`] returns `None` whenever
//! [`ErrorReportingConsent::Disabled`] is active, which *is* the consent-disabled state: no
//! reporter exists, so no code path can build, validate, or write a record anywhere, and no
//! background thread is even spawned. Raw `TelemetryEvent::Error` and provider SDK types never
//! cross this module's boundary — local diagnostics (the telemetry file and the crash files
//! written by `main.rs`'s panic hook) stay exactly as they are.
//!
//! ER-01B adds, behind that same gate: a persisted [`ErrorReportingConsent`] preference (opt-in,
//! defaults to [`ErrorReportingConsent::Disabled`]); a bounded on-disk queue
//! ([`enqueue_to_disk_in`]/[`load_queue`]/[`delete_all_queued`], mirroring the ER-01 doc's 20-record/
//! 20 MiB/7-day bounds already encoded in `avcore`'s [`avcore::QUEUE_MAX_RECORDS`] etc.); a
//! background delivery worker thread (same `std::thread::spawn` + `blocking_recv` shape
//! `telemetry.rs`'s writer thread already uses) that persists every report before attempting
//! delivery, so a report survives a crash between being handed to the reporter and actually
//! reaching the network; and a minimal, provider-owned-format-free Sentry envelope builder
//! ([`build_sentry_envelope`]) behind an [`EnvelopeSender`] trait so the transport is swappable
//! and testable without a live Sentry project. The post-crash one-time "Send once / Always send
//! / Do not send" review offer (ER-01B item 1's other half) is now wired too — see
//! `crash_review.rs`, which scans `main.rs`'s existing `crash_<unix>.txt` files for one newer
//! than the last one reviewed and stages it on [`App`] for a startup modal; [`build_crash_report`]
//! and [`one_shot_reporter`] are this module's half of that (turning the crash file's captured
//! location/message/backtrace into a validated [`avcore::ErrorReport`] and a reporter reachable
//! regardless of the steady-state consent preference). Also not done: an actual configured
//! Sentry DSN — `OCA_SENTRY_DSN` is read at startup and, when
//! unset (true in every build today, since the ER-01 doc's own "Sentry organization/project
//! ownership" open decision is unresolved), the worker still queues and validates every report
//! but never attempts a network call, so opting in today is inert-but-safe until a real DSN is
//! configured.

use std::backtrace::Backtrace;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use avcore::{
    Breadcrumb, ErrorCode, ErrorReporter, ErrorSeverity, Operation, QueueEnvelope, RecoveryOutcome,
    ReleaseMetadata,
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use super::App;
use crate::i18n::Locale;

/// The one report builder per process launch, staging the session id, launch-wide build
/// identity, and breadcrumb trail every report is built from. A `Mutex`: report building is
/// memory-only and moment-sized, fine to serialize across the UI thread and worker threads.
static BUILDER: OnceLock<Mutex<avcore::ErrorReportBuilder>> = OnceLock::new();

/// Fresh builder for the process — used both by [`seed_error_reporting`] (the real seeding
/// path from `App::new`) and as the `OnceLock` default so tests that drive
/// [`App::report_error`] without going through `App::new` still get a well-formed identity.
/// `commit` is `unknown` until ER-01C wires CI to bake a real SHA in via `OCA_COMMIT_SHA`;
/// likewise `release`/`build_channel` are the best available stand-ins until packaging
/// canonicalizes them.
fn default_builder() -> Mutex<avcore::ErrorReportBuilder> {
    Mutex::new(avcore::ErrorReportBuilder::new(ReleaseMetadata {
        release: format!("oca-{}", env!("CARGO_PKG_VERSION")),
        build_channel: if cfg!(debug_assertions) {
            "debug"
        } else {
            "stable"
        }
        .to_owned(),
        target_triple: option_env!("TARGET").unwrap_or("unknown").to_owned(),
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        commit: option_env!("OCA_COMMIT_SHA")
            .unwrap_or("unknown")
            .to_owned(),
    }))
}

fn builder() -> &'static Mutex<avcore::ErrorReportBuilder> {
    BUILDER.get_or_init(default_builder)
}

fn locale_code(locale: Locale) -> &'static str {
    match locale {
        Locale::PtBr => "pt-BR",
        Locale::En => "en",
    }
}

/// Seeds the process-wide report builder with this launch's identity and current locale (and
/// its `app_started` breadcrumb), then returns the reporter this launch reports through —
/// `None` when `consent` is [`ErrorReportingConsent::Disabled`], which is by design the
/// consent-disabled state: with no reporter held, [`App::report_error`] short-circuits before
/// touching the builder, so "consent disabled" means *nothing* runs, not even a
/// drop-through-`NullReporter`, and no delivery thread is spawned. Safe to call more than once
/// (e.g. on a locale change): the builder is a process-wide singleton and later calls just
/// refresh the locale; [`ensure_worker_spawned`] is likewise idempotent.
pub(super) fn seed_error_reporting(
    locale: Locale,
    consent: ErrorReportingConsent,
) -> Option<Arc<dyn ErrorReporter>> {
    let mut builder = builder()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    builder.set_locale(locale_code(locale));
    if builder.breadcrumbs().is_empty() {
        // `app_started` is allowlisted in core's STATE_TRANSITIONS, so this can't be `None`
        // today — but a future core edit removing it must not make error paths panic.
        if let Some(crumb) = Breadcrumb::state_transition("app_started") {
            builder.add_breadcrumb(crumb);
        }
    }
    drop(builder);
    match consent {
        ErrorReportingConsent::Disabled => None,
        ErrorReportingConsent::AlwaysSend => Some(reporter_handle()),
    }
}

impl App {
    /// Central remote-error path (ER-01A): captures a full backtrace, builds a structured
    /// [`ErrorReport`] through the process-wide builder (which sanitizes and caps the stack),
    /// runs the core schema validator as the last line of defense, and hands the validated
    /// report to this launch's reporter. Returns `false` — and does nothing else — when no
    /// reporter exists, which is the consent-disabled state in ER-01A. A validation failure
    /// drops the report with a `warn!`; a report must never reach a transport unvalidated.
    /// Never panics and never blocks on I/O.
    pub(crate) fn report_error(
        &self,
        error_code: ErrorCode,
        severity: ErrorSeverity,
        operation: Operation,
        recovery_outcome: RecoveryOutcome,
        retried: bool,
    ) -> bool {
        let Some(reporter) = &self.error_reporter else {
            return false;
        };
        let backtrace = Backtrace::force_capture();
        let report = builder()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .build_report(
                error_code,
                severity,
                operation,
                recovery_outcome,
                retried,
                Some(&backtrace.to_string()),
            );
        match avcore::validate_report(&report) {
            Ok(()) => {
                reporter.report(report);
                true
            }
            Err(e) => {
                tracing::warn!(error = %e, "dropping invalid error report");
                false
            }
        }
    }

    /// Applies a new ER-01B consent decision: persists it to [`super::PrefsState`], live-swaps
    /// `self.error_reporter` (no restart needed — the very next `report_error` call picks up the
    /// new state), and, on revocation, deletes every queued-but-undelivered report per the ER-01
    /// doc's "revocation ... deletes unsent remote reports" requirement. Opting in spawns the
    /// delivery worker (idempotent — a no-op if it's already running from an earlier session's
    /// opt-in this same launch).
    pub(crate) fn set_error_reporting_consent(&mut self, consent: ErrorReportingConsent) {
        self.prefs.error_reporting_consent = consent;
        self.error_reporter = match consent {
            ErrorReportingConsent::Disabled => {
                delete_all_queued();
                None
            }
            ErrorReportingConsent::AlwaysSend => Some(reporter_handle()),
        };
    }
}

// ---------------------------------------------------------------------------
// ER-01B: consent
// ---------------------------------------------------------------------------

/// The persisted ER-01B consent decision — separate from `PrefsState::telemetry_enabled`
/// (local-only, never sent, a different purpose per the ER-01 doc's "separate preferences with
/// separate explanations" requirement). Only two states: the ER-01 doc's post-crash "Send once"
/// one-time offer (see `crash_review.rs`) is a direct one-shot call through [`one_shot_reporter`]
/// that bypasses this preference entirely — sending one specific crash report once never implies
/// (and never persists) a change to the steady-state decision, so there is no `SendOnce` variant
/// here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorReportingConsent {
    #[default]
    Disabled,
    AlwaysSend,
}

// ---------------------------------------------------------------------------
// ER-01B: bounded on-disk queue
// ---------------------------------------------------------------------------

/// Where queued-but-undelivered envelopes live — next to the rolling `tracing` logs and crash
/// files (`crate::platform_log_dir`), the same "local, operational, on-device" categorization
/// `telemetry.rs`'s own doc comment gives `telemetry.jsonl`, not user configuration (which lives
/// next to `prefs.oc` instead).
fn queue_dir() -> PathBuf {
    crate::platform_log_dir().join("error_reports")
}

fn envelope_path(dir: &std::path::Path, event_id: &str) -> PathBuf {
    dir.join(format!("{event_id}.json"))
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Writes `envelope` into `dir` via a temp-file-then-rename (atomic on every supported
/// platform, per the ER-01 doc's "atomic replacement" requirement), then prunes the directory
/// down to [`avcore::QUEUE_MAX_RECORDS`]/[`avcore::QUEUE_MAX_BYTES`] and drops anything already
/// expired — so a corrupt or runaway local queue can never grow unbounded regardless of how many
/// reports arrive. Returns the (possibly re-)written path, or `None` if the write failed (a full
/// or read-only disk is not escalated — a dropped local diagnostic is not worth degrading the
/// app for). Takes an explicit `dir` so tests can point it at an isolated `tempfile::tempdir()`
/// instead of sharing the one real [`queue_dir`] across every test in the binary.
fn enqueue_to_disk_in(dir: &std::path::Path, envelope: &QueueEnvelope) -> Option<PathBuf> {
    if std::fs::create_dir_all(dir).is_err() {
        return None;
    }
    write_envelope_atomic(dir, envelope)?;
    prune_queue_dir(dir);
    Some(envelope_path(dir, &envelope.event_id))
}

fn write_envelope_atomic(dir: &std::path::Path, envelope: &QueueEnvelope) -> Option<PathBuf> {
    let bytes = serde_json::to_vec(envelope).ok()?;
    let final_path = envelope_path(dir, &envelope.event_id);
    let tmp_path = dir.join(format!("{}.tmp", envelope.event_id));
    std::fs::write(&tmp_path, &bytes).ok()?;
    std::fs::rename(&tmp_path, &final_path).ok()?;
    Some(final_path)
}

/// Reads every well-formed, still-valid envelope currently queued. A file that fails to parse
/// or fails [`avcore::validate_envelope`] is deleted on the spot rather than left to be retried
/// forever — the ER-01 doc's "corrupt entries are quarantined or dropped without repeatedly
/// crashing startup".
pub(crate) fn load_queue() -> Vec<QueueEnvelope> {
    load_queue_in(&queue_dir())
}

fn load_queue_in(dir: &std::path::Path) -> Vec<QueueEnvelope> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let now = now_unix();
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        match serde_json::from_slice::<QueueEnvelope>(&bytes) {
            Ok(envelope) if avcore::validate_envelope(&envelope, now).is_ok() => {
                out.push(envelope);
            }
            _ => {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    out.sort_by_key(|e| e.queued_at_unix);
    out
}

/// Number of currently-queued reports, for the Preferences screen's status line. Cheap enough to
/// call every frame the screen is open — the queue is bounded to
/// [`avcore::QUEUE_MAX_RECORDS`] (20) entries, so this is at most 20 small file reads.
pub(crate) fn queue_len() -> usize {
    load_queue().len()
}

/// Deletes every queued envelope — used on consent revocation (the ER-01 doc's "revocation ...
/// deletes unsent remote reports") and from the Preferences screen's explicit "delete pending
/// reports" control.
pub(crate) fn delete_all_queued() {
    delete_all_queued_in(&queue_dir());
}

fn delete_all_queued_in(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let _ = std::fs::remove_file(entry.path());
    }
}

fn delete_envelope_in(dir: &std::path::Path, event_id: &str) {
    let _ = std::fs::remove_file(envelope_path(dir, event_id));
}

/// Enforces the ER-01 doc's queue bounds (20 records, 20 MiB, whichever comes first) and drops
/// anything past [`avcore::QUEUE_RETENTION_SECS`], oldest-queued-first when a bound is exceeded.
/// Runs after every write rather than on a timer — the queue only ever grows one record at a
/// time from [`enqueue_to_disk_in`], so checking right after each write is sufficient and avoids a
/// separate background sweep.
fn prune_queue_dir(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let now = now_unix();
    let mut records: Vec<(PathBuf, QueueEnvelope, u64)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let len = bytes.len() as u64;
        match serde_json::from_slice::<QueueEnvelope>(&bytes) {
            Ok(envelope) => {
                if envelope.is_expired(now) {
                    let _ = std::fs::remove_file(&path);
                } else {
                    records.push((path, envelope, len));
                }
            }
            Err(_) => {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    records.sort_by_key(|(_, envelope, _)| envelope.queued_at_unix);
    let mut total_bytes: u64 = records.iter().map(|(_, _, len)| *len).sum();
    let mut count = records.len();
    let mut i = 0;
    while (count > avcore::QUEUE_MAX_RECORDS || total_bytes > avcore::QUEUE_MAX_BYTES)
        && i < records.len()
    {
        let (path, _, len) = &records[i];
        if std::fs::remove_file(path).is_ok() {
            total_bytes = total_bytes.saturating_sub(*len);
            count -= 1;
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------------
// ER-01B: delivery transport
// ---------------------------------------------------------------------------

/// Outcome of one delivery attempt — distinguishes "give up on this record" from "keep it
/// queued, try again later", per the ER-01 doc's "do not retry ... schema, or payload-size
/// failures indefinitely" vs. "retry transient failures with exponential backoff".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliveryOutcome {
    Delivered,
    /// Network/5xx/timeout — worth retrying.
    TransientFailure,
    /// 4xx other than 429, or a locally-detected payload problem — retrying won't help.
    PermanentFailure,
    /// No ingestion endpoint is configured ([`OCA_SENTRY_DSN`] unset) — the record stays queued
    /// forever until a DSN is configured and this launch (or a later one) retries it; never
    /// counted as a failure against the retry budget.
    NotConfigured,
}

/// The provider transport, behind a trait so [`spawn_delivery_worker`] is testable against a
/// fake sender without a live Sentry project or even a real network call — the ER-01 doc's own
/// testing strategy requires exactly this ("a local mock HTTPS collector ... tests must not
/// contact the production provider").
pub(crate) trait EnvelopeSender: Send + Sync {
    fn send(&self, envelope: &QueueEnvelope) -> DeliveryOutcome;
}

/// The real transport: a minimal hand-built Sentry envelope (see [`build_sentry_envelope`]) over
/// `ureq`, no Sentry SDK dependency — keeps the provider entirely behind this one adapter, per
/// the ER-01 doc's "hide the provider behind an internal interface" goal. `ingest_url` is
/// `None` whenever `OCA_SENTRY_DSN` is unset or fails to parse, in which case `send` always
/// returns [`DeliveryOutcome::NotConfigured`] without attempting a connection.
pub(crate) struct SentryEnvelopeSender {
    ingest_url: Option<String>,
    public_key: Option<String>,
}

/// Env var carrying a standard Sentry DSN (`https://<public_key>@<host>/<project_id>`).
/// Deliberately just the *public* ingestion DSN — administrative/auth tokens for release
/// management stay in CI secrets per the ER-01 doc and never belong here.
const DSN_ENV_VAR: &str = "OCA_SENTRY_DSN";

impl SentryEnvelopeSender {
    pub(crate) fn from_env() -> Self {
        match std::env::var(DSN_ENV_VAR)
            .ok()
            .and_then(|dsn| parse_dsn(&dsn))
        {
            Some((public_key, ingest_url)) => Self {
                ingest_url: Some(ingest_url),
                public_key: Some(public_key),
            },
            None => Self {
                ingest_url: None,
                public_key: None,
            },
        }
    }

    #[cfg(test)]
    pub(crate) fn with_ingest_url(ingest_url: impl Into<String>) -> Self {
        Self {
            ingest_url: Some(ingest_url.into()),
            public_key: Some("test".to_owned()),
        }
    }
}

impl EnvelopeSender for SentryEnvelopeSender {
    fn send(&self, envelope: &QueueEnvelope) -> DeliveryOutcome {
        let (Some(ingest_url), Some(public_key)) = (&self.ingest_url, &self.public_key) else {
            return DeliveryOutcome::NotConfigured;
        };
        let body = build_sentry_envelope(public_key, envelope);
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(10)))
            .timeout_global(Some(Duration::from_secs(20)))
            // Without this, ureq turns any non-2xx status into `Err`, collapsing the
            // permanent-vs-transient distinction this function exists to make (a 4xx and a 5xx
            // would both just be "an error").
            .http_status_as_error(false)
            .build();
        let result = ureq::Agent::new_with_config(config)
            .post(ingest_url)
            .header("Content-Type", "application/x-sentry-envelope")
            .send(&body[..]);
        match result {
            Ok(response) => {
                let status = response.status().as_u16();
                if (200..300).contains(&status) {
                    DeliveryOutcome::Delivered
                } else if status == 429 || (500..600).contains(&status) {
                    DeliveryOutcome::TransientFailure
                } else {
                    DeliveryOutcome::PermanentFailure
                }
            }
            Err(_) => DeliveryOutcome::TransientFailure,
        }
    }
}

/// Parses a standard Sentry DSN into `(public_key, ingest_url)`. Hand-rolled rather than pulling
/// in a `url` crate dependency for one narrow, fixed format:
/// `https://<public_key>@<host>[:<port>][/<path_prefix>]/<project_id>`. Returns `None` for
/// anything that doesn't match — an unset or malformed DSN degrades to
/// [`DeliveryOutcome::NotConfigured`], never a panic.
fn parse_dsn(dsn: &str) -> Option<(String, String)> {
    let rest = dsn.strip_prefix("https://")?;
    let (key, rest) = rest.split_once('@')?;
    if key.is_empty() || rest.is_empty() {
        return None;
    }
    let (host_and_prefix, project_id) = rest.rsplit_once('/')?;
    if host_and_prefix.is_empty()
        || project_id.is_empty()
        || !project_id.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    Some((
        key.to_owned(),
        format!("https://{host_and_prefix}/api/{project_id}/envelope/"),
    ))
}

fn error_code_str(report: &avcore::ErrorReport) -> String {
    serde_json::to_value(report.error_code)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn operation_str(report: &avcore::ErrorReport) -> String {
    serde_json::to_value(report.operation)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn severity_level(severity: ErrorSeverity) -> &'static str {
    match severity {
        ErrorSeverity::Warning => "warning",
        ErrorSeverity::Error => "error",
        ErrorSeverity::Fatal => "fatal",
    }
}

/// The Sentry event body alone — everything [`build_sentry_envelope`] puts under its envelope's
/// `payload` item, factored out so [`App::crash_review_payload_preview`] can show the user
/// exactly this JSON (pretty-printed) as the ER-01 doc's required pre-send payload preview,
/// without duplicating the field list or actually building an envelope.
pub(super) fn sentry_event_payload(
    report: &avcore::ErrorReport,
    queued_at_unix: u64,
) -> serde_json::Value {
    let sentry_event_id = format!("{0}{0}", report.event_id);
    serde_json::json!({
        "event_id": sentry_event_id,
        "timestamp": queued_at_unix,
        "platform": "other",
        "release": report.release,
        "environment": report.build_channel,
        "level": severity_level(report.severity),
        "message": {
            "formatted": format!("{} during {}", error_code_str(report), operation_str(report)),
        },
        "tags": {
            "error_code": error_code_str(report),
            "operation": operation_str(report),
            "recovery_outcome": serde_json::to_value(report.recovery_outcome).ok(),
            "os": report.os,
            "arch": report.arch,
            "target_triple": report.target_triple,
            "portable": report.portable,
            "retried": report.retried,
        },
        "extra": {
            "oca_event_id": report.event_id,
            "session_id": report.session_id,
            "commit": report.commit,
            "breadcrumb_count": report.breadcrumbs.len(),
            "stack_trace": report.sanitized_stack_trace,
        },
        "contexts": {
            "os": { "name": report.os, "version": report.os_version },
            "runtime": { "name": "oca", "version": report.app_version },
        },
    })
}

/// Builds the raw bytes of a minimal Sentry envelope (the newline-delimited
/// header/item-header/payload framing Sentry's ingestion endpoint expects) from one queued
/// [`ErrorReport`], using only the allowlisted fields already on the report — no additional
/// provider-owned type crosses into this function, and every value placed under `tags`/`extra`
/// is either an `oca`-owned enum's stable string form or an already-sanitized field, never a raw
/// caller string. Pure and unit-tested without any network dependency.
pub(crate) fn build_sentry_envelope(public_key: &str, envelope: &QueueEnvelope) -> Vec<u8> {
    let report = &envelope.report;
    // Sentry event ids are 32 lowercase hex chars; our own `event_id` is a 16-hex-char
    // dedup key (see `avcore::error_reporting::next_id_hex`) — doubled rather than pulling in
    // a UUID dependency for one field. `tags.oca_event_id` below carries the real, undoubled id.
    let sentry_event_id = format!("{0}{0}", report.event_id);
    let header = serde_json::json!({
        "event_id": sentry_event_id,
        "sent_at": envelope.queued_at_unix,
        "dsn_key": public_key,
    });
    let payload = sentry_event_payload(report, envelope.queued_at_unix);
    let payload_bytes = serde_json::to_vec(&payload).unwrap_or_default();
    let item_header = serde_json::json!({
        "type": "event",
        "length": payload_bytes.len(),
    });
    let mut out = Vec::new();
    out.extend_from_slice(header.to_string().as_bytes());
    out.push(b'\n');
    out.extend_from_slice(item_header.to_string().as_bytes());
    out.push(b'\n');
    out.extend_from_slice(&payload_bytes);
    out.push(b'\n');
    out
}

// ---------------------------------------------------------------------------
// ER-01B: background delivery worker
// ---------------------------------------------------------------------------

/// Bounded inline retry budget for one delivery attempt within this worker cycle — short and
/// small on purpose (worst case ~7s) since it blocks this thread from picking up the next
/// queued report while it runs. A record that's still failing after this budget stays on disk
/// (queued) and gets a fresh attempt on the next call to [`ensure_worker_spawned`] (a later
/// launch, or the next report this launch), which *is* this worker's exponential-backoff-across-
/// restarts behavior, not a continuously-running in-process scheduler.
const INLINE_RETRY_BACKOFFS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];

/// Attempts delivery of one envelope, retrying transient failures inline up to
/// [`INLINE_RETRY_BACKOFFS`] times. Returns `true` when the record should be removed from the
/// queue (delivered, or permanently unrecoverable) and `false` when it should stay queued for a
/// later attempt.
fn deliver_with_retry(sender: &dyn EnvelopeSender, envelope: &QueueEnvelope) -> bool {
    match sender.send(envelope) {
        DeliveryOutcome::Delivered => return true,
        DeliveryOutcome::PermanentFailure => return true,
        DeliveryOutcome::NotConfigured => return false,
        DeliveryOutcome::TransientFailure => {}
    }
    for backoff in INLINE_RETRY_BACKOFFS {
        std::thread::sleep(backoff);
        match sender.send(envelope) {
            DeliveryOutcome::Delivered | DeliveryOutcome::PermanentFailure => return true,
            DeliveryOutcome::NotConfigured => return false,
            DeliveryOutcome::TransientFailure => continue,
        }
    }
    false
}

/// Drains `rx` for the app's whole lifetime: every incoming [`avcore::ErrorReport`] is wrapped
/// in a [`QueueEnvelope`] and persisted to disk *before* any delivery attempt (so a crash mid-
/// delivery never loses the record), then delivery is attempted. Also sweeps whatever was
/// already on disk from a previous launch once, at startup, before entering the main loop — the
/// same "retry across restarts" behavior [`deliver_with_retry`]'s own doc comment describes.
/// Own thread, same shape as `telemetry.rs::spawn_telemetry_writer` — an error report must not
/// block the thread that produced it, and disk/network I/O must never run on the UI thread.
pub(crate) fn spawn_delivery_worker(
    rx: UnboundedReceiver<avcore::ErrorReport>,
    sender: Arc<dyn EnvelopeSender>,
) {
    spawn_delivery_worker_in(queue_dir(), rx, sender);
}

/// The actual worker body, parameterized on `dir` so tests can point it at an isolated
/// `tempfile::tempdir()` — see [`enqueue_to_disk_in`]'s own doc comment for why the real queue
/// directory must never be shared across concurrently-running tests.
fn spawn_delivery_worker_in(
    dir: PathBuf,
    mut rx: UnboundedReceiver<avcore::ErrorReport>,
    sender: Arc<dyn EnvelopeSender>,
) {
    std::thread::spawn(move || {
        for envelope in load_queue_in(&dir) {
            if deliver_with_retry(sender.as_ref(), &envelope) {
                delete_envelope_in(&dir, &envelope.event_id);
            }
        }
        while let Some(report) = rx.blocking_recv() {
            let envelope = QueueEnvelope::new(report, now_unix());
            if enqueue_to_disk_in(&dir, &envelope).is_none() {
                tracing::warn!("failed to persist queued error report");
                continue;
            }
            if deliver_with_retry(sender.as_ref(), &envelope) {
                delete_envelope_in(&dir, &envelope.event_id);
            }
        }
    });
}

/// The one delivery-worker channel per process, lazily created on first opt-in — mirrors
/// [`BUILDER`]'s `OnceLock` singleton shape. No worker thread runs at all while consent stays
/// [`ErrorReportingConsent::Disabled`] for the whole launch (the common case today, since the
/// preference defaults to disabled).
static WORKER_TX: OnceLock<UnboundedSender<avcore::ErrorReport>> = OnceLock::new();

/// Spawns the delivery worker on first call; every later call reuses the same channel. Safe to
/// call from any thread (`OnceLock::get_or_init` synchronizes it).
fn ensure_worker_spawned() -> UnboundedSender<avcore::ErrorReport> {
    WORKER_TX
        .get_or_init(|| {
            let (tx, rx) = mpsc::unbounded_channel();
            spawn_delivery_worker(rx, Arc::new(SentryEnvelopeSender::from_env()));
            tx
        })
        .clone()
}

/// The [`avcore::ErrorReporter`] handed to [`App::report_error`] once consent is
/// [`ErrorReportingConsent::AlwaysSend`] — `report` only ever pushes onto an unbounded channel
/// (never blocks, never touches disk/network itself, per the `ErrorReporter` trait's own
/// contract), so the actual persistence/delivery work in [`spawn_delivery_worker`] never runs on
/// the calling thread.
struct SentryReporter {
    tx: UnboundedSender<avcore::ErrorReport>,
}

impl ErrorReporter for SentryReporter {
    fn report(&self, report: avcore::ErrorReport) {
        let _ = self.tx.send(report);
    }
}

fn reporter_handle() -> Arc<dyn ErrorReporter> {
    Arc::new(SentryReporter {
        tx: ensure_worker_spawned(),
    })
}

/// The reporter [`App::send_pending_crash_once`] hands a crash review's one-off report to —
/// same handle as steady-state [`ErrorReportingConsent::AlwaysSend`] reporting (spawning the
/// delivery worker on first use), but reachable regardless of the persisted consent, since
/// "Send once" is one explicit user action on one specific report, not a change to the
/// steady-state preference.
pub(super) fn one_shot_reporter() -> Arc<dyn ErrorReporter> {
    reporter_handle()
}

// ---------------------------------------------------------------------------
// ER-01B: post-crash review report building
// ---------------------------------------------------------------------------

/// Builds the ER-01 structured report for a previous launch's captured panic (see
/// `crash_review.rs`, this pass's post-crash review offer). `crash_stack` is the
/// location/message/backtrace `ui::install_panic_hook` wrote to its `crash_<unix>.txt` file,
/// already assembled by the caller into one string — folded into `sanitized_stack_trace` through
/// the same [`avcore::ErrorReportBuilder::build_report`]/sanitizer path every other report goes
/// through, rather than a bespoke crash schema. Uses *this* launch's session/locale/OS identity
/// (the crashed launch's own session no longer exists to report through) but overrides
/// `release`/`app_version` with the crashed launch's own recorded version, since an update
/// between the crash and this review would otherwise misattribute which build crashed.
pub(super) fn build_crash_report(
    crash_stack: &str,
    crashed_app_version: &str,
) -> avcore::ErrorReport {
    let mut report = builder()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .build_report(
            ErrorCode::Panic,
            ErrorSeverity::Fatal,
            Operation::App,
            RecoveryOutcome::Aborted,
            false,
            Some(crash_stack),
        );
    report.release = format!("oca-{crashed_app_version}");
    report.app_version = crashed_app_version.to_owned();
    report
}

#[cfg(test)]
#[path = "error_reporting/error_reporting_test.rs"]
mod tests;
