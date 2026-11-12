#!/usr/bin/env python3
"""Generate Verbatim app and extension icons with Pillow (no network)."""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
TAURI = ROOT / "src-tauri" / "icons"
EXT = ROOT / "extension" / "icons"
WEB = ROOT / "web" / "public"


def icon(size: int) -> Image.Image:
    scale = 4
    canvas = size * scale
    image = Image.new("RGBA", (canvas, canvas), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    radius = int(canvas * 0.23)
    draw.rounded_rectangle((0, 0, canvas - 1, canvas - 1), radius=radius, fill=(22, 33, 62, 255))
    draw.rounded_rectangle(
        (int(canvas * 0.035), int(canvas * 0.035), int(canvas * 0.965), int(canvas * 0.965)),
        radius=int(radius * 0.85),
        outline=(15, 52, 96, 255),
        width=max(1, int(canvas * 0.012)),
    )
    heights = [0.16, 0.32, 0.56, 0.72, 0.44, 0.25, 0.12]
    bar_width = canvas * 0.045
    gap = canvas * 0.028
    total = len(heights) * bar_width + (len(heights) - 1) * gap
    x = (canvas - total) / 2
    mid = canvas / 2
    for index, height in enumerate(heights):
        top = mid - canvas * height / 2
        bottom = mid + canvas * height / 2
        color = (78, 204, 163, 255) if index not in (1, 5) else (234, 234, 234, 220)
        draw.rounded_rectangle(
            (round(x), round(top), round(x + bar_width), round(bottom)),
            radius=max(1, round(bar_width / 2)),
            fill=color,
        )
        x += bar_width + gap
    return image.resize((size, size), Image.Resampling.LANCZOS)


def main() -> None:
    for directory in (TAURI, EXT, WEB):
        directory.mkdir(parents=True, exist_ok=True)
    sizes = {
        TAURI / "32x32.png": 32,
        TAURI / "128x128.png": 128,
        TAURI / "128x128@2x.png": 256,
        TAURI / "icon.png": 512,
        EXT / "icon16.png": 16,
        EXT / "icon48.png": 48,
        EXT / "icon128.png": 128,
        WEB / "favicon.png": 64,
    }
    rendered: dict[int, Image.Image] = {}
    for path, size in sizes.items():
        if size not in rendered:
            rendered[size] = icon(size)
        rendered[size].save(path)
    ico_sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    icon(256).save(TAURI / "icon.ico", sizes=ico_sizes)
    print(f"wrote {len(sizes) + 1} icons")


if __name__ == "__main__":
    main()
