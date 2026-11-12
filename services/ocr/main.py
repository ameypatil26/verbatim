#!/usr/bin/env python3
"""
Verbatim OCR microservice — FastAPI + RapidOCR/PaddleOCR + PyMuPDF, port 8002.

Contract:
  GET  /health           -> {status, model, loaded}
  POST /ocr              -> multipart image + masks field -> {text, boxes}
  POST /ocr-batch        -> multipart files[] + masks_json -> {results: [...]}
  POST /pdf-searchable   -> multipart PDF -> application/pdf (image + hidden text)
  POST /unload           -> {status}

Additive endpoints used by the desktop queue:
  POST /ocr-pdf          -> OCR every page of a PDF and return JSON

The service binds to 127.0.0.1 only. Models load on demand; a cross-process
lease ensures ASR's SenseVoice model is unloaded before OCR runs, and vice versa.
"""

from __future__ import annotations

import io
import json
import os
import re
import sys
import threading
import time
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Any

SERVICES_DIR = Path(__file__).resolve().parents[1]
if str(SERVICES_DIR) not in sys.path:
    sys.path.insert(0, str(SERVICES_DIR))

from common.verbatim_common import (  # noqa: E402
    MODEL_DIR,
    IdleUnloader,
    ModelLease,
    configure_app,
    filter_masked_boxes,
    is_cjk,
    parse_masks,
    read_upload_limited,
    setup_logging,
    unload_peer,
)

from fastapi import FastAPI, File, Form, HTTPException, Request, UploadFile  # noqa: E402
from fastapi.responses import JSONResponse, Response  # noqa: E402
from PIL import Image, ImageOps  # noqa: E402

SERVICE_NAME = "ocr"
MODEL_ID = "paddle"  # stable public contract id, even when RapidOCR is the engine
OCR_MODEL_DIR = Path(os.environ.get("VERBATIM_OCR_MODEL", MODEL_DIR / "ocr"))
MAX_BATCH = 50
LOG = setup_logging(SERVICE_NAME)


# ---------------------------------------------------------------------------
# Engines
# ---------------------------------------------------------------------------


class MockEngine:
    loaded = False

    def load(self) -> None:
        self.loaded = True

    def unload(self) -> None:
        self.loaded = False

    def run(self, image: Image.Image) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
        width, height = image.size
        boxes = [
            {
                "x": 12,
                "y": 12,
                "w": max(10, width - 24),
                "h": max(12, min(40, height // 4)),
                "confidence": 0.99,
                "text": "Verbatim OCR mock engine",
            }
        ]
        return boxes, []


class RapidOCREngine:
    loaded = False

    def __init__(self) -> None:
        self.model = None
        self.last_error: str | None = None

    def load(self) -> None:
        if self.loaded:
            return
        try:
            from rapidocr_onnxruntime import RapidOCR  # type: ignore

            LOG.info("loading RapidOCR-ONNX models")
            self.model = RapidOCR()
            self.loaded = True
        except Exception as exc:
            self.last_error = str(exc)
            self.loaded = False
            self.model = None
            LOG.exception("RapidOCR load failed")
            raise

    def unload(self) -> None:
        if self.model is not None:
            del self.model
            self.model = None
        self.loaded = False
        import gc

        gc.collect()
        LOG.info("RapidOCR unloaded")

    def run(self, image: Image.Image) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
        if not self.loaded or self.model is None:
            raise RuntimeError("model is not loaded")
        import numpy as np  # type: ignore

        # RapidOCR expects BGR for OpenCV-loaded arrays. RGB -> BGR keeps colors
        # correct for the detector; grayscale scans are unaffected.
        rgb = np.array(image.convert("RGB"))
        bgr = rgb[:, :, ::-1].copy()
        result, _elapse = self.model(bgr)
        boxes: list[dict[str, Any]] = []
        for item in result or []:
            try:
                points, text, score = item
            except (TypeError, ValueError):
                continue
            xs = [float(p[0]) for p in points]
            ys = [float(p[1]) for p in points]
            x, y = min(xs), min(ys)
            w, h = max(xs) - x, max(ys) - y
            if w < 1 or h < 1 or not str(text).strip():
                continue
            boxes.append(
                {
                    "x": round(x, 2),
                    "y": round(y, 2),
                    "w": round(w, 2),
                    "h": round(h, 2),
                    "confidence": round(float(score), 4),
                    "text": str(text),
                }
            )
        return boxes, []


class PaddleOCREngine:
    loaded = False

    def __init__(self) -> None:
        self.model = None
        self.last_error: str | None = None

    def load(self) -> None:
        if self.loaded:
            return
        try:
            from paddleocr import PaddleOCR  # type: ignore

            LOG.info("loading PaddleOCR models")
            self.model = PaddleOCR(use_angle_cls=True, lang=os.environ.get("VERBATIM_OCR_LANG", "en"))
            self.loaded = True
        except Exception:
            self.loaded = False
            self.model = None
            LOG.exception("PaddleOCR load failed")
            raise

    def unload(self) -> None:
        self.model = None
        self.loaded = False
        import gc

        gc.collect()
        LOG.info("PaddleOCR unloaded")

    def run(self, image: Image.Image) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
        if not self.loaded or self.model is None:
            raise RuntimeError("model is not loaded")
        import numpy as np  # type: ignore

        result = self.model.ocr(np.array(image.convert("RGB")), cls=True)
        boxes: list[dict[str, Any]] = []
        for page in result or []:
            for line in page or []:
                try:
                    points, (text, score) = line
                except (TypeError, ValueError):
                    continue
                xs = [float(p[0]) for p in points]
                ys = [float(p[1]) for p in points]
                x, y = min(xs), min(ys)
                w, h = max(xs) - x, max(ys) - y
                if str(text).strip():
                    boxes.append(
                        {
                            "x": round(x, 2),
                            "y": round(y, 2),
                            "w": round(w, 2),
                            "h": round(h, 2),
                            "confidence": round(float(score), 4),
                            "text": str(text),
                        }
                    )
        return boxes, []


def make_engine():
    choice = os.environ.get("VERBATIM_OCR_ENGINE", "rapidocr").strip().lower()
    if choice in {"mock", "test"}:
        LOG.warning("using mock OCR engine")
        return MockEngine()
    if choice == "paddle":
        return PaddleOCREngine()
    return RapidOCREngine()


engine = make_engine()
model_lock = threading.RLock()
unloader = IdleUnloader(SERVICE_NAME, lambda idle=False: False)


def idle_unload(idle: bool = False) -> bool:
    if not engine.loaded:
        return False
    if not model_lock.acquire(blocking=False):
        return False
    try:
        engine.unload()
        return True
    finally:
        model_lock.release()


unloader.unload_callable = idle_unload


# ---------------------------------------------------------------------------
# OCR helpers
# ---------------------------------------------------------------------------


def _reading_order(boxes: list[dict[str, Any]]) -> list[dict[str, Any]]:
    if not boxes:
        return []
    heights = sorted(float(b["h"]) for b in boxes)
    median_h = heights[len(heights) // 2] or 10.0
    ordered = sorted(boxes, key=lambda b: (float(b["y"]) + float(b["h"]) / 2, float(b["x"])))
    lines: list[list[dict[str, Any]]] = []
    for box in ordered:
        cy = float(box["y"]) + float(box["h"]) / 2
        if not lines:
            lines.append([box])
            continue
        anchor = lines[-1][0]
        anchor_cy = float(anchor["y"]) + float(anchor["h"]) / 2
        if abs(cy - anchor_cy) <= max(median_h * 0.65, 8):
            lines[-1].append(box)
        else:
            lines.append([box])
    result: list[dict[str, Any]] = []
    for line in lines:
        result.extend(sorted(line, key=lambda b: float(b["x"])))
    return result


def _join_line(items: list[dict[str, Any]]) -> str:
    parts: list[str] = []
    for item in items:
        text = str(item.get("text", "")).strip()
        if not text:
            continue
        if parts:
            previous = parts[-1]
            if is_cjk(previous) or is_cjk(text):
                parts[-1] = previous + text
                continue
        parts.append(text)
    return " ".join(parts)


def boxes_to_text(boxes: list[dict[str, Any]]) -> str:
    ordered = _reading_order(boxes)
    if not ordered:
        return ""
    heights = sorted(float(b["h"]) for b in ordered)
    median_h = heights[len(heights) // 2] or 10.0
    lines: list[list[dict[str, Any]]] = []
    for box in ordered:
        cy = float(box["y"]) + float(box["h"]) / 2
        if not lines:
            lines.append([box])
            continue
        anchor = lines[-1][0]
        anchor_cy = float(anchor["y"]) + float(anchor["h"]) / 2
        if abs(cy - anchor_cy) <= max(median_h * 0.65, 8):
            lines[-1].append(box)
        else:
            lines.append([box])
    return "\n".join(filter(None, (_join_line(line) for line in lines)))


def _decode_qr(image: Image.Image) -> list[dict[str, Any]]:
    try:
        from pyzbar.pyzbar import decode  # type: ignore

        codes = []
        for item in decode(image):
            rect = item.rect
            codes.append(
                {
                    "type": item.type,
                    "data": item.data.decode("utf-8", errors="replace"),
                    "rect": {"x": rect.left, "y": rect.top, "w": rect.width, "h": rect.height},
                }
            )
        return codes
    except Exception as exc:
        LOG.debug("QR decode unavailable: %s", exc)
        return []


def _engine_run(image: Image.Image, masks: list[dict[str, int]]) -> dict[str, Any]:
    boxes, _ = engine.run(image)
    boxes = filter_masked_boxes(boxes, masks)
    codes = filter_masked_boxes(
        [
            {
                "x": c["rect"]["x"],
                "y": c["rect"]["y"],
                "w": c["rect"]["w"],
                "h": c["rect"]["h"],
                **c,
            }
            for c in _decode_qr(image)
        ],
        masks,
    )
    public_boxes = [
        {
            "x": b["x"],
            "y": b["y"],
            "w": b["w"],
            "h": b["h"],
            "confidence": b["confidence"],
        }
        for b in _reading_order(boxes)
    ]
    text = boxes_to_text(boxes)
    return {
        "text": text,
        "boxes": public_boxes,
        "codes": [{"type": c["type"], "data": c["data"], "rect": c["rect"]} for c in codes],
    }


def _open_image(data: bytes) -> Image.Image:
    try:
        image = Image.open(io.BytesIO(data))
        image.load()
    except Exception as exc:
        raise ValueError(f"unsupported or corrupt image: {exc}") from exc
    image = ImageOps.exif_transpose(image)
    if image.mode not in {"RGB", "L"}:
        image = image.convert("RGB")
    return image


def _ocr_pdf_bytes(data: bytes, masks: list[dict[str, int]] | None = None) -> dict[str, Any]:
    import fitz  # type: ignore

    try:
        doc = fitz.open(stream=data, filetype="pdf")
    except Exception as exc:
        raise ValueError(f"could not open PDF: {exc}") from exc
    all_boxes: list[dict[str, Any]] = []
    all_codes: list[dict[str, Any]] = []
    page_texts: list[str] = []
    try:
        for page_index, page in enumerate(doc):
            pix = page.get_pixmap(matrix=fitz.Matrix(2.0, 2.0), alpha=False)
            image = _open_image(pix.tobytes("png"))
            payload = _engine_run(image, masks or [])
            if payload["text"]:
                page_texts.append(f"--- Page {page_index + 1} ---\n{payload['text']}")
            for box in payload["boxes"]:
                all_boxes.append({**box, "page": page_index + 1})
            for code in payload["codes"]:
                all_codes.append({**code, "page": page_index + 1})
    finally:
        doc.close()
    return {"text": "\n\n".join(page_texts), "boxes": all_boxes, "codes": all_codes}


def _hidden_font(text: str) -> str:
    if any("\u3040" <= c <= "\u30ff" or "\u4e00" <= c <= "\u9fff" for c in text):
        if any("\u3040" <= c <= "\u30ff" for c in text):
            return "japan"
        return "china-s"
    if any("\uac00" <= c <= "\ud7af" for c in text):
        return "korea"
    return "helv"


def _searchable_pdf(data: bytes) -> bytes:
    import fitz  # type: ignore

    try:
        source = fitz.open(stream=data, filetype="pdf")
    except Exception as exc:
        raise ValueError(f"could not open PDF: {exc}") from exc
    target = fitz.open()
    try:
        for page in source:
            rect = page.rect
            pix = page.get_pixmap(matrix=fitz.Matrix(2.0, 2.0), alpha=False)
            image = _open_image(pix.tobytes("png"))
            # Keep the internal boxes: unlike the public response they carry the
            # recognized text needed to place the invisible layer.
            raw_boxes, _ = engine.run(image)
            target_page = target.new_page(width=rect.width, height=rect.height)
            target_page.insert_image(target_page.rect, stream=pix.tobytes("png"))
            # Text boxes are in raster pixels at 2x scale; map back to PDF points.
            scale_x = rect.width / max(1, image.width)
            scale_y = rect.height / max(1, image.height)
            for item in _line_groups(raw_boxes):
                text = _join_line(item)
                if not text:
                    continue
                first = min(item, key=lambda b: float(b["x"]))
                x = float(first["x"]) * scale_x
                top = min(float(b["y"]) for b in item) * scale_y
                height = max(float(b["h"]) for b in item) * scale_y
                baseline = top + max(2.0, height * 0.82)
                if x < 0 or top < 0 or baseline > rect.height + 2:
                    continue
                font = _hidden_font(text)
                try:
                    target_page.insert_text(
                        (x, baseline),
                        text,
                        fontname=font,
                        fontsize=max(3.0, height * 0.82),
                        color=(0, 0, 0),
                        render_mode=3,
                        overlay=True,
                    )
                except Exception as exc:
                    LOG.debug("could not place hidden text %r: %s", text, exc)
            target_page.set_rotation(page.rotation)
        target.set_metadata({"producer": "Verbatim (local)", "creator": "Verbatim", "title": "Searchable PDF"})
        output = target.tobytes(garbage=4, deflate=True)
        return output
    finally:
        source.close()
        target.close()


def _line_groups(boxes: list[dict[str, Any]]) -> list[list[dict[str, Any]]]:
    if not boxes:
        return []
    heights = sorted(float(b["h"]) for b in boxes)
    median_h = heights[len(heights) // 2] or 10.0
    lines: list[list[dict[str, Any]]] = []
    for box in sorted(boxes, key=lambda b: (float(b["y"]) + float(b["h"]) / 2, float(b["x"]))):
        cy = float(box["y"]) + float(box["h"]) / 2
        if not lines:
            lines.append([box])
            continue
        anchor_cy = float(lines[-1][0]["y"]) + float(lines[-1][0]["h"]) / 2
        if abs(cy - anchor_cy) <= max(median_h * 0.65, 8):
            lines[-1].append(box)
        else:
            lines.append([box])
    return lines


# ---------------------------------------------------------------------------
# App
# ---------------------------------------------------------------------------


@asynccontextmanager
async def lifespan(_: FastAPI):
    unloader.start()
    LOG.info("OCR service ready on 127.0.0.1 (model loaded=%s)", engine.loaded)
    yield
    unloader.stop()
    with model_lock:
        engine.unload()


app = FastAPI(
    title="Verbatim OCR",
    version="1.0.0",
    description="Local-only OCR, QR/barcode detection, and searchable PDF generation.",
    lifespan=lifespan,
)
configure_app(app, SERVICE_NAME)


def _run_with_model(image: Image.Image, masks: list[dict[str, int]]) -> dict[str, Any]:
    with model_lock:
        with ModelLease(SERVICE_NAME):
            unload_peer("asr")
            try:
                engine.load()
            except Exception as exc:
                raise HTTPException(status_code=503, detail=f"OCR model failed to load: {exc}") from exc
            payload = _engine_run(image, masks)
            unloader.touch()
            return payload


@app.get("/health")
def health() -> dict[str, Any]:
    return {"status": "ok", "model": MODEL_ID, "loaded": bool(engine.loaded)}


@app.post("/unload")
def unload() -> dict[str, str]:
    with model_lock:
        engine.unload()
    return {"status": "ok"}


@app.post("/ocr")
async def ocr(
    request: Request,
    file: UploadFile | None = File(default=None),
    image: UploadFile | None = File(default=None),
    masks: str | None = Form(default=None),
):
    upload = file or image
    if upload is None:
        raise HTTPException(status_code=400, detail="send an image in the 'file' field")
    try:
        mask_list = parse_masks(masks)
        data = await read_upload_limited(upload)
        if not data:
            raise ValueError("image is empty")
        pil = _open_image(data)
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    payload = _run_with_model(pil, mask_list)
    return JSONResponse(payload)


@app.post("/ocr-batch")
async def ocr_batch(
    request: Request,
    files: list[UploadFile] = File(...),
    masks_json: str | None = Form(default=None),
):
    if not files:
        raise HTTPException(status_code=400, detail="no files supplied")
    if len(files) > MAX_BATCH:
        raise HTTPException(status_code=400, detail=f"at most {MAX_BATCH} files per batch")
    try:
        batch_masks = parse_masks(masks_json)
        payloads: list[tuple[str, Any]] = []
        for upload in files:
            data = await read_upload_limited(upload)
            if not data:
                raise ValueError(f"{upload.filename or 'file'} is empty")
            payloads.append((upload.filename or "untitled", data))
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    results = []
    with model_lock:
        with ModelLease(SERVICE_NAME):
            unload_peer("asr")
            try:
                engine.load()
            except Exception as exc:
                raise HTTPException(status_code=503, detail=f"OCR model failed to load: {exc}") from exc
            for filename, data in payloads:
                try:
                    if data[:5] == b"%PDF-":
                        payload = _ocr_pdf_bytes(data, batch_masks)
                    else:
                        payload = _engine_run(_open_image(data), batch_masks)
                except ValueError as exc:
                    raise HTTPException(status_code=400, detail=f"{filename}: {exc}") from exc
                results.append({"filename": filename, **payload})
            unloader.touch()
    return {"results": results}


@app.post("/ocr-pdf")
async def ocr_pdf(
    request: Request,
    file: UploadFile = File(...),
    masks: str | None = Form(default=None),
):
    try:
        masks_list = parse_masks(masks)
        data = await read_upload_limited(file)
        if not data.startswith(b"%PDF-"):
            raise ValueError("file is not a PDF")
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    with model_lock:
        with ModelLease(SERVICE_NAME):
            unload_peer("asr")
            try:
                engine.load()
            except Exception as exc:
                raise HTTPException(status_code=503, detail=f"OCR model failed to load: {exc}") from exc
            payload = _ocr_pdf_bytes(data, masks_list)
            unloader.touch()
    return JSONResponse(payload)


@app.post("/pdf-searchable")
async def pdf_searchable(
    request: Request,
    file: UploadFile = File(...),
    masks: str | None = Form(default=None),
):
    try:
        _ = parse_masks(masks)  # accepted for contract symmetry; page masks are not applied
        data = await read_upload_limited(file)
        if not data.startswith(b"%PDF-"):
            raise ValueError("file is not a PDF")
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    with model_lock:
        with ModelLease(SERVICE_NAME):
            unload_peer("asr")
            try:
                engine.load()
            except Exception as exc:
                raise HTTPException(status_code=503, detail=f"OCR model failed to load: {exc}") from exc
            output = _searchable_pdf(data)
            unloader.touch()
    filename = Path(file.filename or "document.pdf").stem + "-searchable.pdf"
    return Response(
        content=output,
        media_type="application/pdf",
        headers={"Content-Disposition": f'attachment; filename="{filename}"'},
    )


@app.exception_handler(HTTPException)
async def http_exception_handler(request: Request, exc: HTTPException):
    return JSONResponse(status_code=exc.status_code, content={"detail": exc.detail})


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(
        "main:app",
        host="127.0.0.1",
        port=int(os.environ.get("VERBATIM_OCR_PORT", "8002")),
        log_level=os.environ.get("VERBATIM_LOG_LEVEL", "warning"),
        reload=False,
    )
