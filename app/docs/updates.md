# Updates

The real app reads `GET /v1/update` for the camera journal.
It checks `https://grkit.app/release/index.json`, authenticates the official signed
statement, and offers the release only for a supported camera and firmware.
An unavailable index is an error, not proof that the camera is up to date.

## Preparing a package

The app downloads the indexed files, verifies each SHA-256 and size, and uses
`grkit_core::packaging` to assemble the camera's selected modules, effects and
`left_out` entries. It verifies the final ZIP with `OFFICIAL_KEYS` before upload.
Missing custom or withdrawn files stop preparation: omitting them would cause
the service to delete them. The current camera API reports names without hashes,
so it cannot distinguish a custom effect that reuses an official filename.

Validated files are cached by digest in the app's `release-cache-v1` directory,
limited to 128 MB. Offline retries need all required files cached beforehand.
The per-origin, per-camera cached index is a fallback; its signature is checked
again. Download progress is not upload progress: NSH currently supplies a whole
response, so preparation and upload show an indeterminate state.

## Mutation and recovery

- Update sends one raw ZIP with `PUT /v1/update`. The mutation is not
  automatically replayed after a failure.
- Before PUT, the app rechecks the package and journal. It saves camera
  identity, kind, baseline and target before mutating the camera.
- `waiting` shows camera confirmation steps. `confirmed` asks the user to restart
  the camera. A lost HTTP link during restart remains recoverable; Check Installed
  Version reconnects via the normal camera link.
- Only journal `none` plus an exact installed target version means success.
  Journal disappearance at the baseline version reports an unconfirmed operation.
- Reopening the app and reconnecting resumes the saved intent for that camera.
  Recovery records are kept on uncertain failures. Unknown pending journals from
  another client require completion or cancellation on the camera first.
- Preparation can be cancelled immediately. Once PUT starts, cancellation waits
  for it to settle, sends `DELETE /v1/update`, and reads the journal back. It does
  not promise that a package is unchanged when the connection is unavailable.

## Simulator verification

`tests/update_http.rs` uses the real app and a controlled HTTP transport to test
an update whose answer was lost, confirmed version verification, and
cancellation.
`tests/demo_ux.rs` covers the visual install sequence and error states.

The production release index returned 404 during this implementation's QA.
A complete official-download/upload test therefore still needs a published index
and signed release. Real-camera checks must cover SD space, custom effects,
confirmation expiry, power loss, Wi-Fi reassociation and interrupted uploads.
