from __future__ import annotations

import io

import fitz
from fastapi.testclient import TestClient
from PIL import Image, ImageDraw, ImageFont

from services.ocr.main import app


def text_png(text: str = "VERBATIM TEST", width: int = 720, height: int = 180) -> bytes:
    image = Image.new("RGB", (width, height), "white")
    draw = ImageDraw.Draw(image)
    try:
        font = ImageFont.truetype("C:/Windows/Fonts/arial.ttf", 64)
    except OSError:
        font = ImageFont.load_default(size=48)
    draw.text((30, 50), text, fill="black", font=font)
    buffer = io.BytesIO()
    image.save(buffer, format="PNG")
    return buffer.getvalue()


def scanned_pdf() -> bytes:
    image = Image.open(io.BytesIO(text_png("SEARCHABLE PDF TEST", 900, 240)))
    image_bytes = io.BytesIO()
    image.save(image_bytes, format="PNG")
    doc = fitz.open()
    page = doc.new_page(width=612, height=200)
    page.insert_image(page.rect, stream=image_bytes.getvalue())
    data = doc.tobytes()
    doc.close()
    return data


def test_health_contract():
    with TestClient(app) as client:
        response = client.get("/health")
    assert response.status_code == 200
    assert response.json() == {"status": "ok", "model": "paddle", "loaded": False}


def test_ocr_and_unload():
    with TestClient(app) as client:
        response = client.post("/ocr", files={"file": ("scan.png", text_png(), "image/png")})
        assert response.status_code == 200, response.text
        body = response.json()
        assert "VERBATIM" in body["text"].upper()
        assert body["boxes"]
        assert client.get("/health").json()["loaded"] is True
        assert client.post("/unload").json() == {"status": "ok"}


def test_masks_exclude_all_text():
    png = text_png()
    with Image.open(io.BytesIO(png)) as image:
        width, height = image.size
    masks = f'[{{"x":0,"y":0,"w":{width},"h":{height}}}]'
    with TestClient(app) as client:
        response = client.post(
            "/ocr",
            files={"file": ("scan.png", png, "image/png")},
            data={"masks": masks},
        )
        assert response.status_code == 200, response.text
        body = response.json()
        assert body["text"] == ""
        assert body["boxes"] == []


def test_batch_ocr():
    with TestClient(app) as client:
        response = client.post(
            "/ocr-batch",
            files=[
                ("files", ("one.png", text_png("BATCH ONE"), "image/png")),
                ("files", ("two.png", text_png("BATCH TWO"), "image/png")),
            ],
            data={"masks_json": "[]"},
        )
        assert response.status_code == 200, response.text
        results = response.json()["results"]
        assert [r["filename"] for r in results] == ["one.png", "two.png"]
        assert "BATCH ONE" in results[0]["text"].upper()
        assert "BATCH TWO" in results[1]["text"].upper()


def test_pdf_ocr_and_dual_layer():
    pdf = scanned_pdf()
    with TestClient(app) as client:
        ocr_response = client.post("/ocr-pdf", files={"file": ("scan.pdf", pdf, "application/pdf")})
        assert ocr_response.status_code == 200, ocr_response.text
        assert "SEARCHABLE" in ocr_response.json()["text"].upper()

        response = client.post("/pdf-searchable", files={"file": ("scan.pdf", pdf, "application/pdf")})
        assert response.status_code == 200, response.text
        assert response.headers["content-type"] == "application/pdf"
        assert response.content.startswith(b"%PDF-")
        result = fitz.open(stream=response.content, filetype="pdf")
        try:
            assert "SEARCHABLE" in result[0].get_text().upper()
            assert result[0].get_images()
        finally:
            result.close()


def test_qr_detection_if_zbar_available():
    import qrcode

    qr = qrcode.make("https://verbatim.local/privacy")
    buffer = io.BytesIO()
    qr.save(buffer, format="PNG")
    with TestClient(app) as client:
        response = client.post("/ocr", files={"file": ("qr.png", buffer.getvalue(), "image/png")})
    assert response.status_code == 200, response.text
    codes = response.json().get("codes", [])
    # zbar may not be installed on every development machine; if it is, this
    # must decode. The app remains useful without zbar.
    if codes:
        assert any("verbatim.local" in code["data"] for code in codes)


def test_batch_limit_and_bad_input():
    with TestClient(app) as client:
        too_many = [("files", (f"{i}.png", text_png(), "image/png")) for i in range(51)]
        assert client.post("/ocr-batch", files=too_many).status_code == 400
        assert client.post("/ocr", data={"masks": "[]"}).status_code == 400
        assert (
            client.post("/ocr", files={"file": ("a.png", b"not an image", "image/png")}).status_code
            == 400
        )
