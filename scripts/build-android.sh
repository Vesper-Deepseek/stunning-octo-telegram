#!/usr/bin/env bash
# Build the native cdylib for Android and copy it into the Flutter project's
# jniLibs directory. Must be run from `app/native/`.
#
# Usage:  scripts/build-android.sh [target]
#   target defaults to arm64-v8a; alternatives: armeabi-v7a, x86_64, x86
set -euo pipefail

TARGET="${1:-arm64-v8a}"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NATIVE_DIR="$PROJECT_ROOT/app/native"
VOICE_NATIVE_DIR="$PROJECT_ROOT/app/voice-native"
JNI_OUT="$PROJECT_ROOT/app/android/app/src/main/jniLibs"

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  if [[ -d "$HOME/Android/Sdk/ndk" ]]; then
    ANDROID_NDK_HOME="$(ls -d "$HOME"/Android/Sdk/ndk/* | sort -V | tail -1)"
    echo "Auto-detected NDK: $ANDROID_NDK_HOME"
  else
    echo "ANDROID_NDK_HOME is not set and no SDK found at \$HOME/Android/Sdk/ndk" >&2
    exit 1
  fi
fi

if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "cargo-ndk not found; install with: cargo install cargo-ndk" >&2
  exit 1
fi

# Cross-compile needs a clean env. Conda sets x86-only -march flags that
# the NDK clang rejects for aarch64 targets.
env -i \
  HOME="$HOME" \
  PATH="$HOME/.cargo/bin:/usr/bin:/bin" \
  ANDROID_NDK_HOME="$ANDROID_NDK_HOME" \
  cargo ndk -t "$TARGET" -o "$JNI_OUT" build --release --features jni

# Build the Whisper bridge as its own cdylib (libloose_ends_voice.so).
# Kotlin's VoiceNativeBridge/VoiceNative call System.loadLibrary("loose_ends_voice");
# if this crate is not compiled and copied into jniLibs, every voice
# transcription throws UnsatisfiedLinkError and force-closes the app.
cd "$VOICE_NATIVE_DIR"
env -i \
  HOME="$HOME" \
  PATH="$HOME/.cargo/bin:/usr/bin:/bin" \
  ANDROID_NDK_HOME="$ANDROID_NDK_HOME" \
  cargo ndk -t "$TARGET" -o "$JNI_OUT" build --release

# cargo-ndk names the output after the crate (libloose_ends_voice_native.so);
# rename it to the exact soname loadLibrary("loose_ends_voice") expects.
ABI_DIR="$JNI_OUT/$TARGET"
if [[ -f "$ABI_DIR/libloose_ends_voice_native.so" ]]; then
  mv -f "$ABI_DIR/libloose_ends_voice_native.so" "$ABI_DIR/libloose_ends_voice.so"
fi

if [[ ! -f "$ABI_DIR/libloose_ends_voice.so" ]]; then
  echo "ERROR: $ABI_DIR/libloose_ends_voice.so was not produced; voice will crash at runtime." >&2
  exit 1
fi

echo
echo "Built artifacts:"
find "$JNI_OUT" -name "*.so" -exec ls -lh {} \;
