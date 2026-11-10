#!/bin/sh
# Runs the checks of the Tests workflow on this computer.
#
# Without options it runs the Rust and the web checks. It adds the ARM builds
# when GRKIT_CLANG and GRKIT_LLD name the release toolchain, and the firmware
# tests when GRKIT_TEST_FIRMWARE, GRKIT_TEST_FIRMWARE_GR3_221,
# GRKIT_TEST_FIRMWARE_GR4, GRKIT_TEST_FIRMWARE_GR4_120,
# GRKIT_TEST_FIRMWARE_GR4_120_0528, and GRKIT_TEST_FIRMWARE_GR4_121 name the
# firmware files (CONTRIBUTING.md).
set -eu
cd "$(dirname "$0")/.."

step() { printf '\n== %s\n' "$*"; }

step Rust format
cargo fmt --all -- --check
step Clippy
cargo clippy --workspace --all-targets --locked -- -D warnings
step Rust and host C tests
GRKIT_REQUIRE_NATIVE_TESTS=1 cargo test --workspace --locked
step WebAssembly build
cargo build -p grkit-wasm --target wasm32-unknown-unknown --release --locked

if [ -n "${GRKIT_CLANG:-}" ] && [ -n "${GRKIT_LLD:-}" ]; then
    step ARM builds
    cargo test -p grkit-build --locked -- --ignored synthetic_ dump_
    if [ -n "${GRKIT_TEST_FIRMWARE:-}" ] && [ -n "${GRKIT_TEST_FIRMWARE_GR3_221:-}" ] &&
        [ -n "${GRKIT_TEST_FIRMWARE_GR4:-}" ] &&
        [ -n "${GRKIT_TEST_FIRMWARE_GR4_120:-}" ] && [ -n "${GRKIT_TEST_FIRMWARE_GR4_120_0528:-}" ] &&
        [ -n "${GRKIT_TEST_FIRMWARE_GR4_121:-}" ]; then
        step Firmware tests
        cargo test -p grkit-build --locked --release -- --ignored
    else
        step "Firmware tests skipped: set the six GRKIT_TEST_FIRMWARE variables"
    fi
else
    step "ARM builds skipped: set GRKIT_CLANG and GRKIT_LLD"
fi

step Web
cd web
pnpm install --frozen-lockfile
pnpm wasm
pnpm typecheck
pnpm test
