from __future__ import annotations

import base64
import io
import wave

from fastapi.testclient import TestClient

from services.asr.main import app


def make_wav(seconds: float = 1.0, rate: int = 16000) -> bytes:
    buffer = io.BytesIO()
    with wave.open(buffer, "wb") as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(rate)
        wav.writeframes(b"\x00\x00" * int(seconds * rate))
    return buffer.getvalue()


def test_health_contract():
    with TestClient(app) as client:
        response = client.get("/health")
    assert response.status_code == 200
    assert response.json() == {"status": "ok", "model": "sensevoice", "loaded": False}


def test_transcribe_multipart_then_unload():
    with TestClient(app) as client:
        response = client.post(
            "/transcribe",
            files={"file": ("clip.wav", make_wav(), "audio/wav")},
            data={"language": "en"},
        )
        assert response.status_code == 200, response.text
        body = response.json()
        assert set(body) == {"text", "duration", "language"}
        assert isinstance(body["text"], str) and body["text"]
        assert 0.9 <= body["duration"] <= 1.1
        assert body["language"] == "en"
        assert client.get("/health").json()["loaded"] is True
        assert client.post("/unload").json() == {"status": "ok"}
        assert client.get("/health").json()["loaded"] is False


def test_transcribe_json_base64():
    with TestClient(app) as client:
        response = client.post(
            "/transcribe",
            json={"audio_base64": base64.b64encode(make_wav(0.5)).decode(), "language": "fr"},
        )
        assert response.status_code == 200, response.text
        assert response.json()["language"] == "fr"


def test_bad_inputs_return_400():
    with TestClient(app) as client:
        assert client.post("/transcribe", json={}).status_code == 400
        assert client.post("/transcribe", json={"audio_base64": "not base64!"}).status_code == 400
        assert client.post("/transcribe", content=b"hello", headers={"content-type": "text/plain"}).status_code == 400
        assert client.post("/transcribe", files={"file": ("x.wav", b"tiny", "audio/wav")}).status_code == 400


def test_missing_file_returns_400():
    with TestClient(app) as client:
        assert client.post("/transcribe", files={"other": ("x.wav", make_wav(), "audio/wav")}).status_code == 400
