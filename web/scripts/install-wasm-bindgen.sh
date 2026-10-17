#!/bin/sh
# Installs the prebuilt wasm-bindgen CLI on x86_64 Linux in CI and adds it to PATH.
# The version must match the exact wasm-bindgen version that Cargo.toml pins.
set -eu
version=0.2.128
sha256=b51f0208fdff83515a787bd8ab9ac5865ed84dabb66d0c709957bb59793c645f
name="wasm-bindgen-$version-x86_64-unknown-linux-musl"
directory="${RUNNER_TEMP:?}/wasm-bindgen"
mkdir -p "$directory"
curl --fail --location --silent --show-error --retry 3 -o "$directory/$name.tar.gz" \
  "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$version/$name.tar.gz"
echo "$sha256  $directory/$name.tar.gz" | sha256sum --check --strict
tar -xzf "$directory/$name.tar.gz" -C "$directory"
echo "$directory/$name" >> "${GITHUB_PATH:?}"
