# Loose Ends Android APK - Pre-Flight Verification Checklist

## Critical Issues Currently Present

### ❌ ISSUE 1: Missing Voice Native Library
**Problem:** `VoiceNative.kt` calls `System.loadLibrary("loose_ends_voice")` but `libloose_ends_voice.so` is not packaged.

**Location:** `app/android/app/src/main/jniLibs/arm64-v8a/`

**Expected files:**
- `libloose_ends_voice.so` (from `voice-native/target/<abi>/release/libloose_ends_voice_native.so`)

**Fix:**
```bash
# Build voice native for all ABIs
cargo build --manifest-path app/voice-native/Cargo.toml --release \
  --target aarch64-linux-android
cargo build --manifest-path app/voice-native/Cargo.toml --release \
  --target armv7-linux-androideabi  
cargo build --manifest-path app/voice-native/Cargo.toml --release \
  --target x86_64-linux-android

# Copy to jniLibs with correct name
cp app/voice-native/target/aarch64-linux-android/release/libloose_ends_voice_native.so \
   app/android/app/src/main/jniLibs/arm64-v8a/libloose_ends_voice.so
cp app/voice-native/target/armv7-linux-androideabi/release/libloose_ends_voice_native.so \
   app/android/app/src/main/jniLibs/armeabi-v7a/libloose_ends_voice.so
cp app/voice-native/target/x86_64-linux-android/release/libloose_ends_voice_native.so \
   app/android/app/src/main/jniLibs/x86_64/libloose_ends_voice.so
```

---

### ❌ ISSUE 2: Missing ABI Architectures
**Problem:** Only `arm64-v8a` exists. Need all three ABIs for universal APK.

**Required directories:**
- `app/android/app/src/main/jniLibs/arm64-v8a/` ✓ exists
- `app/android/app/src/main/jniLibs/armeabi-v7a/` ✗ missing
- `app/android/app/src/main/jniLibS/x86_64/` ✗ missing

**Fix:** Create directories and copy libraries for all ABIs (see workflow in release.yml)

---

### ❌ ISSUE 3: Missing OpenCV Native Libraries
**Problem:** `build.gradle.kts` has `extractOpenCvNativeLibs` task but it hasn't been run.

**Fix:**
```bash
cd app
flutter pub get
./android/gradlew :app:extractOpenCvNativeLibs
```

This extracts `libopencv_java4.so` for all ABIs from the OpenCV AAR.

---

### ⚠️ ISSUE 4: No APK Built
**Problem:** No APK exists at expected locations.

**Fix:**
```bash
cd app
flutter build apk --release
```

---

## Complete Fix Sequence

Run these commands in order:

```bash
# 1. Setup Flutter dependencies
cd /workspace/app
flutter pub get

# 2. Extract OpenCV native libs
chmod +x android/gradlew
./android/gradlew -p android :app:extractOpenCvNativeLibs

# 3. Build Rust native libraries for all ABIs
export NDK_VERSION=28.2.13676358
export ANDROID_API=24
NDK_DIR="${ANDROID_HOME}/ndk/${NDK_VERSION}"
TOOLCHAIN="${NDK_DIR}/toolchains/llvm/prebuilt/linux-x86_64/bin"

export AR_aarch64_linux_android="${TOOLCHAIN}/llvm-ar"
export AR_armv7_linux_androideabi="${TOOLCHAIN}/llvm-ar"
export AR_x86_64_linux_android="${TOOLCHAIN}/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${TOOLCHAIN}/aarch64-linux-android${ANDROID_API}-clang"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="${TOOLCHAIN}/armv7a-linux-androideabi${ANDROID_API}-clang"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="${TOOLCHAIN}/x86_64-linux-android${ANDROID_API}-clang"
export CC_aarch64_linux_android="${TOOLCHAIN}/aarch64-linux-android${ANDROID_API}-clang"
export CC_armv7_linux_androideabi="${TOOLCHAIN}/armv7a-linux-androideabi${ANDROID_API}-clang"
export CC_x86_64_linux_android="${TOOLCHAIN}/x86_64-linux-android${ANDROID_API}-clang"
export CXX_aarch64_linux_android="${TOOLCHAIN}/aarch64-linux-android${ANDROID_API}-clang++"
export CXX_armv7_linux_androideabi="${TOOLCHAIN}/armv7a-linux-androideabi${ANDROID_API}-clang++"
export CXX_x86_64_linux_android="${TOOLCHAIN}/x86_64-linux-android${ANDROID_API}-clang++"
export BINDGEN_EXTRA_CLANG_ARGS="--sysroot=${NDK_DIR}/toolchains/llvm/prebuilt/linux-x86_64/sysroot"

# Build core native library
for target in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android; do
  cargo build --manifest-path native/Cargo.toml --release --target "$target" --features jni,neural
done

# Build voice native library
for target in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android; do
  cargo build --manifest-path voice-native/Cargo.toml --release --target "$target"
done

# 4. Copy all native libraries to jniLibs
mkdir -p android/app/src/main/jniLibs/arm64-v8a
mkdir -p android/app/src/main/jniLibs/armeabi-v7a
mkdir -p android/app/src/main/jniLibs/x86_64

cp native/target/aarch64-linux-android/release/libloose_ends_native.so \
   android/app/src/main/jniLibs/arm64-v8a/
cp native/target/armv7-linux-androideabi/release/libloose_ends_native.so \
   android/app/src/main/jniLibs/armeabi-v7a/
cp native/target/x86_64-linux-android/release/libloose_ends_native.so \
   android/app/src/main/jniLibs/x86_64/

cp voice-native/target/aarch64-linux-android/release/libloose_ends_voice_native.so \
   android/app/src/main/jniLibs/arm64-v8a/libloose_ends_voice.so
cp voice-native/target/armv7-linux-androideabi/release/libloose_ends_voice_native.so \
   android/app/src/main/jniLibs/armeabi-v7a/libloose_ends_voice.so
cp voice-native/target/x86_64-linux-android/release/libloose_ends_voice_native.so \
   android/app/src/main/jniLibs/x86_64/libloose_ends_voice.so

# 5. Verify jniLibs contents
ls -la android/app/src/main/jniLibs/*/

# Expected output for each ABI directory:
# - libloose_ends_native.so
# - libloose_ends_voice.so
# - libloose_ends_core.so (if present)
# - libopencv_java4.so (after extraction)

# 6. Build APK
flutter build apk --release

# 7. Run verification script
/workspace/verify-apk-contents.sh app/build/app/outputs/flutter-apk/app-release.apk
```

---

## 10+ Item Pre-Flight Checklist

Before releasing/signing the APK, verify ALL items:

### Native Libraries
- [ ] **1.** `lib/arm64-v8a/libloose_ends_native.so` present in APK
- [ ] **2.** `lib/arm64-v8a/libloose_ends_voice.so` present in APK
- [ ] **3.** `lib/armeabi-v7a/libloose_ends_native.so` present in APK
- [ ] **4.** `lib/armeabi-v7a/libloose_ends_voice.so` present in APK
- [ ] **5.** `lib/x86_64/libloose_ends_native.so` present in APK
- [ ] **6.** `lib/x86_64/libloose_ends_voice.so` present in APK
- [ ] **7.** `libopencv_java4.so` present for all ABIs (if using OCR)

### Configuration
- [ ] **8.** AndroidManifest.xml has `RECORD_AUDIO` permission (for voice)
- [ ] **9.** AndroidManifest.xml has `INTERNET` permission
- [ ] **10.** MainActivity has `android:exported="true"`
- [ ] **11.** Application ID matches: `com.looseends.loose_ends`

### ProGuard/R8
- [ ] **12.** NativeBridge keep rules present
- [ ] **13.** VoiceNative keep rules present
- [ ] **14.** ONNX Runtime keep rules present
- [ ] **15.** OpenCV keep rules present

### Build
- [ ] **16.** APK builds without errors
- [ ] **17.** APK size is reasonable (< 50MB for universal)
- [ ] **18.** Verification script passes

### Testing
- [ ] **19.** App installs on arm64 device
- [ ] **20.** App installs on armeabi-v7a device (if targeting older devices)
- [ ] **21.** App runs on emulator (x86_64)
- [ ] **22.** Native functions don't crash (UnsatisfiedLinkError)

---

## Emulator Testing Commands

```bash
# List available AVD images
avdmanager list avd

# Create new AVD (if needed)
avdmanager create avd -n test_avd -k "system-images;android-34;google_apis_playstore;arm64-v8a"

# Start emulator
emulator -avd test_avd -no-snapshot -wipe-data

# Install APK
adb install -r app/build/app/outputs/flutter-apk/app-release.apk

# Launch app
adb shell am start -n com.looseends.loose_ends/.MainActivity

# Check logs for native library loading
adb logcat | grep -E "(NativeBridge|VoiceNative|UnsatisfiedLinkError)"

# Verify native libraries loaded
adb shell ls /data/app/com.looseends.loose_ends-*/lib/arm64/

# Uninstall for clean test
adb uninstall com.looseends.loose_ends
```

---

## Verification Script Usage

```bash
# After building APK, run verification
/workspace/verify-apk-contents.sh app/build/app/outputs/flutter-apk/app-release.apk

# Or let it auto-detect
/workspace/verify-apk-contents.sh
```

The script will:
1. Extract APK contents
2. List all .so files
3. Check for expected libraries
4. Compare source vs APK
5. Validate ProGuard rules
6. Report mismatches

---

## Before/After Comparison

### BEFORE (Current State)
```
jniLibs/
└── arm64-v8a/
    ├── libloose_ends_core.so
    └── libloose_ends_native.so

Missing:
- armeabi-v7a/ directory
- x86_64/ directory
- libloose_ends_voice.so (all ABIs)
- libopencv_java4.so (all ABIs)
```

### AFTER (Fixed State)
```
jniLibs/
├── arm64-v8a/
│   ├── libloose_ends_core.so
│   ├── libloose_ends_native.so
│   ├── libloose_ends_voice.so
│   └── libopencv_java4.so
├── armeabi-v7a/
│   ├── libloose_ends_native.so
│   ├── libloose_ends_voice.so
│   └── libopencv_java4.so
└── x86_64/
    ├── libloose_ends_native.so
    ├── libloose_ends_voice.so
    └── libopencv_java4.so
```

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `NativeBridge.kt` | Loads `libloose_ends_native.so`, provides SQLite store |
| `VoiceNative.kt` | Loads `libloose_ends_voice.so`, provides Whisper transcription |
| `build.gradle.kts` | Configures packaging, ABI filters, OpenCV extraction |
| `proguard-rules.pro` | Keeps native method signatures from obfuscation |
| `AndroidManifest.xml` | Declares permissions, activities |
| `release.yml` | CI workflow with complete build sequence |

---

## Common Errors & Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| `UnsatisfiedLinkError: dlopen failed: library "libloose_ends_voice.so" not found` | Voice lib not packaged | Copy `libloose_ends_voice_native.so` as `libloose_ends_voice.so` |
| `java.lang.UnsatisfiedLinkError: No implementation found for ...` | ProGuard stripped native methods | Add `-keepclassmembers class * { native <methods>; }` |
| `INSTALL_FAILED_NO_MATCHING_ABIS` | Device ABI not in APK | Include all ABIs or use split APKs |
| App crashes on startup | Missing JNI dependencies | Check `ldd` on native libs, ensure all deps bundled |
