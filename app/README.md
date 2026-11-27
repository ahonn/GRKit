# GRKit app

Phone app for connecting to a RICOH GR camera and managing GRKit. The iOS link
discovers cameras over BLE, joins Wi-Fi, and reads GRKit status. Updates use
official signed packages. See
[camera connection](docs/connection.md) and [updates](docs/updates.md) for protocol,
cancellation, recovery, and hardware verification.

## Requirements

- Rust 1.99.1 (see `rust-toolchain.toml`)
- NSH CLI 0.6.0 (`nsh --version`)
- Xcode (for iOS simulator packaging)

## Commands

```sh
# Unit + headless UI tests
cargo test

# Format and lint
cargo fmt --check
cargo clippy --all-targets -- -D warnings

# Desktop (macOS), demo mode
GRKIT_DEMO=1 cargo run
# or
cargo run -- --demo
# a release build has demo mode only with the feature
cargo run --release --features demo -- --demo

# iOS simulator
GRKIT_DEMO=1 nsh run --target ios
nsh package --target ios-simulator
```

### Demo failures

Force an error path in demo mode:

```sh
GRKIT_DEMO=1 GRKIT_DEMO_FAIL=card_full cargo run
# also: refused | connection_lost
```

### Demo screens (screenshots)

With demo on, jump straight to a design screen (sheet / alert already open).
The ids and what each shows are the screen tables in `src/dev/screens/`, one per tab.

```sh
GRKIT_DEMO=1 GRKIT_DEMO_SCREEN=A5 cargo run
GRKIT_DEMO=1 cargo run -- --demo-screen B3
# or alone (also turns demo on):
cargo run -- --demo-screen C4
```

| Ids | Screens |
|-----|---------|
| A1–A14 | Connect, camera tab, Settings, saved cameras |
| B1–B5, C4, U1–U19 | Software Update and the update sheet |
| H1–H3 | Camera tab while it reads the camera |
| L1–L15, L17–L28 | Film Looks |
| R1–R13 | Recipes |
| M1–M16 | Modules |
| S1–S4, S8–S9 | GRKit Settings, Look unlock, Shutdown Screen picture |

Unknown ids fall back to the normal demo home.

An id names the same screen here and in the Figma file (page Screens). A
new screen takes the next free number; an id is never reused for another
screen.

### Development camera URL (no Bluetooth)

In a debug build, a build with `--features demo`, or a build started with the
variable set (as below), when demo mode is off and
`GRKIT_CAMERA_URL` is set, Add Camera skips Bluetooth and
Wi-Fi and talks HTTP to that base URL (status poll with 503 retry, then package).
Useful on the iOS simulator, which has no Bluetooth:

```sh
# Run a local GRKit HTTP service, then:
GRKIT_CAMERA_URL=http://127.0.0.1:8080 nsh run --target ios-simulator
```

Without the variable, behavior is unchanged. On a build where Bluetooth is
unavailable (simulator Unsupported), the Camera tab still shows A1; Add Camera
shows a Bluetooth unavailable message instead of an endless scan.

### Device build (signing)

`GRKit.xcodeproj` wraps `nsh package --target ios` so Xcode can sign for a device:

```sh
cd app
xcodebuild -project GRKit.xcodeproj -scheme GRKit \
  -destination 'generic/platform=iOS' -allowProvisioningUpdates build
```

The shell script `scripts/xcode-build.sh` is the only build phase. Entitlements
live only in `ios/GRKit.entitlements` (HotspotConfiguration), which Xcode signs
into device builds. `nsh.json` has no `ios.entitlements`: `nsh package` signs
them into the ad hoc simulator build, and the simulator then refuses to launch
the app.

## Layout

- `src/domain/` — camera model, update state machine, request and answer types
- `src/camera_api/` — `CameraApi`, what the store reads and writes: the camera service, the release site, the phone's records
- `src/store/` — tabs, sheets, the camera session and the update flow
- `src/dev/` — demo mode (debug builds and `--features demo`): `FakeCamera`, demo screens
- `src/shell/` — tab bar and nav stacks
- `src/pages/` — Camera, Settings, Software Update, update sheets
- `src/ui/` — shared chrome
- `locales/app.yml` — all UI text in the five languages; `android.<key>` replaces `<key>` on Android
- `assets/` — camera line art, confirm screenshots, GR Snap icon
