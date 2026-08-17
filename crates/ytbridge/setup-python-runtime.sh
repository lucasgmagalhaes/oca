#!/usr/bin/env bash
# One-time setup: fetches a self-contained Python runtime (python-build-standalone) and
# installs yt-dlp into it, so `cargo build -p ytbridge` links against and ships a Python that
# needs nothing pre-installed on the machine it runs on. Run once after cloning, and again
# whenever bumping PYTHON_VERSION/RELEASE_TAG below. Linux twin of setup-python-runtime.ps1 —
# see ytbridge/build.rs's doc comment for how the resulting layout differs from Windows'.
#
# PYO3_PYTHON isn't pinned for Linux the way .cargo/config.toml pins it for Windows (that file
# is a single global value, no per-target override) — export it yourself before building:
#   export PYO3_PYTHON=$(pwd)/vendor/python-runtime/bin/python3.10

set -euo pipefail

PYTHON_VERSION="3.10.21"
RELEASE_TAG="20260814"
ASSET_NAME="cpython-${PYTHON_VERSION}+${RELEASE_TAG}-x86_64-unknown-linux-gnu-install_only.tar.gz"
URL="https://github.com/astral-sh/python-build-standalone/releases/download/${RELEASE_TAG}/${ASSET_NAME}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VENDOR_DIR="${REPO_ROOT}/vendor"
RUNTIME_DIR="${VENDOR_DIR}/python-runtime"
ARCHIVE_PATH="${VENDOR_DIR}/${ASSET_NAME}"

if [ -d "${RUNTIME_DIR}" ]; then
    echo "vendor/python-runtime already exists — delete it first to re-fetch."
    exit 0
fi

mkdir -p "${VENDOR_DIR}"

echo "Downloading ${ASSET_NAME}..."
curl -fL -o "${ARCHIVE_PATH}" "${URL}"

echo "Extracting..."
tar -xzf "${ARCHIVE_PATH}" -C "${VENDOR_DIR}"
rm "${ARCHIVE_PATH}"
# python-build-standalone's tarball extracts to a top-level "python/" directory.
mv "${VENDOR_DIR}/python" "${RUNTIME_DIR}"

echo "Installing yt-dlp into the vendored runtime..."
"${RUNTIME_DIR}/bin/python3.10" -m pip install --no-warn-script-location yt-dlp

echo "Done. vendor/python-runtime is ready."
echo "Export PYO3_PYTHON=${RUNTIME_DIR}/bin/python3.10 before 'cargo build -p ytbridge' —"
echo "unlike Windows, this isn't pinned in .cargo/config.toml."
