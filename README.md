# GRKit

An experimental SD-loaded plugin runtime for RICOH GR III firmware 2.10. Other models and firmware versions are unsupported.

GRKit adds a sixth ADJ entry with four modules:

- **Image Control**: bundled or custom effects, with live-view preview. Effects can add a post-curve color matrix and a soft glow.
- **Framing**: XPan (65:24), 16:9, or 4:3 photos with a 3:2 viewfinder mask.
- **Guides**: thirds, center, golden ratio, or diagonal lines inside the Framing area.
- **Date Stamp**: a film data-back date on photos, in glowing orange seven-segment digits.

The project is in development. Its behavior, file formats, and package layout can change.

## Build

Requirements: Rust 1.92 or later, ARM-capable Clang and LLD, and the official GR III 2.10 firmware file.

```sh
cargo install --path crates/grkit-cli --locked
grkit effects build --source effects --out build/effects
grkit build --target gr3-2.10 --firmware firmware/fwdc239b.bin --out build/runtime
grkit package --build build/runtime --effects build/effects --out dist/GRKit.zip
grkit verify dist/GRKit.zip
```

## Documents

- [Installation](resources/INSTALLATION.md) and [recovery](resources/RECOVERY.md)
- [Architecture](docs/architecture.md)
- [Contributing](CONTRIBUTING.md)
- [Effects](effects/README.md)

Software and artwork distribution terms are undecided. Effect data has [separate notices](effects/NOTICE.md).
Icon text uses the [Inter](https://github.com/rsms/inter) typeface under the [SIL Open Font License 1.1](crates/grkit-core/assets/Inter-LICENSE.txt).
Generated installers contain firmware-derived bytes.
