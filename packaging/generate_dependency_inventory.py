#!/usr/bin/env python3
"""Generate the exact Cargo plus native/model dependency inventory shipped with a bundle."""

import argparse
import hashlib
import json
import pathlib
import tomllib


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=pathlib.Path)
    parser.add_argument("--repo", type=pathlib.Path, default=pathlib.Path(__file__).parents[1])
    parser.add_argument("--bundle-root", type=pathlib.Path)
    args = parser.parse_args()

    lock = tomllib.loads((args.repo / "Cargo.lock").read_text(encoding="utf-8"))
    manifest = json.loads(
        (args.repo / "packaging" / "bundle-manifest.json").read_text(encoding="utf-8")
    )
    rust_packages = [
        {
            key: package[key]
            for key in ("name", "version", "source", "checksum")
            if key in package
        }
        for package in lock["package"]
    ]
    inventory = {
        "schema_version": 1,
        "rust_packages": sorted(rust_packages, key=lambda item: (item["name"], item["version"])),
        "models": manifest["models"],
        "native_dependencies": manifest["native_dependencies"],
        "system_contract": manifest["system_contract"],
    }
    if args.bundle_root:
        bundle_root = args.bundle_root.resolve()
        output = args.output.resolve()
        inventory["bundled_files"] = [
            {
                "path": path.relative_to(bundle_root).as_posix(),
                "size": path.stat().st_size,
                "sha256": sha256(path),
            }
            for path in sorted(bundle_root.rglob("*"))
            if path.is_file() and path.resolve() != output
        ]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(inventory, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {len(rust_packages)} Rust packages to {args.output}")


if __name__ == "__main__":
    main()
