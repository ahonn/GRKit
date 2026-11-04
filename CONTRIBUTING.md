# Contributing

## Set up

- Rust 1.92 or later. Use the checked-in `Cargo.lock`.
- Clang and LLD with ARM support for runtime builds. No ARM libc or sysroot is necessary. Effect builds and package verification need no ARM compiler.
- The official firmware for runtime builds, from Ricoh's download page: `fwdc239b.bin` in `gr3_v210.zip` (GR III 2.10), and `fwdc248b.bin` in `gr4_v111.zip` (GR IV 1.11). The build rejects other firmware.

If a compiler is not on `PATH`, pass `--clang` or `--lld`. `grkit doctor` checks the toolchain.
The reference toolchain is Apple Clang 21.0.0 with LLD 21.1.8. Other versions can change instructions or code sizes.

## Build packages

The [README](README.md#build-from-source) builds a GR III package. For other packages:

- GR IIIx: package with `--camera gr3x` and `--guide build/guides/gr3x/en.pdf`. The runtime is the same as for GR III.
- HDF models: package for the base camera, for example `--camera gr3` for GR III HDF.
- GR IV: build with `--target gr4-1.11` from `fwdc248b.bin`, then package with `--camera gr4` and `--guide build/guides/gr4/en.pdf`.
- GR IV Monochrome: package as for GR IV, without the Film Look module (`FILMLOOK.GRM`) and without `--effects`.
- Fewer modules: give `--module` only for the modules to install. Effects need the Film Look module.
- Guide language: add `--language zh-Hans`, `--language zh-Hant`, `--language ja`, or `--language ko`, and give the guide in that language, for example `build/guides/gr3/zh-Hans.pdf`.

## Test

`scripts/check.sh` runs the Rust and Web checks. It adds ARM and firmware tests when their variables are set (see below).
CI also runs dependency policy, sanitizer, and stack-budget checks.
The basic commands are:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
GRKIT_REQUIRE_NATIVE_TESTS=1 cargo test --workspace --locked
cargo deny --locked check
```

`cargo deny` checks the dependency licenses, the security advisories, and the crate sources against `deny.toml`. Each ignored advisory in that file gives its reason.

Host C tests compile parts of the runtime with a host C compiler. If no host compiler is available, they are skipped. `GRKIT_REQUIRE_NATIVE_TESTS=1` makes them fail instead.

The ARM builds are ignored tests. The first command also runs the NEON parity test of Glow, which needs `qemu-arm` (`GRKIT_QEMU_ARM` names another program). The second command needs the firmware:

```sh
GRKIT_CLANG=clang GRKIT_LLD=ld.lld cargo test -p grkit-build --locked -- --ignored synthetic_ dump_
GRKIT_TEST_FIRMWARE="$PWD/firmware/fwdc239b.bin" GRKIT_CLANG=clang GRKIT_LLD=ld.lld \
  cargo test -p grkit-build --locked pinned_firmware_build_is_reproducible -- --ignored
```

Linux CI also runs the Date Imprint stack budget with the release toolchain:

```sh
GRKIT_CLANG=clang-21 GRKIT_LLD=ld.lld-21 cargo test -p grkit-build --locked --lib -- \
  --ignored --exact tests::modules::date_blur_stays_within_stack_budget
```

Use a host Clang with sanitizer support for the memory checks:

```sh
export GRKIT_HOST_CC="$PWD/.github/scripts/clang-sanitized.sh"
export GRKIT_SANITIZER_CC=clang
export GRKIT_REQUIRE_NATIVE_TESTS=1
cargo run --locked -p grkit-cli -- module test --dir modules/glow --cc "$GRKIT_HOST_CC"
cargo run --locked -p grkit-cli -- module test --dir modules/date --cc "$GRKIT_HOST_CC"
cargo test --locked -p grkit-core --test native_contracts
unset GRKIT_HOST_CC GRKIT_SANITIZER_CC GRKIT_REQUIRE_NATIVE_TESTS
```

The GR IV relocation test `gr4_relocation_resolves_other_builds` also needs an RTOS capture of another GR IV build. Only maintainers have it. The test gives the expected input.

The port test needs the official GR IV 1.11 and 1.20 firmware files, but no toolchain:

```sh
GRKIT_TEST_FIRMWARE_GR4="$PWD/firmware/gr4-1.11/fwdc248b.bin" \
  GRKIT_TEST_FIRMWARE_GR4_120="$PWD/firmware/gr4-1.20/fwdc248b.bin" \
  cargo test -p grkit-build --locked --release port_of_gr4 -- --ignored
```

To port a target to a new firmware build, see [Porting](docs/architecture.md#porting).

For the web site, see [web/README.md](web/README.md).

## Rules

- Keep filesystem and process access out of `grkit-core`.
- Put target addresses in the target definition or its native adapter. Check the hook bytes before you patch them.
- Do not edit `runtime/abi.h` or `web/src/generated/`. Change the layout tables in `crates/grkit-core/src/abi.rs`, the capabilities in `crates/grkit-core/src/capabilities.rs`, or the official modules in `crates/grkit-core/src/modules.rs`. Then run `GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core --test generated`.
- For independent controls in one ADJ entry, use `[page] kind = "settings"` and list the controls in `page.settings`. See [Page syntax](docs/module-reference.md#page).
- For separate settings pages under one ADJ entry, use `[page] kind = "group"` and declare `[[page.children]]`. Each child has a title and its own settings. MENU keeps one module entry and opens the same child pages.
- To add a module, follow [Writing a module](docs/modules.md). [docs/module-reference.md](docs/module-reference.md) gives the keys, services, limits, and error numbers.
- Keep firmware details in the core. Modules under `modules/<folder>/` can include local headers, `runtime/sdk/grkit.h`, and the generated `grkit_module.h`. If a module needs a firmware function, add a core service: an export of `grkit_core::grm::EXPORTS`, or a capability.
- Add each new hook and its extension point to the hooks of its family in `crates/grkit-build/src/target/<family>.rs`. Give its site in each target module of the family. A build without a site for a hook of its family fails.
- Keep processing records valid while camera tasks use them.
- Do not change persistent native camera settings.
- Add focused tests for new behavior. Keep GRF bytes unchanged in compatible refactors.
- Host tests and builds do not verify device behavior. State in the pull request what you tested on a camera.
- Changes to cache synchronization or camera task lifetimes need a camera test.
- Use conventional commits.
- Add a change file in `.changeset/` for each change that a user can see. [docs/releasing.md](docs/releasing.md#record-a-change) describes the format.
- Keep firmware, build outputs, archives, and local paths out of Git.
- Keep third-party notices. Review each new effect input and its license.
- Change the English, Simplified Chinese, Traditional Chinese, Japanese, and Korean guides in `resources/` together, with their headings and steps in the same order. Use only the Markdown that `grkit guides` converts to Typst: headings, paragraphs, lists, images, strong text, code spans, and links. To check the PDFs, run `.github/scripts/build-guides.sh`.

## License of contributions

- A code contribution is licensed under the MIT License.
- An effect contribution uses the license that its source requires. A fit of a spektrafilm LUT uses CC BY-SA 4.0. A fit of a Spectral Film LUT uses the MIT License.
- For a new effect source, add its attribution and license to `effects/NOTICE.md`. Add its license text and its changes to `effects/sources/`.
- Do not contribute an effect if its source license does not permit redistribution of derived data.

Maintainers release GRKit as [docs/releasing.md](docs/releasing.md) describes.
