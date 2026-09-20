# Privacy and security

## Data model

Loose Ends is designed around local-first processing.

Commitments, drafts, provenance, confidence, and reminder metadata are stored in the local Rust/SQLite layer. The core local workflow does not require a user account.

## Capture sources

### Text

Typed or pasted text enters the extraction path and becomes a reviewable draft.

### Screenshot OCR

A screenshot is handled locally. OCR processing uses local runtime/model assets. The image is copied to app-local cache for processing.

### Voice

Audio is recorded locally and passed to the isolated Whisper native library for local transcription.

## Network boundaries

The application has model-management flows that may download model assets when the user explicitly requests them. This is separate from commitment-data synchronization.

Network-egress behavior should be treated as a testable property; packet-level verification is a separate validation activity.

## Model integrity

Official model assets are not treated as ready merely because a file exists. Expected SHA-256 values are verified before an official model is marked ready.

## User confirmation

Automatic extraction is not an automatic write-to-database path. Users review and confirm extracted facts before they become durable commitments.

## Reporting

Do not include private commitment content in public bug reports. Use synthetic data and provide version, device, Android version, and reproduction steps.
