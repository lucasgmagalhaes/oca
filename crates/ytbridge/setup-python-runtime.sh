#!/usr/bin/env bash
# One-time setup: fetches a self-contained Python runtime (python-build-standalone) and
# installs yt-dlp, its EJS scripts and Deno, so `cargo build -p ytbridge` ships every runtime
# needed for YouTube extraction. Run once after cloning, and again
# whenever bumping PYTHON_VERSION/RELEASE_TAG below. Linux/macOS twin of
# setup-python-runtime.ps1 — see ytbridge/build.rs for the runtime layouts.
#
# PYO3_PYTHON is selected by the Makefile when this runtime exists. For direct Cargo builds,
# export it yourself before building:
#   export PYO3_PYTHON=$(pwd)/vendor/python-runtime/bin/python3.10

set -euo pipefail

PYTHON_VERSION="3.10.21"
RELEASE_TAG="20260814"
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)
        RUNTIME_PLATFORM="x86_64-unknown-linux-gnu"
        PYTHON_SHA256="391e2bbe4da892fd7dd9f773f42ad8eae82f33d3d4fc8f0025af80b4dfa134b3"
        DENO_PLATFORM="x86_64-unknown-linux-gnu"
        DENO_SHA256="8b010a3b1a4a0188a67cdb8a7a27348b2a501af78aec7fc74f2ace167368d530"
        ;;
    Darwin-arm64)
        RUNTIME_PLATFORM="aarch64-apple-darwin"
        PYTHON_SHA256="935e7112b2a567388d2f9d99eec2eb9957a7b6098248f10890073dc870537514"
        DENO_PLATFORM="aarch64-apple-darwin"
        DENO_SHA256="b796aadd131f6930560c1ee040cf0d6f53933fbb987464e9ff46bd7ea4830615"
        ;;
    Darwin-x86_64)
        RUNTIME_PLATFORM="x86_64-apple-darwin"
        PYTHON_SHA256="1f2b5c7fd75ccb6bbb02078562c148470aea61fdea41442f0199ea9cbdad1a8d"
        DENO_PLATFORM="x86_64-apple-darwin"
        DENO_SHA256="c1b8b89a81e91b2a8b3f96def3195d08cfe3a105651da7908d53061f7140510d"
        ;;
    *)
        echo "Unsupported Python runtime platform: $(uname -s)-$(uname -m)" >&2
        exit 1
        ;;
esac
ASSET_NAME="cpython-${PYTHON_VERSION}+${RELEASE_TAG}-${RUNTIME_PLATFORM}-install_only.tar.gz"
URL="https://github.com/astral-sh/python-build-standalone/releases/download/${RELEASE_TAG}/${ASSET_NAME}"
YTDLP_VERSION="2026.07.04"
YTDLP_ASSET="yt-dlp.tar.gz"
YTDLP_URL="https://github.com/yt-dlp/yt-dlp/releases/download/${YTDLP_VERSION}/${YTDLP_ASSET}"
YTDLP_SHA256="31c32457d1a573a341bb0929386c624fe47339a5338829e6e9c9454bdfa7397a"
EJS_VERSION="0.8.0"
EJS_ASSET="yt_dlp_ejs-${EJS_VERSION}-py3-none-any.whl"
EJS_URL="https://github.com/yt-dlp/ejs/releases/download/${EJS_VERSION}/${EJS_ASSET}"
EJS_SHA256="79300e5fca7f937a1eeede11f0456862c1b41107ce1d726871e0207424f4bdb4"
DENO_VERSION="2.9.5"
DENO_ASSET="deno-${DENO_PLATFORM}.zip"
DENO_URL="https://github.com/denoland/deno/releases/download/v${DENO_VERSION}/${DENO_ASSET}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VENDOR_DIR="${REPO_ROOT}/vendor"
RUNTIME_DIR="${VENDOR_DIR}/python-runtime"
ARCHIVE_PATH="${VENDOR_DIR}/${ASSET_NAME}"
YTDLP_PATH="${VENDOR_DIR}/${YTDLP_ASSET}"
EJS_PATH="${VENDOR_DIR}/${EJS_ASSET}"
DENO_PATH="${VENDOR_DIR}/${DENO_ASSET}"
MARKER_PATH="${RUNTIME_DIR}/.oca-runtime-versions"
EXPECTED_MARKER="platform=${RUNTIME_PLATFORM}
python=${PYTHON_VERSION}+${RELEASE_TAG}
yt-dlp=${YTDLP_VERSION}
yt-dlp-ejs=${EJS_VERSION}
deno=${DENO_VERSION}"

verify_sha256() {
    expected="$1"
    file="$2"
    actual="$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest())' "${file}")"
    [ "${actual}" = "${expected}" ]
}

if [ -f "${MARKER_PATH}" ] && [ "$(cat "${MARKER_PATH}")" = "${EXPECTED_MARKER}" ]; then
    echo "vendor/python-runtime already contains every pinned dependency."
    exit 0
fi

if [ -d "${RUNTIME_DIR}" ] && {
    [ ! -f "${MARKER_PATH}" ] || ! grep -qx "platform=${RUNTIME_PLATFORM}" "${MARKER_PATH}";
}; then
    echo "vendor/python-runtime belongs to another or unknown platform; delete it and run again." >&2
    exit 1
fi

mkdir -p "${VENDOR_DIR}"

if [ ! -d "${RUNTIME_DIR}" ]; then
    echo "Downloading ${ASSET_NAME}..."
    curl -fL -o "${ARCHIVE_PATH}" "${URL}"
    verify_sha256 "${PYTHON_SHA256}" "${ARCHIVE_PATH}" || {
        echo "Python runtime SHA-256 mismatch" >&2
        rm -f "${ARCHIVE_PATH}"
        exit 1
    }

    echo "Extracting Python..."
    tar -xzf "${ARCHIVE_PATH}" -C "${VENDOR_DIR}"
    rm "${ARCHIVE_PATH}"
    # python-build-standalone's tarball extracts to a top-level "python/" directory.
    mv "${VENDOR_DIR}/python" "${RUNTIME_DIR}"
elif [ "$("${RUNTIME_DIR}/bin/python3.10" --version 2>&1)" != "Python ${PYTHON_VERSION}" ]; then
    echo "vendor/python-runtime has a different Python version; delete it and run again." >&2
    exit 1
fi

echo "Downloading and verifying yt-dlp ${YTDLP_VERSION}..."
curl -fL -o "${YTDLP_PATH}" "${YTDLP_URL}"
verify_sha256 "${YTDLP_SHA256}" "${YTDLP_PATH}" || {
    echo "yt-dlp SHA-256 mismatch" >&2
    rm -f "${YTDLP_PATH}"
    exit 1
}
YTDLP_EXTRACT_DIR="${VENDOR_DIR}/yt-dlp-extracted"
rm -rf -- "${YTDLP_EXTRACT_DIR}"
mkdir -p "${YTDLP_EXTRACT_DIR}" "${RUNTIME_DIR}/lib/python3.10/site-packages"
tar -xzf "${YTDLP_PATH}" -C "${YTDLP_EXTRACT_DIR}"
rm -rf -- "${RUNTIME_DIR}/lib/python3.10/site-packages/yt_dlp"
cp -a "${YTDLP_EXTRACT_DIR}/yt-dlp/yt_dlp" "${RUNTIME_DIR}/lib/python3.10/site-packages/"
rm -rf -- "${YTDLP_EXTRACT_DIR}"
rm "${YTDLP_PATH}"

echo "Downloading and verifying yt-dlp EJS scripts ${EJS_VERSION}..."
curl -fL -o "${EJS_PATH}" "${EJS_URL}"
verify_sha256 "${EJS_SHA256}" "${EJS_PATH}" || {
    echo "yt-dlp EJS SHA-256 mismatch" >&2
    rm -f "${EJS_PATH}"
    exit 1
}
rm -rf -- "${RUNTIME_DIR}/lib/python3.10/site-packages/yt_dlp_ejs" \
    "${RUNTIME_DIR}/lib/python3.10/site-packages/yt_dlp_ejs-${EJS_VERSION}.dist-info"
"${RUNTIME_DIR}/bin/python3.10" -m zipfile -e "${EJS_PATH}" \
    "${RUNTIME_DIR}/lib/python3.10/site-packages"
rm "${EJS_PATH}"

echo "Downloading and verifying Deno ${DENO_VERSION}..."
curl -fL -o "${DENO_PATH}" "${DENO_URL}"
verify_sha256 "${DENO_SHA256}" "${DENO_PATH}" || {
    echo "Deno SHA-256 mismatch" >&2
    rm -f "${DENO_PATH}"
    exit 1
}
rm -rf -- "${RUNTIME_DIR}/tools"
mkdir -p "${RUNTIME_DIR}/tools"
"${RUNTIME_DIR}/bin/python3.10" -m zipfile -e "${DENO_PATH}" "${RUNTIME_DIR}/tools"
chmod +x "${RUNTIME_DIR}/tools/deno"
rm "${DENO_PATH}"
printf '%s\n' "${EXPECTED_MARKER}" > "${MARKER_PATH}"

echo "Done. vendor/python-runtime includes CPython, yt-dlp, EJS scripts and Deno."
echo "Export PYO3_PYTHON=${RUNTIME_DIR}/bin/python3.10 before 'cargo build -p ytbridge' —"
echo "or use the Makefile, which selects this runtime automatically."
