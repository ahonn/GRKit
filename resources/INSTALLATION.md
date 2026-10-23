# GRKit

For RICOH GR III and GR III HDF firmware 2.10 only.

## Install

1. Turn the camera off.
2. Copy the contents of `SD_ROOT` to the root of the SD card.
3. Insert the card. Hold MENU while you turn the camera on. Wait until the camera turns itself off.
4. Turn the camera on normally.

Step 3 turns on the camera's SD scripts. The camera shows Mode Set Command, then Complete, and then turns itself off.

![The Mode Set Command screen shows Script - Enable and Complete.](images/modeset.png)

If the SD scripts are already on, skip step 3.
After the installation, you can remove `00078350.588` and `DEVELOP.MOD` from the card root. This step is optional.

## Use

Open ADJ and select the sixth entry, GRK. GRKit loads a few seconds after power-on, so the entry can appear late. When GRKit has loaded, the shooting screen shows **GRKit** at the top for a few seconds. This label needs the Framing or Guides module.

![The GRKit page lists the modules and shows the current effect.](images/grkit-root.jpg)

Photo: [Snow in Akihabara](https://www.flickr.com/photos/cafetricot/51803682755/) by nagi usano, [CC BY 2.0](https://creativecommons.org/licenses/by/2.0/).

- **Image Control:** highlight an effect to preview it. Press OK to keep it. Press Fn to adjust the Standard parameters of the effect. On the GRK menu, press Fn on the Image Control row to adjust the selected effect. Each effect keeps its own adjustments, and they do not change the camera's Image Control settings.
- **Framing:** select XPan, 2.35:1, 16:9, 4:3, 5:4, or 1:1. 1:1 sets the camera's own 1:1 aspect ratio. Each other ratio sets the native aspect ratio to 3:2, and the viewfinder dims the area outside the photo. DNG files keep the full sensor image and record the crop. A raw converter that applies the recorded crop shows only the photo area.
- **Guides:** select a guide style. The lines show only in the viewfinder.
- **Date Imprint:** press OK to open its settings. In **Date Style**, select a date style. In **Date Look**, select the digits: Classic, GR1s, or GR II. Photos get the date at the bottom right.
- **Grain:** select Weak, Medium, or Strong. Only photos get the grain.

Highlight the last row, GRK, to see the GRKit version.

Press MENU to go back one level. GRKit keeps your settings after power-off.

## Troubleshooting

### The camera starts a firmware update when you hold MENU

The card root has a firmware update file. Turn the camera off. Remove each `.bin` file from the card root. Then do step 3 again.

### The camera does not show Mode Set Command

- Hold MENU, then turn the camera on. If you press MENU after the camera is on, the SD scripts stay off.
- Copy the contents of `SD_ROOT` to the card root, not into `DCIM` or another folder.

### GRK does not show in ADJ

1. Wait a few seconds after power-on. GRKit loads after the camera starts. When it has loaded, the shooting screen shows **GRKit** at the top for a few seconds.
2. Look for GRK in ADJ. It is not in MENU.
3. If the camera did not show Mode Set Command in step 3, do step 3 again.
4. Make sure that the card root has the `SCRIPT` folder. Copy all the contents of `SD_ROOT`, including `SCRIPT`.
5. If the card has `SCRIPT/PLUGINS/INSTALL.ERR`, GRKit did not install. Make sure that the package is for the firmware version of your camera. To see the firmware version, open the camera menu. If the version is correct, send a [device report](https://github.com/ahonn/grkit/issues/new?template=device-report.yml) with all the lines of `INSTALL.ERR`.
6. GRKit runs only on the official Ricoh firmware. It does not run on modified firmware.

## Remove or recover

To remove GRKit, turn the camera off. Then delete the `SCRIPT` and `GRKIT` folders from the card. `GRKIT` holds your GRKit settings.

The SD script setting stays on. To turn it off:

1. On https://grkit.app, download the files that turn SD scripts off for your camera. The button is below the installation steps.
2. Copy the contents of `SD_ROOT` to the root of the SD card.
3. Insert the card. Hold MENU while you turn the camera on. Wait until the camera turns itself off.
4. Remove `00078350.588` and `DEVELOP.MOD` from the card root.

If the camera stops during startup, remove the battery and the card. Then start the camera without the card. If it still does not start, see the [recovery guide](https://github.com/ahonn/grkit/blob/main/resources/RECOVERY.md).

GRKit is not affiliated with Ricoh. Source code and licenses: https://github.com/ahonn/grkit
