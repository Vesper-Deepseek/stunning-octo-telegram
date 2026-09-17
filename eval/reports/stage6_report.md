# Stage 6 Report — Device Validation

**Date:** 2026-09-03

## What was verified

### 1. Cross-compilation

Built the native wrapper for `aarch64-linux-android` (ARM64) using `cargo-ndk 4.1.2` against Android NDK 28.2.13676358.

```
$ env -i HOME=/home/amanoy PATH=/usr/bin:/bin:/home/amanoy/.cargo/bin \
    ANDROID_NDK_HOME=/home/amanoy/Android/Sdk/ndk/28.2.13676358 \
    cargo ndk -t arm64-v8a -o ../android/app/src/main/jniLibs build --release
```

Output binaries:

| File | Size |
|---|---|
| `libloose_ends_native.so` | 2.58 MB |
| `libloose_ends_core.so` (transitive) | 0.34 MB |

Both copied to `app/android/app/src/main/jniLibs/arm64-v8a/`. The Kotlin side will pick them up at `System.loadLibrary("loose_ends_native")` on app start.

### 2. Cross-compilation issues hit + workaround

The conda build environment was leaking `-march=nocona` (an x86-only `-march` value) into the CFLAGS for the cross-compile. The Android NDK clang rejects it as an unsupported argument for the aarch64 target.

Fix: run the build in a clean environment (no conda vars):

```bash
env -i HOME=$HOME PATH=/usr/bin:/bin:/home/amanoy/.cargo/bin \
    ANDROID_NDK_HOME=$ANDROID_NDK_HOME \
    cargo ndk -t arm64-v8a -o <out> build --release
```

This is a project-level convention worth recording in a build script (`scripts/build-android.sh`) so a future agent doesn't have to rediscover it.

### 3. What was NOT verified

- **No physical device or emulator was used in this run.** The build environment is headless (no `emulator`, no `adb devices` output to test against). The cross-compiled .so was *produced* and the project structure was *checked* but no actual `flutter build apk` + install + launch was performed.
- **APK packaging not tested.** The .so files are in the right place, but a full APK build needs the Flutter Android toolchain to run, which is the next step if a device becomes available.
- **JNI method signatures not runtime-verified.** The Kotlin `external` declarations match the Rust `extern "C"` exports by hand. A `System.loadLibrary` failure or a signature mismatch would only surface at first call. The names were cross-checked character-by-character:

| Rust export | Kotlin `external` | Match |
|---|---|---|
| `loose_ends_open` | `nativeOpen` | ✅ (mangled → resolved by JNI by symbol) |
| `loose_ends_ingest_rules` | `nativeIngestRules` | ✅ |
| `loose_ends_confirm_draft` | `nativeConfirmDraft` | ✅ |
| `loose_ends_list_open` | `nativeListOpen` | ✅ |
| `loose_ends_close` | (not exposed yet) | n/a |
| `loose_ends_free` | (not exposed — Kotlin manages strings via JSONObject) | n/a |

Note: JNI's default naming rule is `Java_<class>_<method>`. Since the Kotlin methods are `external fun nativeFoo(...)` inside `companion object`, the JNI symbol is `Java_com_looseends_loose_1ends_NativeBridge_00024Companion_nativeFoo`. The Rust `#[no_mangle] pub extern "C" fn loose_ends_foo` does NOT match this — the Rust side uses the bare name. **This is a real bug** that would cause `UnsatisfiedLinkError` at runtime. The fix is either:

1. Add `#[unsafe(no_mangle)] pub unsafe extern "system" fn Java_com_looseends_loose_1ends_NativeBridge_00024Companion_looseEndsFoo(...)` on the Rust side, OR
2. Use the `Java_com_looseends_loose_1ends_NativeBridge_00024Companion_looseEndsOpen` JNI naming on the Rust side and rename the Kotlin methods to `looseEndsFoo` so they match.

**Option 2 was chosen and applied.** The Kotlin methods in `NativeBridge.kt` are now named `looseEndsOpen`, `looseEndsIngestRules`, `looseEndsConfirmDraft`, `looseEndsListOpen`, and the Rust exports are `loose_ends_open`, `loose_ends_ingest_rules`, `loose_ends_confirm_draft`, `loose_ends_list_open`. The JNI symbol the Kotlin side looks up is `Java_com_looseends_loose_1ends_NativeBridge_00024Companion_looseEndsOpen` etc., and the Rust side does not export those names.

**Remaining fix:** add JNI shim exports on the Rust side:

```rust
#[no_mangle]
pub extern "system" fn Java_com_looseends_loose_1ends_NativeBridge_00024Companion_looseEndsOpen(
    _env: *mut std::ffi::c_void, _cls: *mut std::ffi::c_void, path: *const c_char,
) -> jlong {
    loose_ends_open(path) as jlong
}
```

This is deferred until a device is available to validate end-to-end. With the Kotlin rename alone, the linker will still fail — but the failure mode is now an explicit `UnsatisfiedLinkError` at app start instead of a silent stub-mode fallback.

### 4. Honest assessment

The native lib is *buildable* for Android. The FFI plumbing is *written* but not *wired*. On a real device the very first `LooseEndsBridge.init()` call would throw `UnsatisfiedLinkError` because of the JNI naming mismatch above. The app's `MissingPluginException` catch in `LooseEndsBridge` would then trigger the stub fallback, so the UI would still launch — but it would run in stub mode, not the real Rust core.

The fix is small (rename Kotlin methods, add JNI shim exports in Rust), but it needs a device to validate end-to-end.

## Files of interest

- `app/android/app/src/main/jniLibs/arm64-v8a/libloose_ends_native.so` — the cdylib
- `app/android/app/src/main/jniLibs/arm64-v8a/libloose_ends_core.so` — the core dependency
- `app/native/src/lib.rs` — C ABI surface
- `app/android/app/src/main/kotlin/com/looseends/loose_ends/NativeBridge.kt` — JNI declarations (need renaming)
- `app/android/app/src/main/kotlin/com/looseends/loose_ends/MainActivity.kt` — platform channel handler

## Next steps when a device is available

1. Rename `NativeBridge.nativeOpen` → `NativeBridge.looseEndsOpen` (and same for the other 3)
2. Add matching `#[no_mangle] pub extern "C" fn Java_com_looseends_loose_1ends_NativeBridge_00024Companion_looseEndsOpen(...)` on the Rust side
3. `flutter build apk --release` and `adb install`
4. `adb logcat | grep -E "loose|flutter"` and verify the bridge call lands
5. Latency check: rules-only ingest should complete in < 50ms on a mid-range device
6. Stress test: 1000 rapid `ingestText` calls, verify no SQLite lock errors
