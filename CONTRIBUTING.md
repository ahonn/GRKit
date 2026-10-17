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

## Release

Two manual workflows publish the site to Cloudflare Pages:

- `Release` builds the site. It downloads the official firmware, checks its SHA-256, and builds the runtime with LLVM 21. Then it deletes the firmware.
- `Deploy` publishes the site artifact of one successful `Release` run on `main`. To start `Deploy` is the approval step, so the deployed bytes are the tested bytes.

`grkit site` writes only `build.json` and `sd/` to `site/runtime/gr3-2.10/`. The `objects/` directory stays out, because its generated header holds firmware bytes.
Required reviewers for environments are not available for this private repository on its plan, so the approval is a separate workflow.

Set up the release one time:

1. Create a Pages project for direct upload: `npx wrangler pages project create <name> --production-branch=main`.
2. In the repository settings, add the secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`. The token needs the Cloudflare Pages Edit permission.
3. Add the repository variable `CLOUDFLARE_PAGES_PROJECT` with the project name.
4. Keep a private backup of the firmware, so that a release does not depend on the Ricoh download:
   1. Download `gr3_v210.zip` from Ricoh and accept the license. Do not change the zip.
   2. Create a private repository: `gh repo create <owner>/<backup-repo> --private`.
   3. Upload the zip as a release asset: `gh release create gr3-2.10 gr3_v210.zip --repo <owner>/<backup-repo> --notes "Private backup. Do not redistribute."`.
   4. Create a fine-grained token that can access only that repository, with read-only Contents permission.
   5. In this repository, add the secret `FIRMWARE_BACKUP_TOKEN` with the token, and the variable `FIRMWARE_BACKUP_REPO` with `<owner>/<backup-repo>`.
   6. Start the `Release` workflow with `firmware_source` set to `backup`. If the run succeeds, the backup works.

The backup must stay private. The Ricoh license permits a copy only to save and back up the software.
With `firmware_source` set to `auto`, the build uses the backup only if the Ricoh download fails or its SHA-256 differs. Both sources must match the same SHA-256.

Release a version:

1. Start the `Release` workflow on `main`.
2. When the run is complete, download its `device-test` artifact. It contains `GRKit-device-test.zip`, a package with the published runtime and no effects.
3. Install that package on a camera and test it.
4. If the test passes, start the `Deploy` workflow with the run ID of that `Release` run. The run summary shows the ID.

The release toolchain is LLVM 21 on Ubuntu, not the Apple Clang baseline. Test each CI runtime on a camera before you deploy it.

## Rules

- Keep filesystem and process access outside `grkit-core`.
- Put target addresses in the target definition or its native adapter. Check hook bytes before patching.
- Keep processing records valid while camera tasks reference them.
- Start each session with Off. Do not select an effect automatically.
- Add focused tests for new behavior. Preserve GRF bytes during compatible refactoring.
- Do not claim device verification from host tests or builds.
- Use conventional commits. Keep firmware, build outputs, archives, and local paths out of Git.
- Preserve third-party notices and review new effect inputs individually.
