"""Bake the spektrafilm LUTs that the GRKit effects are fitted from.

Requirements: spektrafilm 0.3.4 (git 3bb2c2d2801ff68b92019cf1dbcbb133d60832bc) on Python 3.13.

    python bake_spektrafilm.py OUT_DIR

The script writes OUT_DIR/<effect-id>.cube: Panasonic V-Log / V-Gamut input, sRGB output, 33^3.
It removes the "# Created:" header line, so the same environment writes the same bytes.
Floating-point results can differ on other platforms. manifest.json records the hashes of the
cubes that the effects were fitted from.

spektrafilm-lut always renders through a print stock. A slide (positive) film printed on
negative-working paper gives an inverted image, so this script scans slide films directly
(`io.scan_film`), as the spektrafilm GUI does. The print argument is then unused.

A lab scan of a print sets its black and white points. For the effects in SCAN_LEVELS, the
script turns on the scanner black and white correction (default levels 0.01 and 0.98).
spektrafilm-lut turns it off for every LUT, but the correction uses fixed references (the
film base and the film's maximum density), not image statistics, so a LUT can hold it.
"""

import shutil
import sys
import tempfile
from pathlib import Path

import spektrafilm.runtime.params_builder as params_builder
from spektrafilm_lut_creator.cli import main as lut_main

# effect ID -> (film profile, print profile)
RECIPES = {
    "portra-400-endura-v1": ("kodak_portra_400", "kodak_portra_endura"),
    "gold-200-premier-v1": ("kodak_gold_200", "kodak_endura_premier"),
    "ektar-100-edge-v1": ("kodak_ektar_100", "kodak_ektacolor_edge"),
    "pro-400h-crystal-v1": ("fujifilm_pro_400h", "fujifilm_crystal_archive_typeii"),
    "xtra-400-crystal-v1": ("fujifilm_xtra_400", "fujifilm_crystal_archive_typeii"),
    "vision3-250d-2383-v1": ("kodak_vision3_250d", "kodak_2383"),
    "vision3-500t-supra-v1": ("kodak_vision3_500t", "kodak_supra_endura"),
    "portra-800-push2-v1": ("kodak_portra_800_push2", "kodak_portra_endura"),
    "kodachrome-64-v1": ("kodak_kodachrome_64", "kodak_portra_endura"),
    "velvia-100-v1": ("fujifilm_velvia_100", "kodak_portra_endura"),
    "ektachrome-e100-v1": ("kodak_ektachrome_100", "kodak_portra_endura"),
    "c200-premier-v1": ("fujifilm_c200", "kodak_endura_premier"),
    "ultramax-400-ultra-v1": ("kodak_ultramax_400", "kodak_ultra_endura"),
    "provia-100f-v1": ("fujifilm_provia_100f", "kodak_portra_endura"),
}

# Effects whose print is scanned with fixed black and white levels.
SCAN_LEVELS = {"vision3-500t-supra-v1"}

_scan_levels = False

_digest = params_builder.digest_params


def _digest_scanning_slides(params):
    if params.film.is_positive:
        params.io.scan_film = True
    params = _digest(params)
    if _scan_levels:
        # After _digest, which turns the correction off in LUT mode.
        params.scanner.black_correction = True
        params.scanner.white_correction = True
    return params


params_builder.digest_params = _digest_scanning_slides


def bake(effect_id: str, film: str, paper: str, out: Path) -> None:
    global _scan_levels
    _scan_levels = effect_id in SCAN_LEVELS
    with tempfile.TemporaryDirectory() as tmp:
        status = lut_main([
            "build", "--film", film, "--print", paper, "--input", "vlog", "--output", "srgb",
            "--resolution", "33", "--name", effect_id, "--out", tmp,
        ])
        if status != 0:
            raise SystemExit(f"spektrafilm-lut failed for {effect_id}")
        (cube,) = (Path(tmp) / effect_id).glob("*.cube")
        lines = cube.read_text().splitlines(keepends=True)
        (out / f"{effect_id}.cube").write_text(
            "".join(line for line in lines if not line.startswith("# Created:"))
        )


if __name__ == "__main__":
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    for effect_id, (film, paper) in RECIPES.items():
        bake(effect_id, film, paper, out)
        print(f"{effect_id}.cube")
    shutil.rmtree(out / "__pycache__", ignore_errors=True)
