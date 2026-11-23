//! Look files made on the phone: LUT and Lightroom preset imports, copies,
//! and new covers, and the camera icon of a cover.

use anyhow::{Context, Result, ensure};
use grkit_core::{color::Encoding, cover::Cover, effects, fit, site};

/// The largest LUT file the phone converts (the site's limit).
pub const MAX_LUT_SIZE: usize = 128 * 1024 * 1024;
/// The side of the camera icon, in pixels.
pub const ICON_SIDE: usize = 40;

/// The camera's base gamma for demo imports (GR III 2.21 site catalog).
const DEMO_BASE_GAMMA: &str = include_str!("../../data/demo-base-gamma.json");

/// The demo base gamma table.
pub fn demo_base_gamma() -> Vec<u16> {
    serde_json::from_str(DEMO_BASE_GAMMA).expect("bundled base gamma parses")
}

/// Name, ID and monochrome flag of a GRF effect.
pub fn label(grf: &[u8]) -> Result<(String, String, bool)> {
    let (effect, _) = effects::decode(grf)?;
    Ok((
        effect.id,
        effect.name,
        effect.flags & effects::MONOCHROME != 0,
    ))
}

/// A Look made from a file: its GRF effect, the LUT input that the fit
/// used, and how closely the effect follows the file.
#[derive(Clone, Debug)]
pub struct Imported {
    pub grf: Vec<u8>,
    /// `None` for a Lightroom preset, which has no LUT input.
    pub input: Option<Encoding>,
    pub grade: fit::Grade,
}

/// Fit a `.cube` file, a LUT image, or a Lightroom `.xmp` preset into a GRF
/// effect with the given ID and name, and the default cover of the name.
/// `input` is the LUT input, or `None` for the one the data suggests.
pub fn import(
    bytes: &[u8],
    file_name: &str,
    id: &str,
    name: &str,
    base_gamma: &[u16],
    input: Option<Encoding>,
) -> Result<Imported> {
    ensure!(
        bytes.len() <= MAX_LUT_SIZE,
        "The file is larger than {} MB",
        MAX_LUT_SIZE >> 20
    );
    effects::check_identity(id, name)?;
    let base_gamma = site::base_gamma(base_gamma)?;
    let icon = Cover::for_name(name).to_json();
    let imported = fit::import(
        bytes,
        &base_gamma,
        &fit::Import {
            effect_id: id,
            name,
            icon: &icon,
            source: file_name,
            input,
            exposure: 0.0,
            options: fit::Options::default(),
        },
    )?;
    Ok(Imported {
        grf: effects::compile_definition(&effects::parse_json(imported.text.as_bytes())?)?,
        input: imported.hints.is_none().then_some(imported.input),
        grade: fit::Grade::of(&imported.fit),
    })
}

/// A GRF effect with the icon of `cover`. The rest of the file stays, so an
/// official Look stays official.
pub fn with_cover(grf: &[u8], cover: &Cover) -> Result<Vec<u8>> {
    let (effect, _) = effects::decode(grf)?;
    effect.encode(&cover.pixels()?)
}

/// A GRF effect with another ID, name and cover: a copy, or an import whose
/// name changed before it was sent. The effect itself stays.
pub fn relabel(grf: &[u8], id: &str, name: &str, cover: &Cover) -> Result<Vec<u8>> {
    let (mut effect, _) = effects::decode(grf)?;
    effects::check_identity(id, name).context("The Look needs another name")?;
    effect.id = id.to_owned();
    effect.name = name.to_owned();
    effect.encode(&cover.pixels()?)
}

/// The 40 x 40 RGBA camera icon enlarged `factor` times, each pixel a block:
/// the UI shows it at one image pixel per screen pixel, so nothing smooths it.
pub fn enlarge_icon(icon: &[u8], factor: usize) -> Vec<u8> {
    let side = ICON_SIDE * factor;
    let mut out = Vec::with_capacity(side * side * 4);
    for y in 0..side {
        let row = &icon[(y / factor) * ICON_SIDE * 4..][..ICON_SIDE * 4];
        for pixel in row.as_chunks::<4>().0 {
            for _ in 0..factor {
                out.extend_from_slice(pixel);
            }
        }
    }
    out
}

/// Compile a bundled effect definition (demo mode).
pub fn compile(definition: &str) -> Result<Vec<u8>> {
    effects::compile_definition(&effects::parse_json(definition.as_bytes())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VELVIA: &str = include_str!("../../../effects/velvia-100-v1/effect.json");

    #[test]
    fn covers_and_copies_change_only_the_icon_and_label() {
        let grf = compile(VELVIA).unwrap();
        let (original, _) = effects::decode(&grf).unwrap();
        let cover = Cover::for_name("Night 400");
        let covered = with_cover(&grf, &cover).unwrap();
        let (effect, icon) = effects::decode(&covered).unwrap();
        assert_eq!((effect, icon), (original.clone(), cover.pixels().unwrap()));
        let copy = relabel(&grf, "lut-velvia-100", "Velvia 100", &cover).unwrap();
        let (effect, icon) = effects::decode(&copy).unwrap();
        assert_eq!(
            (effect.id.as_str(), effect.curves),
            ("lut-velvia-100", original.curves)
        );
        assert!(cover.matches(&icon));
    }

    #[test]
    fn enlarged_icons_repeat_each_pixel_as_a_block() {
        let icon: Vec<u8> = (0..ICON_SIDE * ICON_SIDE * 4).map(|i| i as u8).collect();
        let big = enlarge_icon(&icon, 3);
        let side = ICON_SIDE * 3;
        assert_eq!(big.len(), side * side * 4);
        let at = |x: usize, y: usize| &big[(y * side + x) * 4..][..4];
        assert_eq!(at(5, 8), &icon[(2 * ICON_SIDE + 1) * 4..][..4]);
        assert_eq!(at(3, 3), at(5, 5));
        assert_ne!(at(2, 2), at(3, 2));
    }

    #[test]
    fn imports_a_cube_file() {
        // A 2-point identity cube.
        let cube = "LUT_3D_SIZE 2\n0 0 0\n1 0 0\n0 1 0\n1 1 0\n0 0 1\n1 0 1\n0 1 1\n1 1 1\n";
        let imported = import(
            cube.as_bytes(),
            "identity.cube",
            "lut-identity",
            "Identity",
            &demo_base_gamma(),
            Some(Encoding::Srgb),
        )
        .unwrap();
        assert_eq!(imported.input, Some(Encoding::Srgb));
        assert_eq!(label(&imported.grf).unwrap().1, "Identity");
        assert!(Cover::for_name("Identity").matches(&effects::decode(&imported.grf).unwrap().1));
        assert!(
            import(
                b"not a lut",
                "x.cube",
                "lut-x",
                "X",
                &demo_base_gamma(),
                None
            )
            .is_err()
        );
    }
}
