#!/usr/bin/env python3
"""Prepare OCR models and emit JSON progress for the desktop setup wizard.

RapidOCR-ONNX bundles its models in the wheel. This script warms the engine once
so every ONNX file is present, then writes an install marker into the Verbatim
model directory. PaddleOCR, when selected, fetches its models on first init.
"""

from __future__ import annotations

import json
import os
from pathlib import Path

DATA_DIR = Path(os.environ.get("VERBATIM_DATA_DIR", Path.home() / ".verbatim"))
MODEL_DIR = Path(os.environ.get("VERBATIM_OCR_MODEL", DATA_DIR / "models" / "ocr"))


def emit(stage: str, progress: int, message: str, **extra) -> None:
    print(json.dumps({"stage": stage, "progress": progress, "message": message, **extra}), flush=True)


def main() -> int:
    MODEL_DIR.mkdir(parents=True, exist_ok=True)
    emit("ocr", 5, "Preparing RapidOCR-ONNX models")
    engine_name = os.environ.get("VERBATIM_OCR_ENGINE", "rapidocr").lower()
    try:
        if engine_name == "paddle":
            from paddleocr import PaddleOCR  # type: ignore

            PaddleOCR(use_angle_cls=True, lang=os.environ.get("VERBATIM_OCR_LANG", "en"))
        else:
            from rapidocr_onnxruntime import RapidOCR  # type: ignore

            emit("ocr", 25, "Warming RapidOCR engine (bundled ONNX models)")
            RapidOCR()
    except Exception as exc:
        emit("ocr", 0, f"OCR model preparation failed: {exc}", error=True)
        return 1

    (MODEL_DIR / ".installed").write_text(engine_name + "\n", encoding="utf-8")
    emit("ocr", 100, "OCR models ready", engine=engine_name)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
