#!/usr/bin/env python3
"""Write src/cover_font.rs from IBM Plex Sans Condensed Bold, the font of the site's covers.

Run with fonttools and brotli:
    uv run --no-project --with fonttools --with brotli python crates/grkit-core/scripts/cover_font.py \
        web/node_modules/@fontsource/ibm-plex-sans-condensed/files/ibm-plex-sans-condensed-latin-700-normal.woff2
"""
from pathlib import Path
import sys

from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.ttLib import TTFont

CHARACTERS = range(0x20, 0x7f)
OUT = Path(__file__).resolve().parents[1] / "src/cover_font.rs"


def outline(glyphs, name):
    pen = DecomposingRecordingPen(glyphs)
    glyphs[name].draw(pen)
    parts = []
    for operator, points in pen.value:
        if operator == "qCurveTo":
            # TrueType implied on-curve points between the off-curve ones.
            off, end = list(points[:-1]), points[-1]
            for index, control in enumerate(off):
                if index + 1 < len(off):
                    target = ((control[0] + off[index + 1][0]) / 2, (control[1] + off[index + 1][1]) / 2)
                else:
                    target = end
                parts += ["Q", *map(fmt, control), *map(fmt, target)]
        elif operator == "closePath":
            parts.append("Z")
        elif operator in ("moveTo", "lineTo"):
            parts += ["M" if operator == "moveTo" else "L", *map(fmt, points[0])]
        elif operator != "endPath":
            raise ValueError(f"Unexpected {operator} in {name}")
    return " ".join(parts)


def fmt(value):
    return f"{value:g}"


def subtables(gpos):
    indexes = sorted({i for record in gpos.FeatureList.FeatureRecord if record.FeatureTag == "kern"
                      for i in record.Feature.LookupListIndex})
    for index in indexes:
        lookup = gpos.LookupList.Lookup[index]
        yield [t.ExtSubTable if t.LookupType == 9 else t for t in lookup.SubTable]


def pair_value(table, left, right):
    """The kerning of the pair in this subtable, or None when it does not apply."""
    if table.LookupType != 2 or left not in table.Coverage.glyphs:
        return None
    if table.Format == 1:
        for record in table.PairSet[table.Coverage.glyphs.index(left)].PairValueRecord:
            if record.SecondGlyph == right:
                return getattr(record.Value1, "XAdvance", 0) if record.Value1 else 0
        return None
    first = table.ClassDef1.classDefs.get(left, 0)
    second = table.ClassDef2.classDefs.get(right, 0)
    value = table.Class1Record[first].Class2Record[second].Value1
    return getattr(value, "XAdvance", 0) if value else 0


def main():
    font = TTFont(sys.argv[1])
    assert font["head"].unitsPerEm == 1000
    cmap, glyphs, metrics = font.getBestCmap(), font.getGlyphSet(), font["hmtx"]
    names = [cmap[c] for c in CHARACTERS]
    lookups = list(subtables(font["GPOS"].table))
    kerning = []
    for a, left in zip(CHARACTERS, names):
        for b, right in zip(CHARACTERS, names):
            total = 0
            for tables in lookups:
                for table in tables:
                    value = pair_value(table, left, right)
                    if value is not None:
                        total += value
                        break
            if total:
                kerning.append((a, b, total))
    lines = [
        "//! IBM Plex Sans Condensed Bold 5.3 for the characters ' ' to '~', in thousandths of the",
        "//! font size. Written by `scripts/cover_font.py`. The font is under the SIL Open Font",
        "//! License 1.1 (`assets/IBMPlexSansCondensed-LICENSE.txt`).",
        "",
        "/// The advance width of each character.",
        f"pub const ADVANCE: [u16; {len(names)}] = [",
        *[f"    {metrics[n][0]}," for n in names],
        "];",
        "",
        "/// The outline of each character, y up: M, L, Q, and Z with absolute coordinates.",
        f"pub const OUTLINE: [&str; {len(names)}] = [",
        *[f'    "{outline(glyphs, n)}",' for n in names],
        "];",
        "",
        "/// The kerning of the pairs that have one, sorted by the pair.",
        f"pub const KERNING: [(u8, u8, i16); {len(kerning)}] = [",
        *[f"    ({a}, {b}, {v})," for a, b, v in kerning],
        "];",
        "",
    ]
    OUT.write_text("\n".join(lines))
    print(len(kerning), OUT.stat().st_size)


main()
