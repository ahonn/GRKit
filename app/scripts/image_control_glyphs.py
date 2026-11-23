#!/usr/bin/env python3
"""Write src/ui/image_control_glyphs.rs: the text of the Image Control icons as outlines of
D-DIN and D-DIN Condensed Bold, on the 40-unit grid of the icon.

Run with fonttools:
    uv run --no-project --with fonttools python app/scripts/image_control_glyphs.py \\
        D-DIN.ttf D-DINCondensed-Bold.ttf
"""
from pathlib import Path
import sys

from fontTools.pens.basePen import decomposeQuadraticSegment
from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.ttLib import TTFont

OUT = Path(__file__).resolve().parents[1] / "src/ui/image_control_glyphs.rs"

STD = [("Std", 22.625, 28.125, 20.625, 0.938, 3)]
HDR = [("HDR", 17.625, 26.125, 19.75, 1.071, 3)]
BW = ("BW", 21.125, 33.5, 19.5, 1.015, 3)
CINE = ("Cine", 19.75, 20.125, 20.0, 0.894, 3)
# Text, size, baseline, center, width scale, and added weight in eighths of a unit.
ICONS = [
    STD,
    [("Vi", 21.875, 27.25, 20.75, 1.06, 3)],
    [("BW", 21.125, 27.5, 19.5, 1.015, 3)],
    [("Soft", 14.125, 15.0, 20.625, 0.937, 2), BW],
    [("Hard", 13.625, 14.75, 20.25, 1.06, 2), BW],
    [("Hi", 14.25, 15.0, 20.25, 0.99, 2), BW],
    [("Nega", 17.875, 26.375, 20.25, 0.875, 4)],
    [("Posi", 20.0, 27.125, 20.25, 0.95, 3)],
    [("BB", 22.5, 27.75, 20.5, 0.94, 3)],
    [("Retro", 17.25, 26.0, 19.5, 0.969, 0)],
    HDR,
    [],
    [CINE, ("Y", 16.5, 35.0, 20.0, 1.03, 3)],
    [CINE, ("G", 16.0, 34.5, 20.5, 1.085, 2)],
    STD,
    [("Soft", 21.25, 27.875, 20.375, 0.907, 5)],
    [("Hi", 20.625, 27.5, 19.5, 1.075, 3)],
    [("Gra", 21.75, 28.0, 19.875, 1.026, 3)],
    [("Sol", 22.0, 28.0, 20.0, 1.018, 3)],
    HDR,
    [],
]
RETRO = 9


def number(value):
    return f"{value:.2f}".rstrip("0").rstrip(".")


def outline(font, line):
    text, size, baseline, center, scale_x, _ = line
    glyphs = font.getGlyphSet()
    names = font.getBestCmap()
    unit = size / font["head"].unitsPerEm
    width = sum(font["hmtx"][names[ord(c)]][0] for c in text)
    left = center - width * unit * scale_x / 2
    out, pen_x = [], 0

    def point(p):
        return f"{number(left + (pen_x + p[0]) * unit * scale_x)} {number(baseline - p[1] * unit)}"

    for character in text:
        name = names[ord(character)]
        pen = DecomposingRecordingPen(glyphs)
        glyphs[name].draw(pen)
        for op, args in pen.value:
            if op == "moveTo":
                out.append("M" + point(args[0]))
            elif op == "lineTo":
                out.append("L" + point(args[0]))
            elif op == "qCurveTo":
                for control, end in decomposeQuadraticSegment(args):
                    out.append(f"Q{point(control)} {point(end)}")
            elif op == "curveTo":
                out.append(f"C{point(args[0])} {point(args[1])} {point(args[2])}")
            elif op in ("closePath", "endPath"):
                out.append("Z")
        pen_x += font["hmtx"][name][0]
    return "".join(out)


def main():
    regular, condensed = TTFont(sys.argv[1]), TTFont(sys.argv[2])
    rows = []
    for index, lines in enumerate(ICONS):
        font = condensed if index == RETRO else regular
        items = "".join(
            f'(\n        "{outline(font, line)}",\n        {line[5] / 4:.2f},\n    ), '
            for line in lines
        )
        rows.append(f"    &[{items}],")
    OUT.write_text(
        "//! The text of the Image Control icons: outlines of D-DIN and D-DIN Condensed Bold\n"
        "//! on the 40-unit grid, and the stroke that adds weight. Written by\n"
        "//! `scripts/image_control_glyphs.py`. The fonts are under the SIL Open Font License\n"
        "//! 1.1 (`assets/D-DIN-LICENSE.txt`).\n\n"
        "/// The lines of each type, in the order of `recipes.type.*`.\n"
        "#[rustfmt::skip]\n"
        "pub const LINES: [&[(&str, f32)]; 21] = [\n" + "\n".join(rows) + "\n];\n"
    )


main()
