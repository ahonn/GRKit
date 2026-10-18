# Licenses

This file gives the licenses of the files in this package.

## GRKit

The files in `SD_ROOT/` are GRKit software, except the effect files. GRKit is licensed under the MIT License. The license text is in "GRKit license" below.

`INSTALL.BIN` also contains some bytes of the official RICOH firmware. GRKit uses them to identify the firmware before it changes the camera memory. These bytes are not GRKit software, and the MIT License does not apply to them.

The text in the menu icons uses the [Inter](https://github.com/rsms/inter) typeface, which is licensed under the SIL Open Font License 1.1.

## Effects

Each file in `SD_ROOT/SCRIPT/PLUGINS/EFFECTS/` is one Image Control effect. Each effect of the GRKit catalog is a fit of one film simulation LUT. GRKit contributors made the fits. The files contain the fitted parameters, not the LUTs. "Changes to the LUTs" below describes the changes.

If an effect is not in the GRKit catalog, its author gives its license.

### spektrafilm

Portra 400, Gold 200, Ektar 100, Pro 400H, X-Tra 400, Vision3 250D, Vision3 500T, Portra 800 +2, Kodachrome 64, Velvia 100, and Portra 160 derive from LUTs of [spektrafilm](https://github.com/andreavolpato/spektrafilm) 0.3.4 by Andrea Volpato.

These effects are licensed under the [Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/) (CC BY-SA 4.0). The upstream notice is at <https://github.com/andreavolpato/spektrafilm/blob/main/SPEKTRAFILM_LICENSE.txt>. The effects are provided as is, without warranty. Section 5 of the license gives the disclaimer.

### Spectral Film LUT

Tri-X 400, Tri-X 400 Red, FP-100C, Aerocolor IV, and EXR 200T derive from LUTs of [Spectral Film LUT](https://github.com/JanLohse/spectral_film_lut) by Jan Lohse.

These effects are licensed under the MIT License. The copyright and permission notice is in "Spectral Film LUT license" below.

### Names

Effect names and film, paper, and filter names identify the simulated materials. Some of these names are trademarks of their owners. GRKit is not affiliated with or endorsed by RICOH IMAGING COMPANY, LTD., Eastman Kodak Company, Kodak Alaris, or Fujifilm. The simulations are models of published data, not measured equivalents of the products.
