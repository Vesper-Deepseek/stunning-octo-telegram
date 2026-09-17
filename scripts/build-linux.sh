#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="${1:-release}"

case "$MODE" in
  debug|release) ;;
  *)
    echo "Usage: $0 [debug|release]" >&2
    exit 2
    ;;
esac

cd "$ROOT/app"

unset CMAKE_ARGS CMAKE_PREFIX_PATH CC CXX CC_FOR_BUILD CXX_FOR_BUILD
unset CPP CPP_FOR_BUILD AR AS LD GCC GCC_AR GCC_NM GCC_RANLIB CXXFILT
unset CFLAGS CXXFLAGS CPPFLAGS LDFLAGS LDFLAGS_LD
unset LD_LIBRARY_PATH PKG_CONFIG PKG_CONFIG_EXECUTABLE PKG_CONFIG_LIBDIR
unset PKG_CONFIG_SYSROOT_DIR LIBRARY_PATH CPATH C_INCLUDE_PATH CPLUS_INCLUDE_PATH
unset CONDA_PREFIX CONDA_DEFAULT_ENV CONDA_SHLVL CONDA_TOOLCHAIN_BUILD
unset CONDA_TOOLCHAIN_HOST CONDA_BUILD_SYSROOT CONDA_EXE CONDA_PYTHON_EXE

export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:$PATH
export CC=/usr/bin/cc CXX=/usr/bin/c++ AR=/usr/bin/ar AS=/usr/bin/as LD=/usr/bin/ld
export GCC=/usr/bin/gcc GCC_AR=/usr/bin/gcc-ar GCC_NM=/usr/bin/gcc-nm
export GCC_RANLIB=/usr/bin/gcc-ranlib
export PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/lib/pkgconfig:/usr/share/pkgconfig

echo "==> Building Flutter Linux app ($MODE)..."
flutter clean >/dev/null
flutter build linux "--$MODE"

echo "==> Building Rust native library ($MODE)..."
bash "$ROOT/scripts/build-linux-native.sh" "$MODE"

BINARY="build/linux/x64/$MODE/bundle/loose_ends"
NATIVE_SOURCE="native/target/x86_64-unknown-linux-gnu/$MODE/libloose_ends_native.so"
NATIVE_DESTINATION="build/linux/x64/$MODE/bundle/lib/libloose_ends_native.so"

if [ ! -f "$BINARY" ]; then
  echo "Flutter binary was not created: $BINARY" >&2
  exit 1
fi
if [ ! -f "$NATIVE_SOURCE" ]; then
  echo "Native library was not created: $NATIVE_SOURCE" >&2
  exit 1
fi

mkdir -p "$(dirname "$NATIVE_DESTINATION")"
cp "$NATIVE_SOURCE" "$NATIVE_DESTINATION"

if command -v readelf >/dev/null 2>&1; then
  for ARTIFACT in "$BINARY" "$NATIVE_DESTINATION"; do
    RPATH="$(readelf -d "$ARTIFACT" | grep -E 'RPATH|RUNPATH' || true)"
    if [[ "$RPATH" == *conda* || "$RPATH" == *miniforge* || "$RPATH" == *mambaforge* ]]; then
      echo "Build still contains a conda runtime path in $ARTIFACT: $RPATH" >&2
      exit 1
    fi
  done
fi

echo "==> Done. Bundle: $(dirname "$BINARY")"
