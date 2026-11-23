# Camera connection

The connection path is BLE discovery → encrypted Wi-Fi credentials → camera wake
and AP start → AP warm-up → Wi-Fi join → readiness of the GRKit service. The
shared flow is `run_connect` (`src/domain/connect_flow.rs`); its budgets are in
`src/domain/connect_policy.rs`. iOS and Android supply the operations through
`ConnectTransport`. Demo mode uses `FakeCamera`; updates use the
[camera update protocol](updates.md).

## Ownership and cancellation

`AppState` permits one connection task at a time. Its generation fences UI writes;
an abort handle drops the operation when the user cancels. A new task cannot start
until the previous task has unwound. Native BLE cleanup and scan resumption live
in drop guards, including failures after BLE setup.

One attempt is bounded by 90 s of foreground time; the system Wi-Fi dialog does
not count. When the time runs out during the join, the error is a Wi-Fi timeout;
otherwise it is a Bluetooth timeout. A user cancel after BLE connected closes the
camera access point (`NETWORK_TYPE` 0x00); a timeout leaves it on.

ASK joins have separate request ids. A late completion can only settle its own
request. Cancelling a system Wi-Fi request does not cancel the underlying iOS
association: NetworkExtension exposes no cancellation API. An outstanding ASK
picker blocks another picker until it closes, and the picker has a 120 s limit.

## Bluetooth

- Scanning runs only while the app is in the foreground. Every advertisement
  frame goes to `NearbyWatch` (`src/domain/nearby_watch.rs`). A camera is on
  only when the power byte and the Camera Information service agree; a standby
  frame removes it at once; a frame that says neither is ignored. Sightings
  expire after 6 s without a frame.
- Saved, powered-on cameras have prompt priority. An unknown camera must be the
  only eligible nearby candidate. Proximity enters at -60 dBm and leaves below
  -70 dBm; unknown RSSI cannot establish proximity.
- A camera that was prompted for, connected to or lost is not prompted for again
  during its current power-on. A camera unseen for 10 minutes starts a new one.
- One connect call makes up to 3 attempts 2 s apart; a timeout is retried once.
  Pending connections are cancelled before a retry.
- The flow retries Bluetooth setup 3 s apart: up to 6 times for incomplete
  authentication or a failed connection (the user may still be answering the
  pairing prompt), once after the camera drops the link before the credentials,
  and at most 6 times in total. Then it asks the user to pair on the camera. A
  pairing the camera has removed is not retried.
- The wake write is skipped only when a fresh advertisement says the camera is
  on. AP start failures do not stop the flow once the credentials are read.

## Wi-Fi and readiness

- The AP warm-up is learned per camera: a soft-failed join raises it, a fast clean
  join that alone reached the camera lowers it. A reconnect within 180 s waits
  500 ms. A missed AP-ready notification adds 1.5 s.
- Each join waits for the foreground. A join error that HTTP can still disprove
  is soft: the flow probes the service before it reports the error.
- Readiness polls `GET /v1/props` on port 8080 every 600 ms, 1.5 s per probe,
  within 15 s of foreground time. A soft-failed join off the camera subnet gets
  a 3 s probe and then one more join. A native join without a camera route is
  forced to reassociate (the saved configuration is removed first), except on
  GR IV when every probe was refused. At most 2 joins per attempt.
- The phone's Wi-Fi address decides the route: a camera-AP lease, a settled
  lease on another network, or still transitioning. Repeated non-camera HTTP
  answers at 192.168.0.1 from inside that /24 are a subnet conflict.
- A failure that may be a lost race gets a silent verify pass (3 tries, 3 s
  apart, 5 s each) unless the phone is on another network or already
  reassociated. A camera that never answered is checked for denied local network
  access before the error shows.
- A camera API that answers on port 80 while the GRKit service does not means
  the camera needs GRKit 1.7.0 or later. A mounting card is reported separately.

Android does not bind the whole process to the camera network: the app downloads
releases and the Look catalog from the internet while it is connected, and each
camera request already uses the camera network.

## Established link

HTTP decides whether the camera is still there. Every 5 s in the foreground the
app reads `/v1/props` (10 s timeout) and checks that the same camera answers; 4
failures in a row, then a failed recovery probe, mark it disconnected. A
different camera, a failed probe right after the app returns to the foreground
(2 tries of 2.5 s), or a power-off advertisement from the camera mark it
disconnected at once. A lost camera never opens a system Wi-Fi dialog: the app
reconnects by itself only when the phone is already on the camera network.

## Hardware checks

Before release, verify on GR III / IIIx and GR IV hardware: first pairing, declined
ASK consent and declined local network access, saved authorization, AP startup
with BLE loss, rapid cancel/retry, background/foreground during a join, camera
power-off/on, a home network on 192.168.0.0/24, and missing or mounting SD card.
Simulator and headless tests do not validate radio or system-dialog timing.
