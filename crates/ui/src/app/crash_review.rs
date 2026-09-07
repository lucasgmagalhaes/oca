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

//! ER-01B's post-crash "Send once / Always send / Do not send" review offer (the half of
//! ER-01B item 1 `error_reporting.rs`'s own doc comment used to flag as not wired) — the next
//! launch after `main.rs`'s panic hook writes a `crash_<unix>.txt` file stages that crash for
//! one-time user review rather than ever sending it automatically from the crashing process
//! itself (which cannot reliably complete a network request while unwinding).
//!
//! [`find_latest_unreviewed_crash`] scans the log directory (the same one `main.rs`'s panic hook
//! and rolling `tracing` logs already use) for the newest crash file newer than
//! `PrefsState::last_reviewed_crash_unix`, parsed back into [`PendingCrashReview`]. `App::new`
//! stages the result on [`App::pending_crash_review`]; [`App::show_crash_review_modal`] renders
//! the three-way offer, with a payload-preview section showing the exact post-sanitization JSON
//! that "Send once"/"Always send" would transmit, per the ER-01 doc's own requirement. Any of the
//! three choices marks the crash (and anything older) reviewed and persists that immediately —
//! this is a rare, user-driven, one-time decision, not something to leave to the next incidental
//! prefs save.

use std::path::{Path, PathBuf};

use eframe::egui;

use super::App;
use crate::i18n::Text;

/// One crash file staged for the user's post-crash review, parsed back into the fields
/// `ui::install_panic_hook` wrote. `path`/`timestamp` identify it; `app_version` is the crashed
/// launch's own recorded version (which may differ from this launch's, after an update).
#[derive(Debug, Clone)]
pub(crate) struct PendingCrashReview {
    pub(crate) timestamp: u64,
    pub(crate) app_version: String,
    pub(crate) location: String,
    pub(crate) message: String,
    pub(crate) backtrace: String,
    /// Immutable payload for this crash occurrence. The review modal is rendered every frame,
    /// so rebuilding it there would assign a fresh event identity on every frame.
    pub(crate) report: avcore::ErrorReport,
}

impl PendingCrashReview {
    /// Captures one stable report when the previous launch's crash is discovered.
    ///
    /// This makes Sentry's `event_id` and OCA's `oca_event_id` identify the same crash in the
    /// preview and in every eventual delivery attempt.
    pub(crate) fn new(
        timestamp: u64,
        app_version: String,
        location: String,
        message: String,
        backtrace: String,
    ) -> Self {
        let crash_stack = format!("panic at {location}: {message}\n\n{backtrace}");
        let report = super::error_reporting::build_crash_report(&crash_stack, &app_version);
        Self {
            timestamp,
            app_version,
            location,
            message,
            backtrace,
            report,
        }
    }
}

/// Scans `log_dir` for `crash_<unix>.txt` files newer than `after_unix` (the last one already
/// reviewed) and returns the most recent, parsed. `None` is the common case — `install_panic_hook`
/// only ever writes a file on a real panic. A file whose name or body doesn't match the exact
/// format that hook writes is skipped rather than escalated: this reads a local diagnostic file
/// this same binary wrote, but a malformed one must never fail or block startup.
pub(crate) fn find_latest_unreviewed_crash(
    log_dir: &Path,
    after_unix: u64,
) -> Option<PendingCrashReview> {
    let entries = std::fs::read_dir(log_dir).ok()?;
    let mut candidates: Vec<(u64, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(ts_str) = stem.strip_prefix("crash_") else {
            continue;
        };
        let Ok(timestamp) = ts_str.parse::<u64>() else {
            continue;
        };
        if timestamp > after_unix {
            candidates.push((timestamp, path));
        }
    }
    candidates.sort_by_key(|(ts, _)| *ts);
    let (timestamp, path) = candidates.pop()?;
    let body = std::fs::read_to_string(&path).ok()?;
    let (app_version, location, message, backtrace) = parse_crash_report(&body)?;
    Some(PendingCrashReview::new(
        timestamp,
        app_version,
        location,
        message,
        backtrace,
    ))
}

/// Parses the exact body `install_panic_hook` writes in `main.rs`:
/// `oca v{ver} crash report\ntimestamp (unix): {ts}\nlocation: {loc}\nmessage: {msg}\n\n`
/// `backtrace:\n{bt}\n`. Returns `None` if any required line is missing — a partially-written or
/// hand-edited file is skipped, not treated as a review with blank fields.
fn parse_crash_report(body: &str) -> Option<(String, String, String, String)> {
    let app_version = body
        .lines()
        .next()?
        .strip_prefix("oca v")?
        .strip_suffix(" crash report")?
        .to_owned();
    let location = body.lines().find_map(|l| l.strip_prefix("location: "))?;
    let message = body.lines().find_map(|l| l.strip_prefix("message: "))?;
    let backtrace = body
        .split_once("backtrace:\n")
        .map(|(_, rest)| rest.trim_end().to_owned())
        .unwrap_or_default();
    Some((
        app_version,
        location.to_owned(),
        message.to_owned(),
        backtrace,
    ))
}

/// Assembles the one `crash_stack` string [`super::error_reporting::build_crash_report`] folds
/// into `sanitized_stack_trace` — location and message aren't separate report fields, so they're
/// prefixed onto the backtrace here rather than the report builder growing crash-specific fields.
fn crash_stack_text(crash: &PendingCrashReview) -> String {
    format!(
        "panic at {}: {}\n\n{}",
        crash.location, crash.message, crash.backtrace
    )
}

impl App {
    /// Builds and validates the pending crash's ER-01 structured payload, in the exact
    /// post-sanitization JSON form "Send once"/"Always send" would transmit — the ER-01 doc's
    /// "before a one-time send, the user can inspect the exact structured payload" requirement.
    /// `None` when there's nothing pending or the freshly-built report fails validation (should
    /// not happen, but a broken payload must never be silently shown as sendable either).
    pub(crate) fn crash_review_payload_preview(&self) -> Option<String> {
        let crash = self.pending_crash_review.as_ref()?;
        avcore::validate_report(&crash.report).ok()?;
        serde_json::to_string_pretty(&super::error_reporting::sentry_event_payload(
            &crash.report,
            crash.timestamp,
        ))
        .ok()
    }

    /// "Send once": builds, validates, and sends the pending crash report through
    /// [`super::error_reporting::one_shot_reporter`] regardless of the persisted
    /// [`super::error_reporting::ErrorReportingConsent`] — this is one explicit user action on
    /// one specific report, not a change to the steady-state preference. Marks the crash
    /// reviewed either way: a validation failure must not keep re-prompting for the same
    /// unsendable report on every future launch.
    pub(crate) fn send_pending_crash_once(&mut self) {
        if let Some(crash) = self.pending_crash_review.clone() {
            let report = crash.report;
            match avcore::validate_report(&report) {
                Ok(()) => super::error_reporting::one_shot_reporter().report(report),
                Err(e) => tracing::warn!(error = %e, "dropping invalid crash report"),
            }
        }
        self.finish_crash_review();
    }

    /// "Always send": opts into steady-state remote reporting (persists the consent, spawns the
    /// delivery worker if it isn't already running this launch) and, through that now-active
    /// reporter, sends the pending crash report as its first delivery.
    pub(crate) fn always_send_pending_crash(&mut self) {
        if self.pending_crash_review.is_none() {
            return;
        }
        self.set_error_reporting_consent(super::error_reporting::ErrorReportingConsent::AlwaysSend);
        if let (Some(crash), Some(reporter)) = (
            self.pending_crash_review.clone(),
            self.error_reporter.clone(),
        ) {
            let report = crash.report;
            match avcore::validate_report(&report) {
                Ok(()) => reporter.report(report),
                Err(e) => tracing::warn!(error = %e, "dropping invalid crash report"),
            }
        }
        self.finish_crash_review();
    }

    /// "Do not send": dismisses the review without transmitting anything.
    pub(crate) fn dismiss_pending_crash_review(&mut self) {
        self.finish_crash_review();
    }

    /// Common to all three review actions: marks this crash (and anything older) reviewed so
    /// [`App::new`] never re-prompts for it, then persists that immediately rather than waiting
    /// for the next incidental prefs save.
    fn finish_crash_review(&mut self) {
        if let Some(crash) = self.pending_crash_review.take() {
            self.prefs.last_reviewed_crash_unix = crash.timestamp;
            self.save_prefs();
        }
    }

    /// Renders the post-crash review modal when [`App::pending_crash_review`] is `Some` — same
    /// `egui::Modal` shape `pump_autosave_restore` already established. A collapsible section
    /// shows the exact payload preview; the three buttons map straight onto the ER-01 doc's
    /// "Send once", "Always send error reports", and "Do not send" offer.
    pub(super) fn show_crash_review_modal(&mut self, ctx: &egui::Context) {
        if self.pending_crash_review.is_none() {
            return;
        }
        let locale = self.locale;
        let modal = egui::Modal::new(egui::Id::new("crash_review"));
        let response = modal.show(ctx, |ui| {
            ui.set_width(440.0);
            ui.label(Text::CrashReviewOffer.tr(locale));
            ui.add_space(8.0);
            egui::CollapsingHeader::new(Text::CrashReviewShowPayload.tr(locale))
                .default_open(false)
                .show(ui, |ui| {
                    let mut preview = self
                        .crash_review_payload_preview()
                        .unwrap_or_else(|| "-".to_owned());
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut preview)
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(ui.available_width())
                                    .interactive(false),
                            );
                        });
                });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button(Text::CrashReviewSendOnce.tr(locale)).clicked() {
                    self.send_pending_crash_once();
                }
                if ui.button(Text::CrashReviewAlwaysSend.tr(locale)).clicked() {
                    self.always_send_pending_crash();
                }
                if ui.button(Text::CrashReviewDoNotSend.tr(locale)).clicked() {
                    self.dismiss_pending_crash_review();
                }
            });
        });
        if response.should_close() {
            self.dismiss_pending_crash_review();
        }
    }
}

#[cfg(test)]
#[path = "crash_review/crash_review_test.rs"]
mod tests;
