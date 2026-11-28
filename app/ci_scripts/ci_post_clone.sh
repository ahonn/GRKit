#!/bin/bash
# Xcode Cloud runs this script after it clones the repository. The images have
# no Rust and no nsh, which scripts/xcode-build.sh needs to build the app.
set -euo pipefail

if [[ "${CI_XCODE_CLOUD:-}" != "TRUE" ]]; then
  echo "Run this script only in an Xcode Cloud checkout." >&2
  exit 1
fi
if [[ ! "${CI_BUILD_NUMBER:-}" =~ ^[1-9][0-9]*$ ]]; then
  echo "Xcode Cloud must provide a positive CI_BUILD_NUMBER." >&2
  exit 1
fi

APP_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$APP_DIR"

# Keep it at the version of the nsh crates in Cargo.toml, as App Build does.
NSH_VERSION=v0.6.0

# rust-toolchain.toml pins the channel and the iOS targets. scripts/xcode-build.sh
# finds rustup and nsh in ~/.cargo/bin and ~/.local/bin.
if [[ ! -x "$HOME/.cargo/bin/rustup" ]]; then
  curl --proto '=https' --tlsv1.2 --fail --silent --show-error --retry 3 https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain none --no-modify-path
fi
"$HOME/.cargo/bin/rustup" toolchain install

curl --fail --location --silent --show-error --retry 3 https://nsh.dev/install.sh | NSH_VERSION="$NSH_VERSION" sh
"$HOME/.local/bin/nsh" --version

# scripts/xcode-build.sh gives CURRENT_PROJECT_VERSION to `nsh package` as the
# build number, so each Xcode Cloud build uploads a new one.
sed -i '' -E "s/CURRENT_PROJECT_VERSION = [0-9]+;/CURRENT_PROJECT_VERSION = $CI_BUILD_NUMBER;/" \
  GRKit.xcodeproj/project.pbxproj
grep -q "CURRENT_PROJECT_VERSION = $CI_BUILD_NUMBER;" GRKit.xcodeproj/project.pbxproj
