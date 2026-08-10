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

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=avformat");
    println!("cargo:rustc-link-lib=dylib=avcodec");
    println!("cargo:rustc-link-lib=dylib=avutil");
}
