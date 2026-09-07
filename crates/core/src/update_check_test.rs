use super::*;

#[test]
fn is_newer_detects_a_bumped_patch_version() {
    assert!(is_newer("1.2.3", "1.2.4"));
    assert!(!is_newer("1.2.4", "1.2.3"));
}

#[test]
fn is_newer_detects_a_bumped_minor_or_major_version() {
    assert!(is_newer("1.2.9", "1.3.0"));
    assert!(is_newer("1.9.9", "2.0.0"));
    assert!(!is_newer("2.0.0", "1.9.9"));
}

#[test]
fn is_newer_is_false_for_identical_versions() {
    assert!(!is_newer("1.2.3", "1.2.3"));
}

#[test]
fn is_newer_treats_a_missing_trailing_segment_as_zero() {
    // "1.2" has no patch segment; comparing against "1.2.0" must not consider it newer, and
    // "1.2.1" must still read as newer than "1.2" alone.
    assert!(!is_newer("1.2", "1.2.0"));
    assert!(is_newer("1.2", "1.2.1"));
}

#[test]
fn is_newer_reads_a_non_numeric_segment_as_zero_without_panicking() {
    // A `-beta` suffix glued onto the last number rather than split by its own dot: the digit
    // run "3" is read, the trailing letters are ignored, never a parse panic.
    assert!(!is_newer("1.2.3-beta", "1.2.3"));
    assert!(is_newer("1.2.3", "1.2.4-beta"));
}

#[test]
fn is_newer_reads_a_fully_non_numeric_segment_as_zero() {
    assert!(!is_newer("abc", "abc"));
    assert!(is_newer("abc", "1"));
}

#[test]
fn expected_update_asset_name_covers_windows_native() {
    assert_eq!(
        expected_update_asset_name("x86_64-pc-windows-msvc", UpdatePackage::NativeBinary),
        Some("oca-x86_64-pc-windows-msvc.zip".to_string())
    );
}

#[test]
fn expected_update_asset_name_covers_linux_native() {
    assert_eq!(
        expected_update_asset_name("x86_64-unknown-linux-gnu", UpdatePackage::NativeBinary),
        Some("oca-x86_64-unknown-linux-gnu.tar.gz".to_string())
    );
}

#[test]
fn expected_update_asset_name_covers_linux_appimage() {
    assert_eq!(
        expected_update_asset_name("x86_64-unknown-linux-gnu", UpdatePackage::LinuxAppImage),
        Some("oca-x86_64-unknown-linux-gnu-appimage.tar.gz".to_string())
    );
}

#[test]
fn expected_update_asset_name_rejects_appimage_on_a_non_linux_target() {
    assert_eq!(
        expected_update_asset_name("x86_64-pc-windows-msvc", UpdatePackage::LinuxAppImage),
        None
    );
}

#[test]
fn expected_update_asset_name_rejects_an_unrecognized_target() {
    assert_eq!(
        expected_update_asset_name("x86_64-apple-darwin", UpdatePackage::NativeBinary),
        None
    );
}

#[test]
fn package_supports_atomic_update_is_true_only_for_linux_appimage() {
    assert!(!package_supports_atomic_update(UpdatePackage::NativeBinary));
    assert!(package_supports_atomic_update(UpdatePackage::LinuxAppImage));
}

#[test]
fn release_supports_auto_update_is_always_false_for_native_binary() {
    let assets = ["oca-x86_64-unknown-linux-gnu.tar.gz"];
    assert!(!release_supports_auto_update(
        assets,
        "x86_64-unknown-linux-gnu",
        UpdatePackage::NativeBinary
    ));
}

#[test]
fn release_supports_auto_update_requires_the_exact_appimage_asset() {
    let matching = ["oca-x86_64-unknown-linux-gnu-appimage.tar.gz"];
    assert!(release_supports_auto_update(
        matching,
        "x86_64-unknown-linux-gnu",
        UpdatePackage::LinuxAppImage
    ));

    let missing = ["oca-x86_64-unknown-linux-gnu.tar.gz"];
    assert!(!release_supports_auto_update(
        missing,
        "x86_64-unknown-linux-gnu",
        UpdatePackage::LinuxAppImage
    ));

    let empty: [&str; 0] = [];
    assert!(!release_supports_auto_update(
        empty,
        "x86_64-unknown-linux-gnu",
        UpdatePackage::LinuxAppImage
    ));
}
