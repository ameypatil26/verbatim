#!/usr/bin/env python3
"""
Install Python wheels into a workspace target directory.

The DeepSeek Windows sandbox creates `tempfile.mkdtemp()` directories with an
ACL that the sandboxed Python process cannot reopen. pip uses mkdtemp for every
download, so a normal `pip install` fails with EPERM even though the workspace
is writable. This wrapper keeps pip entirely inside the workspace and creates
its temp directories with normal inherited permissions.

Usage:
    python scripts/pip_install_local.py .pydeps fastapi uvicorn ...
"""

from __future__ import annotations

import os
import pathlib
import runpy
import sys
import tempfile

PROJECT_ROOT = pathlib.Path(__file__).resolve().parents[1]
TEMP = PROJECT_ROOT / ".tmp"
TEMP.mkdir(parents=True, exist_ok=True)
os.environ["TEMP"] = str(TEMP)
os.environ["TMP"] = str(TEMP)
os.environ["PIP_CACHE_DIR"] = str(PROJECT_ROOT / ".pip-cache")


def _mkdtemp(suffix: str | None = None, prefix: str | None = None, dir: str | None = None) -> str:
    base = pathlib.Path(dir or tempfile.gettempdir())
    base.mkdir(parents=True, exist_ok=True)
    while True:
        name = next(tempfile._get_candidate_names())
        candidate = base / f"{prefix or 'tmp'}{name}{suffix or ''}"
        try:
            os.makedirs(candidate, mode=0o777)
            return str(candidate)
        except FileExistsError:
            continue


tempfile.mkdtemp = _mkdtemp  # type: ignore[assignment]

target = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".pydeps").resolve()
target.mkdir(parents=True, exist_ok=True)
packages = sys.argv[2:]
if not packages:
    raise SystemExit("usage: pip_install_local.py <target> <package...>")

sys.argv = [
    "pip",
    "install",
    "--no-cache-dir",
    "--no-warn-script-location",
    "--target",
    str(target),
    *packages,
]
runpy.run_module("pip", run_name="__main__")
