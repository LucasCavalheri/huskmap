#!/usr/bin/env python3
"""Cut the round huskmap icon out of the opaque Skia render.

The testing runner paints on white; the icon is a disc of radius 120/256.
Everything outside becomes transparent, with a supersampled edge.
Usage: scripts/icon-alpha.py packaging/icons/huskmap-*.png
"""
import sys

from PIL import Image, ImageDraw

SUPER = 8
RADIUS = 120.5 / 256


def cut(path: str) -> None:
    img = Image.open(path).convert("RGBA")
    w, h = img.size
    mask = Image.new("L", (w * SUPER, h * SUPER), 0)
    r = RADIUS * w * SUPER
    c = w * SUPER / 2
    ImageDraw.Draw(mask).ellipse((c - r, c - r, c + r, c + r), fill=255)
    img.putalpha(mask.resize((w, h), Image.LANCZOS))
    img.save(path, optimize=True)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    for p in sys.argv[1:]:
        cut(p)
