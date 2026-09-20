# Loose Ends

Privacy-first, on-device commitment tracking for Android.

Loose Ends turns notes, messages, screenshots, and voice captures into reviewable commitments. The app keeps the durable commitment store local, uses a Rust/SQLite core, and provides an Android/Flutter UI for capture, review, tracking, and resolution.

## MVP release

**Current release: v1.0.5**

The universal Android APK is built and published directly from pushes to `main` by the release workflow.

[Download the v1.0.5 APK](https://github.com/Vesper-Deepseek/stunning-octo-telegram/releases/download/v1.0.5/app-release.apk) · [Release notes](https://github.com/Vesper-Deepseek/stunning-octo-telegram/releases/tag/v1.0.5)

The release workflow verifies that the APK is uploaded to GitHub Releases, can be downloaded through its public release URL, and matches the build checksum.

## What is in the MVP

- Capture and manual commitment entry.
- Rule-based offline extraction with a review/confirmation gate.
- Local Rust + SQLite persistence.
- Separate views for **You Owe** and **Owed to You**.
- Review, edit, confirm, dismiss, resolve, and snooze flows.
- Screenshot OCR using an on-device ONNX path.
- Local voice recording and Whisper-based transcription.
- Optional local neural-model infrastructure.
- Android native libraries for arm64-v8a, armeabi-v7a, and x86_64 in the universal release.
- SHA-256 verification for model assets and the published release APK.

## Repository layout

```text
.
├── app/                    # Flutter application + Android project
├── core/                   # Rust commitment engine, SQLite, extraction, planning
├── eval/                   # Offline evaluation dataset and reports
├── scripts/                # Development and evaluation utilities
├── vendor/                 # Vendored llama-cpp-rs source
├── docs/                   # Project documentation
├── LICENSE                 # GPL-3.0-only project license
├── LICENSES/               # Third-party license references/notices
└── THIRD_PARTY_NOTICES.md  # High-level dependency attribution
```

## Development prerequisites

- Flutter stable with Dart 3.12+
- Rust stable
- Android SDK and Android NDK 28.2.13676358
- Java 17

## Local development

```bash
cd app
flutter pub get
flutter analyze
flutter test
```

```bash
cd core
cargo fmt -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

## Documentation

- [Documentation index](docs/README.md)
- [Architecture](docs/architecture.md)
- [Development guide](docs/development.md)
- [Privacy and security](docs/privacy.md)
- [OCR and voice](docs/ocr-and-voice.md)
- [Release process](docs/release.md)
- [Troubleshooting](docs/troubleshooting.md)
- [MVP/spec status](docs/mvp-spec-status.md)

## Licensing

Loose Ends is released under the **GNU General Public License v3.0**. See [LICENSE](LICENSE).

Third-party components retain their upstream licenses. See [LICENSES/THIRD_PARTY.md](LICENSES/THIRD_PARTY.md) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
