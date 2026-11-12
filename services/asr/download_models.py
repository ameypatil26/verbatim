#!/usr/bin/env python3
"""Download the local SenseVoice model into ~/.verbatim/models/asr.

Emits machine-readable JSON progress lines on stdout so the Tauri setup wizard
can render a real progress bar. No credentials and no cloud runtime: this is a
one-time model download, after which the app works offline.
"""

from __future__ import annotations

import json
import os
import shutil
import sys
from pathlib import Path

MODEL_ID = os.environ.get("VERBATIM_ASR_MODEL_ID", "iic/SenseVoiceSmall")
DATA_DIR = Path(os.environ.get("VERBATIM_DATA_DIR", Path.home() / ".verbatim"))
MODEL_DIR = Path(os.environ.get("VERBATIM_ASR_MODEL", DATA_DIR / "models" / "asr"))


def emit(stage: str, progress: int, message: str, **extra) -> None:
    print(json.dumps({"stage": stage, "progress": progress, "message": message, **extra}), flush=True)


def main() -> int:
    MODEL_DIR.mkdir(parents=True, exist_ok=True)
    if (MODEL_DIR / "model.pt").exists() or (MODEL_DIR / "configuration.json").exists():
        emit("asr", 100, "SenseVoice already installed")
        return 0

    emit("asr", 2, "Preparing SenseVoice download")
    try:
        from modelscope.hub.snapshot_download import snapshot_download  # type: ignore

        emit("asr", 8, "Downloading SenseVoice from ModelScope (this can take a few minutes)")
        path = snapshot_download(MODEL_ID, local_dir=str(MODEL_DIR))
        emit("asr", 92, f"Model files downloaded to {path}")
    except Exception as first_error:
        emit("asr", 8, f"ModelScope failed ({first_error}); trying Hugging Face")
        try:
            from huggingface_hub import snapshot_download as hf_snapshot_download  # type: ignore

            hf_snapshot_download(
                repo_id=os.environ.get("VERBATIM_ASR_HF_MODEL", "FunAudioLLM/SenseVoiceSmall"),
                local_dir=str(MODEL_DIR),
                local_dir_use_symlinks=False,
            )
        except Exception as second_error:
            emit("asr", 0, f"Download failed: {second_error}", error=True)
            return 1

    marker = MODEL_DIR / ".installed"
    marker.write_text(MODEL_ID + "\n", encoding="utf-8")
    total = sum(p.stat().st_size for p in MODEL_DIR.rglob("*") if p.is_file())
    emit("asr", 100, f"SenseVoice ready ({total / (1024 * 1024):.0f} MB)", bytes=total)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        emit("asr", 0, "Cancelled", cancelled=True)
        raise SystemExit(130)
