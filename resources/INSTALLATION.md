# GRKit

For RICOH GR III and GR III HDF firmware 2.10 only.

GRKit is experimental and has no warranty. If GRKit fails, the camera can stop during startup. Read the [recovery guide](https://github.com/ahonn/grkit/blob/main/resources/RECOVERY.md) before you install. When you install GRKit, you accept the [terms of use](https://github.com/ahonn/grkit/blob/main/resources/TERMS.md).

## Install

In Chrome or Edge on a computer, [grkit.app](https://grkit.app/) can write GRKit to the card in a card reader. It replaces step 2.

1. Turn the camera off.
2. Copy the contents of `SD_ROOT`, not the folder itself, to the root of the SD card. The card root then has the `SCRIPT` folder.
3. Insert the card. Hold MENU while you turn the camera on. Wait until the camera turns itself off.
4. Turn the camera on normally.

Step 3 turns on the SD scripts of the camera: the camera shows Mode Set Command and Complete, then turns itself off. If the SD scripts are already on, skip step 3.

![The Mode Set Command screen shows Script - Enable and Complete.](images/modeset.png)

After the installation, you can remove `00078350.588` and `DEVELOP.MOD` from the card root.

## Use

A few seconds after power-on, the shooting screen shows **GRKit** at the top: GRKit has loaded. The first start after an install or update takes longer. Photos taken before GRKit has loaded have no GRKit effects. In shooting mode, press the ADJ lever and select the sixth entry, GRK.

![The GRKit page lists the modules and shows the current effect.](images/grkit-root.jpg)

Photo: [MIKI Yoshihito](https://www.flickr.com/photos/mujitra/53311001084/), [CC BY 2.0](https://creativecommons.org/licenses/by/2.0/).

- **Film Look:** highlight an effect to preview it in the live view, and press OK to keep it. Press Fn to adjust the effect. It does not change the Image Control settings of the camera.
- **Framing:** XPan, 2.35:1, 16:9, and other ratios. It crops JPEG photos, and DNG files stay 3:2. Press Fn to use the frame for composition only, or to set a mask.
- **Guides:** lines on the screen only, not in the photo.
- **Date Imprint:** the date at the bottom right of photos. Press Fn to select the date style and the font.
- **Glow:** Bloom and Halation, on JPEG photos only, with no preview in the live view.
- **Grain:** film grain on photos. Press Fn to select the grain size.

All the settings are also in the **GRKit** section at the end of the Customize tab (C) of MENU. Each module has a row. Turn the module on or off there, check the effects, ratios, or styles that ADJ shows in **Options**, and set the order in **ADJ Order**. GRKit keeps your settings after power-off.

## Update

In Chrome or Edge on a computer, [grkit.app](https://grkit.app/) can update the card in a card reader. It replaces steps 2 and 3.

1. Turn the camera off.
2. Delete the `SCRIPT` folder from the card.
3. Copy the `SCRIPT` folder of `SD_ROOT` of the new package to the root of the SD card.
4. Turn the camera on normally. You do not need to hold MENU.

GRKit keeps your settings and effects in the `GRKIT` folder, so the update keeps them. To update from GRKit 1.2, use grkit.app, which keeps the settings and effects.

## Fast start (optional)

On the official firmware, GRKit waits until the camera has read the card, about 8 to 10 seconds after power-on. With the fast-boot firmware, GRKit loads one to two seconds after power-on. The same package works on both firmwares. Without the fast-boot firmware, GRKit has every feature.

Make the fast-boot firmware on the [Fast start page](https://grkit.app/fastboot/) of grkit.app. Select the official firmware that you downloaded from Ricoh, and the page makes the fast-boot firmware. Install it as you install an official update. The installation updates the camera firmware. If the update stops, the camera may not start. To go back, install the official firmware of the same version.

## Troubleshooting

### The camera starts a firmware update when you hold MENU

The card root has a firmware file. Turn the camera off, remove each `.bin` file from the card root, then do step 3 of the installation again.

### The camera does not show Mode Set Command

- Hold MENU, then turn the camera on.
- Copy the contents of `SD_ROOT` to the card root, not into `DCIM` or another folder.

### GRK does not show in ADJ

1. Wait a few seconds after power-on, until the shooting screen shows **GRKit** at the top.
2. Make sure that the card root has the `SCRIPT` folder.
3. If the card has `SCRIPT/GRKIT/INSTALL.ERR`, open the camera menu and make sure that the package is for the firmware version of your camera. If it is, send a [device report](https://github.com/ahonn/grkit/issues/new?template=device-report.yml) with `INSTALL.ERR`, `GRKIT/REPORT.TXT`, and `GRKIT/BOOT.TXT` if the card has it.
4. GRKit runs only on the official Ricoh firmware and on the fast-boot firmware of grkit.app.

### GRKit stops working after you format the card

A format deletes GRKit and its settings. Before you format the card, copy the `GRKIT` folder to a computer. After the format, copy the contents of `SD_ROOT` again, then copy the `GRKIT` folder back to the card root.

## Remove GRKit

In Chrome or Edge on a computer, [grkit.app](https://grkit.app/) can remove GRKit from the card in a card reader, and can also turn SD scripts off.

Turn the camera off, then delete the `SCRIPT` and `GRKIT` folders from the card.

The SD script setting stays on. To turn it off:

1. On https://grkit.app, download the files that turn SD scripts off for your camera.
2. Copy the contents of `SD_ROOT` to the root of the SD card.
3. Insert the card. Hold MENU while you turn the camera on. Wait until the camera turns itself off.
4. Remove `00078350.588` and `DEVELOP.MOD` from the card root.

If the camera stops during startup, see the [recovery guide](https://github.com/ahonn/grkit/blob/main/resources/RECOVERY.md).

GRKit is not affiliated with Ricoh. Source code and licenses: https://github.com/ahonn/grkit
