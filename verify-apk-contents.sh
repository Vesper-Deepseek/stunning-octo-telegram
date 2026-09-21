#!/bin/bash
# verify-apk-contents.sh - Verify APK packaging for Loose Ends Android app
# Checks native library loading chain, APK contents, manifest, ProGuard rules

set -e

APK_FILE="${1:-}"
EXPECTED_LIBS=(
    "lib/arm64-v8a/libloose_ends_native.so"
    "lib/arm64-v8a/libloose_ends_voice.so"
    "lib/arm64-v8a/libopencv_java4.so"
    "lib/armeabi-v7a/libloose_ends_native.so"
    "lib/armeabi-v7a/libloose_ends_voice.so"
    "lib/armeabi-v7a/libopencv_java4.so"
    "lib/x86_64/libloose_ends_native.so"
    "lib/x86_64/libloose_ends_voice.so"
    "lib/x86_64/libopencv_java4.so"
)

SOURCE_JNILIBS_DIR="app/android/app/src/main/jniLibs"

usage() {
    echo "Usage: $0 [apk-file]"
    echo ""
    echo "Extracts and verifies native library contents of an Android APK"
    echo ""
    echo "Checks performed:"
    echo "  - Lists all .so files in the APK"
    echo "  - Verifies expected native libraries are present"
    echo "  - Checks AndroidManifest.xml permissions"
    echo "  - Compares source jniLibs vs APK contents"
    echo "  - Validates ProGuard/R8 rules"
    echo ""
    echo "If no APK is specified, searches common build locations."
    exit 1
}

if [ -z "$APK_FILE" ]; then
    # Try to find the APK in common build locations
    if [ -f "/workspace/app/build/app/outputs/flutter-apk/app-release.apk" ]; then
        APK_FILE="/workspace/app/build/app/outputs/flutter-apk/app-release.apk"
    elif [ -f "/workspace/app/android/app/build/outputs/apk/debug/app-debug.apk" ]; then
        APK_FILE="/workspace/app/android/app/build/outputs/apk/debug/app-debug.apk"
    elif [ -f "/workspace/app/android/app/build/outputs/apk/release/app-release.apk" ]; then
        APK_FILE="/workspace/app/android/app/build/outputs/apk/release/app-release.apk"
    else
        echo "ERROR: No APK file specified and none found in default locations."
        usage
    fi
fi

if [ ! -f "$APK_FILE" ]; then
    echo "ERROR: APK file not found: $APK_FILE"
    usage
fi

echo "=========================================="
echo "APK Contents Verification Script"
echo "=========================================="
echo ""
echo "APK File: $APK_FILE"
echo ""

# Create temp directory for extraction
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo "--- Step 1: Extracting APK contents ---"
cd "$TEMP_DIR"
unzip -q "$APK_FILE"

echo ""
echo "--- Step 2: Listing all .so files in APK ---"
find . -name "*.so" -type f | sort > so_files.txt
if [ -s so_files.txt ]; then
    cat so_files.txt
else
    echo "WARNING: No .so files found in APK!"
fi

echo ""
echo "--- Step 3: Checking lib/ directory structure ---"
if [ -d "lib" ]; then
    find lib -type f | sort
else
    echo "WARNING: No 'lib/' directory found in APK"
fi

echo ""
echo "--- Step 4: Verifying expected native libraries ---"
MISSING_LIBS=()
FOUND_LIBS=()

for lib in "${EXPECTED_LIBS[@]}"; do
    if [ -f "$lib" ]; then
        FOUND_LIBS+=("$lib")
        SIZE=$(stat -c%s "$lib" 2>/dev/null || stat -f%z "$lib")
        echo "✓ FOUND: $lib ($SIZE bytes)"
    else
        MISSING_LIBS+=("$lib")
        echo "✗ MISSING: $lib"
    fi
done

echo ""
echo "--- Step 5: Checking AndroidManifest.xml ---"
if [ -f "AndroidManifest.xml" ]; then
    echo "Manifest permissions found:"
    grep -o 'android:name="android.permission\.[^"]*"' AndroidManifest.xml | sort -u || true
    
    echo ""
    echo "Application configuration:"
    grep -o 'android:label="[^"]*"' AndroidManifest.xml | head -1 || true
    grep -o 'android:icon="[^"]*"' AndroidManifest.xml | head -1 || true
    
    # Check MainActivity export status
    echo ""
    echo "MainActivity configuration:"
    if grep -q 'android:name=".MainActivity"' AndroidManifest.xml; then
        if grep -q 'android:exported="true"' AndroidManifest.xml; then
            echo "  ✓ MainActivity is exported"
        else
            echo "  ⚠ MainActivity export status unclear"
        fi
    fi
else
    echo "WARNING: AndroidManifest.xml not found in APK"
fi

echo ""
echo "--- Step 6: Source jniLibs comparison ---"
if [ -d "/workspace/$SOURCE_JNILIBS_DIR" ]; then
    echo "Source jniLibs directory exists"
    find "/workspace/$SOURCE_JNILIBS_DIR" -name "*.so" -type f 2>/dev/null | while read -r src_lib; do
        LIB_NAME=$(basename "$src_lib")
        ABI_DIR=$(basename "$(dirname "$src_lib")")
        APK_LIB="lib/$ABI_DIR/$LIB_NAME"
        
        if [ -f "$APK_LIB" ]; then
            SRC_SIZE=$(stat -c%s "$src_lib" 2>/dev/null || stat -f%z "$src_lib")
            APK_SIZE=$(stat -c%s "$APK_LIB" 2>/dev/null || stat -f%z "$APK_LIB")
            
            if [ "$SRC_SIZE" -eq "$APK_SIZE" ]; then
                echo "  ✓ $ABI_DIR/$LIB_NAME (sizes match: $SRC_SIZE bytes)"
            else
                echo "  ✗ SIZE MISMATCH: $ABI_DIR/$LIB_NAME (source=$SRC_SIZE, apk=$APK_SIZE)"
            fi
        else
            echo "  ✗ NOT PACKAGED: $ABI_DIR/$LIB_NAME (in source but not in APK)"
        fi
    done
else
    echo "Source jniLibs directory not found at /workspace/$SOURCE_JNILIBS_DIR"
fi

echo ""
echo "--- Step 7: ProGuard/R8 Rules Check ---"
PROGUARD_RULES="/workspace/app/android/app/proguard-rules.pro"
if [ -f "$PROGUARD_RULES" ]; then
    echo "ProGuard rules file exists"
    
    # Check critical rules
    CRITICAL_PATTERNS=(
        "NativeBridge"
        "VoiceNative"
        "onnxruntime"
        "opencv"
        "native <methods>"
    )
    
    for pattern in "${CRITICAL_PATTERNS[@]}"; do
        if grep -q "$pattern" "$PROGUARD_RULES"; then
            echo "  ✓ Rule present for: $pattern"
        else
            echo "  ⚠ Missing rule for: $pattern"
        fi
    done
else
    echo "ProGuard rules file not found"
fi

echo ""
echo "=========================================="
echo "SUMMARY"
echo "=========================================="
echo "Expected libraries: ${#EXPECTED_LIBS[@]}"
echo "Found libraries: ${#FOUND_LIBS[@]}"
echo "Missing libraries: ${#MISSING_LIBS[@]}"

if [ ${#MISSING_LIBS[@]} -gt 0 ]; then
    echo ""
    echo "Missing:"
    printf '  %s\n' "${MISSING_LIBS[@]}"
    echo ""
    echo "RESULT: VERIFICATION FAILED"
    echo ""
    echo "This typically indicates:"
    echo "  1. Native libraries were not built before packaging"
    echo "  2. Libraries are in wrong location (should be in src/main/jniLibs/<abi>/)"
    echo "  3. Gradle packaging configuration excludes the libraries"
    echo "  4. Voice native library (libloose_ends_voice.so) not copied from voice-native/"
    echo ""
    echo "FIX: Run the CI workflow steps to build and copy all native libs:"
    echo "  - Build core native: cargo build --release --target <abi> --features jni,neural"
    echo "  - Build voice native: cargo build --release --target <abi>"
    echo "  - Copy to jniLibs for all ABIs (arm64-v8a, armeabi-v7a, x86_64)"
    echo "  - Rebuild APK: flutter build apk --release"
    exit 1
else
    echo ""
    echo "RESULT: VERIFICATION PASSED"
    echo "All expected native libraries are present in the APK."
    exit 0
fi
