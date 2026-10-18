# Contributing

## Set up

- Rust 1.92 or later. Use the checked-in `Cargo.lock`.
- Clang and LLD with ARM support for runtime builds. No ARM libc or sysroot is necessary. Effect builds and package verification need no ARM compiler.
- The official GR III 2.10 firmware for runtime builds. Its SHA-256 is `d6a3c9c288d784080e7b283222ecdd8752adfeed6faa36dab44f73115354bfd4`. Other firmware fails validation.

If a compiler is not on `PATH`, pass `--clang` or `--lld`. `grkit doctor` checks the toolchain.
The reference toolchain is Apple Clang 21.0.0 with LLD 21.1.8. Other versions can change instructions or code sizes.

`grkit build` writes `sd/` (the runtime files), `objects/` (sources, intermediate objects, and the installer), and `build.json` (the hashes of the inputs, sources, tools, and outputs).
Identical inputs give identical outputs.

## Test

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
GRKIT_REQUIRE_NATIVE_TESTS=1 cargo test --workspace --locked
cargo deny --locked check
```

`cargo deny` checks the dependency licenses, the security advisories, and the crate sources against `deny.toml`. Each ignored advisory in that file gives its reason.

Host C contract tests compile the runtime's GRF parser, effect catalog, menu, and Framing code with a host C compiler.
They use the GR III target. The picture contract also runs for GR IV, and a syntax check compiles the C sources of each target.
If no host compiler is available, the tests are skipped. `GRKIT_REQUIRE_NATIVE_TESTS=1` makes them fail instead.

The ARM builds are ignored tests. The second command needs the firmware:

```sh
GRKIT_CLANG=clang GRKIT_LLD=ld.lld cargo test -p grkit-build --locked synthetic_ -- --ignored
GRKIT_TEST_FIRMWARE="$PWD/firmware/fwdc239b.bin" GRKIT_CLANG=clang GRKIT_LLD=ld.lld \
  cargo test -p grkit-build --locked pinned_firmware_build_is_reproducible -- --ignored
```

For the web site, see [web/README.md](web/README.md).

## Rules

- Keep filesystem and process access out of `grkit-core`.
- Put target addresses in the target definition or its native adapter. Check the hook bytes before you patch them.
- Keep processing records valid while camera tasks use them.
- Do not change persistent native camera settings.
- Add focused tests for new behavior. Keep GRF bytes unchanged in compatible refactors.
- Host tests and builds do not verify device behavior. State in the pull request what you tested on a camera.
- Changes to cache synchronization or camera task lifetimes need a camera test.
- Use conventional commits.
- Keep firmware, build outputs, archives, and local paths out of Git.
- Keep third-party notices. Review each new effect input and its license.

## License of contributions

- A code contribution is licensed under the MIT License.
- An effect contribution uses the license that its source requires. A fit of a spektrafilm LUT uses CC BY-SA 4.0. A fit of a Spectral Film LUT uses the MIT License.
- For a new effect source, add its attribution and license to `effects/NOTICE.md`. Add its license text and its changes to `effects/sources/`.
- Do not contribute an effect if its source license does not permit redistribution of derived data.

Maintainers release the site as [docs/releasing.md](docs/releasing.md) describes.
