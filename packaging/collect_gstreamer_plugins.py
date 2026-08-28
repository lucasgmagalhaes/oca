#!/usr/bin/env python3
"""Copy only the GStreamer plugins declared by Oca's release allowlist."""

import argparse
import json
import pathlib
import re
import shutil


PLUGIN_ID = re.compile(r"^[a-z0-9_-]+$")
PLUGIN_FILE = re.compile(r"^(?:lib)?gst([a-z0-9_-]+)\.(?:dylib|dll|so(?:\.[0-9]+)*)$")


def plugin_id(path: pathlib.Path) -> str | None:
    match = PLUGIN_FILE.fullmatch(path.name.lower())
    return match.group(1) if match else None


def load_allowlist(path: pathlib.Path, platform: str) -> tuple[set[str], list[list[str]]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if data.get("schema_version") != 1:
        raise ValueError("unsupported GStreamer allowlist schema")
    common = set(data.get("common", []))
    one_of = data.get("platforms", {}).get(platform, {}).get("one_of", [])
    identifiers = common | {item for group in one_of for item in group}
    invalid = sorted(item for item in identifiers if not PLUGIN_ID.fullmatch(item))
    if invalid:
        raise ValueError(f"invalid GStreamer plugin identifiers: {', '.join(invalid)}")
    return common, one_of


def collect(
    source: pathlib.Path,
    destination: pathlib.Path,
    allowlist: pathlib.Path,
    platform: str,
) -> list[pathlib.Path]:
    source = source.resolve(strict=True)
    if not source.is_dir():
        raise ValueError(f"GStreamer plugin source is not a directory: {source}")
    common, one_of = load_allowlist(allowlist, platform)
    requested = common | {item for group in one_of for item in group}
    found: dict[str, pathlib.Path] = {}
    for candidate in sorted(source.iterdir()):
        identifier = plugin_id(candidate)
        if identifier not in requested or not candidate.is_file():
            continue
        resolved = candidate.resolve(strict=True)
        if not resolved.is_relative_to(source):
            raise ValueError(f"plugin resolves outside source directory: {candidate}")
        found.setdefault(identifier, candidate)

    missing = sorted(common - found.keys())
    for group in one_of:
        if not set(group) & found.keys():
            missing.append("one of " + ", ".join(group))
    if missing:
        raise FileNotFoundError("missing required GStreamer plugins: " + "; ".join(missing))

    destination.mkdir(parents=True, exist_ok=True)
    if any(destination.iterdir()):
        raise ValueError(f"GStreamer plugin destination must be empty: {destination}")
    copied = []
    for identifier, candidate in sorted(found.items()):
        target = destination / candidate.name
        shutil.copy2(candidate.resolve(), target)
        copied.append(target)
    return copied


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=pathlib.Path)
    parser.add_argument("destination", type=pathlib.Path)
    parser.add_argument("--platform", required=True, choices=("linux", "macos", "windows"))
    parser.add_argument(
        "--allowlist",
        type=pathlib.Path,
        default=pathlib.Path(__file__).with_name("gstreamer-plugin-allowlist.json"),
    )
    args = parser.parse_args()
    copied = collect(args.source, args.destination, args.allowlist, args.platform)
    print(f"copied {len(copied)} allowlisted GStreamer plugins to {args.destination}")


if __name__ == "__main__":
    main()
