# APK Build Fix - AGP 9.0 Compatibility

## Problem
Build failed with AGP 9.0 deprecation errors:
- `Unresolved reference 'ndk'` at line 52
- `Unresolved reference 'abiFilters'` at line 53
- Deprecated `packagingOptions` DSL

## Root Cause
AGP 9.0 changed the DSL structure:
1. `ndk {}` block must be inside `defaultConfig {}`, not at top level of `android {}`
2. `packagingOptions {}` renamed to `packaging {}`

## Fix Applied

### Before (build.gradle.kts lines 19-55)
```kotlin
defaultConfig {
    applicationId = "com.looseends.loose_ends"
    minSdk = flutter.minSdkVersion
    targetSdk = flutter.targetSdkVersion
    versionCode = flutter.versionCode
    versionName = flutter.versionName
    multiDexEnabled = true
}

buildTypes { ... }

packagingOptions {
    jniLibs {
        useLegacyPackaging = true
    }
}

// Ensure all required ABIs are packaged
ndk {
    abiFilters += listOf("arm64-v8a")
}
```

### After (build.gradle.kts lines 19-55)
```kotlin
defaultConfig {
    applicationId = "com.looseends.loose_ends"
    minSdk = flutter.minSdkVersion
    targetSdk = flutter.targetSdkVersion
    versionCode = flutter.versionCode
    versionName = flutter.versionName
    multiDexEnabled = true

    // Ensure all required ABIs are packaged
    ndk {
        abiFilters += listOf("arm64-v8a")
    }
}

buildTypes { ... }

packaging {
    jniLibs {
        useLegacyPackaging = true
    }
}
```

## Verification Steps

### 1. Local Build Test (if Flutter installed)
```bash
cd /workspace/app
flutter build apk --release --target-platform android-arm64 --split-per-abi
```

### 2. CI/CD Verification
The fix is ready for GitHub Actions. Trigger workflow:
```bash
git add android/app/build.gradle.kts
git commit -m "fix: AGP 9.0 compatibility - move ndk block to defaultConfig"
git push
```

### 3. Post-Build APK Verification
After successful build, verify APK contents:
```bash
# Extract and verify
unzip -l app/build/outputs/apk/release/app-arm64-v8a-release.apk | grep "\.so$"

# Expected libraries:
# lib/arm64-v8a/libloose_ends_native.so
# lib/arm64-v8a/libloose_ends_core.so
# lib/arm64-v8a/libopencv_java4.so
# lib/arm64-v8a/libonnxruntime.so
```

## Additional Notes

### Missing Voice Library
VoiceNative.kt calls `System.loadLibrary("loose_ends_voice")` but the actual file is named differently. Check:
```bash
find /workspace -name "*voice*.so" -o -name "*Voice*.kt" 2>/dev/null
```

### OpenCV Extraction Task
The `extractOpenCvNativeLibs` Gradle task needs to run before assembleRelease:
```kotlin
tasks.named("preBuild") {
    dependsOn("extractOpenCvNativeLibs")
}
```

## Files Modified
- `/workspace/app/android/app/build.gradle.kts` - Fixed AGP 9.0 DSL compatibility
