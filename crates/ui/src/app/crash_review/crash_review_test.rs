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

use super::*;

/// Builds the exact body `install_panic_hook` writes in `main.rs`, so tests exercise the real
/// wire format rather than a hand-simplified stand-in.
fn crash_body(
    app_version: &str,
    timestamp: u64,
    location: &str,
    message: &str,
    backtrace: &str,
) -> String {
    format!(
        "oca v{app_version} crash report\ntimestamp (unix): {timestamp}\nlocation: {location}\nmessage: {message}\n\nbacktrace:\n{backtrace}\n",
    )
}

#[test]
fn returns_none_for_an_empty_directory() {
    let dir = tempfile::tempdir().unwrap();
    assert!(find_latest_unreviewed_crash(dir.path(), 0).is_none());
}

#[test]
fn returns_none_when_the_directory_does_not_exist() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does_not_exist");
    assert!(find_latest_unreviewed_crash(&missing, 0).is_none());
}

#[test]
fn parses_a_real_crash_file() {
    let dir = tempfile::tempdir().unwrap();
    let body = crash_body(
        "1.4.2",
        1_700_000_000,
        "crates/ui/src/app/mod.rs:123:45",
        "index out of bounds: the len is 0 but the index is 0",
        "0: oca::main\n1: std::rt::lang_start",
    );
    std::fs::write(dir.path().join("crash_1700000000.txt"), body).unwrap();

    let crash = find_latest_unreviewed_crash(dir.path(), 0).expect("crash file should parse");
    assert_eq!(crash.timestamp, 1_700_000_000);
    assert_eq!(crash.app_version, "1.4.2");
    assert_eq!(crash.location, "crates/ui/src/app/mod.rs:123:45");
    assert_eq!(
        crash.message,
        "index out of bounds: the len is 0 but the index is 0"
    );
    assert_eq!(crash.backtrace, "0: oca::main\n1: std::rt::lang_start");
}

#[test]
fn a_crash_already_reviewed_is_not_returned_again() {
    let dir = tempfile::tempdir().unwrap();
    let body = crash_body("1.4.2", 1_700_000_000, "a.rs:1:1", "boom", "trace");
    std::fs::write(dir.path().join("crash_1700000000.txt"), body).unwrap();

    // `after_unix` equal to the crash's own timestamp means it's already been reviewed.
    assert!(find_latest_unreviewed_crash(dir.path(), 1_700_000_000).is_none());
    // Strictly older `after_unix` still surfaces it.
    assert!(find_latest_unreviewed_crash(dir.path(), 1_699_999_999).is_some());
}

#[test]
fn picks_the_most_recent_unreviewed_crash_among_several() {
    let dir = tempfile::tempdir().unwrap();
    for (ts, msg) in [
        (1_700_000_000u64, "first"),
        (1_700_000_500u64, "second"),
        (1_700_001_000u64, "third"),
    ] {
        let body = crash_body("1.4.2", ts, "a.rs:1:1", msg, "trace");
        std::fs::write(dir.path().join(format!("crash_{ts}.txt")), body).unwrap();
    }

    let crash = find_latest_unreviewed_crash(dir.path(), 0).expect("a crash should be found");
    assert_eq!(crash.timestamp, 1_700_001_000);
    assert_eq!(crash.message, "third");
}

#[test]
fn skips_files_with_an_unrecognized_name_or_malformed_body() {
    let dir = tempfile::tempdir().unwrap();
    // Not a crash file at all.
    std::fs::write(dir.path().join("oca.log"), b"unrelated").unwrap();
    // Right prefix, non-numeric timestamp.
    std::fs::write(dir.path().join("crash_not_a_number.txt"), b"garbage").unwrap();
    // Right name, body missing the required `location:` line.
    std::fs::write(
        dir.path().join("crash_1700000000.txt"),
        b"oca v1.4.2 crash report\ntimestamp (unix): 1700000000\nmessage: boom\n",
    )
    .unwrap();

    assert!(find_latest_unreviewed_crash(dir.path(), 0).is_none());
}

#[test]
fn a_valid_crash_is_still_found_alongside_malformed_siblings() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("crash_not_a_number.txt"), b"garbage").unwrap();
    let body = crash_body("2.0.0", 1_700_002_000, "b.rs:9:9", "real crash", "trace");
    std::fs::write(dir.path().join("crash_1700002000.txt"), body).unwrap();

    let crash = find_latest_unreviewed_crash(dir.path(), 0).expect("the valid crash should win");
    assert_eq!(crash.timestamp, 1_700_002_000);
    assert_eq!(crash.app_version, "2.0.0");
}

#[test]
fn parse_crash_report_extracts_all_four_fields() {
    let body = crash_body("9.9.9", 42, "x.rs:7:1", "oops", "line one\nline two");
    let (version, location, message, backtrace) = parse_crash_report(&body).unwrap();
    assert_eq!(version, "9.9.9");
    assert_eq!(location, "x.rs:7:1");
    assert_eq!(message, "oops");
    assert_eq!(backtrace, "line one\nline two");
}

#[test]
fn parse_crash_report_rejects_a_body_with_no_version_header() {
    assert!(parse_crash_report("not a crash report at all").is_none());
}

#[test]
fn parse_crash_report_rejects_a_body_missing_the_message_line() {
    let body =
        "oca v1.0.0 crash report\ntimestamp (unix): 1\nlocation: a.rs:1:1\n\nbacktrace:\ntrace\n";
    assert!(parse_crash_report(body).is_none());
}

#[test]
fn crash_stack_text_folds_location_and_message_onto_the_backtrace() {
    let crash = PendingCrashReview::new(
        1,
        "1.0.0".to_owned(),
        "a.rs:1:1".to_owned(),
        "boom".to_owned(),
        "0: frame_one".to_owned(),
    );
    let stack = crash_stack_text(&crash);
    assert!(stack.contains("a.rs:1:1"));
    assert!(stack.contains("boom"));
    assert!(stack.contains("0: frame_one"));
}
