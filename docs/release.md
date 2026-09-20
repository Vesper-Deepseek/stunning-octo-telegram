# Release process

## Versioning

The application version is stored in `app/pubspec.yaml`.

Example:

```yaml
version: 1.0.6+1
```

The GitHub release workflow converts the semantic version to a Git tag:

```text
1.0.6 -> v1.0.6
```

## OpenCV packaging

Before the Flutter APK build, the release job runs the Android Gradle release task and explicitly copies `libopencv_java4.so` from Gradle build intermediates into each ABI-specific `jniLibs/` directory. The final APK verification also requires the OpenCV library for arm64-v8a, armeabi-v7a, and x86_64.

## Trigger

The release workflow runs on:

- every push to `main`
- manual `workflow_dispatch`

It no longer depends on a completed CI `workflow_run` event.

## Universal APK

The release job builds native Rust libraries for arm64-v8a, armeabi-v7a, and x86_64 and packages them into one universal Flutter APK.

The v1.0.6 release notes identify the universal APK contents, including the main Rust bridge and isolated Whisper voice library.

## Integrity verification

The workflow creates `app-release.apk.sha256`, uploads it with the APK, then downloads the public release URL again and compares SHA-256 and file size.

A successful release therefore verifies both artifact creation and public GitHub asset retrieval.

## v1.0.6

v1.0.6 is the current release generated from the direct push-to-main release pipeline.
