## 1.3.4 (2026-10-30)

### Fixes

- ADJ opens as usual again after the Fn adjustments page was left for playback or MENU.
- In MENU, a module settings page shows its whole title, and ADJ Order lists only the modules that ADJ shows.
- On GR IV, a photo in an aspect ratio set in the camera menu keeps its full size, where a saved Framing ratio of 5:4, 1:1, or 7:6 cropped it.
- The camera no longer stops when a MENU item with its own list, such as ISO Setting, was opened before GRKit loaded and is opened again after it loads.

## 1.3.3 (2026-10-29)

### Fixes

- The Fn adjustments of a Film Look effect stay on screen after a half-press, where they showed and changed the native Image Control settings.
- GRKit loads again on every start with a new card, after an update, and with cards that macOS copied files to, where 1.3.1 and 1.3.2 could miss GRK, Film Look, or effects.

## 1.3.2 (2026-10-29)

### Fixes

- The viewfinder shows DATE while Date Imprint is on.
- Framing has a 7:6 ratio, the 6x7 frame of medium-format film.
- Grain has an Extra Strong level, half again as strong as Strong.
- The histogram reaches its right edge and shows the clipping line again while a Film Look effect is on.
- GRKit is in Korean on a camera set to 한국어, and on grkit.app and in its guides, and a module built for an earlier GRKit must be built again with a Korean text file.
- GRKit loads again on the first starts with a new card or after an update, where 1.3.1 could stop with no GRK in ADJ.

## 1.3.1 (2026-10-28)

### Fixes

- GRKit now loads in about 40% less time after power-on on GR III and GR IIIx, and in about half the time on GR IV.
- On GR III and GR IIIx, medium and large grain no longer make the camera slow to save a photo, open playback, or turn off.
- GRKit holds up to 10 modules, and ADJ Order lists all of them in one MENU row.

## 1.3.0 (2026-10-27)

### Features

- Updates keep your settings and effects: delete the old SCRIPT folder and copy the new one, or update on grkit.app, which also keeps those of GRKit 1.2.
- Date Imprint has a Position setting: Upright keeps the date at the bottom right of vertical photos.
- Image Control in GRKit is now called Film Look.
- Framing keeps a 3:2 viewfinder for every ratio, and can show a black mask or frame lines and hide its label.
- Grain has a Size setting: Auto (by ISO), Small, Medium, or Large.
- Guides can be white or yellow, opaque, and shown while DISP hides the camera information.
- All GRKit settings are now in MENU, at the end of the Customize tab, where you turn features and their options on or off at once.
- In Chrome or Edge on a computer, grkit.app installs, updates, and uninstalls GRKit directly on the SD card.

### Fixes

- The highlight warning shows the brightest areas again while a Film Look effect is on.

## 1.2.1 (2026-10-24)

### Fixes

- Packages add the Ektachrome E100, Eterna 500T, Instax, Double-X 5222, and HP5 Plus 400 effects, and no longer include Portra 160, EXR 200T, and FP-100C.
- Framing offers XPan, 2.35:1, 16:9, 4:3, 5:4, and 1:1 on every camera, and uses the camera's own aspect ratio where it has one.
- GR IV Monochrome firmware 1.10 is supported as experimental, with the GR IV package.
- One GR IV package installs on firmware 1.11 and 1.20, and the 1.20 firmware that GR IV 30th Anniversary Edition cameras have from the factory is supported as experimental.
- The live view builds the curves of an effect only when they change, Framing does less work to draw the area outside the photo, and GRKit saves menu selections to the card only after they stop changing, so the camera uses less power.
- The shooting screen shows GRKit for a few seconds when GRKit has loaded.
- Menu icons, effect icons, and the Framing label use Roboto Condensed, which looks like the camera's own text.
- Vision3 500T now looks like the film printed on photo paper and scanned, as CineStill 800T is, and the motion picture print version is removed (Eterna 500T gives that look; delete `SCRIPT/PLUGINS/EFFECTS/1A3DC043.GRF` from the card if the menu shows Vision3 500T twice).

## 1.2.0 (2026-10-22)

### Features

- OK on Date Imprint in the GRK menu opens its settings, where Date Look selects the digits of the date imprint: the classic seven-segment digits, the dot matrix of the GR1s, or the digits of the GR II.
- Color film looks and imported LUTs follow their source more closely with the camera's hue-sector color stage, in photos and in the viewfinder.
- Fn on the Image Control row of the GRK menu opens the adjustments of the selected effect.
- GR III and GR III HDF firmware 2.20 and GR IIIx and GR IIIx HDF firmware 1.70 are supported as experimental, with the same package as 2.10 and 1.60.
- GR IV and GR IV HDF firmware 1.20 are supported as experimental, with a package of their own.

### Fixes

- Guides draw 2-pixel lines in the color of the native grid guide, at the positions of the native thirds.
- GRKit starts 2 seconds later after power-on, which avoids a stop of the camera when the script talks to the camera before it is ready.
- Playback shows the Fn adjustments of the effect that took the photo, not the parameters of the camera's Image Control setting.
- The date stamp is white on monochrome pictures, including the camera's monochrome Image Control types and GR IV Monochrome.

## 1.1.1 (2026-10-21)

### Fixes

- On another build of firmware 1.11, GR IV and GR IV HDF no longer stop when GRKit loads.
- The GRK icon in the ADJ toolbar has the size of the native toolbar icons.
- Guides hide while DISP hides the live-view information, and the Framing ratio label shows only with the full display.
- The GRKit menu icons have the size of the native list icons, and their text keeps clear of the frame.

## 1.1.0 (2026-10-20)

### Features

- Left and Right on a GRKit page move to the next ADJ entry, as they do on the GRKit root.
- OK or an ADJ press on a GRKit page selects the highlighted row and closes ADJ.
- GRKit opens on the row that was last used, and ADJ opens on GRKit if ADJ closed while GRKit was open.
- GRKit in ADJ shows all of its rows and scrolls a long page one row at a time, as a native ADJ entry does.
- Add a Date Stamp style with the day first and the time: `D M 'YY  H:MM`.
- Add a Guides style that shows the thirds grid and the center cross together.

### Fixes

- The About row of the GRKit menu shows a VER icon, so the GRKit root shows only one GRK icon.
- An ADJ press or one MENU press on the GRKit root closes ADJ.
- GR IV and GR IV HDF install effects from an exFAT card with long file names, such as the `._` files that macOS writes.
- On another build of firmware 1.11, GR IV and GR IV HDF install GRKit, or stop with status 47 and list missing addresses in `INSTALL.ERR`.
- The camera runtime has room for more plugins, and saved selections, Fn adjustments, and effects keep working after the update.

## 1.0.1 (2026-10-19)

### Fixes

- Show the film numbers 5219, 5207, and 5293 on the icons of Vision3 500T, Vision3 250D, and EXR 200T, in the camera menu, the viewfinder, and playback.
- Remove the experimental label from GR IV and GR IV HDF firmware 1.11, after a user tested GRKit on a GR IV.
- Support RICOH GR III HDF and GR IIIx HDF. They run the GR III 2.10 and GR IIIx 1.60 firmware and use the same packages, and the site shows each with its base camera.
- Draw the film icons on the site with web fonts, so they stay sharp. The camera keeps its 40 x 40 pixel icons.
- Show the film number under the name of each motion picture and aerial film look on the site, for example "5219 · 2393 print" under Vision3 500T.

## 1.0.0 (2026-10-19)

### Features

- Add GRK, a sixth entry of ADJ mode, with five modules: Image Control, Framing, Guides, Date Stamp, and Grain.
- Add 16 film looks to Image Control, with film cover icons and a live-view preview.
- Adjust each film look with Fn. The adjustments stay with that look and do not change the native Image Control settings.
- Show the GRKit version in the last row of GRK.
- Support RICOH GR III firmware 2.10 and GR IIIx firmware 1.60. GR IV and GR IV HDF firmware 1.11 support is experimental.
- Add the GRKit site, which builds a package in the browser from the selected camera, modules, and effects.
- Preview the effects on a GR IIIx sample or on your own DNG. The site develops the RAW data in the browser.
- Show the camera menu, the site, and the guides in English, Simplified Chinese, Traditional Chinese, and Japanese.
