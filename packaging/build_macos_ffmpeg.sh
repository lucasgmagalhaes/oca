#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "usage: $0 OUTPUT_DIR" >&2
    exit 2
fi

OUTPUT_DIR="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FFMPEG_VERSION="$(python3 -c 'import json, sys; print(next(x for x in json.load(open(sys.argv[1], encoding="utf-8"))["native_dependencies"] if x["name"] == "FFmpeg")["macos_source_version"])' "$REPO_ROOT/packaging/bundle-manifest.json")"
FFMPEG_URL="$(python3 -c 'import json, sys; print(next(x for x in json.load(open(sys.argv[1], encoding="utf-8"))["native_dependencies"] if x["name"] == "FFmpeg")["macos_source_url"])' "$REPO_ROOT/packaging/bundle-manifest.json")"
FFMPEG_SHA256="$(python3 -c 'import json, sys; print(next(x for x in json.load(open(sys.argv[1], encoding="utf-8"))["native_dependencies"] if x["name"] == "FFmpeg")["macos_source_sha256"])' "$REPO_ROOT/packaging/bundle-manifest.json")"

if [ -e "$OUTPUT_DIR" ] && [ -n "$(find "$OUTPUT_DIR" -mindepth 1 -maxdepth 1 -print -quit)" ]; then
    echo "OUTPUT_DIR must be empty: $OUTPUT_DIR" >&2
    exit 2
fi
mkdir -p "$OUTPUT_DIR"

OPENH264_DIR="$(brew --prefix openh264)"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf -- "$WORK_DIR"' EXIT

curl --fail --location --proto '=https' --tlsv1.2 \
    --output "$WORK_DIR/ffmpeg.tar.xz" "$FFMPEG_URL"
echo "$FFMPEG_SHA256  $WORK_DIR/ffmpeg.tar.xz" | shasum -a 256 --check
tar -xf "$WORK_DIR/ffmpeg.tar.xz" -C "$WORK_DIR"

SOURCE_DIR="$WORK_DIR/ffmpeg-$FFMPEG_VERSION"
cd "$SOURCE_DIR"
PKG_CONFIG_PATH="$OPENH264_DIR/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}" \
    ./configure \
        --prefix="$OUTPUT_DIR" \
        --enable-shared \
        --disable-static \
        --enable-libopenh264 \
        --enable-videotoolbox \
        --enable-audiotoolbox \
        --enable-pic \
        --disable-debug
make -j"$(sysctl -n hw.ncpu)" install
python3 "$SCRIPT_DIR/verify_ffmpeg_runtime.py" "$OUTPUT_DIR/bin/ffmpeg" --platform macos
