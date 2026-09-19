# Loose Ends MVP verification checklist

Status is updated from GitHub CI/emulator evidence. A feature is only marked PASS when the automated run exercises the user-visible behavior or a deterministic lower-level test proves it.

## Baseline / build

| # | Check | Status | Evidence |
|---|---|---|---|
| 1 | Rust core unit tests | PASS (previous run) | CI Rust job completed successfully on the prior main revision |
| 2 | Rust Clippy / formatting | PASS (previous run) | CI Rust job completed successfully |
| 3 | Flutter analyze | PASS (previous run) | CI Flutter Analyze completed successfully |
| 4 | Flutter widget test suite | RETEST | New capture/media UI is included; awaiting fresh CI result |
| 5 | Android x86_64 native Rust build | RETEST | Smoke workflow now targets emulator ABI |
| 6 | Android release APK build | RETEST | Smoke workflow builds and installs the release artifact |

## User flows on the Android emulator

| # | Check | Status |
|---|---|---|
| 7 | App starts without an account | RETEST |
| 8 | First-run onboarding appears | RETEST |
| 9 | Onboarding "Skip for now" reaches Home | RETEST |
| 10 | Home exposes Capture / Review / You Owe / Owed to You / Settings | RETEST |
| 11 | Manual entry creates a commitment | RETEST |
| 12 | Owed to You direction stores the other person as debtor | RETEST |
| 13 | You Owe direction stores the other person as recipient | RETEST |
| 14 | Capture -> rule extraction works offline | RETEST |
| 15 | Review shows provenance/confidence metadata | RETEST |
| 16 | Review Edit changes commitment fields before confirmation | RETEST |
| 17 | Review Confirm promotes draft to a fact | RETEST |
| 18 | Review Dismiss does not create a fact | RETEST |
| 19 | Resolve action removes an open commitment | RETEST |
| 20 | Snooze action removes the commitment from the open view | RETEST |
| 21 | Future-date reminder can be scheduled | RETEST |
| 22 | Owed-to-user view displays the correct direction | RETEST |
| 23 | Settings -> AI model opens | RETEST |
| 24 | Qwen 2.5 1.5B model is listed | RETEST |
| 25 | MiniCPM5 2B model is listed | RETEST |
| 26 | Mobile-data override is OFF by default and user-toggleable | RETEST |

## Privacy / model-manager checks

| # | Check | Status |
|---|---|---|
| 27 | Commitment capture works with device networking disabled | RETEST |
| 28 | Model download requires explicit user action | CODE VERIFIED; emulator dialog RETEST |
| 29 | Wi-Fi is default for model download | CODE VERIFIED; emulator path RETEST |
| 30 | SHA-256 is checked before an official model becomes ready | CODE VERIFIED; live binary verification not yet completed |
| 31 | Partial download is deleted on cancellation/failure | CODE VERIFIED; emulator cancellation RETEST |
| 32 | Model delete/select/import paths are present | CODE VERIFIED; interactive import RETEST |

## Features still requiring end-to-end implementation/validation

| # | Check | Status |
|---|---|---|
| 33 | Screenshot capture -> offline OCR -> Capture text | CODE COMPLETE; RETEST | PP-OCRv5 ONNX path implemented; emulator execution still pending |
| 34 | Voice recording -> local Whisper transcription -> Capture text | CODE COMPLETE; RETEST | Local WAV recorder + whispercpp path implemented; emulator execution still pending |
| 35 | Flutter production path uses flutter_rust_bridge as the primary boundary | PARTIAL | Existing Android production path remains MethodChannel/JNI |
| 36 | No commitment data leaves the device under an instrumented network test | PARTIAL: offline capture test + code audit; packet-level proof pending |
| 37 | New OCR/Whisper model/runtime licensing uses MIT/Apache-only components | CODE VERIFIED | OCR models Apache-2.0; ONNX Runtime MIT; OpenCV Apache-2.0; whispercpp MIT/Apache-2.0 |
| 38 | Full universal release APK contains arm64-v8a, armeabi-v7a, and x86_64 native libraries | RETEST |

## Current score

The media features are implemented in source. Emulator execution, packet-level egress proof, and the full universal release remain intentionally RETEST/pending until their CI evidence exists.
