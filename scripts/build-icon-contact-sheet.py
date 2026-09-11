"""Build a non-destructive 5-by-2 comparison sheet for ten icon concepts.

Usage:
    python scripts/build-icon-contact-sheet.py --input-dir PATH --output PATH.png

Requires Pillow. Input names must match 01-*.png through 10-*.png, exactly
one file per number. Small previews are rendered at their actual pixel size.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont, ImageOps


COLS = 5
ROWS = 2
CELL_WIDTH = 320
CELL_HEIGHT = 402
GAP = 24
MARGIN = 48
HEADER_HEIGHT = 92
LARGE_PREVIEW = 260
INK = "#27313A"
MUTED = "#6C747D"


def load_font(size: int, bold: bool = False) -> ImageFont.FreeTypeFont:
    """Prefer Windows system fonts, with portable Pillow/fontconfig fallbacks."""
    names = (
        ["C:/Windows/Fonts/segoeuib.ttf", "C:/Windows/Fonts/arialbd.ttf", "DejaVuSans-Bold.ttf"]
        if bold
        else ["C:/Windows/Fonts/segoeui.ttf", "C:/Windows/Fonts/arial.ttf", "DejaVuSans.ttf"]
    )
    for name in names:
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            continue
    return ImageFont.load_default(size=size)


def paste_preview(sheet: Image.Image, source: Image.Image, x: int, y: int, size: int) -> None:
    """Contain the whole source in a square; never crop or modify the source."""
    preview = ImageOps.contain(source, (size, size), method=Image.Resampling.LANCZOS)
    sheet.paste(preview, (x + (size - preview.width) // 2, y + (size - preview.height) // 2), preview)


def build_sheet(input_dir: Path, output: Path) -> dict:
    if not input_dir.is_dir():
        raise ValueError(f"Input directory does not exist: {input_dir}")
    if output.suffix.lower() != ".png":
        raise ValueError("The output file must have a .png extension.")

    sources: list[tuple[Path, Image.Image]] = []
    for number in range(1, 11):
        matches = sorted(input_dir.glob(f"{number:02d}-*.png"))
        if len(matches) != 1:
            raise ValueError(f"Expected exactly one {number:02d}-*.png; found {len(matches)}.")
        if matches[0].resolve() == output.resolve():
            raise ValueError("The output must not overwrite an input image.")
        with Image.open(matches[0]) as image:
            source = ImageOps.exif_transpose(image).convert("RGBA")
            if source.width == 0 or source.height == 0:
                raise ValueError(f"Empty source image: {matches[0]}")
            sources.append((matches[0], source))

    width = MARGIN * 2 + COLS * CELL_WIDTH + (COLS - 1) * GAP
    height = MARGIN * 2 + HEADER_HEIGHT + ROWS * CELL_HEIGHT + (ROWS - 1) * GAP
    sheet = Image.new("RGB", (width, height), "white")
    draw = ImageDraw.Draw(sheet)
    title_font = load_font(30, bold=True)
    number_font = load_font(27, bold=True)
    label_font = load_font(13)
    subtitle_font = load_font(15)

    draw.text((MARGIN, MARGIN), "ShiLu | 10 Icon Concepts", fill=INK, font=title_font)
    draw.text(
        (MARGIN, MARGIN + 43),
        "Original proportions preserved  /  Small previews shown at actual 32 px and 48 px",
        fill=MUTED,
        font=subtitle_font,
    )

    metrics = []
    for index, (path, source) in enumerate(sources):
        column = index % COLS
        row = index // COLS
        left = MARGIN + column * (CELL_WIDTH + GAP)
        top = MARGIN + HEADER_HEIGHT + row * (CELL_HEIGHT + GAP)
        draw.text((left + 14, top), f"{index + 1:02d}", fill=INK, font=number_font)

        large_left = left + (CELL_WIDTH - LARGE_PREVIEW) // 2
        large_top = top + 46
        paste_preview(sheet, source, large_left, large_top, LARGE_PREVIEW)

        preview_top = top + 326
        # Their lower edges align, with enough whitespace to judge tiny silhouettes.
        small_left = left + 106
        medium_left = left + 168
        paste_preview(sheet, source, small_left, preview_top + 16, 32)
        paste_preview(sheet, source, medium_left, preview_top, 48)
        draw.text((small_left + 16, preview_top + 58), "32 px", fill=MUTED, font=label_font, anchor="mt")
        draw.text((medium_left + 24, preview_top + 58), "48 px", fill=MUTED, font=label_font, anchor="mt")

        metrics.append(
            {
                "number": f"{index + 1:02d}",
                "file": path.name,
                "source_dimensions": [source.width, source.height],
                "cell_bounds": [left, top, left + CELL_WIDTH, top + CELL_HEIGHT],
                "preview_sizes_px": [LARGE_PREVIEW, 32, 48],
            }
        )

    output.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(output, format="PNG", optimize=True)
    return {
        "output": str(output.resolve()),
        "dimensions": [width, height],
        "bytes": output.stat().st_size,
        "columns": COLS,
        "rows": ROWS,
        "images": metrics,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        metrics = build_sheet(args.input_dir, args.output)
    except (OSError, ValueError) as error:
        parser.exit(1, f"Contact sheet error: {error}\n")
    print(json.dumps(metrics, ensure_ascii=True, indent=2))


if __name__ == "__main__":
    main()
