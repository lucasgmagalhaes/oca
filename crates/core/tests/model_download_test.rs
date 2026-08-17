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

//! Real network tests — actually hits huggingface.co. Not gated behind an env var (unlike
//! transcribe_test.rs's WHISPER_MODEL_PATH) since there's nothing local to set up; a machine
//! with no network access will just fail these, same as any other network-dependent test would.

use std::sync::atomic::{AtomicBool, Ordering};

use avcore::model_download::{download_whisper_model, DownloadOutcome, WhisperModelSize};

fn temp_dest_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("avcore_test_model_download_{name}"))
}

#[test]
fn downloads_a_real_model_file() {
    let dest_dir = temp_dest_dir("real");
    let _ = std::fs::remove_dir_all(&dest_dir);
    let cancel = AtomicBool::new(false);
    let mut last_downloaded = 0u64;
    let mut saw_nonzero_total = false;

    let outcome = download_whisper_model(
        WhisperModelSize::Tiny,
        &dest_dir,
        &cancel,
        |downloaded, total| {
            last_downloaded = downloaded;
            if total > 0 {
                saw_nonzero_total = true;
            }
        },
    )
    .unwrap();

    let DownloadOutcome::Completed(path) = outcome else {
        panic!("expected Completed");
    };
    assert_eq!(path, dest_dir.join("ggml-tiny.bin"));
    assert!(path.exists());
    assert!(!dest_dir.join("ggml-tiny.bin.part").exists());
    assert!(last_downloaded > 0);
    assert!(saw_nonzero_total, "server should report Content-Length");

    let metadata = std::fs::metadata(&path).unwrap();
    assert_eq!(metadata.len(), last_downloaded);

    let _ = std::fs::remove_dir_all(&dest_dir);
}

#[test]
fn cancelling_leaves_no_partial_or_final_file() {
    let dest_dir = temp_dest_dir("cancelled");
    let _ = std::fs::remove_dir_all(&dest_dir);
    let cancel = AtomicBool::new(false);
    let mut calls = 0;

    let outcome = download_whisper_model(WhisperModelSize::Tiny, &dest_dir, &cancel, |_, _| {
        calls += 1;
        if calls >= 2 {
            cancel.store(true, Ordering::Relaxed);
        }
    })
    .unwrap();

    assert_eq!(outcome, DownloadOutcome::Cancelled);
    assert!(!dest_dir.join("ggml-tiny.bin").exists());
    assert!(!dest_dir.join("ggml-tiny.bin.part").exists());

    let _ = std::fs::remove_dir_all(&dest_dir);
}
