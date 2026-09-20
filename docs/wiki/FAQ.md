# FAQ

## Where is my data stored?

Core commitment data is stored in the local Rust/SQLite layer.

## Does OCR upload screenshots?

The OCR flow copies screenshots into app-local cache for local processing. It is not designed as a cloud OCR service.

## Does voice use a cloud transcription service?

The MVP voice path uses the isolated local Whisper native library.

## Why does a capture become a draft first?

The draft/review boundary prevents automatic extraction from silently creating durable commitments.

## Which license covers Loose Ends?

Project-owned material is GPL-3.0-only. Third-party projects retain their upstream licenses, documented in `LICENSES/THIRD_PARTY.md`.
