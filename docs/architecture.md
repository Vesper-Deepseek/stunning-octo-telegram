# Architecture

## System overview

Loose Ends has four major layers:

1. **Flutter UI** — capture screens, review/edit flows, commitment lists, settings, and model-management UX.
2. **Android host** — permissions, media/audio capture, JNI/MethodChannel integration, and packaging.
3. **Rust native layer** — commitment operations exposed to Android/Flutter and isolated native libraries.
4. **Rust core** — SQLite persistence, extraction rules, planning, and optional neural/OCR/voice features.

```text
Flutter UI
   │
   └── Android host
          ├── loose_ends_native.so
          │       └── loose-ends-core
          │
          └── loose_ends_voice.so
                  └── loose-ends-core + whispercpp

loose-ends-core
   ├── SQLite store
   ├── rule extraction
   ├── draft/review/confirmation model
   ├── planner/reminder logic
   └── optional neural / OCR / voice features
```

## Persistent data

The Rust store is the source of truth for commitments. Extracted content first enters the draft path with provenance/confidence metadata. The UI explicitly confirms a draft before it becomes a persistent fact.

## Native library split

The project intentionally keeps Whisper in a separate shared library:

- `loose_ends_native.so` contains the main Rust bridge and optional llama-based neural code.
- `loose_ends_voice.so` contains the voice/Whisper path.

This avoids collisions between native dependency stacks and keeps the voice dependency isolated.

## OCR

Screenshot OCR is an on-device path. Images are handled through app-local cache/storage and passed to a local inference runtime. Recognized text returns to the same capture/review pipeline used by text capture.

## Model management

Models are treated as data. Download/import flows are explicit, and official model readiness is guarded by checksum verification.

## Release architecture

The release workflow builds all three Android ABIs:

- arm64-v8a
- armeabi-v7a
- x86_64

Those native libraries are packaged into a universal APK. The workflow uploads the APK and checksum to GitHub Releases and fetches the public APK URL again to verify download integrity.
