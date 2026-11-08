# Effect attribution

Each film effect is a fit of one film simulation LUT to the effect model of the GR cameras: a
3 x 3 look matrix, optional hue sectors, three tone curves, and an optional post-curve matrix.
GRKit contributors made the fits. The repository and the SD packages contain the fitted
parameters, not the LUTs.

## spektrafilm

Portra 400, Gold 200, Ektar 100, Pro 400H, Superia 400, Vision3 250D, Vision3 500T, Portra 800 +2,
Kodachrome 64, Velvia 100, Ektachrome E100, Fujicolor C200, UltraMax 400, and Provia 100F derive
from LUTs of [spektrafilm](https://github.com/andreavolpato/spektrafilm) 0.3.4 by Andrea Volpato,
licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). These effect
definitions are also licensed under CC BY-SA 4.0.

`sources/SPEKTRAFILM_LICENSE.txt` contains the upstream notice. `sources/CHANGELOG.txt` describes
the changes: GRKit scans the four slide films directly instead of printing them, sets the scan
levels of Vision3 500T, and fits each LUT to the effect model.

## Spectral Film LUT

Tri-X 400, Tri-X 400 Red, Aerocolor IV, Eterna 500T, Instax, and Double-X 5222 derive from LUTs
of [Spectral Film LUT](https://github.com/JanLohse/spectral_film_lut) by Jan Lohse, licensed under
the MIT License. `sources/SPECTRAL_FILM_LUT_LICENSE.txt` contains its copyright and permission
notice. These effect definitions are also licensed under the MIT License. Tri-X 400 Red applies
the library's Kodak Wratten 25 data to Tri-X 400.

## t3mujinpack

HP5 Plus 400 derives from a HaldCLUT of the darktable styles of
[t3mujinpack](https://github.com/t3mujin/t3mujinpack) by João Almeida, licensed under the MIT
License. `sources/T3MUJINPACK_LICENSE.txt` contains its notice. This effect definition is also
licensed under the MIT License.

## GRKit looks

Positive GRII is a look of this repository, in the spirit of the Positive Film image setting of the
RICOH GR II: deep blacks, a hard curve around the midtones, a warm cast that fades toward white,
more saturation, blues toward teal, greens toward yellow-olive, reds toward orange.
`sources/make_positive_grii.py` writes its LUT, a HaldCLUT on the Standard JPEG, from parameters set
by eye against GR II photographs, not from camera data, and `grkit effects import --input standard`
fits it. This effect definition is licensed under the MIT License of GRKit. RICOH and GR are
trademarks of RICOH IMAGING COMPANY, LTD.; the name says which camera setting the look follows.

## Names and limits

Effect names and film, paper, and filter names identify the simulated materials. Some of these
names are trademarks of their owners. GRKit is not affiliated with or endorsed by Eastman Kodak Company,
Kodak Alaris, Fujifilm, or HARMAN technology Limited (Ilford Photo). The simulations are models
of published data or hand-made styles, not measured equivalents of the products.

The effects operate in the camera's native image-processing domain. They are not display-sRGB
JPEG transforms. They add no grain, halation, or other spatial effect. The fits assume 18% gray
at base-gamma index 7.8.

Icon text uses the Inter typeface (SIL Open Font License 1.1, see
`crates/grkit-core/assets/Inter-LICENSE.txt`). Icons show manufacturer and film names as
text in Inter or in a pixel font. They use no manufacturer logo.
The project code is licensed under the MIT License (see `LICENSE` at the repository root).

## Packages

Each SD package contains `LICENSES.md`, which gives these notices for the package files. Its source is
`resources/LICENSES.md`. When you add an effect or a source, update both files. A test checks that both
files name each catalog effect under its source.
