# GRKit app

Phone app for connecting to a RICOH GR camera and managing GRKit. The iOS link
discovers cameras over BLE, joins Wi-Fi, and reads GRKit status. Updates use
official signed packages. See
[camera connection](docs/connection.md) and [updates](docs/updates.md) for protocol,
cancellation, recovery, and hardware verification.

## Requirements

- Rust 1.99.1 (see `rust-toolchain.toml`)
- NSH CLI 0.5.1 (`nsh --version`)
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

# iOS simulator
GRKIT_DEMO=1 nsh run --target ios
nsh package --target ios-simulator
```

### Demo failures

Force an error path in demo mode:

```sh
GRKIT_DEMO=1 GRKIT_DEMO_FAIL=card_full cargo run
# also: refused | not_confirmed | connection_lost
```

### Demo screens (screenshots)

With demo on, jump straight to a design screen (sheet / alert already open).
Mapping lives in `src/store/demo_screen.rs`.

```sh
GRKIT_DEMO=1 GRKIT_DEMO_SCREEN=A5 cargo run
GRKIT_DEMO=1 cargo run -- --demo-screen B3
# or alone (also turns demo on):
cargo run -- --demo-screen C4
```

| Id | Screen |
|----|--------|
| A1 | First launch |
| A2 | Camera nearby sheet |
| A3 | Add a camera sheet |
| A4 | Connecting sheet |
| A5 | Update available |
| A6 | Up to date |
| A7 | Settings |
| B1 | Software Update |
| B2 | Sending (~62%) |
| B3 | Confirm on camera |
| B4 | Restart |
| B5 | Installed |
| C4 | Card full |
| C5 | Not confirmed |

Unknown ids fall back to the normal demo home.

### Development camera URL (no Bluetooth)

When demo mode is off and `GRKIT_CAMERA_URL` is set, Add Camera skips Bluetooth and
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

- `src/domain/` — camera model, update state machine, `CameraLink`, `FakeCamera`
- `src/store/` — tabs, sheets, demo wiring (`demo_screen.rs` for screenshot ids)
- `src/shell/` — tab bar and nav stacks
- `src/pages/` — Camera, Settings, Software Update, update sheets
- `src/ui/` — shared chrome
- `locales/app.yml` — English and Simplified Chinese (kept with `src/strings.rs`)
- `assets/` — camera line art, confirm screenshots, GR Snap icon
