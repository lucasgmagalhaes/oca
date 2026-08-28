#!/usr/bin/env python3
"""Generate release license files and THIRD_PARTY_NOTICES.txt without network access."""

import argparse
import hashlib
import json
import pathlib
import re
import subprocess


DOCUMENT_PREFIXES = ("license", "copying", "notice", "copyright", "unlicense")
MAX_DOCUMENT_SIZE = 2 * 1024 * 1024
SAFE_SEGMENT = re.compile(r"[^A-Za-z0-9._+-]+")
SPDX_ID = re.compile(r"^[A-Za-z0-9.-]+$")
SHA256 = re.compile(r"^[a-f0-9]{64}$")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def safe_segment(value: str) -> str:
    segment = SAFE_SEGMENT.sub("_", value).strip("._")
    if not segment:
        raise ValueError(f"unsafe empty path segment derived from {value!r}")
    return segment


def read_document(path: pathlib.Path, root: pathlib.Path) -> bytes:
    resolved = path.resolve(strict=True)
    if not resolved.is_relative_to(root.resolve(strict=True)):
        raise ValueError(f"license document resolves outside package root: {path}")
    if resolved.stat().st_size > MAX_DOCUMENT_SIZE:
        raise ValueError(f"license document exceeds {MAX_DOCUMENT_SIZE} bytes: {path}")
    data = resolved.read_bytes()
    return data


def is_license_document(path: pathlib.Path) -> bool:
    name = path.name.lower()
    return any(name.startswith(prefix) for prefix in DOCUMENT_PREFIXES)


def package_documents(package: dict) -> list[tuple[pathlib.Path, bytes]]:
    root = pathlib.Path(package["manifest_path"]).parent.resolve(strict=True)
    candidates = {path for path in root.rglob("*") if path.is_file() and is_license_document(path)}
    license_file = package.get("license_file")
    if license_file:
        candidate = pathlib.Path(license_file)
        candidates.add(candidate if candidate.is_absolute() else root / candidate)
    return [(path.relative_to(root), read_document(path, root)) for path in sorted(candidates)]


def load_spdx(spdx_root: pathlib.Path) -> dict[str, bytes]:
    manifest = json.loads((spdx_root.parent / "spdx-manifest.json").read_text(encoding="utf-8"))
    if manifest.get("schema_version") != 1:
        raise ValueError("unsupported SPDX manifest schema")
    result = {}
    for identifier, expected_hash in manifest["files"].items():
        if not SPDX_ID.fullmatch(identifier) or not SHA256.fullmatch(expected_hash):
            raise ValueError(f"invalid SPDX manifest entry: {identifier}")
        data = (spdx_root / f"{identifier}.txt").read_bytes()
        if sha256(data) != expected_hash:
            raise ValueError(f"SPDX text checksum mismatch: {identifier}")
        result[identifier] = data
    return result


def component_documents(repo: pathlib.Path) -> list[tuple[str, pathlib.Path, bytes]]:
    license_root = repo / "packaging" / "licenses"
    manifest = json.loads(
        (license_root / "components-manifest.json").read_text(encoding="utf-8")
    )
    if manifest.get("schema_version") != 1:
        raise ValueError("unsupported component license manifest schema")
    documents = []
    for component in manifest["components"]:
        owner = f'{component["name"]}-{component["version"]}'
        for relative_text, expected_hash in component["files"].items():
            relative = pathlib.PurePosixPath(relative_text)
            if (
                relative.is_absolute()
                or ".." in relative.parts
                or not SHA256.fullmatch(expected_hash)
            ):
                raise ValueError(f"invalid component license manifest path: {relative_text}")
            path = license_root / "components" / pathlib.Path(*relative.parts)
            data = read_document(path, license_root / "components")
            if sha256(data) != expected_hash:
                raise ValueError(f"component license checksum mismatch: {relative_text}")
            documents.append((owner, pathlib.Path(*relative.parts), data))
    return documents


def expression_ids(expression: str, known: set[str]) -> set[str]:
    found = {
        identifier
        for identifier in known
        if re.search(
            rf"(?<![A-Za-z0-9.-]){re.escape(identifier)}(?![A-Za-z0-9.-])",
            expression,
        )
    }
    if not found:
        raise ValueError(f"license expression has no vendored full text: {expression}")
    return found


def cargo_metadata(repo: pathlib.Path, metadata_file: pathlib.Path | None) -> dict:
    if metadata_file:
        return json.loads(metadata_file.read_text(encoding="utf-8"))
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--offline"],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def copy_document(
    output: pathlib.Path,
    owner: str,
    relative: pathlib.Path,
    data: bytes,
) -> pathlib.Path:
    owner_dir = output / "packages" / safe_segment(owner)
    parts = [safe_segment(part) for part in relative.parts]
    target = owner_dir.joinpath(*parts)
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.exists() and target.read_bytes() != data:
        target = target.with_name(f"{target.stem}-{sha256(data)[:12]}{target.suffix}")
    target.write_bytes(data)
    return target.relative_to(output)


def add_document_group(
    document_groups: dict[str, dict],
    owner: str,
    relative: pathlib.Path,
    data: bytes,
) -> None:
    try:
        if b"\0" in data:
            raise UnicodeDecodeError("utf-8", data, 0, 1, "embedded NUL")
        data.decode("utf-8")
    except UnicodeDecodeError:
        return
    group = document_groups.setdefault(
        sha256(data), {"data": data, "owners": [], "paths": []}
    )
    group["owners"].append(owner)
    group["paths"].append(f"{owner}/{relative.as_posix()}")


def generate(repo: pathlib.Path, output: pathlib.Path, metadata_file: pathlib.Path | None = None) -> None:
    repo = repo.resolve(strict=True)
    spdx_root = repo / "packaging" / "licenses" / "spdx"
    spdx = load_spdx(spdx_root)
    metadata = cargo_metadata(repo, metadata_file)
    manifest = json.loads(
        (repo / "packaging" / "bundle-manifest.json").read_text(encoding="utf-8")
    )

    if output.exists() and any(output.iterdir()):
        raise ValueError(f"license output directory must be empty: {output}")
    (output / "spdx").mkdir(parents=True)
    for identifier, data in sorted(spdx.items()):
        (output / "spdx" / f"{identifier}.txt").write_bytes(data)

    sections = [
        "OCA THIRD-PARTY NOTICES",
        "",
        "Generated from Cargo metadata, the release bundle manifest, and vendored SPDX texts.",
        "Package-specific LICENSE, COPYING, COPYRIGHT, UNLICENSE, and NOTICE documents are",
        "preserved under packages/. Canonical full license texts are preserved under spdx/.",
        "",
    ]
    required_ids: set[str] = set()
    document_groups: dict[str, dict] = {}
    package_count = 0
    for package in sorted(
        metadata["packages"],
        key=lambda item: (item["name"], item["version"], item["id"]),
    ):
        if package["id"].startswith("path+"):
            continue
        package_count += 1
        owner = f'{package["name"]}-{package["version"]}'
        expression = package.get("license")
        if not expression:
            raise ValueError(f"Cargo package has no license metadata: {owner}")
        required_ids.update(expression_ids(expression, set(spdx)))
        documents = package_documents(package)
        copied = []
        for relative, data in documents:
            copied.append(copy_document(output, owner, relative, data).as_posix())
            add_document_group(document_groups, owner, relative, data)
        sections.extend([f"Cargo package: {owner}", f"License expression: {expression}"])
        sections.append(
            "Preserved documents: "
            + (", ".join(copied) if copied else "none published in crate archive")
        )
        sections.append("")

    python_runtime = repo / "vendor" / "python-runtime"
    if python_runtime.is_dir():
        owner = "bundled-python-runtime"
        copied = []
        for path in sorted(python_runtime.rglob("*")):
            if not path.is_file() or not is_license_document(path):
                continue
            relative = path.relative_to(python_runtime)
            data = read_document(path, python_runtime)
            copied.append(copy_document(output, owner, relative, data).as_posix())
            add_document_group(document_groups, owner, relative, data)
        sections.extend(
            [
                "Bundled component files: private CPython runtime and Python packages",
                "Preserved documents: " + ", ".join(copied),
                "",
            ]
        )

    sections.extend(["PINNED NATIVE COMPONENT LICENSE FILES", ""])
    for owner, relative, data in component_documents(repo):
        copied = copy_document(output, owner, relative, data).as_posix()
        add_document_group(document_groups, owner, relative, data)
        sections.extend(
            [
                f"Native component: {owner}",
                f"Preserved document: {copied}",
                "",
            ]
        )

    sections.extend(["DISTRIBUTED NATIVE AND MODEL COMPONENTS", ""])
    components = manifest["models"] + [
        item
        for item in manifest["native_dependencies"]
        if "build-time only" not in item["bundle"]
    ]
    for component in components:
        expression = component["license"]
        required_ids.update(expression_ids(expression, set(spdx)))
        sections.extend(
            [
                f'Component: {component["name"]}',
                f'Version: {component.get("version", "artifact pinned by SHA-256")}',
                f"License expression: {expression}",
                "",
            ]
        )

    sections.extend(["FULL SPDX LICENSE TEXTS", ""])
    for identifier in sorted(required_ids):
        sections.extend([f"===== {identifier} =====", spdx[identifier].decode("utf-8").rstrip(), ""])
    sections.extend(["PACKAGE-SPECIFIC LICENSE AND NOTICE DOCUMENTS", ""])
    for digest, group in sorted(document_groups.items()):
        sections.extend(
            [
                f"===== Document SHA-256: {digest} =====",
                "Packages: " + ", ".join(sorted(set(group["owners"]))),
                "Source paths: " + ", ".join(sorted(set(group["paths"]))),
                "",
                group["data"].decode("utf-8").rstrip(),
                "",
            ]
        )
    output.mkdir(parents=True, exist_ok=True)
    (output / "THIRD_PARTY_NOTICES.txt").write_text(
        "\n".join(sections).rstrip() + "\n", encoding="utf-8"
    )
    print(
        f"generated notices for {package_count} Cargo packages and "
        f"{len(components)} bundled components"
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=pathlib.Path)
    parser.add_argument("--repo", type=pathlib.Path, default=pathlib.Path(__file__).parents[1])
    parser.add_argument("--metadata-file", type=pathlib.Path)
    args = parser.parse_args()
    generate(args.repo, args.output, args.metadata_file)


if __name__ == "__main__":
    main()
