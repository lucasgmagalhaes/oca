use std::time::Duration;

use super::*;

fn test_dir(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "oca_autosave_is_newer_test_{tag}_{}",
        std::process::id()
    ))
}

#[test]
fn is_newer_when_the_autosave_was_written_after_the_project() {
    let dir = test_dir("newer");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let project_path = dir.join("project.ocproj");
    let autosave_path = dir.join("project.autosave.ocproj");

    std::fs::write(&project_path, b"old").unwrap();
    std::thread::sleep(Duration::from_millis(20));
    std::fs::write(&autosave_path, b"new").unwrap();

    assert!(autosave_is_newer(&autosave_path, &project_path));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn is_not_newer_when_the_project_was_written_after_the_autosave() {
    let dir = test_dir("older");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let project_path = dir.join("project.ocproj");
    let autosave_path = dir.join("project.autosave.ocproj");

    std::fs::write(&autosave_path, b"old").unwrap();
    std::thread::sleep(Duration::from_millis(20));
    std::fs::write(&project_path, b"new").unwrap();

    assert!(!autosave_is_newer(&autosave_path, &project_path));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn is_not_newer_when_the_autosave_file_does_not_exist() {
    let dir = test_dir("missing_autosave");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let project_path = dir.join("project.ocproj");
    let autosave_path = dir.join("project.autosave.ocproj");
    std::fs::write(&project_path, b"x").unwrap();

    assert!(!autosave_is_newer(&autosave_path, &project_path));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn is_newer_when_the_project_file_does_not_exist_yet() {
    // An autosave with no corresponding saved project (e.g. the project was never saved
    // before the app crashed) should still be offered for restoration.
    let dir = test_dir("missing_project");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let project_path = dir.join("project.ocproj");
    let autosave_path = dir.join("project.autosave.ocproj");
    std::fs::write(&autosave_path, b"x").unwrap();

    assert!(autosave_is_newer(&autosave_path, &project_path));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn is_not_newer_when_neither_file_exists() {
    let dir = test_dir("neither");
    let _ = std::fs::remove_dir_all(&dir);
    let project_path = dir.join("project.ocproj");
    let autosave_path = dir.join("project.autosave.ocproj");

    assert!(!autosave_is_newer(&autosave_path, &project_path));
}
