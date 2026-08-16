//! `fetch_latest_release` itself isn't tested here — it's a real network call against GitHub's
//! API, the same "not gated behind an env var, just fails offline" posture as
//! `model_download_test.rs`'s Whisper download tests. `is_newer` is pure and fully covered.

use avcore::is_newer;

#[test]
fn a_newer_patch_version_is_newer() {
    assert!(is_newer("0.1.0", "0.1.1"));
}

#[test]
fn a_newer_minor_version_is_newer() {
    assert!(is_newer("0.1.9", "0.2.0"));
}

#[test]
fn a_newer_major_version_is_newer() {
    assert!(is_newer("0.9.9", "1.0.0"));
}

#[test]
fn the_same_version_is_not_newer() {
    assert!(!is_newer("1.2.3", "1.2.3"));
}

#[test]
fn an_older_version_is_not_newer() {
    assert!(!is_newer("1.2.3", "1.2.2"));
}

#[test]
fn a_shorter_latest_version_compares_missing_segments_as_zero() {
    // "1.3" reads as "1.3.0" - not newer than "1.3.0", but newer than "1.2.9".
    assert!(!is_newer("1.3.0", "1.3"));
    assert!(is_newer("1.2.9", "1.3"));
}

#[test]
fn non_numeric_suffixes_do_not_panic_and_are_stripped_from_each_segment() {
    // "3-beta"'s leading digits ("3") are what gets compared - the "-beta" tail is ignored,
    // not treated as an error or a fourth segment.
    assert!(!is_newer("1.2.3", "1.2.3-beta"));
    assert!(is_newer("1.2.3", "1.2.4-beta"));
}

#[test]
fn a_segment_with_no_leading_digits_at_all_reads_as_zero() {
    assert!(!is_newer("1.2.0", "1.2.beta"));
}

#[test]
fn empty_strings_do_not_panic() {
    assert!(!is_newer("", ""));
    assert!(!is_newer("1.0.0", ""));
}
