#!/usr/bin/env bash
# Convert YouTube URL to MP3 via yt-dlp.
set -euo pipefail

if [ $# -lt 1 ]; then
    echo "Usage: $0 <youtube-url> [output-dir]" >&2
    exit 1
fi

URL="$1"
OUTDIR="${2:-.}"

command -v yt-dlp >/dev/null 2>&1 || { echo "yt-dlp not found. Install: pip install yt-dlp" >&2; exit 1; }
command -v ffmpeg >/dev/null 2>&1 || { echo "ffmpeg not found in PATH." >&2; exit 1; }

mkdir -p "$OUTDIR"

yt-dlp -x --audio-format mp3 --audio-quality 0 \
    -o "$OUTDIR/%(title)s.%(ext)s" \
    "$URL"
