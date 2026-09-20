# Third-party notices

Loose Ends is distributed under GPL-3.0-only for project-owned material. Third-party components remain under their respective upstream licenses.

The standard license references used by this repository are kept in [LICENSES/](LICENSES/):

- Apache-2.0: [LICENSES/Apache-2.0.txt](LICENSES/Apache-2.0.txt)
- MIT: [LICENSES/MIT.txt](LICENSES/MIT.txt)
- Project license: [LICENSE](LICENSE) and [LICENSES/GPL-3.0.txt](LICENSES/GPL-3.0.txt)
- Dependency mapping and attribution: [LICENSES/THIRD_PARTY.md](LICENSES/THIRD_PARTY.md)

## PaddleOCR / OCR model assets

The screenshot OCR implementation adapts components of the PaddleOCR Android deployment code and uses PP-OCRv5 ONNX model assets from the gitakoos/ocr-models release repository.

Copyright (C) 2026 PaddlePaddle Authors and contributors.
Copyright (C) 2026 Akoos.

The PaddleOCR-derived code and OCR model assets are licensed under the Apache License, Version 2.0. The asset repository's NOTICE identifies:

- det.onnx — PP-OCRv5 mobile text-detection network
- rec_latin.onnx — Latin-script PP-OCRv5 mobile recognition network
- ppocrv5_latin_dict.txt — recognition dictionary

Sources:

- https://github.com/PaddlePaddle/PaddleOCR
- https://github.com/gitakoos/ocr-models

The exact downloaded asset bytes are pinned and SHA-256 verified in OcrModelManager.kt.

## Runtime dependencies

- ONNX Runtime Android 1.21.1 — MIT License.
- OpenCV Android 4.10.0 — Apache License 2.0 for the OpenCV 4.x code line used by this app.
- whispercpp 0.2.1 — MIT OR Apache-2.0.
- flutter_rust_bridge 2.13.0 — MIT.
- Vendored utilityai/llama-cpp-rs source — MIT OR Apache-2.0.

These dependencies are used for local inference and native integration. Screenshot images are copied to the app cache for OCR processing and are not uploaded by the OCR flow.

For the full attribution map, see [LICENSES/THIRD_PARTY.md](LICENSES/THIRD_PARTY.md).
