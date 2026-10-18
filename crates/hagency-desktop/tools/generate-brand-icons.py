#!/usr/bin/env python3
"""Render desktop icons from resources/hagency-mark.svg.

Requires CairoSVG and Pillow: python -m pip install cairosvg pillow
Run from any directory: python tools/generate-brand-icons.py
"""

from io import BytesIO
from pathlib import Path

import cairosvg
from PIL import Image


def main():
    resources = Path(__file__).resolve().parents[1] / "resources"
    source = (resources / "hagency-mark.svg").read_bytes()
    sizes = (16, 24, 32, 48, 64, 128, 256, 512, 1024)
    images = {}
    for size in sizes:
        png = cairosvg.svg2png(
            bytestring=source, output_width=size * 4, output_height=size * 4
        )
        with Image.open(BytesIO(png)) as image:
            images[size] = image.convert("RGBA").resize(
                (size, size), Image.Resampling.LANCZOS
            )
        if size >= 32:
            images[size].save(resources / f"icon_{size}.png", optimize=True)

    ico_sizes = [size for size in sizes if size <= 256]
    images[256].save(
        resources / "icon.ico",
        sizes=[(size, size) for size in ico_sizes],
        append_images=[images[size] for size in ico_sizes if size != 256],
    )
    images[1024].save(resources / "icon.icns")
    print("Generated desktop PNG, ICO and ICNS icons from hagency-mark.svg")


if __name__ == "__main__":
    main()
