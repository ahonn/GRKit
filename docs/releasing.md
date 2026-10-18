# Releasing

Two manual workflows publish the site to Cloudflare Pages:

- `Release` builds the site. It downloads the official GR III 2.10 and GR IV 1.11 firmware and checks their SHA-256. Then it builds a runtime for each with LLVM 21, publishes the effects in `effects/` with their preview tables, and deletes the firmware. It also builds the web page in `web/`.
- `Deploy` publishes the site artifact of one successful `Release` run on `main`. The deployed bytes are therefore the tested bytes.

`grkit site` writes only `build.json` and `sd/` to `site/runtime/<target>/`, for `gr3-2.10` and `gr4-1.11`. It leaves out `objects/`, because the generated header there holds firmware bytes.
`grkit effects publish` writes `catalog.json`, the `.GRF` files, and `preview/<id>.lut` to `site/effects/<target>/`, for both targets. The web build adds `index.html`, `assets/`, and `_headers`.

## Set up

1. Create a Pages project for direct upload: `npx wrangler pages project create <name> --production-branch=main`.
2. In the repository settings, add the secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`. The token needs the Cloudflare Pages Edit permission.
3. Add the repository variable `CLOUDFLARE_PAGES_PROJECT` with the project name.
4. Keep a private backup of the firmware, so that a release does not depend on the Ricoh download:
   1. Download `gr3_v210.zip` and `gr4_v111.zip` from Ricoh and accept the license. Do not change the zips.
   2. Create a private repository: `gh repo create <owner>/<backup-repo> --private`.
   3. Upload each zip as a release asset with the tag of its target:
      - `gh release create gr3-2.10 gr3_v210.zip --repo <owner>/<backup-repo> --notes "Private backup. Do not redistribute."`
      - `gh release create gr4-1.11 gr4_v111.zip --repo <owner>/<backup-repo> --notes "Private backup. Do not redistribute."`
   4. Create a fine-grained token that can read only the Contents of that repository.
   5. In this repository, add the secret `FIRMWARE_BACKUP_TOKEN` with the token, and the variable `FIRMWARE_BACKUP_REPO` with `<owner>/<backup-repo>`.
   6. Start the `Release` workflow with `firmware_source` set to `backup`. If the run succeeds, the backup works.

The backup must stay private. The Ricoh license permits a copy only to save and back up the software.
With `firmware_source` set to `auto`, the build uses the backup of a zip only if the Ricoh download of that zip fails or its SHA-256 differs. Both sources must have the same SHA-256.

## Release a version

1. Start the `Release` workflow on `main`.
2. When the run is complete, download its `device-test` artifact. It contains `GRKit-device-test-gr3.zip`, `GRKit-device-test-gr3x.zip`, and `GRKit-device-test-gr4.zip`: packages with the published runtime and no effects, one for each camera.
3. Install the package of each camera that you have, and test it. The GR IV runtime stays experimental until it passes a test on a camera.
4. If the test passes, start the `Deploy` workflow with the run ID of that `Release` run. The run summary shows the ID.

GitHub keeps the `site` and `device-test` artifacts of a `Release` run for 14 days. `Deploy` downloads the `site` artifact, so it fails after that period. If the period ends before the device test is complete, start a new `Release` run.

The release toolchain is LLVM 21 on Ubuntu, not the reference Apple Clang. Test each CI runtime on a camera before you deploy it.
