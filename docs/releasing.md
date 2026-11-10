# Releasing

Four workflows release GRKit:

- `Release PR` keeps one pull request from the `release` branch open while change files are pending. knope sets the version and writes `CHANGELOG.md` from the change files. `Release PR` then starts `Tests` on the `release` branch, so the pull request shows the test results.
- `Release` starts when the release pull request is merged. It tags the merge commit, runs the tests, builds the site and the packages from the official firmware, and creates a draft GitHub release. Then it runs `Deploy`.
- `Deploy` commits the site of one release to the `site` branch, which Cloudflare Pages deploys. When the site serves the new version, `Deploy` publishes the release. Start `Deploy` by hand to deploy a release again.
- `Site` starts when a push to `main` changes `web/`. It rebuilds the web page with the runtimes and effects of the latest published release. If `web/` needs a packager change that is not released yet, the build fails and the site stays as it is. If a camera entry of `web/` needs a runtime build that the latest release does not have, `Site` deploys nothing, and the release that has the build deploys the page.

The `site` branch holds only deployed sites and shares no history with `main`. Do not push to it by hand. The repository holds no Cloudflare credentials, because Cloudflare reads the branch through its GitHub app.

The version is the `[workspace.package]` version in `Cargo.toml`. A release tag is `v` and the version, for example `v1.2.0`.

The site holds no firmware bytes: `grkit site` leaves out the generated header of each runtime, and the release deletes the firmware after the build.

## Set up

1. In the repository settings, under Actions, General, select "Allow GitHub Actions to create and approve pull requests". `Release PR` needs it.
2. If the repository has no `site` branch, push an orphan `site` branch with one placeholder `index.html`. Cloudflare can connect only to a branch that exists.
3. Connect a Cloudflare Pages project to the repository:
   1. In the Cloudflare dashboard, go to Workers & Pages. Select Create, Pages, and Connect to Git.
   2. Install the Cloudflare GitHub app for this repository only.
   3. Set the production branch to `site` and the framework preset to None. Leave the build command empty. Set the build output directory to `/`.
   4. After Cloudflare creates the project, go to Settings, Builds, Branch control. Set the preview branch to None. Otherwise Cloudflare deploys the source of each other branch as a preview.
4. Add the repository variable `SITE_URL` with the production URL of the site and no trailing slash, for example `https://<project>.pages.dev`. `Deploy` reads the `build.json` of a runtime under `SITE_URL/runtime/` to find the version that the site serves.
5. Optional: add the repository variable `PLAUSIBLE_HOST` with the origin of a Plausible server, for example `https://plausible.example.com`. Release then adds Plausible analytics to the site for the domain `grkit.app`.
6. Keep a private backup of the firmware, so that a release does not depend on the Ricoh download:
   1. Download `gr3_v210.zip`, `gr3_v221.zip`, `gr4_v111.zip`, `gr4_v120.zip`, and `gr4_v121.zip` from Ricoh and accept the license. Do not change the zips. Ricoh does not publish `gr4_v120_0528.zip`, a dump of a camera. Ask a maintainer for it.
   2. Create a private repository: `gh repo create <owner>/<backup-repo> --private`.
   3. Upload each zip as a release asset with the tag of its target:
      - `gh release create gr3-2.10 gr3_v210.zip --repo <owner>/<backup-repo> --notes "Private backup. Do not redistribute."`
      - `gh release create gr3-2.21 gr3_v221.zip --repo <owner>/<backup-repo> --notes "Private backup. Do not redistribute."`
      - `gh release create gr4-1.11 gr4_v111.zip --repo <owner>/<backup-repo> --notes "Private backup. Do not redistribute."`
      - `gh release create gr4-1.20 gr4_v120.zip --repo <owner>/<backup-repo> --notes "Private backup. Do not redistribute."`
      - `gh release create gr4-1.20-0528 gr4_v120_0528.zip --repo <owner>/<backup-repo> --notes "Private backup. Do not redistribute."`
      - `gh release create gr4-1.21 gr4_v121.zip --repo <owner>/<backup-repo> --notes "Private backup. Do not redistribute."`
   4. Create a fine-grained token that can read only the Contents of that repository.
   5. In this repository, add the secret `FIRMWARE_BACKUP_TOKEN` with the token, and the variable `FIRMWARE_BACKUP_REPO` with `<owner>/<backup-repo>`.
   6. Start the `Release` workflow with an existing release tag and `firmware_source` set to `backup`. If the `build` job succeeds, the backup works.

The backup must stay private. The Ricoh license permits a copy only to save and back up the software.
With `firmware_source` set to `auto`, the build uses the backup of a zip only if the Ricoh download of that zip fails or its SHA-256 differs. Both sources must have the same SHA-256. A zip that Ricoh does not publish always comes from the backup.

## Record a change

Each change that a user can see needs a change file in `.changeset/`. Add it in the commit of the change. Write it with `knope document-change`, or by hand:

```md
---
default: patch
---

Show the GRKit version in the site footer.
```

Write the text in English, for people who use GRKit. The change type sets the version and the `CHANGELOG.md` section:

| Type | Section | Version |
| --- | --- | --- |
| `patch` | Fixes | 1.0.0 to 1.0.1 |
| `minor` | Features | 1.0.0 to 1.1.0 |
| `major` | Breaking Changes | 1.0.0 to 2.0.0 |

Use `major` for a change that an earlier package cannot follow, for example a new format of `STATE.TXT`, the module files, or the GRF files.

## Release a version

1. Merge the `Release vX.Y.Z` pull request. `Release` starts on the merge commit. It builds the release, deploys the site, and publishes the release.
2. If the site does not serve the version within 15 minutes, `Deploy` fails and the release stays a draft. Examine the deployment in the Cloudflare dashboard. If it failed, retry it there, because a second `Deploy` run makes no new commit. Then start `Deploy` with the tag `vX.Y.Z`.
3. The release holds these files. Install the package of each camera that you have, and test it.
   - `GRKit-X.Y.Z-site.tar.gz`: the site. Releases before 1.2.0 name it `site.tar.gz`, and `Deploy` and `Site` accept both names.
   - `GRKit-X.Y.Z-gr3.zip`, `GRKit-X.Y.Z-gr3x.zip`, and `GRKit-X.Y.Z-gr4.zip`: the package of each camera, with the published runtime of each of its builds, each effect in `effects/`, each official module, and the English guide. The site makes the same package by default.
   - `GRKit-X.Y.Z-dump.zip`: the firmware dump tool of `runtime/dump/` for each camera. The site footer links to it.
4. If a test fails, start `Deploy` with the tag of the previous release. The site then serves that release again.

To release a commit on `main` without a release pull request, push a tag `vX.Y.Z` that matches the version in `Cargo.toml`. The first release, `v1.0.0`, uses this path.
To build a tag again, for example with `firmware_source` set to `backup`, start `Release` with that tag. A published release is never replaced.

A release keeps its files without an expiry date. To go back to an earlier version, start `Deploy` with its tag.

The release toolchain is LLVM 21 on Ubuntu, not the reference Apple Clang. `Release` deploys its runtimes before a camera test, so test the camera packages right after each release. To test a runtime before the release, build it with LLVM 21 as `release.yml` does.
