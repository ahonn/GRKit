# Recovery

GRKit installs session patches in RAM. The activation procedure separately enables the camera's persistent SD-script setting.
GRKit does not provide an automatic rollback after a camera hang.

## Camera hangs during normal startup

Do not interrupt an active official firmware update or remove power while an update progress screen is visible.

If the camera has frozen during normal startup:

1. Attempt normal power-off.
2. If the camera remains unresponsive, remove the battery.
3. Remove the SD card.
4. Reinsert a charged battery.
5. Try startup without a card.

Do not push or pull the lens barrel. Keep the card out during recovery.

## Normal startup still fails

1. Hold the playback button to start in playback mode without the card.
2. If settings remain accessible, use the camera's reset function.
3. Restart without the card before trying any SD scripts again.

Resetting the camera removes personal settings. Record those settings first if the camera permits it.
An earlier implementation that stored a custom native Image Control value required this reset after a startup hang.
Avoid opening Image Control if that screen itself causes a hang. Use the general reset setting instead.

If playback startup or reset is unavailable, stop trying script installations and contact camera service.
Reinstalling official firmware does not necessarily clear a stored setting that causes startup failure.
