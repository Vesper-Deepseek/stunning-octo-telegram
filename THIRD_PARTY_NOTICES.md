# Third-party notices

## PaddleOCR / OCR model assets

The screenshot OCR implementation adapts components of the PaddleOCR Android deployment code and uses PP-OCRv5 ONNX model assets from the gitakoos/ocr-models release repository.

Copyright (C) 2026 PaddlePaddle Authors and contributors.
Copyright (C) 2026 Akoos.

The PaddleOCR-derived code and OCR model assets are licensed under the Apache License, Version 2.0. The asset repository's NOTICE identifies:

- det.onnx — PP-OCRv5 mobile text-detection network
- rec_latin.onnx — Latin-script PP-OCRv5 mobile recognition network
- ppocrv5_latin_dict.txt — recognition dictionary

Source:
https://github.com/PaddlePaddle/PaddleOCR
https://github.com/gitakoos/ocr-models

The exact downloaded asset bytes are pinned and SHA-256 verified in OcrModelManager.kt.

The Apache License, Version 2.0 applies to the PaddleOCR-derived material:

http://www.apache.org/licenses/LICENSE-2.0

## Runtime dependencies

- ONNX Runtime Android — MIT License.
- OpenCV Android — Apache License 2.0 for the OpenCV 4.x line used by this app.

These dependencies are used only for local inference; screenshot images are copied to the app cache for processing and are not uploaded by the OCR flow.