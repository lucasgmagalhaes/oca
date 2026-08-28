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

//! ER-01A wiring (the UI side of `avcore::error_reporting`): owns the one process-wide
//! [`avcore::ErrorReportBuilder`], this launch's build identity, and the central
//! [`App::report_error`] path every module routes handled failures through.
//!
//! ER-01A scope is deliberately tight (see `spec/architecture/client-error-reporting.md`): no
//! provider SDK, no networking, no consent UI. The consent gate is "does a reporter exist this
//! launch" — [`seed_error_reporting`] returns `None` today, which *is* the consent-disabled
//! state: no reporter exists, so no code path can build, validate, or write a record anywhere.
//! ER-01B flips just this function to return the provider adapter behind a consent flag;
//! nothing below this module changes shape. Raw `TelemetryEvent::Error` and provider SDK types
//! never cross this module's boundary — local diagnostics (the telemetry file and the crash
//! files written by `main.rs`'s panic hook) stay exactly as they are.

use std::backtrace::Backtrace;
use std::sync::{Arc, Mutex, OnceLock};

use avcore::{
    Breadcrumb, ErrorCode, ErrorReporter, ErrorSeverity, Operation, RecoveryOutcome,
    ReleaseMetadata,
};

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
/// its `app_started` breadcrumb), then returns the reporter this launch reports through.
/// ER-01A always returns `None` — consent UI and the provider adapter are ER-01B — and `None`
/// is by design the consent-disabled state: with no reporter held, [`App::report_error`]
/// short-circuits before touching the builder, so "consent disabled" means *nothing* runs,
/// not even a drop-through-`NullReporter`. Safe to call more than once (e.g. on a locale
/// change): the builder is a process-wide singleton and later calls just refresh the locale.
pub(super) fn seed_error_reporting(locale: Locale) -> Option<Arc<dyn ErrorReporter>> {
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
    None
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
}
