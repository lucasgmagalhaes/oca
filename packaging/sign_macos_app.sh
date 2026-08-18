#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "usage: $0 APP" >&2
    exit 2
fi

APP="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
IDENTITY="${OCA_CODESIGN_IDENTITY:--}"
MAIN="$APP/Contents/MacOS/oca"

sign_quietly() {
    output="$(codesign "$@" 2>&1)" || {
        printf '%s\n' "$output" >&2
        return 1
    }
}

chmod -R u+w "$APP"
xattr -cr "$APP"
while IFS= read -r -d '' candidate; do
    if [ "$candidate" = "$MAIN" ] || ! file -b "$candidate" | grep -q 'Mach-O'; then
        continue
    fi
    if [ "$IDENTITY" = "-" ]; then
        sign_quietly --force --sign - "$candidate"
    else
        sign_quietly --force --options runtime --timestamp --sign "$IDENTITY" \
            --preserve-metadata=entitlements "$candidate"
    fi
done < <(find "$APP" -type f -print0)

if [ "$IDENTITY" = "-" ]; then
    sign_quietly --force --sign - "$APP"
else
    sign_quietly --force --options runtime --timestamp --sign "$IDENTITY" "$APP"
fi
codesign --verify --deep --strict "$APP"
