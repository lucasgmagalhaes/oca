use std::fs;
use std::time::Duration;

use super::*;

// `is_up_to_date` is a private helper, so unlike the rest of this module's tests (see
// `nivela-core/tests/proxy.rs` for those — they only exercise the public API), these have to
// stay unit tests: an integration test in `tests/` can't see non-`pub` items.

#[test]
fn missing_proxy_is_not_up_to_date() {
    let dir = std::env::temp_dir().join("nivela_proxy_unit_test_missing");
    fs::create_dir_all(&dir).unwrap();
    let source = dir.join("source.mp4");
    fs::write(&source, b"src").unwrap();
    let proxy_path = dir.join("does_not_exist_proxy.mp4");

    let result = is_up_to_date(&source, &proxy_path);

    let _ = fs::remove_file(&source);
    let _ = fs::remove_dir(&dir);
    assert!(!result);
}

#[test]
fn a_proxy_older_than_its_source_is_not_up_to_date() {
    let dir = std::env::temp_dir().join("nivela_proxy_unit_test_stale");
    fs::create_dir_all(&dir).unwrap();
    let proxy_path = dir.join("proxy.mp4");
    fs::write(&proxy_path, b"old proxy").unwrap();
    std::thread::sleep(Duration::from_millis(20));
    let source = dir.join("source.mp4");
    fs::write(&source, b"newer source").unwrap();

    let result = is_up_to_date(&source, &proxy_path);

    let _ = fs::remove_file(&source);
    let _ = fs::remove_file(&proxy_path);
    let _ = fs::remove_dir(&dir);
    assert!(!result);
}

#[test]
fn a_proxy_newer_than_its_source_is_up_to_date() {
    let dir = std::env::temp_dir().join("nivela_proxy_unit_test_fresh");
    fs::create_dir_all(&dir).unwrap();
    let source = dir.join("source.mp4");
    fs::write(&source, b"source").unwrap();
    std::thread::sleep(Duration::from_millis(20));
    let proxy_path = dir.join("proxy.mp4");
    fs::write(&proxy_path, b"fresh proxy").unwrap();

    let result = is_up_to_date(&source, &proxy_path);

    let _ = fs::remove_file(&source);
    let _ = fs::remove_file(&proxy_path);
    let _ = fs::remove_dir(&dir);
    assert!(result);
}
