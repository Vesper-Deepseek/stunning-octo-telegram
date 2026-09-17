#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="${1:-debug}"

case "$MODE" in
  debug|release) ;;
  *)
    echo "Usage: $0 [debug|release]" >&2
    exit 2
    ;;
esac

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

cd "$ROOT"

echo "==> Building loose-ends-core (no default features, $MODE)..."
if [[ "$MODE" == "release" ]]; then
  cargo build --manifest-path core/Cargo.toml --lib --release
else
  cargo build --manifest-path core/Cargo.toml --lib
fi

echo "==> Building loose_ends_native for Linux (cdylib, no jni, $MODE)..."
if [[ "$MODE" == "release" ]]; then
  cargo build --manifest-path app/native/Cargo.toml \
    --lib \
    --target x86_64-unknown-linux-gnu \
    --release
else
  cargo build --manifest-path app/native/Cargo.toml \
    --lib \
    --target x86_64-unknown-linux-gnu
fi

ARTIFACT="app/native/target/x86_64-unknown-linux-gnu/$MODE/libloose_ends_native.so"
if [ ! -f "$ARTIFACT" ]; then
  ARTIFACT="app/native/target/$MODE/libloose_ends_native.so"
fi

echo "==> Native library at: $ARTIFACT"
mkdir -p app/build/linux/native_libs
cp "$ARTIFACT" app/build/linux/native_libs/libloose_ends_native.so

echo "==> Done. Native library copied to app/build/linux/native_libs/"
