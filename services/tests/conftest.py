from __future__ import annotations

import os
import sys
from pathlib import Path

# Configure the services before either module is imported. Tests never download
# or load the real ASR model; OCR uses the real bundled RapidOCR models.
_TEST_DATA = Path(__file__).resolve().parents[2] / ".tmp" / f"verbatim-test-{os.getpid()}"
_TEST_DATA.mkdir(parents=True, exist_ok=True)
os.environ["VERBATIM_DATA_DIR"] = str(_TEST_DATA)
os.environ["VERBATIM_DISABLE_IDLE"] = "1"
os.environ["VERBATIM_ASR_ENGINE"] = "mock"
os.environ["VERBATIM_LOG_DIR"] = str(_TEST_DATA / "logs")
os.environ["VERBATIM_TMP_DIR"] = str(_TEST_DATA / "tmp")

PROJECT_ROOT = Path(__file__).resolve().parents[2]
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))
