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

//! Copies the runtime pieces of the vendored `python-build-standalone` distribution
//! (`vendor/python-runtime/` at the workspace root — fetched by `setup-python-runtime.ps1`,
//! not checked in) next to `ytbridge`'s own build output: `python310.dll` (what `pyo3` linked
//! against — selected through `PYO3_PYTHON` by the Makefile/CI), `DLLs/` (compiled stdlib C
//! extension modules — `_socket`, `_ssl`, etc.; without these, `import socket` and anything
//! built on it, including `yt_dlp` itself, fails with `ModuleNotFoundError`), and `Lib/`
//! (pure-Python stdlib plus the verified `yt_dlp` and `yt_dlp_ejs` packages), and `tools/deno`
//! (the JavaScript runtime yt-dlp uses for current YouTube extraction). Without this,
//! `cargo build`'s `ytbridge.exe` would only run
//! on the machine that built it, and only by accident (whatever Python happened to be on that
//! machine's `PATH` at the time) — this makes a normal `cargo build -p ytbridge` produce a
//! fully self-contained, runnable binary with no separate Python install needed, confirmed by
//! running it with the system `PATH` stripped of every Python installation.
//!
//! Windows and Unix (`python-build-standalone`'s `install_only` tarball layout differs between
//! the two: Windows is flat (`python.exe`, `python310.dll`, `DLLs/`, `Lib/`) while Linux nests
//! everything under `bin/`/`lib/` and links the interpreter against a shared
//! `libpython3.10.so.1.0` instead of baking the stdlib C-extensions into a separate `DLLs/`
//! folder. macOS uses the same `bin/`/`lib/` shape with `.dylib` binaries. The branches below
//! mirror `setup-python-runtime.ps1`/`.sh` respectively.

use std::path::{Path, PathBuf};

fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" && target_os != "linux" && target_os != "macos" {
        return;
    }

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("crates/ytbridge is two levels under the workspace root");
    let runtime_dir = workspace_root.join("vendor").join("python-runtime");
    if !runtime_dir.is_dir() {
        let setup_script = if target_os == "windows" {
            "crates/ytbridge/setup-python-runtime.ps1"
        } else {
            "crates/ytbridge/setup-python-runtime.sh"
        };
        println!(
            "cargo:warning=vendor/python-runtime not found — run {setup_script} first, or \
             ytbridge won't run standalone (it'll fall back to whatever Python is on PATH, \
             if any)"
        );
        return;
    }

    // OUT_DIR is target/<profile>/build/ytbridge-<hash>/out — three levels up is
    // target/<profile>/, where cargo actually places the built ytbridge binary.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let target_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR is nested three levels under target/<profile>/")
        .to_path_buf();

    if target_os == "windows" {
        copy_file(
            &runtime_dir.join("python310.dll"),
            &target_dir.join("python310.dll"),
        );
        copy_dir_all(&runtime_dir.join("DLLs"), &target_dir.join("DLLs"));
        copy_dir_all(&runtime_dir.join("Lib"), &target_dir.join("Lib"));
        copy_file(
            &runtime_dir.join("tools").join("deno.exe"),
            &target_dir.join("deno.exe"),
        );
    } else {
        // Unix install_only layouts keep the stdlib (incl. lib-dynload/'s compiled C
        // extensions) and the shared library together under lib/ — one copy covers both, unlike
        // Windows' split DLLs/ + Lib/.
        copy_dir_all(&runtime_dir.join("lib"), &target_dir.join("lib"));
        copy_file(
            &runtime_dir.join("tools").join("deno"),
            &target_dir.join("deno"),
        );
        // python-build-standalone records its build-time `/install/lib` in sysconfig. PyO3
        // consequently asks the linker for libpython there even though the extracted runtime
        // lives under vendor/python-runtime. Supply the real location explicitly.
        println!(
            "cargo:rustc-link-search=native={}",
            runtime_dir.join("lib").display()
        );
        if target_os == "linux" {
            // The copied ytbridge binary finds libpython3.10.so.1.0 in ./lib next to it.
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/lib");
        } else {
            // The first path supports target/<profile>; the second supports Oca.app, where the
            // private stdlib lives under Contents/Resources/python and libpython is bundled in
            // Contents/Frameworks by the release assembler.
            println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/lib");
            println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
        }
    }

    println!("cargo:rerun-if-changed={}", runtime_dir.display());
}

fn copy_file(src: &Path, dst: &Path) {
    if let Err(e) = std::fs::copy(src, dst) {
        println!(
            "cargo:warning=failed to copy {} to {}: {e}",
            src.display(),
            dst.display()
        );
    }
}

fn copy_dir_all(src: &Path, dst: &Path) {
    if let Err(e) = std::fs::create_dir_all(dst) {
        println!("cargo:warning=failed to create {}: {e}", dst.display());
        return;
    }
    let entries = match std::fs::read_dir(src) {
        Ok(e) => e,
        Err(e) => {
            println!("cargo:warning=failed to read {}: {e}", src.display());
            return;
        }
    };
    for entry in entries.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path);
        } else {
            // Skip re-copying a file that's already there with the same size — this directory
            // has thousands of small stdlib files, and a full copy on every rebuild (this
            // build script has no narrower rerun-if-changed granularity than "the whole vendor
            // directory") would otherwise noticeably slow down every `cargo build -p ytbridge`.
            let up_to_date = std::fs::metadata(&dst_path)
                .and_then(|dst_meta| {
                    std::fs::metadata(&src_path).map(|src_meta| (dst_meta, src_meta))
                })
                .is_ok_and(|(d, s)| d.len() == s.len());
            if !up_to_date {
                copy_file(&src_path, &dst_path);
            }
        }
    }
}
