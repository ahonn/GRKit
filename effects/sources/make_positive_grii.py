"""Positive GRII: a look in the spirit of the Positive Film image setting of the RICOH GR II,
as a HaldCLUT (level 8, 512 x 512) on the Standard JPEG of the camera (`--input standard`).

    python3 -I effects/sources/make_positive_grii.py positive-film-grii-v1.png [NAME=VALUE ...] [preview.jpg ...]

The look, from the Standard rendering: deep blacks and a hard S-curve around the midtones as
a reversal film, a warm cast that is strongest in the shadows and midtones and fades toward
white, saturation around the luma (so colored areas keep their brightness), saturated colors
that darken a little (the "chrome" depth of a slide), blues toward teal, greens toward
yellow-olive and a little quieter, reds toward orange. The numbers are by eye against GR II
Positive Film photographs, not a measurement. The fit of the camera model is grade A (mean
CIEDE2000 0.84 on the ColorChecker).
With extra image paths, the script also writes `<name>.positive-grii.jpg` previews next to
each one. Needs numpy and Pillow.
"""
import sys, pathlib
import numpy as np
from PIL import Image

LEVEL = 8  # HaldCLUT level: LEVEL^2 = 64 steps per channel, 512 x 512 pixels

# ---- parameters -----------------------------------------------------------------------------
BLACK = 0.002         # linear black offset that the look crushes
CONTRAST = 1.20       # log2 contrast gain around the pivot
SHOULDER = 0.80       # linear value where the highlight shoulder starts
SHADOW_GAMMA = 1.0    # extra gamma below the pivot (1 = none)
MID = 0.11            # the pivot of the contrast: the linear value of a typical midtone of a JPEG
WARM_R, WARM_B = 0.06, -0.10   # warm cast at black (R up, B down), fading to WARM_WHITE at white
WARM_WHITE = 0.25
SATURATION = 1.22     # saturation gain around the luma
VIBRANCE = 0.0        # extra saturation gain for unsaturated colors (0 = none; the camera model
                      # cannot follow it well)
CHROME = 0.04         # darkening of saturated colors
BLUE_TURN = -8.0      # degrees: blues toward cyan/teal
GREEN_TURN = -6.0     # degrees: greens toward yellow
GREEN_QUIET = 0.90    # saturation factor of greens
RED_TURN = 4.0        # degrees: reds toward orange


def srgb_decode(v):
    v = np.clip(v, 0.0, 1.0)
    return np.where(v <= 0.04045, v / 12.92, ((v + 0.055) / 1.055) ** 2.4)


def srgb_encode(v):
    v = np.clip(v, 0.0, 1.0)
    return np.where(v <= 0.0031308, v * 12.92, 1.055 * v ** (1 / 2.4) - 0.055)


def luminance(rgb):
    return rgb[..., 0] * 0.2126 + rgb[..., 1] * 0.7152 + rgb[..., 2] * 0.0722


def tone(v):
    """The tone curve on linear values: black crush, shadow gamma, log contrast, shoulder."""
    v = np.clip((v - BLACK) / (1.0 - BLACK), 0.0, None)
    below = v < MID
    v = np.where(below, MID * (np.maximum(v, 1e-6) / MID) ** SHADOW_GAMMA, v)
    v = MID * 2.0 ** (np.log2(np.maximum(v, 1e-6) / MID) * CONTRAST)
    # A soft shoulder: above SHOULDER, approach 1 asymptotically.
    over = v > SHOULDER
    room = 1.0 - SHOULDER
    v = np.where(over, SHOULDER + room * (1.0 - np.exp(-(v - SHOULDER) / room)), v)
    return np.clip(v, 0.0, 1.0)


def rgb_to_hsv(rgb):
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    mx = rgb.max(axis=-1)
    mn = rgb.min(axis=-1)
    d = mx - mn
    h = np.zeros_like(mx)
    nz = d > 1e-9
    rmax = nz & (mx == r)
    gmax = nz & (mx == g) & ~rmax
    bmax = nz & ~rmax & ~gmax
    h[rmax] = ((g - b)[rmax] / d[rmax]) % 6.0
    h[gmax] = (b - r)[gmax] / d[gmax] + 2.0
    h[bmax] = (r - g)[bmax] / d[bmax] + 4.0
    h = h * 60.0
    s = np.where(mx > 1e-9, d / np.maximum(mx, 1e-9), 0.0)
    return h, s, mx


def hsv_to_rgb(h, s, v):
    h = (h % 360.0) / 60.0
    i = np.floor(h).astype(int)
    f = h - i
    p = v * (1 - s)
    q = v * (1 - s * f)
    t = v * (1 - s * (1 - f))
    i = i % 6
    r = np.choose(i, [v, q, p, p, t, v])
    g = np.choose(i, [t, v, v, q, p, p])
    b = np.choose(i, [p, p, t, v, v, q])
    return np.stack([r, g, b], axis=-1)


def bell(h, center, width):
    """A smooth weight 1 at `center` degrees, 0 beyond `width` degrees away."""
    d = np.abs(((h - center + 180.0) % 360.0) - 180.0)
    return np.clip(np.cos(np.clip(d / width, 0, 1) * np.pi / 2) ** 2, 0, 1)


def look(srgb):
    """The look on an array of sRGB values in 0..1 (shape [..., 3])."""
    lin = srgb_decode(srgb)
    # Tone on the luminance, so that the curve keeps the hue; chroma follows the ratio.
    y = luminance(lin)
    y2 = tone(y)
    ratio = np.where(y > 1e-6, y2 / np.maximum(y, 1e-6), 0.0)[..., None]
    lin = np.clip(lin * ratio, 0.0, 1.0)
    # Warm cast: strongest at black, fading toward white.
    w = 1.0 - (1.0 - WARM_WHITE) * np.clip(y2, 0, 1) ** 0.6
    gain = np.stack([1.0 + WARM_R * w, np.ones_like(w), 1.0 + WARM_B * w], axis=-1)
    lin = np.clip(lin * gain, 0.0, 1.0)
    # Saturation around the luma of the gamma-encoded values, so that it keeps the brightness
    # of colored areas; then the chrome depth and the hue turns in HSV.
    enc = srgb_encode(lin)
    luma = (enc[..., 0] * 0.299 + enc[..., 1] * 0.587 + enc[..., 2] * 0.114)[..., None]
    h0, s0, v0 = rgb_to_hsv(enc)
    gain = SATURATION * (1.0 + VIBRANCE * (1.0 - s0) ** 1.5)
    enc = np.clip(luma + (enc - luma) * gain[..., None], 0.0, 1.0)
    h, s, v = rgb_to_hsv(enc)
    v = v * (1.0 - CHROME * s ** 2)
    h = h + BLUE_TURN * bell(h, 235.0, 50.0) + GREEN_TURN * bell(h, 115.0, 45.0) + RED_TURN * bell(h, 5.0, 25.0)
    s = s * (1.0 - (1.0 - GREEN_QUIET) * bell(h, 110.0, 50.0))
    out = hsv_to_rgb(h, np.clip(s, 0, 1), np.clip(v, 0, 1))
    return np.clip(out, 0.0, 1.0)


def hald_identity(level):
    n = level * level
    side = level ** 3
    idx = np.arange(side * side)
    r = idx % n
    g = (idx // n) % n
    b = idx // (n * n)
    rgb = np.stack([r, g, b], axis=-1) / (n - 1)
    return rgb.reshape(side, side, 3)


if __name__ == '__main__':
    # NAME=VALUE arguments override the parameters above, for trials.
    args = []
    for arg in sys.argv[1:]:
        name, eq, value = arg.partition('=')
        if eq and name.isupper() and name in globals():
            globals()[name] = float(value)
        else:
            args.append(arg)
    sys.argv[1:] = args
    out = pathlib.Path(sys.argv[1])
    hald = look(hald_identity(LEVEL))
    Image.fromarray((hald * 255.0 + 0.5).astype(np.uint8), 'RGB').save(out, 'PNG', optimize=True)
    print('wrote', out)
    for path in sys.argv[2:]:
        p = pathlib.Path(path)
        img = np.asarray(Image.open(p).convert('RGB'), dtype=np.float64) / 255.0
        graded = look(img)
        target = p.with_name(p.stem + '.positive-grii.jpg')
        Image.fromarray((graded * 255.0 + 0.5).astype(np.uint8), 'RGB').save(target, 'JPEG', quality=92)
        print('wrote', target)
