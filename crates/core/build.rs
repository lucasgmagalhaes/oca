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

//! Copies `espeak-ng-data` (dictionaries/phoneme rules `espeak-rs` needs at runtime, see
//! `crate::text_to_speech`) next to the workspace's build output.
//!
//! `espeak-rs-sys`'s own build script vendors and cmake-builds espeak-ng from source, copying
//! its data files into *its own* `OUT_DIR` (`target/<profile>/build/espeak-rs-sys-<hash>/out/
//! espeak-ng/espeak-ng-data`) — useless to `ui.exe` at runtime, since nothing else knows that
//! hash-suffixed path. `espeak-rs::text_to_phonemes` looks for a directory literally named
//! `espeak-ng-data` next to the running executable (among other places) — see its
//! `locate_espeak_data()`. Cargo guarantees this build script runs after `espeak-rs-sys`'s
//! (`core` depends on it transitively via `espeak-rs`), so by the time this runs, that OUT_DIR
//! is already populated — this just finds it (by globbing the shared `target/<profile>/build/`
//! directory rather than assuming a hash) and copies it to `target/<profile>/espeak-ng-data`,
//! right next to where `ui.exe` itself lands in this workspace's shared target dir.

use std::path::{Path, PathBuf};

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    let profile = std::env::var("PROFILE").expect("PROFILE not set");

    let Some(profile_dir) = find_profile_dir(&out_dir, &profile) else {
        println!(
            "cargo:warning=text_to_speech: couldn't locate the workspace's target/{profile} \
             directory from OUT_DIR ({}) — espeak-ng-data won't be copied, TTS will fail at \
             runtime until it's placed there manually.",
            out_dir.display()
        );
        return;
    };

    let Some(data_src) = find_espeak_data_source(&profile_dir) else {
        println!(
            "cargo:warning=text_to_speech: couldn't find espeak-rs-sys's built espeak-ng-data \
             under {} — has espeak-rs-sys been built yet?",
            profile_dir.join("build").display()
        );
        return;
    };

    let data_dst = profile_dir.join("espeak-ng-data");
    if data_dst.exists() {
        return; // Already copied by a previous build.
    }

    if let Err(e) = copy_dir_recursive(&data_src, &data_dst) {
        println!(
            "cargo:warning=text_to_speech: failed to copy espeak-ng-data from {} to {}: {e}",
            data_src.display(),
            data_dst.display()
        );
    }
}

/// Walks up from `out_dir` (this crate's own `OUT_DIR`, always
/// `target/<profile>/build/core-<hash>/out`) to find `target/<profile>`.
fn find_profile_dir(out_dir: &Path, profile: &str) -> Option<PathBuf> {
    let mut dir = out_dir;
    while let Some(parent) = dir.parent() {
        if parent.file_name().is_some_and(|n| n == profile) {
            return Some(parent.to_path_buf());
        }
        dir = parent;
    }
    None
}

/// Finds `espeak-rs-sys`'s built `espeak-ng-data` under `profile_dir/build/espeak-rs-sys-*/out/`
/// — the hash suffix is unpredictable, so this globs for it rather than hardcoding a path.
fn find_espeak_data_source(profile_dir: &Path) -> Option<PathBuf> {
    let build_dir = profile_dir.join("build");
    let entries = std::fs::read_dir(&build_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("espeak-rs-sys-") {
            continue;
        }
        let candidate = entry
            .path()
            .join("out")
            .join("espeak-ng")
            .join("espeak-ng-data");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}
