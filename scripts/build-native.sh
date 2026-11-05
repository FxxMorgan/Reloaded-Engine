#!/usr/bin/env bash
set -euo pipefail

# Reloaded Engine alpha: the packaged and device-tested ABI is arm64-v8a.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
NATIVE_DIR="$PROJECT_ROOT/native"
SDK_DIR="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}"
NDK_DIR="${ANDROID_NDK_HOME:-${NDK_HOME:-$SDK_DIR/ndk/28.2.13676358}}"
RELOADED_CARGO="${RELOADED_CARGO:-cargo}"

case "$(uname -s)" in
    Linux) NDK_HOST="linux-x86_64" ;;
    Darwin) NDK_HOST="darwin-x86_64" ;;
    *) echo "Use Linux or macOS to run this build script." >&2; exit 1 ;;
esac
NDK_BIN="$NDK_DIR/toolchains/llvm/prebuilt/$NDK_HOST/bin"
if [ ! -x "$NDK_BIN/aarch64-linux-android26-clang" ]; then
    echo "NDK compiler not found: $NDK_BIN/aarch64-linux-android26-clang" >&2
    echo "Install NDK 28.2.13676358 or set ANDROID_NDK_HOME." >&2
    exit 1
fi
command -v "$RELOADED_CARGO" >/dev/null || { echo "Cargo is required." >&2; exit 1; }

export PATH="$NDK_BIN:$PATH"
export CC_aarch64_linux_android="$NDK_BIN/aarch64-linux-android26-clang"
export CXX_aarch64_linux_android="$NDK_BIN/aarch64-linux-android26-clang++"
export AR_aarch64_linux_android="$NDK_BIN/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$NATIVE_DIR/target}"

printf 'Building Reloaded Engine for arm64-v8a\nNDK: %s\n' "$NDK_DIR"
(cd "$NATIVE_DIR" && "$RELOADED_CARGO" build --locked -p reloaded-jni --target aarch64-linux-android --release)

LIBRARY="$CARGO_TARGET_DIR/aarch64-linux-android/release/libreloaded_jni.so"
DESTINATION="$PROJECT_ROOT/app/src/main/jniLibs/arm64-v8a"
test -s "$LIBRARY"
mkdir -p "$DESTINATION"
cp "$LIBRARY" "$DESTINATION/libreloaded_jni.so"
printf 'Native library ready: %s/libreloaded_jni.so\n' "$DESTINATION"
