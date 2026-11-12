#!/usr/bin/env python3
"""
Verbatim ASR microservice — FastAPI + FunASR SenseVoice, port 8001.

Contract:
  GET  /health       -> {status, model, loaded}
  POST /transcribe   -> multipart file OR JSON {audio_base64, language?}
  POST /unload       -> {status}

The heavy FunASR stack is imported lazily. If it is not installed (or model
download has not happened yet) the service still reports health and returns a
clear 503 from /transcribe, which is what the desktop setup UI needs.
"""

from __future__ import annotations

import base64
import binascii
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Any

# Allow `python services/asr/main.py` and `uvicorn services.asr.main:app`.
SERVICES_DIR = Path(__file__).resolve().parents[1]
if str(SERVICES_DIR) not in sys.path:
    sys.path.insert(0, str(SERVICES_DIR))

from common.verbatim_common import (  # noqa: E402
    DATA_DIR,
    MODEL_DIR,
    IdleUnloader,
    ModelLease,
    configure_app,
    decode_to_wav,
    format_timestamp,
    media_duration,
    read_upload_limited,
    setup_logging,
    temporary_media,
    unload_peer,
    wav_duration,
)

from fastapi import FastAPI, HTTPException, Request  # noqa: E402
from fastapi.responses import JSONResponse  # noqa: E402

SERVICE_NAME = "asr"
MODEL_ID = "sensevoice"
ASR_MODEL_DIR = Path(os.environ.get("VERBATIM_ASR_MODEL", MODEL_DIR / "asr"))
LOG = setup_logging(SERVICE_NAME)


# ---------------------------------------------------------------------------
# Engines
# ---------------------------------------------------------------------------


class MockEngine:
    """Deterministic engine used by tests and UI demos (VERBATIM_ASR_ENGINE=mock)."""

    name = "mock"
    loaded = False

    def load(self) -> None:
        self.loaded = True

    def unload(self) -> None:
        self.loaded = False

    def transcribe(self, wav_path: Path, language: str) -> tuple[str, str]:
        duration = wav_duration(wav_path)
        text = (
            "[00:00] Verbatim local ASR mock engine is active.\n"
            f"[00:01] Received {duration:.2f} seconds of audio.\n"
            "[00:02] Install FunASR and the SenseVoice model for real transcription."
        )
        return text, language or "en"


class SenseVoiceEngine:
    """
    FunASR SenseVoiceSmall adapter.

    `model` may be a local directory or a ModelScope id. Inference is serialized
    by the service; the model lease in front of it also guarantees ASR and OCR
    are never resident at the same time.
    """

    name = "sensevoice"
    loaded = False

    def __init__(self) -> None:
        self.model = None
        self.loaded = False
        self.last_error: str | None = None

    def _model_ref(self) -> str:
        if ASR_MODEL_DIR.exists() and any(ASR_MODEL_DIR.iterdir()):
            return str(ASR_MODEL_DIR)
        return os.environ.get("VERBATIM_ASR_MODEL_ID", "iic/SenseVoiceSmall")

    def load(self) -> None:
        if self.loaded:
            return
        self.last_error = None
        try:
            from funasr import AutoModel  # type: ignore

            LOG.info("loading SenseVoice from %s", self._model_ref())
            started = time.monotonic()
            self.model = AutoModel(
                model=self._model_ref(),
                device=os.environ.get("VERBATIM_ASR_DEVICE", "cpu"),
                disable_update=True,
                hub=os.environ.get("VERBATIM_ASR_HUB", "ms"),
            )
            self.loaded = True
            LOG.info("SenseVoice loaded in %.1fs", time.monotonic() - started)
        except Exception as exc:  # model download / import failure
            self.last_error = str(exc)
            self.loaded = False
            self.model = None
            LOG.exception("SenseVoice load failed")
            raise

    def unload(self) -> None:
        if self.model is not None:
            try:
                del self.model
            finally:
                self.model = None
        self.loaded = False
        try:
            import gc

            gc.collect()
        except Exception:
            pass
        LOG.info("SenseVoice unloaded")

    def transcribe(self, wav_path: Path, language: str) -> tuple[str, str]:
        if not self.loaded or self.model is None:
            raise RuntimeError("model is not loaded")
        from funasr.utils.postprocess_utils import rich_transcription_postprocess  # type: ignore

        result = self.model.generate(
            input=str(wav_path),
            cache={},
            language=language or "auto",
            use_itn=True,
            batch_size_s=60,
            merge_vad=True,
            merge_length_s=15,
        )
        if not result:
            return "", language or "auto"
        item = result[0]
        raw = item.get("text", "") if isinstance(item, dict) else str(item)
        text = rich_transcription_postprocess(raw).strip()
        sentences = item.get("sentence_info") if isinstance(item, dict) else None
        if sentences:
            lines: list[str] = []
            for sentence in sentences:
                start = float(sentence.get("start", 0)) / 1000.0
                content = str(sentence.get("text", "")).strip()
                if content:
                    lines.append(f"{format_timestamp(start)} {content}")
            if lines:
                text = "\n".join(lines)
        return text, language or item.get("lang", "auto") if isinstance(item, dict) else language or "auto"


def make_engine():
    choice = os.environ.get("VERBATIM_ASR_ENGINE", "sensevoice").strip().lower()
    if choice in {"mock", "test"}:
        LOG.warning("using mock ASR engine")
        return MockEngine()
    return SenseVoiceEngine()


engine = make_engine()
model_lock = threading.RLock()
state_lock = threading.Lock()
last_error: str | None = None


def idle_unload(idle: bool = False) -> bool:
    global last_error
    if not engine.loaded:
        return False
    if not model_lock.acquire(blocking=False):
        return False
    try:
        engine.unload()
        return True
    finally:
        model_lock.release()


unloader = IdleUnloader(SERVICE_NAME, idle_unload)


# ---------------------------------------------------------------------------
# URL fallback via yt-dlp
# ---------------------------------------------------------------------------


def _download_url_audio(url: str) -> Path:
    work = Path(tempfile.mkdtemp(dir=os.environ.get("VERBATIM_TMP_DIR", DATA_DIR / "tmp")))
    template = str(work / "source.%(ext)s")
    python = sys.executable
    base = [python, "-m", "yt_dlp"] if shutil.which("yt-dlp") is None else ["yt-dlp"]
    cmd = [
        *base,
        "--no-playlist",
        "--quiet",
        "--no-warnings",
        "-x",
        "--audio-format",
        "wav",
        "--audio-quality",
        "0",
        "-o",
        template,
        url,
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=900)
    if proc.returncode != 0:
        shutil.rmtree(work, ignore_errors=True)
        raise RuntimeError((proc.stderr or proc.stdout or "yt-dlp failed").strip()[:800])
    files = sorted(work.glob("source.*"))
    if not files:
        shutil.rmtree(work, ignore_errors=True)
        raise RuntimeError("yt-dlp did not produce an audio file")
    return files[0]


# ---------------------------------------------------------------------------
# Lifespan and app
# ---------------------------------------------------------------------------


@asynccontextmanager
async def lifespan(_: FastAPI):
    unloader.start()
    LOG.info("ASR service ready on 127.0.0.1 (model loaded=%s)", engine.loaded)
    yield
    unloader.stop()
    with model_lock:
        engine.unload()


app = FastAPI(
    title="Verbatim ASR",
    version="1.0.0",
    description="Local-only SenseVoice speech transcription.",
    lifespan=lifespan,
)
configure_app(app, SERVICE_NAME)


@app.get("/health")
def health() -> dict[str, Any]:
    return {"status": "ok", "model": MODEL_ID, "loaded": bool(engine.loaded)}


@app.post("/unload")
def unload() -> dict[str, str]:
    with model_lock:
        engine.unload()
    return {"status": "ok"}


@app.post("/transcribe")
async def transcribe(request: Request):
    global last_error
    language = "auto"
    audio: bytes | None = None
    suffix = ".bin"
    source_url: str | None = None
    content_type = (request.headers.get("content-type") or "").lower()

    try:
        if "application/json" in content_type:
            payload = json.loads(await request.body() or b"{}")
            if not isinstance(payload, dict):
                raise ValueError("JSON body must be an object")
            language = str(payload.get("language") or "auto")
            source_url = payload.get("url")
            encoded = payload.get("audio_base64")
            if encoded and not source_url:
                try:
                    audio = base64.b64decode(str(encoded), validate=True)
                except (binascii.Error, ValueError) as exc:
                    raise ValueError("audio_base64 is not valid base64") from exc
        elif "multipart/form-data" in content_type:
            form = await request.form()
            upload = form.get("file") or form.get("audio") or form.get("media")
            language = str(form.get("language") or "auto")
            if upload is not None and hasattr(upload, "read"):
                audio = await read_upload_limited(upload)
                suffix = Path(getattr(upload, "filename", "") or "audio.bin").suffix or ".bin"
        else:
            raise ValueError("send multipart/form-data or application/json")

        if source_url and not audio:
            if not isinstance(source_url, str) or not source_url.startswith(("http://", "https://")):
                raise ValueError("url must be an http(s) URL")
            downloaded = _download_url_audio(source_url)
            try:
                audio = downloaded.read_bytes()
                suffix = downloaded.suffix or ".bin"
            finally:
                shutil.rmtree(downloaded.parent, ignore_errors=True)

        if not audio:
            raise ValueError("no audio supplied")
        if len(audio) > 2 * 1024 * 1024 * 1024:
            raise ValueError("audio exceeds the 2 GB limit")
        if len(audio) < 32:
            raise ValueError("audio payload is empty or truncated")
    except ValueError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    except Exception as exc:
        LOG.warning("bad transcribe request: %s", exc)
        raise HTTPException(status_code=400, detail=str(exc)) from exc

    with temporary_media(audio, suffix) as source:
        wav_path = source.with_suffix(source.suffix + ".16k.wav")
        try:
            if suffix.lower() == ".wav":
                decode_to_wav(source, wav_path)
            else:
                decode_to_wav(source, wav_path)
            duration = media_duration(source) or wav_duration(wav_path)
        except Exception as exc:
            wav_path.unlink(missing_ok=True)
            LOG.warning("audio decode failed: %s", exc)
            raise HTTPException(status_code=400, detail=str(exc)) from exc

        try:
            with model_lock:
                with ModelLease(SERVICE_NAME):
                    unload_peer("ocr")
                    try:
                        engine.load()
                    except Exception as exc:
                        last_error = str(exc)
                        raise HTTPException(
                            status_code=503,
                            detail=f"ASR model failed to load: {exc}",
                        ) from exc
                    text, detected = engine.transcribe(wav_path, language)
                    unloader.touch()
        except HTTPException:
            raise
        except Exception as exc:
            last_error = str(exc)
            LOG.exception("transcription failed")
            raise HTTPException(status_code=500, detail=f"transcription failed: {exc}") from exc
        finally:
            unloader.touch()
            wav_path.unlink(missing_ok=True)

    return {"text": text, "duration": round(float(duration), 3), "language": detected}


@app.exception_handler(HTTPException)
async def http_exception_handler(request: Request, exc: HTTPException):
    return JSONResponse(status_code=exc.status_code, content={"detail": exc.detail})


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(
        "main:app",
        host="127.0.0.1",
        port=int(os.environ.get("VERBATIM_ASR_PORT", "8001")),
        log_level=os.environ.get("VERBATIM_LOG_LEVEL", "warning"),
        reload=False,
    )
