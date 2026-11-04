# Licenses

This file gives the licenses of the files in this package.

## GRKit

The files in `SD_ROOT/` are GRKit software, except the effect files. GRKit is licensed under the MIT License. The license text is in "GRKit license" below.

Each `CORE.BIN` file (`CORE1.BIN` and on, in a package for several firmware builds) also contains some bytes of the official RICOH firmware. GRKit uses them to identify the firmware before it changes the camera memory. These bytes are not GRKit software, and the MIT License does not apply to them.

The text in the menu icons and in the live view uses the [Roboto Condensed](https://github.com/googlefonts/roboto-classic) typeface, which is licensed under the SIL Open Font License 1.1. GRKit uses its Light, Bold, and Black weights, with only the ASCII characters.

The installation guide uses the [Noto Sans CJK](https://github.com/notofonts/noto-cjk) typefaces, which are licensed under the SIL Open Font License 1.1. Its code text uses DejaVu Sans Mono, which is licensed under the DejaVu Fonts License.

The screenshot in the installation guide shows the photo [Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/) by nagi usano, which is licensed under [CC BY 2.0](https://creativecommons.org/licenses/by/2.0/). The photo is cropped and scaled, and the screenshot shows the GRKit menu and the Portra 400 effect over it.

## Effects

Each file in `SD_ROOT/GRKIT/EFFECTS/` is one Film Look effect. Each effect of the GRKit catalog is a fit of one film simulation LUT. GRKit contributors made the fits. The files contain the fitted parameters, not the LUTs. "Changes to the LUTs" below describes the changes.

If an effect is not in the GRKit catalog, its author gives its license.

### spektrafilm

Portra 400, Gold 200, Ektar 100, Pro 400H, Superia 400, Vision3 250D, Vision3 500T, Portra 800 +2, Kodachrome 64, Velvia 100, Ektachrome E100, Fujicolor C200, UltraMax 400, and Provia 100F derive from LUTs of [spektrafilm](https://github.com/andreavolpato/spektrafilm) 0.3.4 by Andrea Volpato.

These effects are licensed under the [Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/) (CC BY-SA 4.0). The upstream notice is at <https://github.com/andreavolpato/spektrafilm/blob/main/SPEKTRAFILM_LICENSE.txt>. The effects are provided as is, without warranty. Section 5 of the license gives the disclaimer.

### Spectral Film LUT

Tri-X 400, Tri-X 400 Red, Aerocolor IV, Eterna 500T, Instax, and Double-X 5222 derive from LUTs of [Spectral Film LUT](https://github.com/JanLohse/spectral_film_lut) by Jan Lohse.

These effects are licensed under the MIT License. The copyright and permission notice is in "Spectral Film LUT license" below.

### t3mujinpack

HP5 Plus 400 derives from a HaldCLUT of [t3mujinpack](https://github.com/t3mujin/t3mujinpack) by João Almeida, a set of darktable film styles.

This effect is licensed under the MIT License. The copyright and permission notice is in "t3mujinpack license" below.

### Names

Effect names and film, paper, and filter names identify the simulated materials. Some of these names are trademarks of their owners. GRKit is not affiliated with or endorsed by RICOH IMAGING COMPANY, LTD., Eastman Kodak Company, Kodak Alaris, Fujifilm, or HARMAN technology Limited (Ilford Photo). The simulations are models of published data or hand-made styles, not measured equivalents of the products.
