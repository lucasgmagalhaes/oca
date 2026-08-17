//! Copies the runtime pieces of the vendored `python-build-standalone` distribution
//! (`vendor/python-runtime/` at the workspace root — fetched by `setup-python-runtime.ps1`,
//! not checked in) next to `ytbridge`'s own build output: `python310.dll` (what `pyo3` linked
//! against — pinned via `.cargo/config.toml`'s `PYO3_PYTHON`), `DLLs/` (compiled stdlib C
//! extension modules — `_socket`, `_ssl`, etc.; without these, `import socket` and anything
//! built on it, including `yt_dlp` itself, fails with `ModuleNotFoundError`), and `Lib/`
//! (pure-Python stdlib plus `Lib/site-packages/yt_dlp`, pip-installed into the vendored
//! runtime by the setup script). Without this, `cargo build`'s `ytbridge.exe` would only run
//! on the machine that built it, and only by accident (whatever Python happened to be on that
//! machine's `PATH` at the time) — this makes a normal `cargo build -p ytbridge` produce a
//! fully self-contained, runnable binary with no separate Python install needed, confirmed by
//! running it with the system `PATH` stripped of every Python installation.
//!
//! Windows-only for now (`python-build-standalone`'s Linux/macOS layout differs enough — a
//! `libpythonX.Y.so`/`.dylib` plus `lib-dynload/` instead of `DLLs/`, no `.dll`/`.pyd`
//! extensions — that this would need real adaptation, not just a path rename, and Fase 8's
//! Linux packaging hasn't started at all yet per `CLAUDE.md`).

use std::path::{Path, PathBuf};

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("crates/ytbridge is two levels under the workspace root");
    let runtime_dir = workspace_root.join("vendor").join("python-runtime");
    if !runtime_dir.is_dir() {
        println!(
            "cargo:warning=vendor/python-runtime not found — run \
             crates/ytbridge/setup-python-runtime.ps1 first, or ytbridge.exe won't run \
             standalone (it'll fall back to whatever Python is on PATH, if any)"
        );
        return;
    }

    // OUT_DIR is target/<profile>/build/ytbridge-<hash>/out — three levels up is
    // target/<profile>/, where cargo actually places the built ytbridge.exe.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let target_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR is nested three levels under target/<profile>/")
        .to_path_buf();

    copy_file(
        &runtime_dir.join("python310.dll"),
        &target_dir.join("python310.dll"),
    );
    copy_dir_all(&runtime_dir.join("DLLs"), &target_dir.join("DLLs"));
    copy_dir_all(&runtime_dir.join("Lib"), &target_dir.join("Lib"));

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
