# Development guide

## Prerequisites

- Flutter stable
- Dart SDK compatible with `app/pubspec.yaml`
- Rust stable with `rustfmt` and `clippy`
- Java 17
- Android SDK
- Android NDK 28.2.13676358

## Flutter

```bash
cd app
flutter pub get
flutter analyze
flutter test
```

The CI workflow installs flutter_rust_bridge_codegen 2.13.0 and regenerates bindings before analysis.

## Rust core

```bash
cd core
cargo fmt -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

## Native libraries

Main native library:

```bash
cd app/native
cargo check --features jni
cargo clippy --locked --features jni -- -D warnings
```

Voice library:

```bash
cd app/voice-native
cargo check --locked
cargo clippy --locked -- -D warnings
```

## Bridge generation

```bash
cd app
flutter_rust_bridge_codegen generate --config-file flutter_rust_bridge.yaml
```

Generated bridge outputs should stay consistent with the repository configuration.

## Android native build

For arm64:

```bash
rustup target add aarch64-linux-android
```

The exact NDK compiler/linker environment used by release CI is in `.github/workflows/release.yml`. When Rust dependencies invoke bindgen or a C/C++ build script, provide the Android NDK sysroot and target explicitly.

## Validation

Canonical gates include Rust format/tests/Clippy, Flutter analyze/tests, Android native compilation, APK build, and release asset download verification.

The emulator smoke workflow is intentionally disabled. Physical-device validation should record the device, Android version, build, and exact test steps.

## Commit guidance

Prefer focused messages such as:

```text
docs: update architecture guide
fix: correct native bridge behavior
build: update Android release pipeline
release: bump app version
```

Do not commit build outputs, generated `target/` directories, keystores, or local configuration.
