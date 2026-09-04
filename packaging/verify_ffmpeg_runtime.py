#!/usr/bin/env python3
"""Reject an FFmpeg runtime that lacks Oca's required encoder contract."""

import argparse
import pathlib
import subprocess


REQUIRED_ENCODERS = {
    "windows": frozenset({"libopenh264"}),
    "linux": frozenset({"libopenh264", "h264_vaapi"}),
    "macos": frozenset({"libopenh264", "h264_videotoolbox"}),
}


def encoder_names(output: str) -> set[str]:
    """Return encoder names from `ffmpeg -encoders` output."""
    names: set[str] = set()
    for line in output.splitlines():
        parts = line.split()
        if len(parts) >= 2 and len(parts[0]) == 6 and set(parts[0]) <= set("VASFD."):
            names.add(parts[1])
    return names


def verify(ffmpeg: pathlib.Path, platform: str) -> None:
    result = subprocess.run(
        [str(ffmpeg), "-hide_banner", "-encoders"],
        check=True,
        capture_output=True,
        text=True,
    )
    missing = sorted(REQUIRED_ENCODERS[platform] - encoder_names(result.stdout))
    if missing:
        raise RuntimeError(
            f"FFmpeg runtime for {platform} is missing required encoder(s): "
            + ", ".join(missing)
        )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("ffmpeg", type=pathlib.Path)
    parser.add_argument("--platform", required=True, choices=sorted(REQUIRED_ENCODERS))
    args = parser.parse_args()
    if not args.ffmpeg.is_file():
        raise FileNotFoundError(f"FFmpeg executable not found: {args.ffmpeg}")
    verify(args.ffmpeg, args.platform)
    print(f"verified {args.platform} FFmpeg encoder contract: {args.ffmpeg}")


if __name__ == "__main__":
    main()
