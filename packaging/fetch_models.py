#!/usr/bin/env python3
"""Fetch pinned model assets into a bundle and verify every SHA-256 digest."""

import argparse
import hashlib
import json
import pathlib
import shutil
import tempfile
import urllib.request


def digest(path: pathlib.Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("destination", type=pathlib.Path)
    parser.add_argument(
        "--manifest",
        type=pathlib.Path,
        default=pathlib.Path(__file__).with_name("bundle-manifest.json"),
    )
    args = parser.parse_args()
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))

    for model in manifest["models"]:
        output = args.destination / model["path"]
        output.parent.mkdir(parents=True, exist_ok=True)
        if output.is_file() and digest(output) == model["sha256"]:
            print(f"verified cached {model['name']}")
            continue
        with tempfile.NamedTemporaryFile(dir=output.parent, delete=False) as temporary:
            temporary_path = pathlib.Path(temporary.name)
            request = urllib.request.Request(model["url"], headers={"User-Agent": "oca-release-builder/1"})
            with urllib.request.urlopen(request) as response:
                shutil.copyfileobj(response, temporary, length=1024 * 1024)
        actual = digest(temporary_path)
        if actual != model["sha256"]:
            temporary_path.unlink(missing_ok=True)
            raise SystemExit(
                f"SHA-256 mismatch for {model['name']}: expected {model['sha256']}, got {actual}"
            )
        temporary_path.replace(output)
        print(f"downloaded and verified {model['name']}")


if __name__ == "__main__":
    main()
