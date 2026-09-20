# Loose Ends

Privacy-first, on-device commitment tracking for Android.

Loose Ends turns notes, messages, screenshots, and voice captures into reviewable commitments. The app keeps the durable commitment store local, uses a Rust/SQLite core, and provides an Android/Flutter UI for capture, review, tracking, and resolution.

## MVP release

**Current release: v1.0.4**

The universal Android APK was built and published from commit `e164f70d8f579cd6db9eb031b5acbeab2d8efbc1`.

[Download the v1.0.4 APK](https://github.com/Vesper-Deepseek/stunning-octo-telegram/releases/download/v1.0.4/app-release.apk) · [Release notes](https://github.com/Vesper-Deepseek/stunning-octo-telegram/releases/tag/v1.0.4)

The release workflow verifies that the APK is uploaded to GitHub Releases, can be downloaded through its public release URL, and matches the build checksum.

## What is in the MVP

- Capture and manual commitment entry.
- Rule-based offline extraction with a review/confirmation gate.
- Local Rust + SQLite persistence.
- Separate views for **You Owe** and **Owed to You**.
- Review, edit, confirm, dismiss, resolve, and snooze flows.
- Local reminder/scheduling support in the app flow.
- Screenshot OCR using an on-device ONNX path.
- Local voice recording and Whisper-based transcription.
- Optional local neural-model infrastructure, with the rules path remaining the reliable baseline.
- Android native libraries for arm64-v8a, armeabi-v7a, and x86_64 in the universal release.
- SHA-256 verification for model assets and the published release APK.

## Design principles

### Private by default

Commitment content and extraction are designed around local processing. OCR copies images into app-local cache for processing rather than uploading them to a service.

### Review before truth

Automatic extraction creates a draft. A draft does not become a saved commitment until the user confirms it.

### Native work stays isolated

The Rust core owns durable facts and planning. Android native bridges expose the functionality to Flutter. Whisper is kept in a separate native library because it carries its own native dependency stack.

## Repository layout

```text
.
├── app/                    # Flutter application + Android project
│   ├── lib/                # Flutter UI, models, bridge code
│   ├── native/             # Main Rust Android native library
│   ├── voice-native/       # Isolated Whisper native library
│   └── android/            # Android host project
├── core/                   # Rust commitment engine, SQLite, extraction, planning
├── eval/                   # Offline evaluation dataset and reports
├── scripts/                # Development and evaluation utilities
├── vendor/                 # Vendored llama-cpp-rs source
├── docs/                   # Project documentation
├── LICENSE                 # Project license identification
├── LICENSES/               # Third-party license texts/notices
└── THIRD_PARTY_NOTICES.md  # High-level dependency attribution
```

## Development prerequisites

Install:

- Flutter stable with Dart 3.12+
- Rust stable
- Android SDK and Android NDK 28.2.13676358
- Java 17
- An Android device or emulator for manual runtime testing

## Local development

```bash
cd app
flutter pub get
flutter analyze
flutter test
```

Rust:

```bash
cd core
cargo fmt -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

Generate Flutter/Rust bridge code:

```bash
cd app
flutter_rust_bridge_codegen generate --config-file flutter_rust_bridge.yaml
```

## Android build

For a local arm64 build, the release CI path is the reference configuration:

1. Install the Android NDK.
2. Add the Rust Android target `aarch64-linux-android`.
3. Build the Rust native library.
4. Build the Flutter APK.

The complete reproducible CI configuration is in [.github/workflows/ci.yml](.github/workflows/ci.yml) and [.github/workflows/release.yml](.github/workflows/release.yml).

## Documentation

- [Documentation index](docs/README.md)
- [Architecture](docs/architecture.md)
- [Development guide](docs/development.md)
- [Privacy and security](docs/privacy.md)
- [OCR and voice](docs/ocr-and-voice.md)
- [Release process](docs/release.md)
- [Troubleshooting](docs/troubleshooting.md)
- [MVP and full-spec status](docs/mvp-spec-status.md)
- [Wiki source](docs/wiki/Home.md)

## Licensing

Loose Ends is released under the **GNU General Public License v3.0**. See [LICENSE](LICENSE).

Third-party components retain their upstream licenses. The repository keeps the applicable Apache-2.0 and MIT license references in [LICENSES/](LICENSES/) and maps individual projects in [LICENSES/THIRD_PARTY.md](LICENSES/THIRD_PARTY.md).

See also [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for attribution and model-asset information.

## Status

v1.0.4 is the published MVP release. Broader post-MVP work can include deeper end-to-end device validation, additional model/runtime integrations, and further hardening; the release itself is built and validated by CI and the public download verification step.
