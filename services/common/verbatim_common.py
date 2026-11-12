"""
Shared plumbing for Verbatim's ASR and OCR microservices.

Everything here is deliberately dependency-light: the two services must be able
to start even when their heavy model stack is not installed yet. That lets the
desktop app show a useful "models not installed" state instead of a crash loop.
"""

from __future__ import annotations

import contextlib
import json
import logging
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import wave
from logging.handlers import RotatingFileHandler
from pathlib import Path
from typing import Any, BinaryIO, Iterable, Iterator

from starlette.middleware.base import BaseHTTPMiddleware
from starlette.requests import Request
from starlette.responses import JSONResponse
from starlette.types import ASGIApp

# ---------------------------------------------------------------------------
# Paths and constants
# ---------------------------------------------------------------------------

SERVICE_ROOT = Path(__file__).resolve().parents[1]
PROJECT_ROOT = SERVICE_ROOT.parent

DATA_DIR = Path(os.environ.get("VERBATIM_DATA_DIR", Path.home() / ".verbatim")).expanduser()
LOG_DIR = Path(os.environ.get("VERBATIM_LOG_DIR", DATA_DIR / "logs")).expanduser()
MODEL_DIR = Path(os.environ.get("VERBATIM_MODEL_DIR", DATA_DIR / "models")).expanduser()
TMP_DIR = Path(os.environ.get("VERBATIM_TMP_DIR", DATA_DIR / "tmp")).expanduser()

for _path in (DATA_DIR, LOG_DIR, MODEL_DIR, TMP_DIR):
    _path.mkdir(parents=True, exist_ok=True)

MAX_UPLOAD_BYTES = 2 * 1024 * 1024 * 1024  # 2 GB
DEFAULT_IDLE_SECONDS = 5 * 60
LOOPBACK_HOSTS = {"127.0.0.1", "::1", "localhost"}


# ---------------------------------------------------------------------------
# Logging (rotating, no console spam unless explicitly debugging)
# ---------------------------------------------------------------------------


def setup_logging(name: str) -> logging.Logger:
    logger = logging.getLogger(f"verbatim.{name}")
    if logger.handlers:
        return logger
    logger.setLevel(logging.DEBUG)
    logger.propagate = False

    log_file = LOG_DIR / f"{name}.log"
    formatter = logging.Formatter(
        "%(asctime)s %(levelname)-7s %(name)s: %(message)s", "%Y-%m-%dT%H:%M:%S%z"
    )
    handler = RotatingFileHandler(log_file, maxBytes=5 * 1024 * 1024, backupCount=7, encoding="utf-8")
    handler.setFormatter(formatter)
    logger.addHandler(handler)

    if os.environ.get("VERBATIM_DEBUG") == "1":
        console = logging.StreamHandler(sys.stdout)
        console.setFormatter(formatter)
        logger.addHandler(console)
    return logger


# ---------------------------------------------------------------------------
# Loopback security and CORS
# ---------------------------------------------------------------------------

ALLOWED_ORIGIN_REGEX = (
    r"^(https?://(localhost|127\.0\.0\.1)(:\d+)?|"
    r"tauri://localhost|https?://tauri\.localhost|"
    r"chrome-extension://[a-z0-9]+|moz-extension://[a-z0-9-]+|null)$"
)


class LoopbackOnlyMiddleware(BaseHTTPMiddleware):
    """
    Refuse anything that did not arrive over a loopback socket.

    Binding uvicorn to 127.0.0.1 is the primary control; this middleware is a
    second line of defence (for example if someone changes the bind address).
    TestClient uses the synthetic "testclient" host and is allowed.
    """

    async def dispatch(self, request: Request, call_next):
        host = request.client.host if request.client else ""
        if host and host not in LOOPBACK_HOSTS and host != "testclient":
            return JSONResponse(
                status_code=403,
                content={"detail": "Verbatim services only accept loopback connections."},
            )
        return await call_next(request)


def configure_app(app, service_name: str) -> None:
    """Attach consistent middleware to a FastAPI app."""
    from fastapi.middleware.cors import CORSMiddleware

    app.add_middleware(
        CORSMiddleware,
        allow_origin_regex=ALLOWED_ORIGIN_REGEX,
        allow_credentials=False,
        allow_methods=["GET", "POST", "OPTIONS"],
        allow_headers=["*"],
        expose_headers=["Content-Disposition"],
    )
    app.add_middleware(LoopbackOnlyMiddleware)
    logging.getLogger(f"verbatim.{service_name}").debug("loopback middleware installed")


# ---------------------------------------------------------------------------
# Cross-service model arbitration
# ---------------------------------------------------------------------------


class ModelLease:
    """
    An OS-level exclusive lock shared by the ASR and OCR processes.

    Only one neural model is allowed to be resident at a time. Each service
    holds this lease for the full lifetime of a request (load + inference), then
    releases. While holding it, the service unloads its peer. The peer either
    finishes its own work first or waits at this lock, so downloads and
    inference can never overlap in a way that leaves both models in RAM.
    """

    def __init__(self, name: str) -> None:
        self.name = name
        self.path = DATA_DIR / "model-lease.lock"
        self._fh: BinaryIO | None = None

    def __enter__(self) -> "ModelLease":
        self._fh = open(self.path, "a+b")
        self._fh.seek(0, os.SEEK_END)
        if self._fh.tell() == 0:
            self._fh.write(b"\0")
            self._fh.flush()
        self._fh.seek(0)
        if os.name == "nt":
            import msvcrt

            while True:
                try:
                    msvcrt.locking(self._fh.fileno(), msvcrt.LK_LOCK, 1)
                    break
                except OSError:
                    time.sleep(0.05)
        else:
            import fcntl

            fcntl.flock(self._fh.fileno(), fcntl.LOCK_EX)
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        if not self._fh:
            return
        with contextlib.suppress(Exception):
            self._fh.seek(0)
            if os.name == "nt":
                import msvcrt

                msvcrt.locking(self._fh.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                import fcntl

                fcntl.flock(self._fh.fileno(), fcntl.LOCK_UN)
        self._fh.close()
        self._fh = None


def _peer_url(peer: str | None) -> str | None:
    if not peer:
        return None
    env = "VERBATIM_ASR_URL" if peer == "asr" else "VERBATIM_OCR_URL"
    default_port = 8001 if peer == "asr" else 8002
    return os.environ.get(env, f"http://127.0.0.1:{default_port}")


def unload_peer(peer: str) -> None:
    """Ask the other service to free its model. Best-effort by design."""
    url = _peer_url(peer)
    if not url:
        return
    try:
        import httpx

        with httpx.Client(timeout=2.5) as client:
            client.post(f"{url}/unload")
    except Exception:
        # A dead peer is perfectly fine; it has no model resident.
        pass


# ---------------------------------------------------------------------------
# HTTP input helpers
# ---------------------------------------------------------------------------


async def read_upload_limited(upload, max_bytes: int = MAX_UPLOAD_BYTES) -> bytes:
    """Read an UploadFile while enforcing the 2 GB contract cap."""
    chunks: list[bytes] = []
    total = 0
    while True:
        chunk = await upload.read(1024 * 1024)
        if not chunk:
            break
        total += len(chunk)
        if total > max_bytes:
            raise ValueError(f"file exceeds the {max_bytes // (1024 * 1024 * 1024)} GB limit")
        chunks.append(chunk)
    return b"".join(chunks)


def parse_masks(raw: str | None) -> list[dict[str, int]]:
    if not raw:
        return []
    try:
        value = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise ValueError("masks must be valid JSON") from exc
    if not isinstance(value, list):
        raise ValueError("masks must be a JSON array")
    masks: list[dict[str, int]] = []
    for item in value:
        if not isinstance(item, dict):
            raise ValueError("each mask must be an object")
        try:
            x, y, w, h = (int(round(float(item[k]))) for k in ("x", "y", "w", "h"))
        except (KeyError, TypeError, ValueError) as exc:
            raise ValueError("each mask requires numeric x, y, w and h") from exc
        if w <= 0 or h <= 0:
            raise ValueError("mask width and height must be positive")
        masks.append({"x": x, "y": y, "w": w, "h": h})
    return masks


def _overlap(a: dict[str, float], b: dict[str, int]) -> bool:
    ax2, ay2 = a["x"] + a["w"], a["y"] + a["h"]
    bx2, by2 = b["x"] + b["w"], b["y"] + b["h"]
    return not (ax2 <= b["x"] or bx2 <= a["x"] or ay2 <= b["y"] or by2 <= a["y"])


def filter_masked_boxes(boxes: list[dict[str, Any]], masks: Iterable[dict[str, int]]) -> list[dict[str, Any]]:
    masks = list(masks)
    if not masks:
        return boxes
    return [box for box in boxes if not any(_overlap(box, mask) for mask in masks)]


# ---------------------------------------------------------------------------
# Audio helpers
# ---------------------------------------------------------------------------


def ffmpeg_path() -> str | None:
    return os.environ.get("VERBATIM_FFMPEG") or shutil.which("ffmpeg")


def ffprobe_path() -> str | None:
    return os.environ.get("VERBATIM_FFPROBE") or shutil.which("ffprobe")


def decode_to_wav(source: Path, destination: Path, sample_rate: int = 16000) -> None:
    """
    Convert arbitrary media to 16 kHz mono PCM WAV.

    WAV files are passed through directly when they already match. Everything
    else uses ffmpeg, which is also required by yt-dlp. A clear error is raised
    when ffmpeg is unavailable instead of feeding garbage to the model.
    """
    try:
        with wave.open(str(source), "rb") as wav:
            if wav.getnchannels() == 1 and wav.getframerate() == sample_rate and wav.getsampwidth() == 2:
                shutil.copyfile(source, destination)
                return
    except (wave.Error, EOFError):
        pass

    ffmpeg = ffmpeg_path()
    if not ffmpeg:
        raise RuntimeError(
            "ffmpeg is required to decode this media. Install ffmpeg and put it on PATH."
        )
    cmd = [
        ffmpeg,
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-i",
        str(source),
        "-vn",
        "-ac",
        "1",
        "-ar",
        str(sample_rate),
        "-acodec",
        "pcm_s16le",
        str(destination),
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or "ffmpeg could not decode this media")


def wav_duration(path: Path) -> float:
    try:
        with wave.open(str(path), "rb") as wav:
            frames = wav.getnframes()
            rate = wav.getframerate() or 1
            return round(frames / rate, 3)
    except (wave.Error, EOFError):
        return 0.0


def media_duration(path: Path) -> float:
    duration = wav_duration(path)
    if duration:
        return duration
    ffprobe = ffprobe_path()
    if not ffprobe:
        return 0.0
    proc = subprocess.run(
        [
            ffprobe,
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            str(path),
        ],
        capture_output=True,
        text=True,
        timeout=30,
    )
    try:
        return round(float(proc.stdout.strip()), 3)
    except ValueError:
        return 0.0


def write_wav(path: Path, samples: bytes, sample_rate: int, channels: int = 1) -> None:
    with wave.open(str(path), "wb") as wav:
        wav.setnchannels(channels)
        wav.setsampwidth(2)
        wav.setframerate(sample_rate)
        wav.writeframes(samples)


def temp_media_path(suffix: str = ".bin") -> Path:
    fd, name = tempfile.mkstemp(dir=TMP_DIR, suffix=suffix)
    os.close(fd)
    return Path(name)


@contextlib.contextmanager
def temporary_media(data: bytes, suffix: str) -> Iterator[Path]:
    path = temp_media_path(suffix)
    try:
        path.write_bytes(data)
        yield path
    finally:
        with contextlib.suppress(FileNotFoundError):
            path.unlink()


# ---------------------------------------------------------------------------
# Formatting and model helpers
# ---------------------------------------------------------------------------


def format_timestamp(seconds: float) -> str:
    seconds = max(0, int(seconds))
    return f"[{seconds // 60:02d}:{seconds % 60:02d}]"


def human_size(num_bytes: int) -> str:
    value = float(num_bytes)
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if value < 1024 or unit == "TB":
            return f"{value:.0f} {unit}" if unit in ("B", "KB") else f"{value:.1f} {unit}"
        value /= 1024


def directory_size(path: Path) -> int:
    if not path.exists():
        return 0
    total = 0
    for entry in path.rglob("*"):
        if entry.is_file():
            with contextlib.suppress(OSError):
                total += entry.stat().st_size
    return total


def is_cjk(text: str) -> bool:
    return any("\u4e00" <= char <= "\u9fff" or "\u3040" <= char <= "\u30ff" for char in text)


class IdleUnloader:
    """Background thread that frees the model after a configurable idle period."""

    def __init__(self, name: str, unload_callable, default_seconds: int = DEFAULT_IDLE_SECONDS):
        self.name = name
        self.unload_callable = unload_callable
        self.seconds = max(0, int(os.environ.get("VERBATIM_MODEL_IDLE_SECONDS", default_seconds)))
        self.disabled = os.environ.get("VERBATIM_DISABLE_IDLE") == "1" or self.seconds == 0
        self._last_used = time.monotonic()
        self._thread: threading.Thread | None = None
        self._stop = threading.Event()
        self.log = setup_logging(name)

    def touch(self) -> None:
        self._last_used = time.monotonic()

    def start(self) -> None:
        if self.disabled or self._thread:
            return
        self._thread = threading.Thread(target=self._run, name=f"{self.name}-idle", daemon=True)
        self._thread.start()

    def stop(self) -> None:
        self._stop.set()

    def _run(self) -> None:
        while not self._stop.wait(5):
            if time.monotonic() - self._last_used < self.seconds:
                continue
            try:
                if self.unload_callable(idle=True):
                    self.log.info("unloaded %s model after %ss idle", self.name, self.seconds)
            except Exception as exc:  # pragma: no cover - defensive
                self.log.warning("idle unload failed: %s", exc)
