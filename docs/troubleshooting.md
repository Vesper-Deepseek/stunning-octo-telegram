# Troubleshooting

## Flutter analyze fails

From `app/`:

```bash
flutter pub get
flutter analyze
```

If bindings changed, regenerate them:

```bash
flutter_rust_bridge_codegen generate --config-file flutter_rust_bridge.yaml
```

## Rust Android build cannot find the Android linker

Install the expected Rust target and make sure the NDK compiler paths are exported.

The release workflow explicitly sets the linker, `CC`, and `CXX` variables for each supported Android ABI.

## bindgen cannot find Android system headers

The Android NDK sysroot must be passed through `BINDGEN_EXTRA_CLANG_ARGS`. See the release workflow for the exact form.

## APK is very large

The Android packaging configuration uses legacy JNI packaging so native shared libraries are compressed inside the APK. The v1.0.4 universal APK is about 92.3 MB.

## GitHub release download fails

Check the release asset state first. The release pipeline performs a real public URL download and SHA-256 comparison after upload.

A successful release job with the message:

```text
GitHub release APK download verified successfully.
```

means the GitHub asset was reachable by the runner and matched the build checksum.

## Native voice library is unavailable

Check that the APK contains `libloose_ends_voice.so` for the device ABI and that `VoiceNative.kt` loads `loose_ends_voice`.

## Native Android crash

Collect:

- exact version/tag
- device model and Android version
- relevant logcat lines
- ABI
- whether the failure occurs before or after Flutter renders
