# /// script
# requires-python = ">=3.11"
# dependencies = ["fonttools[woff]", "uharfbuzz"]
# ///
"""Builds the GRKit logo and app icons from IBM Plex Sans Condensed Bold outlines.

The logo is a red box with a white K. The wordmark is GR followed by the red box with Kit.
The app icons stack GR above the red Kit box; the iOS icon is the Icon Composer bundle
app/ios/AppIcon.icon. Every letter is an outline, so no file needs the font.

Run from the repository root after `pnpm install` in web/, with resvg on the PATH:

    uv run resources/brand/build.py
"""

import json
import os
import shutil
import subprocess
import tempfile

import uharfbuzz as hb
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont

FONT = "web/node_modules/@fontsource/ibm-plex-sans-condensed/files/ibm-plex-sans-condensed-latin-700-normal.woff2"
OUT = "resources/brand"
RED, INK, WHITE, BLACK = "#cf142b", "#0b0b0b", "#ffffff", "#000000"

font = TTFont(FONT)
font.flavor = None
glyphs = font.getGlyphSet()

# Font units. The box is as tall as the cap height plus PAD_Y above and below, so the box
# height is close to one em.
CAP = font["OS/2"].sCapHeight
PAD_Y = 160
PAD_X = 120
GAP = 110  # between GR and the box, across in the wordmark and down in the app icon
BOX_H = CAP + 2 * PAD_Y

with tempfile.NamedTemporaryFile(suffix=".ttf") as plain:
    font.save(plain.name)
    HB_FONT = hb.Font(hb.Face(hb.Blob.from_file_path(plain.name)))


def fmt(v):
    return f"{v:.2f}".rstrip("0").rstrip(".")


class Run:
    """A line of text shaped with kerning, with its outline and ink bounds at baseline 0."""

    def __init__(self, text):
        buf = hb.Buffer()
        buf.add_str(text)
        buf.guess_segment_properties()
        hb.shape(HB_FONT, buf, {"kern": True})
        self.glyphs, x = [], 0
        for info, pos in zip(buf.glyph_infos, buf.glyph_positions):
            self.glyphs.append((font.getGlyphName(info.codepoint), x + pos.x_offset))
            x += pos.x_advance
        bounds = BoundsPen(glyphs)
        for name, gx in self.glyphs:
            glyphs[name].draw(TransformPen(bounds, (1, 0, 0, 1, gx, 0)))
        self.x0, _, self.x1, _ = bounds.bounds
        self.w = self.x1 - self.x0

    def path(self, left, baseline, scale=1):
        """SVG path data with the ink's left edge at `left` and the baseline at `baseline`."""
        pen = SVGPathPen(glyphs, ntos=fmt)
        for name, gx in self.glyphs:
            t = (scale, 0, 0, -scale, left + (gx - self.x0) * scale, baseline)
            glyphs[name].draw(TransformPen(pen, t))
        return pen.getCommands()


GR, KIT, K = Run("GR"), Run("Kit"), Run("K")
BOX_W = KIT.w + 2 * PAD_X


def svg(w, h, body, bg=None):
    back = f'<rect width="{fmt(w)}" height="{fmt(h)}" fill="{bg}"/>' if bg else ""
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {fmt(w)} {fmt(h)}" role="img">'
        f"<title>GRKit</title>{back}{body}</svg>\n"
    )


def rect(x, y, w, h):
    return f"M{fmt(x)} {fmt(y)}h{fmt(w)}v{fmt(h)}h{fmt(-w)}Z"


def mark():
    body = f'<path fill="{RED}" d="{rect(0, 0, BOX_H, BOX_H)}"/>'
    body += f'<path fill="{WHITE}" d="{K.path((BOX_H - K.w) / 2, PAD_Y + CAP)}"/>'
    return svg(BOX_H, BOX_H, body)


def wordmark(gr):
    base = PAD_Y + CAP
    box_x = GR.w + GAP
    body = f'<path fill="{gr}" d="{GR.path(0, base)}"/>'
    body += f'<path fill="{RED}" d="{rect(box_x, 0, BOX_W, BOX_H)}"/>'
    body += f'<path fill="{WHITE}" d="{KIT.path(box_x + PAD_X, base)}"/>'
    return svg(box_x + BOX_W, BOX_H, body)


STACK_W, STACK_H = max(GR.w, BOX_W), CAP + GAP + BOX_H


def stack_paths(canvas, height):
    """Path data of GR, the box, and Kit, `height` tall, centered on a square canvas."""
    s = height / STACK_H
    cx, top = canvas / 2, (canvas - height) / 2
    box_x, box_y = cx - BOX_W * s / 2, top + (CAP + GAP) * s
    gr_d = GR.path(cx - GR.w * s / 2, top + CAP * s, s)
    box_d = rect(box_x, box_y, BOX_W * s, BOX_H * s)
    kit_d = KIT.path(box_x + PAD_X * s, box_y + (PAD_Y + CAP) * s, s)
    return gr_d, box_d, kit_d


def stacked(canvas, height, gr, box, kit=None, bg=None):
    """GR above the Kit box, `height` tall, centered on a square canvas.

    Without `kit`, Kit is cut out of the box, for single-color layers.
    """
    gr_d, box_d, kit_d = stack_paths(canvas, height)
    body = f'<path fill="{gr}" d="{gr_d}"/>'
    if kit is None:
        body += f'<path fill="{box}" fill-rule="evenodd" d="{box_d}{kit_d}"/>'
    else:
        body += f'<path fill="{box}" d="{box_d}"/><path fill="{kit}" d="{kit_d}"/>'
    return svg(canvas, canvas, body, bg)


# Android keeps the content inside the 66 dp safe circle of a 108 dp layer, here at 4x.
ANDROID = 432
ANDROID_H = STACK_H * (33 * 4) / ((STACK_W / 2) ** 2 + (STACK_H / 2) ** 2) ** 0.5 * 0.96

# (file, svg, png size)
FILES = [
    ("logo/mark.svg", mark(), None),
    ("logo/wordmark.svg", wordmark(INK), None),
    ("logo/wordmark-inverse.svg", wordmark(WHITE), None),
    ("app/android/foreground.svg", stacked(ANDROID, ANDROID_H, INK, RED, WHITE), ANDROID),
    ("app/android/background.svg", svg(ANDROID, ANDROID, "", bg=WHITE), ANDROID),
    ("app/android/monochrome.svg", stacked(ANDROID, ANDROID_H, BLACK, BLACK), ANDROID),
    # Google Play masks the listing icon itself.
    ("app/android/play-store.svg", stacked(512, 300, INK, RED, WHITE, bg=WHITE), 512),
]

resvg = shutil.which("resvg")
if resvg is None:
    raise SystemExit("resvg is not on the PATH")
for rel, text, size in FILES:
    path = os.path.join(OUT, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as fh:
        fh.write(text)
    if size:
        subprocess.run(
            [resvg, "-w", str(size), "-h", str(size), path, path[: -len(".svg")] + ".png"],
            check=True,
        )
shutil.copyfile(os.path.join(OUT, "logo/mark.svg"), "web/public/favicon.svg")


def srgb(hex_color):
    r, g, b = (int(hex_color[i : i + 2], 16) / 255 for i in (1, 3, 5))
    return {"solid": f"srgb:{r:.5f},{g:.5f},{b:.5f},1.00000"}


def gray(level):
    return {"solid": f"extended-gray:{level:.5f},1.00000"}


def fills(light, dark, tinted):
    return [
        {"value": light},
        {"appearance": "dark", "value": dark},
        {"appearance": "tinted", "value": tinted},
    ]


def write_ios_icon():
    """The iOS app icon as an Icon Composer bundle: one 1024-point SVG per layer.

    Icon Composer adds the corner mask, the Liquid Glass effects, and the dark and tinted
    appearances from the fills in icon.json.
    """
    bundle = os.path.join(OUT, "app/ios/AppIcon.icon")
    shutil.rmtree(bundle, ignore_errors=True)
    os.makedirs(os.path.join(bundle, "Assets"))
    gr_d, box_d, kit_d = stack_paths(1024, 600)
    for name, d, color in (("gr", gr_d, INK), ("box", box_d, RED), ("kit", kit_d, WHITE)):
        with open(os.path.join(bundle, "Assets", f"{name}.svg"), "w") as fh:
            fh.write(svg(1024, 1024, f'<path fill="{color}" d="{d}"/>'))

    def layer(name, light, dark, tinted):
        return {
            "fill-specializations": fills(light, dark, tinted),
            "image-name": f"{name}.svg",
            "name": name,
        }

    group = {"shadow": {"kind": "neutral", "opacity": 0.5}, "translucency": {"enabled": False, "value": 0}}
    icon = {
        "fill-specializations": [
            {"value": srgb(WHITE)},
            {"appearance": "dark", "value": srgb(INK)},
        ],
        # The first group and the first layer of a group are in front.
        "groups": [
            {
                **group,
                "name": "Kit",
                "layers": [
                    layer("kit", srgb(WHITE), srgb(WHITE), gray(0)),
                    layer("box", srgb(RED), srgb(RED), gray(1)),
                ],
            },
            {**group, "name": "GR", "layers": [layer("gr", srgb(INK), srgb(WHITE), gray(1))]},
        ],
        "supported-platforms": {"squares": "shared"},
    }
    with open(os.path.join(bundle, "icon.json"), "w") as fh:
        json.dump(icon, fh, indent=2)
        fh.write("\n")


write_ios_icon()
