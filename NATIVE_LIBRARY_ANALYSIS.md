# Native Library Loading Chain Analysis

## Overview
This document analyzes the native library loading chain for the Loose Ends Android app,
which uses Kotlin/Cargo integration with multiple Rust native libraries.

## Architecture

### Native Libraries

1. **libloose_ends_native.so** (Primary)
   - Source: `/workspace/app/native/Cargo.toml`
   - Package: `loose_ends_native`
   - Crate type: `cdylib`, `staticlib`
   - Features: `jni`, `neural`, `frb`, `ocr`, `multimodal`
   - Dependencies:
     - `loose-ends-core` (path dependency)
     - `chrono`, `serde_json`, `tokio`
     - `flutter_rust_bridge` (optional)
     - `jni-sys` (optional, for JNI feature)
   - Loaded by: `NativeBridge.kt` via `System.loadLibrary("loose_ends_native")`

2. **libloose_ends_core.so** (Supporting)
   - Source: `/workspace/core/Cargo.toml`
   - Package: `loose-ends-core`
   - Crate type: `rlib`, `cdylib`
   - Features: `neural`, `voice`, `ocr`, `multimodal`
   - Dependencies:
     - `rusqlite` (bundled SQLite)
     - `llama-cpp-2` (optional, neural feature)
     - `whispercpp` (optional, voice feature)
     - `ocrs`, `rten` (optional, ocr feature)
   - Note: This is linked into libloose_ends_native.so

3. **libloose_ends_voice_native.so** (Voice/Whisper - Optional)
   - Source: `/workspace/app/voice-native/Cargo.toml`
   - Package: `loose_ends_voice_native`
   - Crate type: `cdylib`
   - Dependencies:
     - `loose-ends-core` with `voice` feature
     - `jni-sys`
   - Loaded by: `VoiceNativeBridge.kt` via `System.loadLibrary("loose_ends_voice_native")`
   - Note: Separate library to avoid GGML symbol conflicts

### Loading Chain

```
App Startup
    ↓
NativeBridge.Companion.init (static initializer)
    ↓
System.loadLibrary("loose_ends_native")
    ↓
JNI finds libloose_ends_native.so in APK's lib/arm64-v8a/
    ↓
Native functions become available:
    - looseEndsOpen()
    - looseEndsExtractText()
    - looseEndsIngestRules()
    - looseEndsConfirmDraft()
    - looseEndsListOpen()
    - looseEndsListDrafts()
    - looseEndsCreateCommitment()
    - looseEndsResolveCommitment()
    - looseEndsSnoozeCommitment()
```

```
Voice Feature Initialization
    ↓
VoiceNativeBridge.Companion.init (static initializer)
    ↓
System.loadLibrary("loose_ends_voice_native")
    ↓
JNI finds libloose_ends_voice_native.so in APK's lib/arm64-v8a/
    ↓
Native function becomes available:
    - looseEndsTranscribeWav()
```

## Current Status

### Pre-packaged Libraries
Location: `/workspace/app/android/app/src/main/jniLibs/arm64-v8a/`
- ✓ libloose_ends_native.so (2.5 MB)
- ✓ libloose_ends_core.so (348 KB)
- ✗ libloose_ends_voice_native.so (MISSING - voice feature not built)

### Expected vs Actual

| Library | Expected Location | Status |
|---------|------------------|--------|
| libloose_ends_native.so | lib/arm64-v8a/ | ✓ Present |
| libloose_ends_core.so | lib/arm64-v8a/ | ✓ Present |
| libloose_ends_voice_native.so | lib/arm64-v8a/ | ✗ Missing |

## Build Configuration

### Gradle (build.gradle.kts)
- NDK version: from flutter.ndkVersion
- ABI filter: arm64-v8a only
- Packaging: useLegacyPackaging = true (compress .so files)
- ProGuard: Enabled for release builds

### ProGuard Rules
- Native methods preserved via `-keepclasseswithmembernames`
- Specific keep rules for NativeBridge, VoiceNative, VoiceNativeBridge
- ONNX Runtime and OpenCV classes preserved

## Known Issues & Resolutions

### Issue 1: Missing libloose_ends_voice_native.so
**Symptom**: Voice features unavailable, log shows "Whisper native library unavailable"

**Resolution**: 
1. Build voice-native crate for Android:
   ```bash
   cd /workspace/app/voice-native
   cargo ndk --target aarch64-linux-android --platform 21 build --release
   ```
2. Copy output to jniLibs directory:
   ```bash
   cp target/aarch64-linux-android/release/libloose_ends_voice_native.so \
      ../android/app/src/main/jniLibs/arm64-v8a/
   ```

### Issue 2: UnsatisfiedLinkError at runtime
**Symptom**: App crashes or native features don't work

**Possible causes**:
1. Library not packaged in APK
2. Wrong ABI (e.g., x86_64 emulator but only arm64-v8a libraries)
3. Missing transitive dependencies (libloose_ends_core.so must be present)

**Verification**: Use `verify-apk-contents.sh` script

### Issue 3: Symbol conflicts between llama.cpp and whisper.cpp
**Resolution**: Already addressed by separating into different .so files
- llama-cpp symbols → libloose_ends_native.so
- whisper.cpp symbols → libloose_ends_voice_native.so

## Version Compatibility

| Component | Version | Notes |
|-----------|---------|-------|
| Kotlin | 2.3.20 | From settings.gradle.kts |
| AGP | 9.0.1 | From settings.gradle.kts |
| NDK | Flutter default | Typically 25.x |
| minSdk | Flutter default | Typically 21+ |
| targetSdk | Flutter default | Typically 34+ |
| ONNX Runtime | 1.21.1 | Android binding |
| OpenCV | 4.10.0 | Android binding |

## Signature/Keystore

Current configuration uses debug signing for release builds:
```kotlin
release {
    signingConfig = signingConfigs.getByName("debug")
}
```

**WARNING**: For production releases, configure proper signing:
```kotlin
android {
    signingConfigs {
        create("release") {
            storeFile = file("release.keystore")
            storePassword = System.getenv("KEYSTORE_PASSWORD")
            keyAlias = "release"
            keyPassword = System.getenv("KEY_PASSWORD")
        }
    }
    buildTypes {
        release {
            signingConfig = signingConfigs.getByName("release")
        }
    }
}
```
