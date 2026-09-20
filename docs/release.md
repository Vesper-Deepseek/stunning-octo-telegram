# Release process

## Versioning

The application version is stored in `app/pubspec.yaml`.

Example:

```yaml
version: 1.0.4+1
```

The GitHub release workflow converts the semantic version to a Git tag:

```text
1.0.4 -> v1.0.4
```

## CI flow

A push to `main` runs CI. The release workflow is triggered from a successful CI run on the default branch.

Release conditions require a successful CI conclusion, the default branch, and the original repository as the workflow head repository.

## Universal APK

The release job builds native Rust libraries for:

- arm64-v8a
- armeabi-v7a
- x86_64

It then creates a universal Flutter APK.

Before publishing, the workflow checks that all six expected native libraries exist in the APK.

## Integrity verification

The workflow creates `app-release.apk.sha256`.

After upload, it fetches the public release URL again with `curl`, computes the downloaded SHA-256, and compares it with the build checksum and file size.

A successful release therefore proves both build success and public asset retrieval from GitHub Releases.

## Release checklist

1. Update `app/pubspec.yaml`.
2. Update version references in documentation when useful.
3. Push to `main`.
4. Confirm CI succeeds.
5. Confirm Release succeeds.
6. Confirm the release contains `app-release.apk` and `app-release.apk.sha256`.
7. Confirm the release log contains the public-download verification success message.
