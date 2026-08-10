//! `oca-avbridge` — thin C bridge over libavformat/libavcodec/libavutil (FFmpeg), called via
//! FFI. Replaces spawning `ffprobe`/`ffmpeg` as external processes: `nivela-core` links this
//! crate directly, so probing and encoding never shell out or parse subprocess stdout.
//!
//! The C surface (`csrc/bridge.c`) is written for this project — not a full auto-generated
//! FFmpeg binding — so it stays small and auditable. See [`build.rs`](../build.rs) for how the
//! FFmpeg dev libs (`FFMPEG_DIR`) are located and linked.

unsafe extern "C" {
    fn oca_avbridge_version() -> u32;
}

/// libavformat's packed version number (same encoding as `LIBAVFORMAT_VERSION_INT` /
/// `avformat_version()`): `(major << 16) | (minor << 8) | micro`.
pub fn avbridge_version() -> u32 {
    // SAFETY: oca_avbridge_version() takes no arguments, returns a plain u32, and has no
    // documented failure mode in libavformat — it's a pure accessor.
    unsafe { oca_avbridge_version() }
}
