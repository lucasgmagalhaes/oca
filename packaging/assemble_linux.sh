#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 3 ]; then
    echo "usage: $0 BUILD_DIR FFMPEG_DIR OUTPUT_DIR" >&2
    exit 2
fi

BUILD_DIR="$(cd "$1" && pwd)"
FFMPEG_DIR="$(cd "$2" && pwd)"
OUTPUT_DIR="$3"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

rm -rf -- "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/resources/runtime/bin" "$OUTPUT_DIR/resources/runtime/lib"
cp "$BUILD_DIR/ui" "$BUILD_DIR/ytbridge" "$BUILD_DIR/deno" "$OUTPUT_DIR/"
cp -a "$BUILD_DIR/espeak-ng-data" "$OUTPUT_DIR/"
cp -a "$BUILD_DIR/lib" "$OUTPUT_DIR/"
cp "$FFMPEG_DIR/bin/ffmpeg" "$OUTPUT_DIR/resources/runtime/bin/"
cp -a "$FFMPEG_DIR/lib/"*.so* "$OUTPUT_DIR/resources/runtime/lib/"

GST_ROOT="$OUTPUT_DIR/resources/runtime/gstreamer"
mkdir -p "$GST_ROOT/lib" "$GST_ROOT/libexec/gstreamer-1.0"
cp -a /usr/lib/x86_64-linux-gnu/gstreamer-1.0 "$GST_ROOT/lib/"
scanner="$(command -v gst-plugin-scanner || true)"
if [ -z "$scanner" ]; then
    scanner="$(find /usr/lib -path '*/gstreamer1.0/gstreamer-1.0/gst-plugin-scanner' -print -quit)"
fi
test -n "$scanner"
cp "$scanner" "$GST_ROOT/libexec/gstreamer-1.0/"

# Collect every non-glibc ELF dependency used by the app, helper, scanner and GStreamer
# plugins. A launcher sets LD_LIBRARY_PATH before starting the real binary.
while IFS= read -r library; do
    cp -Ln "$library" "$GST_ROOT/lib/" 2>/dev/null || true
done < <(
    find "$OUTPUT_DIR/ui" "$OUTPUT_DIR/ytbridge" "$OUTPUT_DIR/deno" "$scanner" \
        "$OUTPUT_DIR/resources/runtime/bin" "$OUTPUT_DIR/resources/runtime/lib" \
        /usr/lib/x86_64-linux-gnu/gstreamer-1.0 -type f -print0 \
        | xargs -0 -r ldd 2>/dev/null \
        | awk '/=> \/|^\// { for (i=1;i<=NF;i++) if ($i ~ /^\//) print $i }' \
        | grep -Ev '/(ld-linux|libc\.so|libm\.so|libpthread\.so|libdl\.so|librt\.so)' \
        | sort -u
)

mv "$OUTPUT_DIR/ui" "$OUTPUT_DIR/ui-bin"
cp "$SCRIPT_DIR/linux-launcher.sh" "$OUTPUT_DIR/ui"
chmod +x "$OUTPUT_DIR/ui" "$OUTPUT_DIR/ui-bin" "$OUTPUT_DIR/ytbridge" "$OUTPUT_DIR/deno"
python3 "$SCRIPT_DIR/fetch_models.py" "$OUTPUT_DIR/resources"
cp "$SCRIPT_DIR/bundle-manifest.json" "$OUTPUT_DIR/resources/"
mkdir -p "$OUTPUT_DIR/resources/licenses/fonts"
cp "$SCRIPT_DIR/../LICENSE" "$OUTPUT_DIR/resources/licenses/oca.txt"
find "$SCRIPT_DIR/../crates/core/assets/fonts" -name OFL.txt -print0 | while IFS= read -r -d '' license; do
    family="$(basename "$(dirname "$license")")"
    cp "$license" "$OUTPUT_DIR/resources/licenses/fonts/${family}-OFL.txt"
done
{
    "$FFMPEG_DIR/bin/ffmpeg" -version | head -1
    gst-launch-1.0 --version | head -2
    PYTHONDONTWRITEBYTECODE=1 "$BUILD_DIR/../../vendor/python-runtime/bin/python3.10" --version
    "$OUTPUT_DIR/deno" --version | head -1
    PYTHONDONTWRITEBYTECODE=1 "$BUILD_DIR/../../vendor/python-runtime/bin/python3.10" -c \
        "import importlib.metadata, yt_dlp.version; print('yt-dlp ' + yt_dlp.version.__version__ + '; yt-dlp-ejs ' + importlib.metadata.version('yt-dlp-ejs'))"
} > "$OUTPUT_DIR/resources/RUNTIME_VERSIONS.txt"
python3 "$SCRIPT_DIR/generate_dependency_inventory.py" \
    "$OUTPUT_DIR/resources/DEPENDENCIES.json" --bundle-root "$OUTPUT_DIR"
python3 "$SCRIPT_DIR/validate_bundle.py" "$OUTPUT_DIR" --platform linux
