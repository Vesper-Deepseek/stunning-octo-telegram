# Architecture

Flutter provides the UI. Rust owns the durable commitment model and SQLite persistence.

The Android layer connects Flutter to native code. The main native bridge is separated from the Whisper library:

- `loose_ends_native.so` — main Rust/llama path.
- `loose_ends_voice.so` — isolated Whisper path.

OCR and voice feed text into the same capture/review model rather than creating independent databases.

The system deliberately distinguishes drafts from confirmed facts.
