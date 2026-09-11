"""Derive Windows and frontend icons from the user-selected concept 08.

Requires Pillow. Run from any directory with: python scripts/build-app-icon.py
The original artwork is preserved; no network or image-generation API is used.
"""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFont, ImageOps


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets/shilu-icon-concepts/08-archive-drawer.png"
ICON_DIR = ROOT / "src-tauri/icons"
ASSET_DIR = ROOT / "assets/shilu-icon"
PUBLIC_DIR = ROOT / "public"
SIZES = (16, 20, 24, 32, 40, 48, 64, 128, 256)


def save_ico(master: Image.Image, destination: Path) -> None:
    master.save(destination, format="ICO", sizes=[(size, size) for size in SIZES])
    data = destination.read_bytes()
    reserved, kind, count = struct.unpack_from("<HHH", data)
    assert (reserved, kind, count) == (0, 1, len(SIZES))
    entries = [data[6 + index * 16 : 22 + index * 16] for index in range(count)]
    # Tauri decodes the first ICO entry for its runtime default window icon.
    # Reorder the directory only; the image data offsets remain unchanged.
    entries.sort(key=lambda entry: entry[0] or 256, reverse=True)
    destination.write_bytes(data[:6] + b"".join(entries) + data[6 + count * 16 :])
    with Image.open(destination) as icon:
        assert icon.ico.sizes() == {(size, size) for size in SIZES}
        for size in SIZES:
            assert icon.ico.getimage((size, size)).size == (size, size)


def build_preview(master: Image.Image) -> None:
    sheet = Image.new("RGB", (1040, 452), "white")
    draw = ImageDraw.Draw(sheet)
    font = ImageFont.truetype("C:/Windows/Fonts/segoeui.ttf", 15)
    draw.text((28, 20), "ShiLu / Selected concept 08 / Actual-size Windows previews", fill="#202020", font=font)
    preview_sizes = (16, 20, 24, 32, 40, 48, 64, 128)
    for row, (background, text_color) in enumerate((("#F4F4F4", "#202020"), ("#202020", "#FFFFFF"))):
        top = 60 + row * 184
        draw.rectangle((16, top, 1024, top + 176), fill=background)
        left = 40
        for size in preview_sizes:
            preview = master.resize((size, size), Image.Resampling.LANCZOS)
            sheet.paste(preview, (left + (112 - size) // 2, top + 12 + (128 - size) // 2), preview)
            draw.text((left + 56, top + 148), f"{size} px", fill=text_color, font=font, anchor="mt")
            left += 120
    sheet.save(ASSET_DIR / "windows-size-preview.png", optimize=True)


def main() -> None:
    source_hash = hashlib.sha256(SOURCE.read_bytes()).hexdigest()
    with Image.open(SOURCE) as original:
        source = ImageOps.exif_transpose(original).convert("RGB")
    difference = ImageChops.difference(source, Image.new("RGB", source.size, "white"))
    foreground = difference.convert("L").point(lambda value: 255 if value > 40 else 0)
    bounds = foreground.getbbox()
    if bounds is None:
        raise ValueError("Selected artwork is blank")
    # Remove only exterior whitespace, leaving the original white areas intact.
    cropped = source.crop(bounds)
    extent = max(cropped.size)
    padding = round(extent * 0.075)
    square = Image.new("RGBA", (extent + padding * 2, extent + padding * 2), "white")
    square.paste(cropped, ((square.width - cropped.width) // 2, (square.height - cropped.height) // 2))
    master = square.resize((512, 512), Image.Resampling.LANCZOS)
    for directory in (ICON_DIR, ASSET_DIR, PUBLIC_DIR):
        directory.mkdir(parents=True, exist_ok=True)
    master.save(ASSET_DIR / "icon.png", optimize=True)
    master.save(ICON_DIR / "icon.png", optimize=True)
    for size in (32, 64, 128, 256):
        name = "128x128@2x.png" if size == 256 else f"{size}x{size}.png"
        master.resize((size, size), Image.Resampling.LANCZOS).save(ICON_DIR / name, optimize=True)
    master.resize((128, 128), Image.Resampling.LANCZOS).save(PUBLIC_DIR / "shilu-icon.png", optimize=True)
    save_ico(master, ICON_DIR / "icon.ico")
    (PUBLIC_DIR / "favicon.ico").write_bytes((ICON_DIR / "icon.ico").read_bytes())
    build_preview(master)
    assert hashlib.sha256(SOURCE.read_bytes()).hexdigest() == source_hash
    assert master.mode == "RGBA"
    print(json.dumps({"source": str(SOURCE), "source_sha256": source_hash, "crop_bounds": bounds,
                      "master_size": list(master.size), "ico_sizes": SIZES,
                      "first_ico_entry": 256, "background": "white", "original_preserved": True}, indent=2))


if __name__ == "__main__":
    main()
