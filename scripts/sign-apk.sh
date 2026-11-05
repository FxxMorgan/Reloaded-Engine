#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 input-unsigned.apk output-signed.apk" >&2
    exit 1
fi
: "${ANDROID_HOME:?Set ANDROID_HOME to the Android SDK}"
: "${ANDROID_KEYSTORE_PATH:?Set ANDROID_KEYSTORE_PATH}"
: "${ANDROID_KEYSTORE_PASSWORD:?Set ANDROID_KEYSTORE_PASSWORD}"
: "${ANDROID_KEY_ALIAS:?Set ANDROID_KEY_ALIAS}"
: "${ANDROID_KEY_PASSWORD:?Set ANDROID_KEY_PASSWORD}"

TOOLS="$ANDROID_HOME/build-tools/35.0.0"
test -f "$ANDROID_KEYSTORE_PATH"
test -s "$1"
mkdir -p "$(dirname "$2")"
"$TOOLS/zipalign" -c -P 16 4 "$1"
"$TOOLS/apksigner" sign \
    --ks "$ANDROID_KEYSTORE_PATH" \
    --ks-key-alias "$ANDROID_KEY_ALIAS" \
    --ks-pass env:ANDROID_KEYSTORE_PASSWORD \
    --key-pass env:ANDROID_KEY_PASSWORD \
    --out "$2" "$1"
"$TOOLS/apksigner" verify --verbose --print-certs "$2"
"$TOOLS/zipalign" -c -P 16 4 "$2"
