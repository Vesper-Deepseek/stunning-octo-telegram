# Release

Application versions live in `app/pubspec.yaml`.

The release workflow now runs directly on every push to `main` and also supports manual dispatch.

For each release it:

1. builds arm64-v8a, armeabi-v7a, and x86_64 native libraries;
2. creates the universal APK;
3. verifies the expected native libraries;
4. creates a SHA-256 checksum;
5. publishes APK + checksum to GitHub Releases;
6. downloads the public APK URL again;
7. verifies checksum and file size.

The current release is **v1.0.5**.
