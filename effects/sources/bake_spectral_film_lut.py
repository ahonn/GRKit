"""Bake the Spectral Film LUT tables that the GRKit effects are fitted from.

Requirements: spectral_film_lut (git bcbcfc6, MIT, Jan Lohse) on Python 3.13.

    python bake_spectral_film_lut.py OUT_DIR

The script writes OUT_DIR/<effect-id>.cube: Panasonic V-Log / V-Gamut input, sRGB output, 33^3.
It calls the library without its GUI and passes the GUI default for each setting, except the
color spaces. Instant films have no print stock; their projection balances mid gray.
A Wratten filter multiplies the film sensitivity by the filter transmission. The library then
normalizes exposure on mid gray, as a meter behind the filter does.
Floating-point results can differ on other platforms. manifest.json records the hashes of the
cubes that the effects were fitted from.
"""

import copy
import os
import sys
from pathlib import Path

from spectral_film_lut.film_loader import load_filmstocks
from spectral_film_lut.utils import create_lut
from spectral_film_lut.wratten_filters import WRATTEN

# effect ID -> (negative or instant film, print stock or None, slide white-balance mode,
# Wratten filter or None)
RECIPES = {
    "trix-400-mono-v1": ("Kodak Tri-X 400", "Kodak Polymax Fine-Art Paper Grade 2", 0, None),
    "red-filter-mono-v1": ("Kodak Tri-X 400", "Kodak Polymax Fine-Art Paper Grade 2", 0, "25"),
    "fp-100c-v1": ("Fuji FP-100C", None, 1, None),
    "aerocolor-iv-v1": ("Kodak Aerocolor IV 2460", "Kodak Portra Endura Paper", 0, None),
    "exr-200t-2383-v1": ("Kodak EXR 200T 5293", "Kodak Vision 2383", 0, None),
}


def bake(
    stocks, effect_id: str, film: str, paper: str | None, balance: int, wratten: str | None
) -> None:
    """Write <effect-id>.cube in the working directory. The file title is the effect ID."""
    negative = stocks[film]
    if wratten:
        negative = copy.deepcopy(negative)
        negative.sensitivity = negative.sensitivity * WRATTEN[wratten].reshape(-1, 1)
    create_lut(
        negative,
        stocks[paper] if paper else None,
        name=effect_id,
        lut_size=33,
        cube=True,
        input_colorspace="V-Gamut",
        output_gamut="Rec. 709",
        gamma_func="sRGB",
        projector_kelvin=6500,
        exp_comp=0,
        white_clip=False,
        white_balance=balance,
        exp_kelvin=6500,
        mode="full",
        color_masking=negative.color_masking,
        apd_intermediate=True,
        reference_negative=stocks["Kodak Vision3 250D 5207"],
        upsampling_method="SFL upsampling",
    )


if __name__ == "__main__":
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    stocks = load_filmstocks(lambda *args: None)
    os.chdir(out)
    for effect_id, (film, paper, balance, wratten) in RECIPES.items():
        bake(stocks, effect_id, film, paper, balance, wratten)
        print(f"{effect_id}.cube")
