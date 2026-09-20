# OCR and voice

## Screenshot OCR

The OCR path uses:

- PaddleOCR-derived material and PP-OCRv5 assets
- ONNX Runtime Android
- OpenCV Android
- local image preprocessing

The app pins and checksum-verifies the OCR model asset bytes before use.

OCR output returns to the normal capture/review path.

## Voice transcription

Voice capture records local WAV audio. The audio is passed to the isolated `loose_ends_voice` native library, which uses `whispercpp` for local speech-to-text inference.

Whisper is isolated from the main native library because its native dependency stack includes its own ggml components.

## Failure behavior

When a native media library is unavailable, the app should return an unavailable/empty result instead of fabricating OCR text or a transcript.

## Licensing

See:

- [LICENSES/THIRD_PARTY.md](../LICENSES/THIRD_PARTY.md)
- [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md)

Do not add an OCR or voice repository without recording its upstream license, version/source, and redistribution requirements first.
