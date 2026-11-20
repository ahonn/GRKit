#!/bin/bash
# The only build phase of the GRKit Xcode iOS target. `nsh package` builds the
# app; this script gives Xcode its executable, resources and Info.plist, and
# Xcode signs the bundle.
set -euo pipefail

cd "${SRCROOT:?Run this script from a GRKit Xcode target}"
if [[ "$ARCHS" != "arm64" ]]; then
  echo "error: NSH builds GRKit for arm64 only; ARCHS is '$ARCHS'." >&2
  exit 1
fi

# The PATH of a plain rustup / nsh install, without Xcode's other build settings.
path="$HOME/.local/bin:/etc/profiles/per-user/$USER/bin:$HOME/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin"

case "$PLATFORM_NAME" in
  iphoneos)
    # No SDKROOT: build scripts and proc macros cargo builds for the Mac would
    # link against the iOS SDK. Give the macOS SDK to Mac-target C compiles only.
    target=ios
    mac_sdk="$(xcrun --sdk macosx --show-sdk-path)"
    platform_env=(
      IPHONEOS_DEPLOYMENT_TARGET="$IPHONEOS_DEPLOYMENT_TARGET"
      CFLAGS_aarch64_apple_darwin="-isysroot $mac_sdk"
    )
    ;;
  *)
    echo "error: unsupported platform '$PLATFORM_NAME' (iOS device only)." >&2
    exit 1
    ;;
esac

out="$TARGET_TEMP_DIR/nsh-package"
rm -rf "$out"
env -i HOME="$HOME" USER="${USER:-}" TMPDIR="${TMPDIR:-/tmp}" LANG=en_US.UTF-8 PATH="$path" \
  DEVELOPER_DIR="$DEVELOPER_DIR" "${platform_env[@]}" \
  ${CARGO_HOME:+CARGO_HOME="$CARGO_HOME"} ${RUSTUP_HOME:+RUSTUP_HOME="$RUSTUP_HOME"} \
  ${RUSTFLAGS:+RUSTFLAGS="$RUSTFLAGS"} \
  nsh package --target "$target" --out "$out" --build "$CURRENT_PROJECT_VERSION"

app="$out/$PRODUCT_NAME.app"
info="$DERIVED_FILE_DIR/$PRODUCT_NAME-Info.plist"
mkdir -p "$DERIVED_FILE_DIR"

# An iOS bundle is flat: the executable, Assets.car, and nsh/. Xcode copies
# the provisioning profile into the bundle before this phase runs, so the sync
# must keep it.
mkdir -p "$TARGET_BUILD_DIR/$WRAPPER_NAME"
rsync -a --delete --exclude Info.plist --exclude _CodeSignature --exclude embedded.mobileprovision \
  "$app/" "$TARGET_BUILD_DIR/$WRAPPER_NAME/"
cp "$app/Info.plist" "$info"

buddy() { /usr/libexec/PlistBuddy -c "$1" "$info"; }
identifier="$(buddy 'Print :CFBundleIdentifier')"
if [[ "$identifier" != "$PRODUCT_BUNDLE_IDENTIFIER" ]]; then
  echo "error: nsh.json id '$identifier' is not the target's '$PRODUCT_BUNDLE_IDENTIFIER'." >&2
  exit 1
fi
# HTTPS through rustls only: exempt encryption, as App Store Connect asks.
buddy 'Add :ITSAppUsesNonExemptEncryption bool false' 2>/dev/null \
  || buddy 'Set :ITSAppUsesNonExemptEncryption bool false'
# 1.0 is an iPhone app (iPads run it in compatibility mode).
buddy 'Delete :UIDeviceFamily' 2>/dev/null || true
buddy 'Add :UIDeviceFamily array'
buddy 'Add :UIDeviceFamily:0 integer 1'

# The app icon: the Icon Composer bundle that resources/brand/build.py writes.
# nsh.json names no icon, so the package has no asset catalog of its own and
# actool writes the bundle's Assets.car.
partial="$DERIVED_FILE_DIR/AppIcon-Info.plist"
xcrun actool "$SRCROOT/../resources/brand/app/ios/AppIcon.icon" \
  --compile "$TARGET_BUILD_DIR/$WRAPPER_NAME" --platform iphoneos \
  --minimum-deployment-target "$IPHONEOS_DEPLOYMENT_TARGET" --target-device iphone \
  --app-icon AppIcon --output-partial-info-plist "$partial" --output-format human-readable-text >/dev/null
buddy "Merge $partial"
