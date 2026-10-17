# GRKit

GRKit is an SD-loaded plugin runtime for the RICOH GR III with firmware 2.10 and the GR IIIx with firmware 1.60. It adds a sixth ADJ entry with four modules:

- **Image Control**: film effects and custom effects, with a live-view preview.
- **Framing**: XPan (65:24), 16:9, or 4:3 photos, with a mask in the 3:2 viewfinder.
- **Guides**: thirds, center, golden ratio, or diagonal lines inside the Framing area.
- **Date Stamp**: a film data-back date in orange seven-segment digits on photos.

A package can leave out modules. The camera menu then shows only the modules in the package.

> [!WARNING]
> GRKit is experimental and is not affiliated with Ricoh. It patches the firmware in RAM for the current session and does not write the firmware. Activation enables the camera's persistent SD-script setting. A failure can stop the camera during startup. Read the [recovery guide](resources/RECOVERY.md) before you install GRKit.
> GR IIIx 1.60 runs the code of GR III 2.10, so it uses the same runtime. GRKit has not been tested on a GR IIIx camera.
> Other models and firmware versions are not supported. File formats and the package layout can change.

## Install

Each package contains `INSTALLATION.md`. The [installation guide](resources/INSTALLATION.md) is the same document.

## Build a package

You need Rust 1.92 or later, Clang and LLD with ARM support, and the official GR III 2.10 firmware file from Ricoh. The repository does not contain firmware.

```sh
cargo install --path crates/grkit-cli --locked
grkit effects build --source effects --out build/effects
grkit build --target gr3-2.10 --firmware firmware/fwdc239b.bin --out build/runtime
grkit package --build build/runtime --effects build/effects --out dist/GRKit.zip
grkit verify dist/GRKit.zip
```

`grkit package --camera gr3x` makes a package for the GR IIIx. The runtime build is the same, and only the file that enables SD scripts changes.
`grkit package --hide framing,guides,date-stamp` hides modules. A package without `--effects` has no Image Control.

## Documentation

- [Architecture](docs/architecture.md)
- [Effects](effects/README.md)
- [Web site](web/README.md)
- [Contributing](CONTRIBUTING.md)

## License

The code is licensed under the [MIT License](LICENSE).

- Effect definitions are licensed under CC BY-SA 4.0 or MIT. [effects/NOTICE.md](effects/NOTICE.md) gives the license of each effect.
- Icon text uses the [Inter](https://github.com/rsms/inter) typeface under the [SIL Open Font License 1.1](crates/grkit-core/assets/Inter-LICENSE.txt).
- The effect fit is ported from [gr3-film-mods](https://github.com/idontknownihil/gr3-film-mods) under the [MIT License](crates/grkit-core/GR3_FILM_MODS_LICENSE.txt).
- Generated installers contain bytes derived from the firmware.
