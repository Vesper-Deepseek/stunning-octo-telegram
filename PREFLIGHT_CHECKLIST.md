# Pre-Flight Verification Checklist for Android Release

## Native Library Packaging (10+ Items)

### 1. ✓ Native Libraries Present in jniLibs Directory
- [ ] `libloose_ends_native.so` exists in `app/src/main/jniLibs/arm64-v8a/`
- [ ] `libloose_ends_core.so` exists in `app/src/main/jniLibs/arm64-v8a/`
- [ ] `libloose_ends_voice_native.so` exists (if voice feature enabled)
- **Verification**: `ls -la app/src/main/jniLibs/*/`

### 2. ✓ APK Contains All Required .so Files
- [ ] Run `./verify-apk-contents.sh <apk-file>` 
- [ ] All expected libraries listed as "FOUND"
- [ ] No "MISSING" libraries reported
- **Command**: `bash verify-apk-contents.sh`

### 3. ✓ ABI Filters Match Available Libraries
- [ ] `build.gradle.kts` ndk.abiFilters matches jniLibs directories
- [ ] Only arm64-v8a targeted (current configuration)
- [ ] No mismatch between declared and actual ABIs
- **File**: `app/build.gradle.kts` line ~52

### 4. ✓ ProGuard/R8 Rules Preserve Native Methods
- [ ] `proguard-rules.pro` exists
- [ ] NativeBridge class kept with native methods
- [ ] VoiceNative/VoiceNativeBridge classes kept
- [ ] `-keepclasseswithmembernames` rule present for native methods
- **File**: `app/proguard-rules.pro`

### 5. ✓ AndroidManifest.xml Permissions Complete
- [ ] INTERNET permission declared
- [ ] RECORD_AUDIO permission (for voice features)
- [ ] Storage permissions for model files
- [ ] READ_MEDIA_IMAGES/AUDIO for Android 13+
- **File**: `app/src/main/AndroidManifest.xml`

### 6. ✓ Build Configuration Correct
- [ ] NDK version specified (via flutter.ndkVersion)
- [ ] compileSdk, targetSdk, minSdk versions appropriate
- [ ] useLegacyPackaging = true for .so compression
- [ ] ProGuard enabled for release builds
- **File**: `app/build.gradle.kts`

### 7. ✓ Version Compatibility Verified
- [ ] Kotlin version: 2.3.20
- [ ] AGP version: 9.0.1
- [ ] ONNX Runtime: 1.21.1
- [ ] OpenCV: 4.10.0
- [ ] No conflicting dependency versions
- **Files**: `settings.gradle.kts`, `build.gradle.kts`

### 8. ✓ Signature/Keystore Configured
- [ ] Debug signing works (current: uses debug keystore)
- [ ] Production: Create release keystore
- [ ] Store password configured via environment variable
- [ ] Key alias and password configured
- **Warning**: Current config uses debug signing for release!

### 9. ✓ Gradle Build Properties Optimized
- [ ] org.gradle.jvmargs sufficient (-Xmx2G)
- [ ] Configuration cache enabled
- [ ] Parallel builds enabled
- [ ] Build cache enabled
- **File**: `gradle.properties`

### 10. ✓ Native Library Loading Chain Tested
- [ ] App starts without UnsatisfiedLinkError
- [ ] NativeBridge.init() succeeds
- [ ] Native methods callable from Kotlin
- [ ] Logcat shows no "native library unavailable" errors
- **Command**: `./test-on-emulator.sh <apk-file>`

### 11. ✓ Symbol Conflict Resolution Verified
- [ ] llama-cpp symbols isolated in libloose_ends_native.so
- [ ] whisper.cpp symbols isolated in libloose_ends_voice_native.so
- [ ] No duplicate GGML/GGUF symbol errors
- **Analysis**: See NATIVE_LIBRARY_ANALYSIS.md

### 12. ✓ Multi-Dex Enabled (if needed)
- [ ] multiDexEnabled = true in defaultConfig
- [ ] Core library desugaring configured
- [ ] Desugar JDK libs version: 2.1.4
- **File**: `app/build.gradle.kts`

---

## Quick Verification Commands

```bash
# 1. Check native libraries in source tree
ls -la /workspace/app/android/app/src/main/jniLibs/arm64-v8a/

# 2. Build debug APK
cd /workspace/app/android && ./gradlew assembleDebug

# 3. Verify APK contents
/workspace/verify-apk-contents.sh /workspace/app/android/app/build/outputs/apk/debug/app-debug.apk

# 4. Test on emulator (requires running emulator)
/workspace/test-on-emulator.sh /workspace/app/android/app/build/outputs/apk/debug/app-debug.apk

# 5. Check ProGuard rules exist
cat /workspace/app/android/app/proguard-rules.pro | head -20

# 6. Verify manifest permissions
grep permission /workspace/app/android/app/src/main/AndroidManifest.xml
```

---

## Common Failure Modes & Fixes

| Symptom | Cause | Fix |
|---------|-------|-----|
| UnsatisfiedLinkError | Missing .so in APK | Add to jniLibs/, run verify script |
| Wrong ABI | x86_64 emulator, arm64 only libs | Use arm64 emulator or build x86_64 libs |
| ClassNotFoundException | ProGuard removed class | Add -keep rule in proguard-rules.pro |
| INSTALL_FAILED_UPDATE_INCOMPATIBLE | Signature mismatch | Uninstall old app first |
| App crashes on startup | Missing transitive deps | Ensure libloose_ends_core.so packaged |
| Voice features unavailable | libloose_ends_voice_native.so missing | Build voice-native crate |

---

## Files Modified in This Analysis

| File | Change Type | Purpose |
|------|-------------|---------|
| `app/build.gradle.kts` | Modified | Added ProGuard, ndk filters, debug config |
| `app/proguard-rules.pro` | Created | R8 rules for native method preservation |
| `app/src/main/AndroidManifest.xml` | Modified | Added storage/media permissions |
| `gradle.properties` | Modified | Added build optimization flags |
| `verify-apk-contents.sh` | Created | APK verification script |
| `test-on-emulator.sh` | Created | Emulator testing script |
| `NATIVE_LIBRARY_ANALYSIS.md` | Created | Architecture documentation |

---

## Before Release: Additional Steps

1. **Generate Release Keystore**
   ```bash
   keytool -genkey -v -keystore release.keystore -alias release \
     -keyalg RSA -keysize 2048 -validity 10000
   ```

2. **Configure Signing in build.gradle.kts**
   Replace debug signing with release configuration

3. **Build Release APK**
   ```bash
   cd /workspace/app/android && ./gradlew assembleRelease
   ```

4. **Run Full Verification**
   ```bash
   ./verify-apk-contents.sh app/build/outputs/apk/release/app-release.apk
   ```

5. **Test on Physical Devices**
   - Test on multiple Android versions (10, 11, 12, 13, 14)
   - Test on different manufacturers (Samsung, Pixel, etc.)

6. **Check Play Store Requirements**
   - Target SDK 34+ (required for new apps)
   - 64-bit binaries (arm64-v8a ✓)
   - App bundle (.aab) recommended over APK
