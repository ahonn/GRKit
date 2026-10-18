# GRKit

For RICOH GR IV and GR IV HDF firmware 1.11 only.

GRKit for GR IV is experimental. It has not been tested on a camera. Use it only if you accept that risk, and keep the recovery steps below at hand.

## Install

1. Turn the camera off.
2. Back up the SD card, including any existing startup scripts.
3. Copy the contents of `SD_ROOT` to the card root.
4. Safely eject the card.
5. Insert the card into the camera.

If SD scripts are not enabled:

1. Hold MENU while turning the camera on.
2. Wait for the camera to turn itself off.

With the camera off, remove `00078560.636` and `DEVELOP.MOD` from the card root.
Keep the `SCRIPT` directory. Use normal power-on from now on.

## Use

GRKit loads a few seconds after power-on. Open ADJ and select the sixth entry, marked GRK. If the entry is absent, wait and open ADJ again.

- **Image Control:** highlight an effect or Off with the direction keys or dial. The viewfinder previews it. Press OK to keep it. MENU or leaving without OK restores the previous effect.
- **Fn on an effect:** keep the effect and open the native Standard parameters, which plugin effects share. Press OK there to return. Fn requires native Image Control Standard.
- **Framing:** set the native aspect ratio to 16:9 first. Then highlight Off or XPan to preview its mask, and press OK. Photos record at 65:24. The viewfinder dims the area outside the photo.
- **Guides:** highlight Off or a guide style to preview it, then press OK. The lines stay inside the Framing area. They do not appear in photos.
- **Date Stamp:** select Off or a date style, then press OK. Photos get the date at the bottom right in glowing orange digits, like a film data back. The digits fade on bright areas. The viewfinder does not show them.
- **MENU:** return one level. From GRKit, return to native ADJ with the selected effect.

The GRKit page shows only the modules in this package. If it has only one module, ADJ opens that module directly, and MENU returns to native ADJ.
Image Control shows when `SCRIPT/PLUGINS/EFFECTS` contains effects. `SCRIPT/PLUGINS/MODULES.TXT` lists the other modules to hide: `framing`, `guides`, or `date-stamp`, one on each line.
If that file names an unknown module, GRKit does not load, and `SCRIPT/PLUGINS/INSTALL.ERR` contains 43.

GRKit requires the official firmware. If the camera runs a modified firmware, GRKit does not load, and `INSTALL.ERR` contains 45. Install the official firmware 1.11 again to use GRKit.

GRKit keeps the selected effect, guide style, and date style in `SCRIPT/PLUGINS/STATE.TXT` and restores them a few seconds after power-on.
Delete that file to start from Off. A hidden module starts from Off. Pressing OK on a native Image Control disables the plugin effect.
Playback information shows the plugin effect icon while that effect is installed.

The effects were made for the GR III. GR IV uses a different Standard color matrix, so the colors of an effect can differ from the GR III.

GR IV offers 4:3 and 16:9 natively, so Framing has only XPan. XPan requires the native 16:9 aspect ratio and still shooting. With native 3:2, XPan has no effect, and photos keep 3:2.
Guides require native 3:2, or native 16:9 with XPan. With another native aspect ratio, the viewfinder shows no guides.
In-camera Crop, Resize, Digital Filter, and RAW development of XPan photos are not tested.
GRKit does not change the native grid guide. Turn it off to avoid two sets of lines.

## Remove or recover

With the camera off, remove `SCRIPT/PLUGINS` and GRKit's `SCRIPT/STARTUP.TTL` from the card.
Remove either activation file if it remains. This does not disable the camera's SD-script setting.

If normal startup freezes, try normal power-off first. If the camera remains unresponsive, remove the battery.
Remove the card, reinsert the battery, and restart without the card.
Do not interrupt an official firmware update. If startup still fails, see the [recovery guide](https://github.com/ahonn/GRKit/blob/main/resources/RECOVERY.md).

GRKit is not affiliated with Ricoh. Source code and license: https://github.com/ahonn/GRKit.
