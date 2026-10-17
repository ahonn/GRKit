#!/bin/sh
# Builds grkit-wasm and writes its browser bindings to web/src/wasm/.
# wasm-bindgen must match the exact wasm-bindgen version that Cargo.toml pins.
set -eu
cd "$(dirname "$0")/../.."
expected="wasm-bindgen 0.2.128"
bindgen="${WASM_BINDGEN:-wasm-bindgen}"
found="$("$bindgen" --version)"
if [ "$found" != "$expected" ]; then
  echo "Expected $expected, found $found" >&2
  exit 1
fi
cargo build -p grkit-wasm --target wasm32-unknown-unknown --release --locked
rm -rf web/src/wasm
"$bindgen" --target web --out-dir web/src/wasm \
  target/wasm32-unknown-unknown/release/grkit_wasm.wasm
