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

//! ER-01B tests. Every disk-queue test uses its own `tempfile::tempdir()` via the `_in(dir)`
//! variants — never [`queue_dir`]/[`load_queue`]/[`enqueue_to_disk`] directly — so parallel
//! `cargo test` runs never share (and flake on) one real on-disk queue.

use std::sync::{Arc, Mutex};

use super::*;

fn sample_report(event_id: &str) -> avcore::ErrorReport {
    avcore::ErrorReport {
        schema_version: avcore::ERROR_REPORT_SCHEMA_VERSION,
        event_id: event_id.to_owned(),
        session_id: "abc123ef".to_owned(),
        release: "oca-1.0.0".to_owned(),
        build_channel: "debug".to_owned(),
        target_triple: "x86_64-pc-windows-msvc".to_owned(),
        app_version: "1.0.0".to_owned(),
        commit: "deadbeef".to_owned(),
        os: "windows".to_owned(),
        os_version: None,
        arch: "x86_64".to_owned(),
        locale: "en".to_owned(),
        portable: false,
        error_code: avcore::ErrorCode::ProjectSave,
        severity: avcore::ErrorSeverity::Error,
        operation: avcore::Operation::ProjectSave,
        recovery_outcome: avcore::RecoveryOutcome::RequiresUserAction,
        retried: false,
        sanitized_stack_trace: None,
        breadcrumbs: Vec::new(),
        media_context: None,
        environment: avcore::RuntimeEnvironment::default(),
    }
}

/// Builds a queue envelope timestamped `offset_secs` after "now" — `load_queue_in`/
/// `prune_queue_dir` check expiry against the *real* wall clock (`now_unix()`, not an injected
/// test clock), so every envelope a queue-round-trip test expects to survive must be built
/// relative to real "now", not a historical fixed epoch second that's already outside the
/// 7-day retention window by the time the test runs.
fn sample_envelope(event_id: &str, offset_secs: u64) -> QueueEnvelope {
    QueueEnvelope::new(sample_report(event_id), now_unix() + offset_secs)
}

// ---------------------------------------------------------------------------
// Disk queue
// ---------------------------------------------------------------------------

#[test]
fn enqueue_then_load_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let envelope = sample_envelope("1111111111111111", 1_000);
    assert!(enqueue_to_disk_in(dir.path(), &envelope).is_some());

    let loaded = load_queue_in(dir.path());
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].event_id, envelope.event_id);
}

#[test]
fn load_queue_drops_corrupt_files_without_crashing() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("garbage.json"), b"not json at all").unwrap();
    let loaded = load_queue_in(dir.path());
    assert!(loaded.is_empty());
    // The corrupt file must actually be removed, not just skipped forever.
    assert!(!dir.path().join("garbage.json").exists());
}

#[test]
fn load_queue_drops_expired_envelopes() {
    let dir = tempfile::tempdir().unwrap();
    let mut expired = sample_envelope("2222222222222222", 0);
    expired.expires_at_unix = 1; // already in the past relative to "now"
    enqueue_to_disk_in(dir.path(), &expired);
    let loaded = load_queue_in(dir.path());
    assert!(loaded.is_empty(), "an expired envelope must not load");
}

#[test]
fn prune_evicts_oldest_first_past_the_record_bound() {
    let dir = tempfile::tempdir().unwrap();
    // One more than QUEUE_MAX_RECORDS, each queued a second apart so ordering is unambiguous.
    for i in 0..(avcore::QUEUE_MAX_RECORDS + 1) {
        let event_id = format!("{i:016x}");
        let envelope = sample_envelope(&event_id, 1_000 + i as u64);
        enqueue_to_disk_in(dir.path(), &envelope);
    }
    let loaded = load_queue_in(dir.path());
    assert_eq!(loaded.len(), avcore::QUEUE_MAX_RECORDS);
    // The very first (oldest) envelope must have been evicted.
    assert!(loaded.iter().all(|e| e.event_id != "0000000000000000"));
    // The most recently queued one must have survived.
    assert!(loaded
        .iter()
        .any(|e| e.event_id == format!("{:016x}", avcore::QUEUE_MAX_RECORDS)));
}

#[test]
fn delete_all_queued_in_empties_the_directory() {
    let dir = tempfile::tempdir().unwrap();
    enqueue_to_disk_in(dir.path(), &sample_envelope("3333333333333333", 1_000));
    enqueue_to_disk_in(dir.path(), &sample_envelope("4444444444444444", 1_001));
    assert_eq!(load_queue_in(dir.path()).len(), 2);

    delete_all_queued_in(dir.path());
    assert!(load_queue_in(dir.path()).is_empty());
}

#[test]
fn enqueue_write_is_atomic_no_tmp_file_left_behind() {
    let dir = tempfile::tempdir().unwrap();
    let envelope = sample_envelope("5555555555555555", 1_000);
    enqueue_to_disk_in(dir.path(), &envelope);
    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(entries.iter().all(|name| !name.ends_with(".tmp")));
    assert!(entries.contains(&format!("{}.json", envelope.event_id)));
}

// ---------------------------------------------------------------------------
// Sentry envelope builder (pure, no network)
// ---------------------------------------------------------------------------

#[test]
fn build_sentry_envelope_is_well_formed_ndjson_with_no_forbidden_content() {
    let envelope = sample_envelope("6666666666666666", 1_700_000_000);
    let bytes = build_sentry_envelope("test_public_key", &envelope);
    let text = String::from_utf8(bytes).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "header, item header, payload");

    let header: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(
        header["event_id"].as_str().unwrap().len(),
        32,
        "sentry event ids must be 32 hex chars"
    );

    let item_header: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(item_header["type"], "event");
    let declared_len = item_header["length"].as_u64().unwrap() as usize;
    assert_eq!(declared_len, lines[2].len());

    let payload: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(payload["release"], "oca-1.0.0");
    assert_eq!(payload["tags"]["error_code"], "project_save");
    assert_eq!(payload["tags"]["operation"], "project_save");

    // Every string value anywhere in the payload must already be forbidden-content-free —
    // the report was built from an already-sanitized `ErrorReport`, and this asserts the
    // envelope builder itself introduces nothing that could reintroduce a path/URL/email.
    fn walk_strings(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::String(s) => out.push(s.clone()),
            serde_json::Value::Array(items) => items.iter().for_each(|v| walk_strings(v, out)),
            serde_json::Value::Object(map) => map.values().for_each(|v| walk_strings(v, out)),
            _ => {}
        }
    }
    let mut strings = Vec::new();
    walk_strings(&payload, &mut strings);
    for s in strings {
        assert!(
            !avcore::contains_forbidden_content(&s),
            "sentry envelope field {s:?} still contains forbidden content"
        );
    }
}

#[test]
fn build_sentry_envelope_uses_the_undoubled_event_id_as_the_oca_dedup_key() {
    let envelope = sample_envelope("7777777777777777", 1_700_000_000);
    let bytes = build_sentry_envelope("k", &envelope);
    let text = String::from_utf8(bytes).unwrap();
    let payload: serde_json::Value = serde_json::from_str(text.lines().nth(2).unwrap()).unwrap();
    assert_eq!(payload["extra"]["oca_event_id"], "7777777777777777");
}

// ---------------------------------------------------------------------------
// DSN parsing
// ---------------------------------------------------------------------------

#[test]
fn parse_dsn_accepts_a_well_formed_dsn() {
    let (key, url) = parse_dsn("https://abc123@o1.ingest.sentry.io/456").unwrap();
    assert_eq!(key, "abc123");
    assert_eq!(url, "https://o1.ingest.sentry.io/api/456/envelope/");
}

#[test]
fn parse_dsn_rejects_malformed_input() {
    assert!(parse_dsn("").is_none());
    assert!(parse_dsn("not-a-url").is_none());
    assert!(
        parse_dsn("http://abc123@host/456").is_none(),
        "must require https"
    );
    assert!(parse_dsn("https://@host/456").is_none(), "empty key");
    assert!(
        parse_dsn("https://abc123@host/not-a-number").is_none(),
        "project id must be numeric"
    );
}

// ---------------------------------------------------------------------------
// Delivery worker (fake sender — never touches the network, per the ER-01 doc's own testing
// strategy requirement)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct FakeSender {
    outcomes: Mutex<Vec<DeliveryOutcome>>,
    calls: Mutex<Vec<String>>,
}

impl FakeSender {
    fn with_outcomes(outcomes: Vec<DeliveryOutcome>) -> Self {
        Self {
            outcomes: Mutex::new(outcomes),
            calls: Mutex::new(Vec::new()),
        }
    }
}

impl EnvelopeSender for FakeSender {
    fn send(&self, envelope: &QueueEnvelope) -> DeliveryOutcome {
        self.calls.lock().unwrap().push(envelope.event_id.clone());
        let mut outcomes = self.outcomes.lock().unwrap();
        if outcomes.len() > 1 {
            outcomes.remove(0)
        } else {
            *outcomes.last().unwrap()
        }
    }
}

#[test]
fn deliver_with_retry_returns_true_immediately_on_success() {
    let sender = FakeSender::with_outcomes(vec![DeliveryOutcome::Delivered]);
    let envelope = sample_envelope("8888888888888888", 1_000);
    assert!(deliver_with_retry(&sender, &envelope));
    assert_eq!(sender.calls.lock().unwrap().len(), 1);
}

#[test]
fn deliver_with_retry_gives_up_immediately_when_not_configured() {
    let sender = FakeSender::with_outcomes(vec![DeliveryOutcome::NotConfigured]);
    let envelope = sample_envelope("9999999999999999", 1_000);
    assert!(!deliver_with_retry(&sender, &envelope));
    assert_eq!(
        sender.calls.lock().unwrap().len(),
        1,
        "must not spend the retry budget probing an unconfigured endpoint"
    );
}

#[test]
fn deliver_with_retry_stops_retrying_after_a_permanent_failure() {
    let sender = FakeSender::with_outcomes(vec![DeliveryOutcome::PermanentFailure]);
    let envelope = sample_envelope("aaaaaaaaaaaaaaaa", 1_000);
    assert!(
        deliver_with_retry(&sender, &envelope),
        "permanent failure means give up on this record (drop it), not keep it queued"
    );
    assert_eq!(sender.calls.lock().unwrap().len(), 1);
}

#[test]
fn spawn_delivery_worker_persists_then_delivers_and_deletes_on_success() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let sender = Arc::new(FakeSender::with_outcomes(vec![DeliveryOutcome::Delivered]));
    spawn_delivery_worker_in(dir.path().to_owned(), rx, sender.clone());

    tx.send(sample_report("bbbbbbbbbbbbbbbb")).unwrap();
    drop(tx);

    // The worker runs on its own thread; poll briefly rather than sleeping a fixed guess.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if sender.calls.lock().unwrap().len() == 1 {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        sender.calls.lock().unwrap().as_slice(),
        ["bbbbbbbbbbbbbbbb"]
    );
    // Delivered => the on-disk envelope must be gone, not left behind.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline && !load_queue_in(dir.path()).is_empty() {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(load_queue_in(dir.path()).is_empty());
}

#[test]
fn spawn_delivery_worker_leaves_transient_failures_queued() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    // Every attempt (initial + all inline retries) fails transiently.
    let sender = Arc::new(FakeSender::with_outcomes(vec![
        DeliveryOutcome::TransientFailure,
    ]));
    spawn_delivery_worker_in(dir.path().to_owned(), rx, sender.clone());

    tx.send(sample_report("cccccccccccccccc")).unwrap();
    drop(tx);

    // Wait for the record to actually land on disk (persisted before delivery is attempted).
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline && load_queue_in(dir.path()).is_empty() {
        std::thread::sleep(Duration::from_millis(20));
    }
    let queued = load_queue_in(dir.path());
    assert_eq!(
        queued.len(),
        1,
        "a persistently-transient failure must stay queued for the next launch, not be dropped"
    );
    assert_eq!(queued[0].event_id, "cccccccccccccccc");
}

#[test]
fn spawn_delivery_worker_sweeps_leftover_queue_at_startup() {
    let dir = tempfile::tempdir().unwrap();
    // Simulate a record left over from a previous, crashed launch.
    enqueue_to_disk_in(dir.path(), &sample_envelope("dddddddddddddddd", 1_000));
    assert_eq!(load_queue_in(dir.path()).len(), 1);

    let (_tx, rx) = tokio::sync::mpsc::unbounded_channel::<avcore::ErrorReport>();
    let sender = Arc::new(FakeSender::with_outcomes(vec![DeliveryOutcome::Delivered]));
    spawn_delivery_worker_in(dir.path().to_owned(), rx, sender.clone());

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline && !load_queue_in(dir.path()).is_empty() {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        load_queue_in(dir.path()).is_empty(),
        "the startup sweep must redeliver whatever was already queued from a prior launch"
    );
}

// ---------------------------------------------------------------------------
// Consent gating
// ---------------------------------------------------------------------------

#[test]
fn disabled_consent_yields_no_reporter() {
    let reporter = seed_error_reporting(crate::i18n::Locale::En, ErrorReportingConsent::Disabled);
    assert!(reporter.is_none());
}

// ---------------------------------------------------------------------------
// Real transport against a local mock HTTP collector (never the production provider, per the
// ER-01 doc's own testing strategy requirement)
// ---------------------------------------------------------------------------

/// Spins a one-shot HTTP server on an ephemeral local port: accepts exactly one connection,
/// reads the request, replies with `status_line`, and returns the request's raw bytes to the
/// caller (via the returned join handle) so the test can assert on what was actually sent.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn spawn_one_shot_http_server(
    status_line: &'static str,
) -> (std::net::SocketAddr, std::thread::JoinHandle<Vec<u8>>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        // Read until the full request (headers + declared Content-Length body) has arrived —
        // closing the socket after only a partial read races the client's own write and shows
        // up as a `ConnectionReset` on the client side instead of a clean response.
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        let mut headers_end = None;
        let mut content_length = 0usize;
        loop {
            let n = stream.read(&mut chunk).unwrap_or(0);
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            if headers_end.is_none() {
                if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
                    headers_end = Some(pos + 4);
                    let header_text = String::from_utf8_lossy(&buf[..pos]);
                    content_length = header_text
                        .lines()
                        .find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or(0);
                }
            }
            if let Some(end) = headers_end {
                if buf.len() >= end + content_length {
                    break;
                }
            }
        }
        let _ = stream.write_all(status_line.as_bytes());
        let _ = stream.flush();
        buf
    });
    (addr, handle)
}

#[test]
fn sentry_envelope_sender_posts_the_envelope_and_reports_delivered_on_2xx() {
    let (addr, handle) = spawn_one_shot_http_server(
        "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    let sender = SentryEnvelopeSender::with_ingest_url(format!("http://{addr}/"));
    let envelope = sample_envelope("eeeeeeeeeeeeeeee", 1_700_000_000);

    let outcome = sender.send(&envelope);
    assert_eq!(outcome, DeliveryOutcome::Delivered);

    let request_bytes = handle.join().unwrap();
    let request = String::from_utf8_lossy(&request_bytes);
    assert!(request.starts_with("POST "));
    assert!(request.contains("application/x-sentry-envelope"));
    assert!(
        request.contains("project_save"),
        "the envelope body (error_code tag) must actually be in the request"
    );
}

#[test]
fn sentry_envelope_sender_treats_5xx_as_transient() {
    let (addr, _handle) = spawn_one_shot_http_server(
        "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    let sender = SentryEnvelopeSender::with_ingest_url(format!("http://{addr}/"));
    let outcome = sender.send(&sample_envelope("ffffffffffffffff", 1_700_000_000));
    assert_eq!(outcome, DeliveryOutcome::TransientFailure);
}

#[test]
fn sentry_envelope_sender_treats_4xx_as_permanent() {
    let (addr, _handle) = spawn_one_shot_http_server(
        "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    let sender = SentryEnvelopeSender::with_ingest_url(format!("http://{addr}/"));
    let outcome = sender.send(&sample_envelope("1010101010101010", 1_700_000_000));
    assert_eq!(outcome, DeliveryOutcome::PermanentFailure);
}

#[test]
fn sentry_envelope_sender_not_configured_without_a_dsn() {
    let sender = SentryEnvelopeSender {
        ingest_url: None,
        public_key: None,
    };
    let outcome = sender.send(&sample_envelope("2020202020202020", 1_700_000_000));
    assert_eq!(outcome, DeliveryOutcome::NotConfigured);
}

#[test]
fn always_send_consent_yields_a_reporter() {
    let reporter = seed_error_reporting(crate::i18n::Locale::En, ErrorReportingConsent::AlwaysSend);
    assert!(reporter.is_some());
}
