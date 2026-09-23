#!/usr/bin/env bash
set -euo pipefail

mkdir -p artifacts

adb wait-for-device
adb shell settings put global window_animation_scale 0
adb shell settings put global transition_animation_scale 0
adb shell settings put global animator_duration_scale 0

echo "[smoke] Installing release APK"
adb install -r artifacts/app-release.apk
adb shell pm grant com.looseends.loose_ends android.permission.POST_NOTIFICATIONS || true
adb shell am force-stop com.looseends.loose_ends
adb logcat -c

echo "[smoke] Launching release APK"
adb shell am start -W -n com.looseends.loose_ends/.MainActivity > artifacts/release_start.txt 2>&1
sleep 8
adb shell pidof com.looseends.loose_ends > artifacts/release_app_pid.txt 2>&1 || true
adb shell dumpsys window windows > artifacts/release_windows.txt 2>&1 || true
adb shell dumpsys activity activities > artifacts/release_activities.txt 2>&1 || true
adb exec-out screencap -p > artifacts/release_startup.png 2> artifacts/release_screenshot-error.txt || true
adb shell uiautomator dump /sdcard/window.xml > artifacts/release_uiautomator.txt 2>&1 || true
adb pull /sdcard/window.xml artifacts/release_window.xml > artifacts/release_uiautomator-pull.txt 2>&1 || true
adb logcat -d -v threadtime > artifacts/release_launch_logcat.txt

test -s artifacts/release_app_pid.txt
grep -Eq '^[0-9]+' artifacts/release_app_pid.txt
if grep -Eq 'FATAL EXCEPTION|Process: com\.looseends\.loose_ends|UnsatisfiedLinkError|NoClassDefFoundError|SIGSEGV|SIGABRT' artifacts/release_launch_logcat.txt; then
  echo "[smoke] Release launch logcat contains a fatal/native error"
  exit 1
fi
echo "[smoke] Release APK sanity check passed"

echo "[smoke] Disabling device networking for offline integration test"
adb shell svc wifi disable || true
adb shell svc data disable || true

echo "[smoke] Starting Flutter integration_test"
set +e
timeout --foreground --signal=TERM --kill-after=30s 7m   bash -lc 'cd app && flutter test --no-pub --verbose integration_test/mvp_smoke_test.dart -d "$ANDROID_SERIAL" --timeout 5m' > artifacts/flutter_integration_test.log 2>&1
test_rc=$?
set -e
cat artifacts/flutter_integration_test.log

if [ "$test_rc" -ne 0 ]; then
  echo "[smoke] Flutter integration_test failed or timed out (exit $test_rc)"
  adb shell svc wifi enable || true
  adb shell svc data enable || true
  adb shell pidof com.looseends.loose_ends > artifacts/integration_app_pid.txt 2>&1 || true
  adb shell dumpsys activity activities > artifacts/integration_activities.txt 2>&1 || true
  adb logcat -d -v threadtime > artifacts/integration_logcat.txt || true
  exit "$test_rc"
fi
echo "[smoke] Flutter integration_test passed"

echo "[smoke] Re-launching release APK after integration_test"
adb shell svc wifi enable || true
adb shell svc data enable || true
adb install -r artifacts/app-release.apk
adb shell pm grant com.looseends.loose_ends android.permission.POST_NOTIFICATIONS || true
adb shell am force-stop com.looseends.loose_ends
adb logcat -c
adb shell am start -W -n com.looseends.loose_ends/.MainActivity > artifacts/final_release_start.txt 2>&1
sleep 8
adb shell pidof com.looseends.loose_ends > artifacts/final_release_pid.txt 2>&1
adb exec-out screencap -p > artifacts/final_release.png 2> artifacts/final_screenshot-error.txt || true
adb logcat -d -v threadtime > artifacts/final_release_logcat.txt

test -s artifacts/final_release_pid.txt
grep -Eq '^[0-9]+' artifacts/final_release_pid.txt
if grep -Eq 'FATAL EXCEPTION|Process: com\.looseends\.loose_ends|UnsatisfiedLinkError|NoClassDefFoundError|SIGSEGV|SIGABRT' artifacts/final_release_logcat.txt; then
  echo "[smoke] Final release launch logcat contains a fatal/native error"
  exit 1
fi
echo "[smoke] Final release APK sanity check passed"
