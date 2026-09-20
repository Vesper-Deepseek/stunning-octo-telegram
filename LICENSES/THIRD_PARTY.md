# Third-party licenses and notices

This directory keeps the applicable standard license families in one place. Individual projects point to those license texts and list project-specific attribution.

## Direct project/repository dependencies

| Project | Version / use | License | Repository |
| --- | --- | --- | --- |
| utilityai/llama-cpp-rs | vendored llama-cpp-2 / llama-cpp-sys-2 | MIT OR Apache-2.0 | https://github.com/utilityai/llama-cpp-rs |
| whispercpp | 0.2.1; isolated voice native library | MIT OR Apache-2.0 | https://github.com/findit-studio/whispercpp |
| PaddleOCR | OCR-derived code and PP-OCR material | Apache-2.0 | https://github.com/PaddlePaddle/PaddleOCR |
| gitakoos/ocr-models | PP-OCRv5 ONNX model assets | Apache-2.0 | https://github.com/gitakoos/ocr-models |
| ONNX Runtime | Android inference runtime 1.21.1 | MIT | https://github.com/microsoft/onnxruntime |
| OpenCV | Android runtime 4.10.0 | Apache-2.0 for the OpenCV 4.x line used here | https://github.com/opencv/opencv |
| flutter_rust_bridge | bridge generator/runtime 2.13.0 | MIT | https://github.com/fzyzcjy/flutter_rust_bridge |

## Project-specific attribution

### llama-cpp-rs

The repository vendors source from utilityai/llama-cpp-rs. Its upstream repository provides both MIT and Apache-2.0 license files.

The vendor directory retains the upstream `LICENSE-MIT` and `LICENSE-APACHE` files.

### whispercpp

The `whispercpp` crate used by this project is published under MIT and Apache-2.0.

Upstream copyright attribution:
Copyright (c) 2026 FinDIT Studio authors.

Upstream:
https://github.com/findit-studio/whispercpp

### PaddleOCR

Loose Ends uses OCR-derived Android material and PP-OCRv5 assets.

Copyright attribution:
Copyright (C) PaddlePaddle Authors.

Upstream:
https://github.com/PaddlePaddle/PaddleOCR

### gitakoos/ocr-models

The OCR model repository states that its model files and dictionary are Apache-2.0 and requests that its LICENSE and NOTICE attribution be retained.

Upstream:
https://github.com/gitakoos/ocr-models

The exact OCR assets used by Loose Ends are documented in [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).

### ONNX Runtime

Android dependency: ONNX Runtime 1.21.1.

Copyright:
Copyright (c) Microsoft Corporation

License: MIT.

Upstream:
https://github.com/microsoft/onnxruntime

### OpenCV

Android dependency: OpenCV 4.10.0.

The OpenCV 4.x code line used by this application is distributed under Apache-2.0.

Upstream:
https://github.com/opencv/opencv

OpenCV also ships separate third-party notices for components contained in its distribution.

### flutter_rust_bridge

The project uses flutter_rust_bridge 2.13.0 for generated Rust/Flutter integration.

Upstream:
https://github.com/fzyzcjy/flutter_rust_bridge

License: MIT.

## Single-copy rule

Apache-2.0 and MIT are referenced once in this directory and reused through this attribution map. Project-specific copyright and NOTICE information is kept here rather than duplicating standard license texts for every dependency.

## Dependency boundary

This document covers the direct project-level dependencies relevant to the application architecture. Cargo and Gradle also bring in transitive dependencies. Those remain subject to their own upstream license metadata and are not silently relicensed by Loose Ends.

For a redistribution build, preserve any additional NOTICE files required by the final dependency graph.
