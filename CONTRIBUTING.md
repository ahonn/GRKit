# Contributing

## Build inputs

Use Rust 1.92 or later and the checked-in lockfile. See the [README](README.md#build) for the build commands.
Runtime builds need ARM-capable Clang and LLD, but no ARM libc or sysroot. Effects-only builds and package verification need no ARM compiler.
If a compiler is not on PATH, pass `--clang` or `--lld`. `grkit doctor` checks the toolchain.

The baseline camera toolchain is Apple Clang 21.0.0 and LLD 21.1.8. Other versions can change instructions or code sizes.

The accepted GR III 2.10 firmware has SHA-256 `d6a3c9c288d784080e7b283222ecdd8752adfeed6faa36dab44f73115354bfd4`.
Unknown firmware fails validation. The repository does not distribute firmware.

`grkit build` writes `sd/` (the runtime files), `objects/` (sources, intermediate objects, and the installer), and `build.json` (input, source, tool, and output hashes).
Identical inputs produce identical outputs and archives.

## Tests

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
GRKIT_REQUIRE_NATIVE_TESTS=1 cargo test --workspace --locked
```

Host C contract tests compile the actual GRF parser, effect catalog, menu, and Framing code. They need a host C compiler.
Without `GRKIT_REQUIRE_NATIVE_TESTS=1`, they are skipped when no compiler is available.

ARM builds are ignored tests. Run the synthetic build, and the full build with local official firmware:

```sh
GRKIT_CLANG=clang GRKIT_LLD=ld.lld cargo test -p grkit-build --locked synthetic_ -- --ignored
GRKIT_TEST_FIRMWARE="$PWD/firmware/fwdc239b.bin" GRKIT_CLANG=clang GRKIT_LLD=ld.lld \
  cargo test -p grkit-build --locked pinned_firmware_build_is_reproducible -- --ignored
```

## Rules

- Keep filesystem and process access outside `grkit-core`.
- Put target addresses in the target definition or its native adapter. Check hook bytes before patching.
- Keep processing records valid while camera tasks reference them.
- Start each session with Off. Do not select an effect automatically.
- Add focused tests for new behavior. Preserve GRF bytes during compatible refactoring.
- Do not claim device verification from host tests or builds.
- Use conventional commits. Keep firmware, build outputs, archives, and local paths out of Git.
- Preserve third-party notices and review new effect inputs individually.
