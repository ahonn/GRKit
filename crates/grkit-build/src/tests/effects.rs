use super::*;
use grkit_core::effects::Effect;
use grkit_core::fit::Model;

/// The camera model on the Standard base gamma of the GR III 2.10 firmware in
/// `GRKIT_TEST_FIRMWARE`.
fn gr3_model() -> Result<Model> {
    let firmware =
        PathBuf::from(std::env::var("GRKIT_TEST_FIRMWARE").context("Set GRKIT_TEST_FIRMWARE")?);
    let image = verify_firmware(&firmware, &target::GR3_210)?;
    Ok(Model::new(&firmware::standard_base_gamma(&image)?))
}

/// The compiled built-in effects of the `effects` directory.
fn builtin_effects() -> Result<Vec<Effect>> {
    let effects = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../effects");
    let mut out = Vec::new();
    for entry in fs::read_dir(effects)? {
        let path = entry?.path().join("effect.json");
        if !path.is_file() {
            continue;
        }
        let definition = grkit_core::effects::parse_json(&fs::read(&path)?)?;
        let (effect, _) =
            grkit_core::effects::decode(&grkit_core::effects::compile_definition(&definition)?)?;
        out.push(effect);
    }
    ensure!(!out.is_empty(), "The effects directory has no effect");
    Ok(out)
}

/// The camera meters for Standard, so a color look must render mid gray as Standard does:
/// otherwise each picture is brighter or darker than the meter intends. If a source LUT moves
/// mid gray, refit it with `--exposure`. Monochrome looks are not checked: they keep the print
/// density of their source.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE"]
fn builtin_color_effects_keep_standard_mid_gray() -> Result<()> {
    use grkit_core::cube::LabConverter;

    let model = gr3_model()?;
    let lab = LabConverter::default();
    let gray = |ev: f64| [0.18 * 2f64.powf(ev); 3];
    let mut offsets = Vec::new();
    for effect in builtin_effects()? {
        if effect.flags & grkit_core::effects::MONOCHROME != 0 {
            continue;
        }
        let target = lab.lab(model.render(&grkit_core::site::params(&effect), gray(0.0)))[0];
        // The exposure at which Standard gives the same L*. Standard brightens monotonically.
        let (mut low, mut high) = (-2.0, 2.0);
        for _ in 0..40 {
            let middle = (low + high) / 2.0;
            if lab.lab(model.standard(gray(middle)))[0] < target {
                low = middle;
            } else {
                high = middle;
            }
        }
        offsets.push((effect.id, low));
    }
    offsets.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
    eprintln!("largest mid-gray offsets in EV: {offsets:.2?}");
    // The color looks are within 0.16 EV. Instax was 0.83 EV before its refit.
    assert!(offsets[0].1.abs() < 0.25, "{offsets:.2?}");
    Ok(())
}

/// The built-in film looks on the camera's Standard base gamma: as the exposure of a color
/// grows from -2 EV up to where the sensor clips, its L* may not fall. The hue sectors can darken or saturate bright
/// colors; the fit bounds this (`fit::sectors::Ramps`), and refits must keep it bounded.
#[test]
#[ignore = "Requires local GRKIT_TEST_FIRMWARE"]
fn builtin_effects_brighten_with_exposure() -> Result<()> {
    use grkit_core::cube::{self, LabConverter};

    let model = gr3_model()?;
    let lab = LabConverter::default();
    let mut falls = Vec::new();
    for effect in builtin_effects()? {
        let params = grkit_core::site::params(&effect);
        let mut worst = 0.0f64;
        for k in 0..36 {
            let hue = std::f64::consts::TAU * k as f64 / 36.0;
            for (l, chroma) in [(30.0, 40.0), (50.0, 50.0), (70.0, 40.0), (40.0, 70.0)] {
                let base = cube::lab_d50_to_srgb([l, chroma * hue.cos(), chroma * hue.sin()]);
                if base.iter().any(|v| !(0.0..=1.0).contains(v)) {
                    continue;
                }
                let mut peak = f64::MIN;
                for step in 0..=48 {
                    let exposure = 2f64.powf(-2.0 + 0.125 * f64::from(step));
                    let linear = base.map(|v| v * exposure);
                    // The sensor clips about 3 times above diffuse white.
                    if linear.iter().any(|v| *v > 3.0) {
                        break;
                    }
                    let lightness = lab.lab(model.render(&params, linear))[0];
                    peak = peak.max(lightness);
                    worst = worst.max(peak - lightness);
                }
            }
        }
        falls.push((effect.id, worst));
    }
    falls.sort_by(|a, b| b.1.total_cmp(&a.1));
    eprintln!("largest L* falls: {falls:.2?}");
    // The fits before the lightness bound fell by up to 30 L*.
    assert!(falls[0].1 < 1.0, "{falls:.2?}");
    Ok(())
}
