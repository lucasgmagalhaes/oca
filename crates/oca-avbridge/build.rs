//! Locates FFmpeg (libavformat/libavcodec/libavutil) via the `FFMPEG_DIR` env var — the
//! convention used by BtbN's Windows shared builds (`FFMPEG_DIR/include`, `FFMPEG_DIR/lib`) —
//! and compiles `csrc/bridge.c` against it.

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=FFMPEG_DIR");
    println!("cargo:rerun-if-changed=csrc/bridge.c");
    println!("cargo:rerun-if-changed=csrc/bridge.h");

    let ffmpeg_dir = env::var("FFMPEG_DIR").unwrap_or_else(|_| {
        panic!(
            "FFMPEG_DIR is not set. Point it at an FFmpeg dev build with `include/` and \
             `lib/` subdirectories (e.g. a BtbN shared build) — oca-avbridge links against \
             libavformat/libavcodec/libavutil and can't find their headers/libs without it."
        )
    });
    let ffmpeg_dir = PathBuf::from(ffmpeg_dir);
    let include_dir = ffmpeg_dir.join("include");
    let lib_dir = ffmpeg_dir.join("lib");

    if !include_dir.is_dir() || !lib_dir.is_dir() {
        panic!(
            "FFMPEG_DIR={} doesn't look like an FFmpeg dev build: expected {} and {} to both \
             be directories.",
            ffmpeg_dir.display(),
            include_dir.display(),
            lib_dir.display()
        );
    }

    cc::Build::new()
        .file("csrc/bridge.c")
        .include(&include_dir)
        .compile("oca_avbridge_c");

    // Copy the exact import libs into OUR OUT_DIR under unique names, then link against
    // those — not `-l<name>` + `-L<lib_dir>` (bare-name search across every `-L` path on the
    // link line). GStreamer's SDK bundles its own FFmpeg build (gst-libav) with generically
    // named import libs (avformat.lib, etc.) for a different, older FFmpeg version
    // (avformat-61.dll vs the avformat-62.dll here). If gstreamer-sys's `-L` also ends up on
    // the link line, a bare `-lavformat` can resolve to GStreamer's mismatched copy instead
    // of this one — an ABI mismatch that doesn't error at build time, just corrupts behavior
    // at runtime (see features/fase1/commit_plan.md, chore-002).
    //
    // `cargo:rustc-link-arg` (a full path) would sidestep this too, but it does NOT
    // propagate from a library crate's build script to a downstream binary's link step —
    // only `cargo:rustc-link-lib`/`cargo:rustc-link-search` do. So instead: rename the file
    // (uniquely prefixed, can't collide with anything another crate's search path might
    // contain) and let bare-name search find it unambiguously.
    let is_msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR set by cargo"));
    for name in ["avfilter", "avformat", "avcodec", "avutil"] {
        let (src_name, unique_name) = if is_msvc {
            (format!("{name}.lib"), format!("oca_avbridge_ffmpeg_{name}.lib"))
        } else {
            (format!("lib{name}.dll.a"), format!("liboca_avbridge_ffmpeg_{name}.dll.a"))
        };
        let src_path = lib_dir.join(&src_name);
        if !src_path.is_file() {
            panic!(
                "expected {} to exist under FFMPEG_DIR/lib ({})",
                src_name,
                lib_dir.display()
            );
        }
        std::fs::copy(&src_path, out_dir.join(&unique_name))
            .unwrap_or_else(|e| panic!("failed to copy {} into OUT_DIR: {e}", src_path.display()));
        println!("cargo:rustc-link-lib=dylib=oca_avbridge_ffmpeg_{name}");
    }
    println!("cargo:rustc-link-search=native={}", out_dir.display());
}
