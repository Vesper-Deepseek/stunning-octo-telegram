# Release

Application versions live in `app/pubspec.yaml`.

The release workflow:

1. builds all supported Android native ABIs;
2. builds the universal APK;
3. checks the expected native libraries;
4. creates a SHA-256 checksum;
5. uploads APK + checksum to GitHub Releases;
6. downloads the public release URL again;
7. verifies checksum and file size.

The published MVP release is **v1.0.4**.
