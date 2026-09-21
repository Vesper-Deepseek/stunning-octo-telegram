#!/bin/bash
# test-on-emulator.sh - Test APK installation and native library loading on Android emulator
# 
# Prerequisites:
# - Android SDK with platform-tools installed
# - Emulator running (adb devices shows a device)
# - APK file available

set -e

APK_FILE="${1:-}"
PACKAGE_NAME="com.looseends.loose_ends"
MAIN_ACTIVITY=".MainActivity"

usage() {
    echo "Usage: $0 [apk-file]"
    echo ""
    echo "Tests APK installation and native library loading on connected emulator/device"
    echo ""
    echo "If no APK file is specified, searches common build locations."
    exit 1
}

echo "=========================================="
echo "Android Emulator Testing Script"
echo "=========================================="
echo ""

# Check for adb
if ! command -v adb &> /dev/null; then
    echo "ERROR: adb (Android Debug Bridge) not found."
    echo "Please install Android SDK platform-tools."
    exit 1
fi

# Find or validate APK file
if [ -z "$APK_FILE" ]; then
    if [ -f "/workspace/app/build/flutter.apk" ]; then
        APK_FILE="/workspace/app/build/flutter.apk"
    elif [ -f "/workspace/app/android/app/build/outputs/apk/debug/app-debug.apk" ]; then
        APK_FILE="/workspace/app/android/app/build/outputs/apk/debug/app-debug.apk"
    elif [ -f "/workspace/app/android/app/build/outputs/apk/release/app-release.apk" ]; then
        APK_FILE="/workspace/app/android/app/build/outputs/apk/release/app-release.apk"
    else
        echo "ERROR: No APK file specified and none found in default locations."
        echo ""
        echo "Build the APK first using one of:"
        echo "  cd /workspace/app/android && ./gradlew assembleDebug"
        echo "  cd /workspace/app/android && ./gradlew assembleRelease"
        echo ""
        usage
    fi
fi

if [ ! -f "$APK_FILE" ]; then
    echo "ERROR: APK file not found: $APK_FILE"
    usage
fi

echo "APK File: $APK_FILE"
echo ""

# Check for connected devices/emulators
echo "--- Step 1: Checking for connected devices/emulators ---"
DEVICES=$(adb devices | grep -v "^List" | grep "device$" | wc -l)
if [ "$DEVICES" -eq 0 ]; then
    echo "ERROR: No Android emulator or device connected."
    echo ""
    echo "To start an emulator:"
    echo "  1. Open Android Studio > Device Manager"
    echo "  2. Create/start an emulator (recommend: Pixel 6, API 34, arm64-v8a)"
    echo ""
    echo "Or use command line:"
    echo "  emulator -list-avds  # List available AVDs"
    echo "  emulator -avd <avd_name> &  # Start emulator"
    echo ""
    exit 1
fi

echo "Connected devices/emulators:"
adb devices | grep -v "^List"
echo ""

# Get first device serial
DEVICE_SERIAL=$(adb devices | grep -v "^List" | grep "device$" | head -1 | awk '{print $1}')
echo "Using device: $DEVICE_SERIAL"
echo ""

# Uninstall existing app (clean slate)
echo "--- Step 2: Uninstalling existing app (if any) ---"
adb -s "$DEVICE_SERIAL" uninstall "$PACKAGE_NAME" 2>/dev/null || echo "App not previously installed"
echo ""

# Install APK
echo "--- Step 3: Installing APK ---"
adb -s "$DEVICE_SERIAL" install -r "$APK_FILE"
if [ $? -ne 0 ]; then
    echo "ERROR: Installation failed!"
    echo ""
    echo "Common causes:"
    echo "  - APK built for different ABI than emulator"
    echo "  - Insufficient storage on emulator"
    echo "  - Signature conflict with existing app"
    exit 1
fi
echo "Installation successful!"
echo ""

# Verify native libraries are present
echo "--- Step 4: Verifying native libraries in APK ---"
adb -s "$DEVICE_SERIAL" shell "ls -la /data/app/${PACKAGE_NAME}-*/lib/arm64/" 2>/dev/null || {
    echo "WARNING: Could not list native libraries from installed app"
    echo "This may indicate packaging issues"
}
echo ""

# Launch app and capture logs
echo "--- Step 5: Launching app and capturing logs ---"
echo "Starting logcat capture (filtering for Loose Ends tags)..."
echo ""

# Clear old logs
adb -s "$DEVICE_SERIAL" logcat -c

# Start app
adb -s "$DEVICE_SERIAL" shell am start -n "${PACKAGE_NAME}/${MAIN_ACTIVITY}"
echo "App launched. Monitoring logs for 10 seconds..."
echo ""

# Capture logs with filtering
timeout 10 adb -s "$DEVICE_SERIAL" logcat -s \
    "NativeBridge:I" \
    "VoiceNative:I" \
    "VoiceNativeBridge:I" \
    "loose_ends:I" \
    "AndroidRuntime:E" \
    "FATAL:*" 2>&1 || true

echo ""
echo "--- Step 6: Checking for native library errors ---"
ERRORS=$(timeout 5 adb -s "$DEVICE_SERIAL" logcat -d | grep -iE "(UnsatisfiedLinkError|link|native|so\.|jni)" | tail -20 || true)
if [ -n "$ERRORS" ]; then
    echo "Potential native library issues found:"
    echo "$ERRORS"
else
    echo "No obvious native library errors detected in recent logs."
fi
echo ""

# Check app process
echo "--- Step 7: Checking app process status ---"
adb -s "$DEVICE_SERIAL" shell "pidof com.looseends.loose_ends" && echo "App process is running" || echo "App process not found (may have crashed)"
echo ""

echo "=========================================="
echo "Testing Complete"
echo "=========================================="
echo ""
echo "Next steps:"
echo "  - Manually test app functionality on emulator"
echo "  - Check full logs: adb logcat | grep -i loose_ends"
echo "  - To stop: adb emu kill"
echo ""
