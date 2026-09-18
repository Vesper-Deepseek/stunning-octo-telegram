# Loose Ends MVP — Specification Status

This document tracks the implementation against the full MVP specification supplied for this repository.

## Current state

The Android app is installable and has a working local Rust/SQLite commitment flow. The current production release is a universal APK, but the full specification is not yet complete.

The most important runtime finding came from the GitHub Actions Android emulator: the universal APK installed successfully, but the first runtime build crashed before Flutter rendered because `JNI_OnLoad` passed the wrong JavaVM pointer into JNI. That pointer handling has been corrected in the Rust bridge. A fresh emulator run against the corrected source is required before this runtime fix can be marked verified.

## Stage status

| Stage | Status | Evidence / remaining work |
| --- | --- | --- |
| 0 — Environment | Partial | Rust, Flutter, Android SDK/NDK and cross-compilation are exercised in GitHub Actions. A `cargo-ndk`-specific verification is still missing. |
| 1 — Evaluation harness | Implemented | 46 examples / 47 expected commitments are tracked in `eval/dataset.jsonl`. Baseline and hybrid reports are committed. |
| 2 — Symbolic core | Implemented | SQLite schema, drafts, confirmation gate, manual CRUD primitives, rule extractor and planner are covered by Rust tests. |
| 3 — Neural path | Partial | llama.cpp/llama-cpp-2 code and GBNF grammar exist, but Android still defaults to the rules path. A user-imported Qwen GGUF model path and production Android neural extraction still need to be wired and verified. Grammar-constrained sampling has not yet passed the repository's runtime validation. |
| 4 — Flutter UI | Partial | Text capture, review/edit UI, You Owe/Owed to You views and manual entry exist. Screenshot OCR, voice transcription and local follow-up notification UX are still incomplete. |
| 5 — FFI integration | Partial | Android currently uses a hand-written JNI + MethodChannel bridge. `flutter_rust_bridge` is declared but generated bindings/codegen are not yet the sole Flutter↔Rust boundary. |
| 6 — Device validation | Partial | A real Android emulator is exercised in CI with APK install/start, screenshot, UIAutomator and logcat collection. Physical mid-range device validation and direct network-egress measurement remain unverified. |

## Measured extraction results

### Stage 1 baseline

- Model: Qwen2.5-1.5B-Instruct Q4_K_M, greedy decoding
- Examples: 46
- Gold commitments: 47
- Clean JSON rate: 100%
- Precision: 37.0%
- Recall: 36.17%
- F1: 36.56%
- Matched party accuracy: 100%
- Matched direction accuracy: 100%
- Matched date accuracy: 70.6%
- Reported mean latency: 10.67 s

### Stage 1 hybrid

- Precision: 60.0%
- Recall: 57.45%
- F1: 58.70%
- Review-usable recall: 68.1%
- Party accuracy on matched: 92.6%
- Date accuracy on matched: 77.8%

### Stage 3 Rust report

- Precision: 59.6%
- Recall: 59.57%
- F1: 59.57%
- Review-usable recall: 72.3%
- Party accuracy on matched: 82.1%
- Date accuracy on matched: 57.1%
- Fabricated dates: 2
- Noise examples: 9
- Current report marks grammar-constrained sampling disabled pending a reliable runtime fix.

## License audit notes

The repository must not silently substitute licenses.

- llama.cpp is MIT-licensed upstream.
- whisper.cpp is MIT-licensed upstream.
- flutter_rust_bridge publishes under MIT.
- The higher-level edgenai llama_cpp-rs binding publishes MIT and Apache-2.0 licenses.
- edgenai whisper_cpp-rs publishes MIT and Apache-2.0 licenses.
- Tesseract itself is Apache-2.0, but its upstream dependency stack includes Leptonica under BSD-2-Clause. That is incompatible with an absolute MIT/Apache-only dependency rule, so Tesseract is intentionally not being integrated.
- ocrs publishes MIT/Apache-2.0 code. Its published model artifacts currently need a separate license determination before they can be redistributed as app assets.
- Direct Flutter packages that were not required by the current app have been removed from `pubspec.yaml` rather than retained unnecessarily.

## Architecture rules

- Flutter does not fabricate extraction facts when the native bridge is unavailable.
- Unconfirmed extractions remain in the Rust draft queue.
- A draft requires a decided direction before it can become a saved commitment.
- Automatically extracted drafts expose provenance and confidence metadata to the review UI.
- Persistent facts live in the Rust SQLite store.
- Python remains evaluation/offline tooling only.

## Remaining acceptance blockers

1. Production Android neural extraction using the Qwen GGUF model with a working GBNF-constrained decoder.
2. Screenshot capture + offline OCR with a license-compliant model/runtime combination.
3. Voice capture + offline whisper.cpp transcription.
4. Generated flutter_rust_bridge bindings replacing the hand-written Android bridge as the primary Flutter↔Rust boundary.
5. On-device local notification scheduling and reminder UI.
6. Fresh emulator validation of the corrected JNI bridge.
7. Physical-device performance and network-egress verification.

A green compile/test run alone does not mark these blockers complete.
