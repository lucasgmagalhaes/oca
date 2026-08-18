#!/usr/bin/env python3
"""Copy non-system Mach-O dependencies into an app and rewrite their load paths."""

import argparse
import hashlib
import os
import pathlib
import shutil
import subprocess


SYSTEM_PREFIXES = (
    "/System/Library/",
    "/usr/lib/",
    "/Library/Apple/System/Library/",
)


def output(*command: object) -> str:
    return subprocess.check_output([str(item) for item in command], text=True)


def run_tool(*command: object) -> None:
    result = subprocess.run(
        [str(item) for item in command], capture_output=True, text=True
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(f"{' '.join(map(str, command))} failed: {detail}")


def is_macho(path: pathlib.Path) -> bool:
    if not path.is_file() or path.is_symlink():
        return False
    result = subprocess.run(
        ["file", "-b", path], capture_output=True, text=True, check=True
    )
    return "Mach-O" in result.stdout


def dependencies(path: pathlib.Path) -> list[str]:
    lines = output("otool", "-L", path).splitlines()[1:]
    return [line.strip().split(" (compatibility version", 1)[0] for line in lines]


def install_ids(path: pathlib.Path) -> set[str]:
    result = subprocess.run(["otool", "-D", path], capture_output=True, text=True)
    if result.returncode != 0:
        return set()
    return {line.strip() for line in result.stdout.splitlines()[1:] if line.strip()}


def rpaths(path: pathlib.Path) -> list[str]:
    lines = output("otool", "-l", path).splitlines()
    result: list[str] = []
    for index, line in enumerate(lines):
        if line.strip() != "cmd LC_RPATH":
            continue
        for candidate in lines[index + 1 : index + 5]:
            stripped = candidate.strip()
            if stripped.startswith("path "):
                result.append(stripped[5:].split(" (offset", 1)[0])
                break
    return result


def expand_special(path: str, binary: pathlib.Path, executable_dir: pathlib.Path) -> pathlib.Path:
    replacements = {
        "@loader_path": str(binary.parent),
        "@executable_path": str(executable_dir),
    }
    for prefix, value in replacements.items():
        if path == prefix or path.startswith(prefix + "/"):
            return pathlib.Path(value + path[len(prefix) :]).resolve()
    return pathlib.Path(path)


def build_index(roots: list[pathlib.Path]) -> dict[str, list[pathlib.Path]]:
    index: dict[str, list[pathlib.Path]] = {}
    for root in roots:
        if not root.is_dir():
            continue
        for path in sorted(root.rglob("*")):
            if path.is_file():
                index.setdefault(path.name, []).append(path.resolve())
    return index


def resolve_dependency(
    dependency: str,
    binary: pathlib.Path,
    executable_dir: pathlib.Path,
    index: dict[str, list[pathlib.Path]],
) -> pathlib.Path | None:
    if dependency.startswith(SYSTEM_PREFIXES):
        return None
    if dependency.startswith("/"):
        path = pathlib.Path(dependency)
        if not path.exists():
            raise RuntimeError(f"missing dependency {dependency} required by {binary}")
        return path.resolve()
    if dependency.startswith("@loader_path") or dependency.startswith("@executable_path"):
        path = expand_special(dependency, binary, executable_dir)
        if path.exists():
            return path.resolve()
    if dependency.startswith("@rpath/"):
        suffix = dependency.removeprefix("@rpath/")
        for runpath in rpaths(binary):
            base = expand_special(runpath, binary, executable_dir)
            candidate = base / suffix
            if candidate.exists():
                return candidate.resolve()
    candidates = index.get(pathlib.Path(dependency).name, [])
    if candidates:
        return candidates[0]
    raise RuntimeError(f"could not resolve {dependency} required by {binary}")


def digest(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def remove_external_rpaths(binary: pathlib.Path) -> None:
    for runpath in rpaths(binary):
        if runpath.startswith("/") and not runpath.startswith(SYSTEM_PREFIXES):
            run_tool("install_name_tool", "-delete_rpath", runpath, binary)


def loader_reference(binary: pathlib.Path, dependency: pathlib.Path) -> str:
    relative = os.path.relpath(dependency, binary.parent)
    return f"@loader_path/{relative}"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("app", type=pathlib.Path)
    parser.add_argument("--architecture", required=True, choices=("arm64", "x86_64"))
    parser.add_argument("--search-root", action="append", type=pathlib.Path, default=[])
    args = parser.parse_args()

    app = args.app.resolve()
    contents = app / "Contents"
    executable_dir = contents / "MacOS"
    frameworks = contents / "Frameworks"
    frameworks.mkdir(parents=True, exist_ok=True)
    search_roots = [app, *[root.resolve() for root in args.search_root]]
    index = build_index(search_roots)

    queue = sorted(path for path in app.rglob("*") if is_macho(path))
    processed: set[pathlib.Path] = set()
    copied_sources: dict[str, pathlib.Path] = {}
    while queue:
        binary = queue.pop(0).resolve()
        if binary in processed:
            continue
        processed.add(binary)
        architectures = output("lipo", "-archs", binary).split()
        if args.architecture not in architectures:
            raise RuntimeError(
                f"{binary} does not contain required {args.architecture} architecture: "
                f"{architectures}"
            )

        own_ids = install_ids(binary)
        for dependency in dependencies(binary):
            if dependency in own_ids:
                continue
            source = resolve_dependency(dependency, binary, executable_dir, index)
            if source is None:
                continue
            try:
                source.relative_to(app)
                bundled = source
            except ValueError:
                bundled = frameworks / source.name
                original = copied_sources.get(source.name)
                if original is not None and digest(original) != digest(source):
                    raise RuntimeError(
                        f"dependency basename collision for {source.name}: "
                        f"{original} and {source}"
                    )
                if not bundled.exists():
                    shutil.copy2(source, bundled)
                    copied_sources[source.name] = source
                    queue.append(bundled)
                    index.setdefault(bundled.name, []).insert(0, bundled.resolve())
            # Direct loader-relative references work for binaries at every depth in the app
            # without adding an LC_RPATH. Some python-build-standalone extension modules have
            # no spare Mach-O header space for a new load command.
            replacement = loader_reference(binary, bundled)
            if dependency != replacement:
                run_tool("install_name_tool", "-change", dependency, replacement, binary)
        remove_external_rpaths(binary)

    leaks: list[str] = []
    for binary in sorted(path for path in app.rglob("*") if is_macho(path)):
        own_ids = install_ids(binary)
        for dependency in dependencies(binary):
            if dependency in own_ids:
                continue
            if dependency.startswith("/") and not dependency.startswith(SYSTEM_PREFIXES):
                leaks.append(f"{binary}: {dependency}")
        for runpath in rpaths(binary):
            if runpath.startswith("/") and not runpath.startswith(SYSTEM_PREFIXES):
                leaks.append(f"{binary}: LC_RPATH {runpath}")
    if leaks:
        raise RuntimeError("unbundled macOS dependencies:\n" + "\n".join(leaks))
    print(f"bundled {len(processed)} Mach-O files into {app}")


if __name__ == "__main__":
    main()
