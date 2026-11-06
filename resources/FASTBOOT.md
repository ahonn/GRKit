# Fast start

On the official firmware, GRKit loads 8 to 10 seconds after power-on, because it waits until the camera has read the SD card. The fast-boot firmware lets GRKit load soon after power-on. It is optional. GRKit works the same without it, and the same GRKit package works on both firmwares.

The fast-boot firmware is the official Ricoh firmware of your camera with three small changes, which let SD scripts start earlier. The page makes it from the official file on your device. Nothing is uploaded.

## Before you start

> [!WARNING]
> To install the fast-boot firmware, you update the camera with a firmware file that Ricoh did not make. If the update stops, the camera may not start. GR III and GR IIIx keep only one copy of the firmware.

- The page makes the fast-boot firmware of the latest version of each camera: GR III 2.20, GR IIIx 1.70, and GR IV 1.20, also for their HDF models. A camera on an older version installs it as an update. GR IV Monochrome and the factory firmware of GR IV 30th Anniversary Edition cameras are not supported.
- Ricoh may decline warranty or repair service for a camera that used modified firmware.
- Read the [Terms of Use](TERMS.md).

## Make the fast-boot firmware

1. On the Ricoh website, download the latest firmware of your camera. It is a ZIP file.
2. Open the [Fast start page](https://grkit.app/fastboot/) of grkit.app. Select the ZIP file.
3. The page downloads the fast-boot firmware. The file has the same name as the official firmware file.

If the page says that the file is not an official firmware file, download it from Ricoh again. The page accepts only the official file of a supported camera and version.

## Install the fast-boot firmware

1. Charge the battery fully.
2. Copy your photos from the SD card. Copy the fast-boot firmware file to the root of the card. Remove other `.bin` files from the card root.
3. Turn the camera off. Hold MENU and press the power button. Follow the instructions on the screen.
4. Do not turn the camera off or open the battery cover until the update is complete.
5. Turn the camera on without GRKit. Make sure that it takes photos and plays them back.
6. Delete the firmware file from the card. Then install the latest GRKit as usual.

Expected result: after power-on, GRK shows in ADJ much sooner than before.

## Go back to the official firmware

Install the official firmware file of the same version, in the same way as the fast-boot firmware. A later official update also replaces the fast-boot firmware.
