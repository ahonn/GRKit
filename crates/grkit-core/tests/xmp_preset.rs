//! Lightroom / Camera Raw `.xmp` preset import: reader, tables, transform, and fit.

mod common;

use common::*;
use grkit_core::cube::Encoding;
use grkit_core::fit::{self, Import, Options};
use grkit_core::xmp::{
    self, blacks_curve, contrast2012, highlights_curve, hue_weights, shadows_curve, table,
    warnings, whites_curve, Pchip, Transform, HUE_BANDS,
};
use serde_json::json;

/// Encode helpers live in the tests only (review item 18).
mod encode {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use grkit_core::xmp::table::{BASE85, BTT_LOOK_TABLE, BTT_RGB_TABLE};
    use std::io::Write;

    pub fn encode_base85(data: &[u8]) -> String {
        let mut padded = data.to_vec();
        padded.extend([0, 0, 0]);
        let mut out = String::new();
        let mut remaining = data.len();
        let mut offset = 0;
        while remaining > 0 {
            let x0 = u32::from_le_bytes(padded[offset..offset + 4].try_into().unwrap());
            offset += 4;
            let x1 = x0 / 85;
            out.push(BASE85[(x0 - x1 * 85) as usize] as char);
            let x2 = x1 / 85;
            out.push(BASE85[(x1 - x2 * 85) as usize] as char);
            remaining -= 1;
            if remaining == 0 {
                break;
            }
            let x3 = x2 / 85;
            out.push(BASE85[(x2 - x3 * 85) as usize] as char);
            remaining -= 1;
            if remaining == 0 {
                break;
            }
            let x4 = x3 / 85;
            out.push(BASE85[(x3 - x4 * 85) as usize] as char);
            remaining -= 1;
            if remaining == 0 {
                break;
            }
            out.push(BASE85[x4 as usize] as char);
            remaining -= 1;
        }
        out
    }

    pub fn wrap_zlib(raw: &[u8]) -> Vec<u8> {
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
        enc.write_all(raw).unwrap();
        let compressed = enc.finish().unwrap();
        let mut out = Vec::with_capacity(4 + compressed.len());
        out.extend((raw.len() as u32).to_le_bytes());
        out.extend(compressed);
        out
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rgb_table_bytes(
        dimensions: u32,
        divisions: u32,
        samples_rgb: &[[u16; 3]],
        primaries: u32,
        gamma: u32,
        gamut: u32,
        min_amount: f64,
        max_amount: f64,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(BTT_RGB_TABLE.to_le_bytes());
        out.extend(1u32.to_le_bytes());
        out.extend(dimensions.to_le_bytes());
        out.extend(divisions.to_le_bytes());
        let nop: Vec<u16> = (0..divisions)
            .map(|i| ((i * 0x0FFFF + (divisions >> 1)) / (divisions - 1)) as u16)
            .collect();
        if dimensions == 1 {
            for (i, rgb) in samples_rgb.iter().enumerate() {
                for &channel in rgb {
                    out.extend(channel.wrapping_sub(nop[i]).to_le_bytes());
                }
            }
        } else {
            let n = divisions as usize;
            for r in 0..n {
                for g in 0..n {
                    for b in 0..n {
                        let rgb = samples_rgb[r * n * n + g * n + b];
                        out.extend(rgb[0].wrapping_sub(nop[r]).to_le_bytes());
                        out.extend(rgb[1].wrapping_sub(nop[g]).to_le_bytes());
                        out.extend(rgb[2].wrapping_sub(nop[b]).to_le_bytes());
                    }
                }
            }
        }
        out.extend(primaries.to_le_bytes());
        out.extend(gamma.to_le_bytes());
        out.extend(gamut.to_le_bytes());
        out.extend(min_amount.to_bits().to_le_bytes());
        out.extend(max_amount.to_bits().to_le_bytes());
        out
    }

    pub fn look_table_bytes(
        hue: u32,
        sat: u32,
        val: u32,
        entries: &[[f32; 3]],
        encoding: u32,
        min_amount: Option<(f64, f64)>,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(BTT_LOOK_TABLE.to_le_bytes());
        let version = if min_amount.is_some() { 2u32 } else { 1u32 };
        out.extend(version.to_le_bytes());
        out.extend(hue.to_le_bytes());
        out.extend(sat.to_le_bytes());
        out.extend(val.to_le_bytes());
        for e in entries {
            for v in e {
                out.extend(v.to_bits().to_le_bytes());
            }
        }
        out.extend(encoding.to_le_bytes());
        if let Some((lo, hi)) = min_amount {
            out.extend(lo.to_bits().to_le_bytes());
            out.extend(hi.to_bits().to_le_bytes());
        }
        out
    }

    pub fn encode_table(raw: &[u8]) -> String {
        encode_base85(&wrap_zlib(raw))
    }

    pub fn identity_rgb_samples(n: u32) -> Vec<[u16; 3]> {
        let nop = |i: u32| ((i * 0x0FFFF + (n >> 1)) / (n - 1)) as u16;
        let mut samples = vec![[0u16; 3]; (n * n * n) as usize];
        for r in 0..n {
            for g in 0..n {
                for b in 0..n {
                    samples[(r * n * n + g * n + b) as usize] = [nop(r), nop(g), nop(b)];
                }
            }
        }
        samples
    }
}

fn packet(attrs: &str, children: &str) -> String {
    format!(
        r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
{attrs}>
{children}
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#
    )
}

fn warn(preset: &xmp::Preset) -> Vec<String> {
    let t = Transform::new(preset).unwrap();
    warnings(preset, &t)
}

fn identity_xmp() -> String {
    packet(
        r#"   crs:Version="15.0" crs:ProcessVersion="11.0" crs:Saturation="0""#,
        "",
    )
}

fn assert_mono(f: impl Fn(f64) -> f64) {
    let mut prev = f(0.0);
    for i in 1..=64 {
        let x = i as f64 / 64.0;
        let y = f(x);
        assert!(y + 1e-9 >= prev, "non-monotonic at {x}: {prev} -> {y}");
        prev = y;
    }
}

#[test]
fn reader_attributes_elements_entities_seq_and_empty() {
    let xmp = packet(
        r#"   crs:Version="15.0"
   crs:Contrast2012="+10"
   crs:Clarity="&lt;12&gt;""#,
        r#"   <crs:ToneCurvePV2012>
    <rdf:Seq>
     <rdf:li>0, 0</rdf:li>
     <rdf:li>128, 140</rdf:li>
     <rdf:li>255, 255</rdf:li>
    </rdf:Seq>
   </crs:ToneCurvePV2012>"#,
    );
    let preset = xmp::parse(xmp.as_bytes()).unwrap();
    assert_eq!(preset.text("Contrast2012"), Some("+10"));
    assert_eq!(preset.text("Clarity"), Some("<12>"));
    assert_eq!(
        preset.seq("ToneCurvePV2012").unwrap(),
        ["0, 0", "128, 140", "255, 255"]
    );

    let empty = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""/>
 </rdf:RDF>
</x:xmpmeta>"#;
    let err = xmp::parse(empty.as_bytes()).unwrap_err().to_string();
    assert!(
        err.contains("Expected Camera Raw develop settings"),
        "{err}"
    );
}

#[test]
fn look_settings_stay_scoped_and_both_layers_apply() {
    let xmp = packet(
        r#"   crs:Version="15.0" crs:Saturation="+20" crs:ConvertToGrayscale="False""#,
        r#"   <crs:Look>
    <rdf:Description crs:Name="Portrait" crs:Amount="1.0" crs:ConvertToGrayscale="True">
     <crs:ToneCurvePV2012>
      <rdf:Seq>
       <rdf:li>0, 0</rdf:li>
       <rdf:li>128, 64</rdf:li>
       <rdf:li>255, 255</rdf:li>
      </rdf:Seq>
     </crs:ToneCurvePV2012>
    </rdf:Description>
   </crs:Look>"#,
    );
    let preset = xmp::parse(xmp.as_bytes()).unwrap();
    assert_eq!(preset.text("Saturation"), Some("+20"));
    assert_eq!(preset.text("ConvertToGrayscale"), Some("False"));
    assert!(preset.seq("ToneCurvePV2012").is_none());
    let look = preset.look.as_ref().unwrap();
    assert!(look.bool_flag("ConvertToGrayscale"));
    assert!(look.seq("ToneCurvePV2012").is_some());

    // Grayscale from the Look wins (stage 3 uses either layer's flag).
    let out = Transform::new(&preset).unwrap().apply([0.9, 0.2, 0.1]);
    assert!(
        (out[0] - out[1]).abs() < 1e-6 && (out[1] - out[2]).abs() < 1e-6,
        "{out:?}"
    );
    // Look tone curve darkens mid gray.
    let mid = Transform::new(&preset).unwrap().apply([0.5, 0.5, 0.5]);
    assert!(mid[0] < 0.4, "{mid:?}");
}

#[test]
fn table_payload_accepted_at_top_level_or_inside_look() {
    use encode::*;
    let n = 3u32;
    let samples = identity_rgb_samples(n);
    let encoded = encode_table(&rgb_table_bytes(3, n, &samples, 0, 1, 0, 0.0, 2.0));
    let fp = "aabbccddeeff00112233445566778899";

    let top = packet(
        &format!(r#"   crs:Version="15.0" crs:RGBTable="{fp}" crs:Table_{fp}="{encoded}""#),
        "",
    );
    let preset = xmp::parse(top.as_bytes()).unwrap();
    assert!(preset.tables.contains_key(fp));
    Transform::new(&preset).unwrap();

    let nested = packet(
        r#"   crs:Version="15.0""#,
        &format!(
            r#"   <crs:Look>
    <rdf:Description crs:Name="L" crs:Amount="1" crs:RGBTable="{fp}" crs:Table_{fp}="{encoded}"/>
   </crs:Look>"#
        ),
    );
    let preset = xmp::parse(nested.as_bytes()).unwrap();
    assert!(preset.tables.contains_key(fp));
    assert_eq!(preset.look.as_ref().unwrap().text("RGBTable"), Some(fp));
    Transform::new(&preset).unwrap();
}

#[test]
fn tables_round_trip_amount_and_reject_gamma() {
    use encode::*;
    use table::*;

    let n = 3u32;
    let samples = identity_rgb_samples(n);
    let mut samples = samples;
    let last = samples.len() - 1;
    samples[last] = [0xFFFF, 0x8000, 0x8000];
    let encoded = encode_table(&rgb_table_bytes(3, n, &samples, 0, 1, 0, 0.0, 2.0));
    let BigTable::Rgb(decoded) = decode_big_table(&encoded).unwrap() else {
        panic!("rgb");
    };
    assert_eq!(decoded.divisions, 3);
    let white = lookup_rgb_3d(&decoded, [1.0, 1.0, 1.0]);
    assert!((white[0] - 1.0).abs() < 1e-4, "{white:?}");
    assert!(white[1] < 0.6, "{white:?}");

    let input = [0.5, 0.5, 0.5];
    let mapped = lookup_rgb_3d(&decoded, input);
    assert_eq!(apply_amount(input, mapped, 0.0), input);
    assert_eq!(apply_amount(input, mapped, 1.0), mapped);
    assert_eq!(
        apply_amount(input, mapped, 2.0),
        [
            input[0] + 2.0 * (mapped[0] - input[0]),
            input[1] + 2.0 * (mapped[1] - input[1]),
            input[2] + 2.0 * (mapped[2] - input[2])
        ]
    );

    let entries = [[10.0f32, 1.2, 1.0]; 4];
    let look = look_table_bytes(2, 2, 1, &entries, 0, Some((0.0, 2.0)));
    let BigTable::Look(look) = decode_big_table(&encode_table(&look)).unwrap() else {
        panic!("look");
    };
    assert!((lookup_look(&look, [0.0, 1.0, 0.5])[0] - 10.0).abs() < 1e-4);

    let bad = rgb_table_bytes(
        1,
        2,
        &[[0, 0, 0], [0xFFFF, 0xFFFF, 0xFFFF]],
        0,
        99,
        0,
        0.0,
        2.0,
    );
    let err = decode_big_table(&encode_table(&bad))
        .unwrap_err()
        .to_string();
    assert!(err.contains("gamma"), "{err}");
}

#[test]
fn rgb_table_supported_spaces_round_trip_identity_and_unsupported_skipped() {
    use encode::*;
    let n = 3u32;
    let samples = identity_rgb_samples(n);
    // Primaries: 0=sRGB 1=Adobe 2=ProPhoto 3=P3 4=Rec2020; gamma 0..4 all implemented.
    for primaries in 0..=4u32 {
        for gamma in 0..=4u32 {
            let encoded = encode_table(&rgb_table_bytes(
                3, n, &samples, primaries, gamma, 0, 0.0, 2.0,
            ));
            let fp = format!("fp{primaries}{gamma}000000000000000000000000");
            let xmp = packet(
                &format!(
                    r#"   crs:Version="15.0" crs:RGBTable="{fp}" crs:Table_{fp}="{encoded}" crs:RGBTableAmount="1""#
                ),
                "",
            );
            let preset = xmp::parse(xmp.as_bytes()).unwrap();
            let t = Transform::new(&preset).unwrap();
            assert!(t.apply([0.5, 0.5, 0.5])[0].is_finite());
            // Identity table ≈ identity display (within space round-trip).
            let out = t.apply([0.4, 0.5, 0.6]);
            for c in 0..3 {
                assert!(
                    (out[c] - [0.4, 0.5, 0.6][c]).abs() < 0.05,
                    "p={primaries} g={gamma} {out:?}"
                );
            }
        }
    }

    // 1D RGB table is unsupported for application.
    let encoded = encode_table(&rgb_table_bytes(
        1,
        3,
        &[
            [0, 0, 0],
            [0x8000, 0x8000, 0x8000],
            [0xFFFF, 0xFFFF, 0xFFFF],
        ],
        0,
        1,
        0,
        0.0,
        2.0,
    ));
    let fp = "deadbeefdeadbeefdeadbeefdeadbeef";
    let xmp = packet(
        &format!(r#"   crs:Version="15.0" crs:RGBTable="{fp}" crs:Table_{fp}="{encoded}""#),
        "",
    );
    let preset = xmp::parse(xmp.as_bytes()).unwrap();
    let w = warn(&preset);
    assert!(
        w.iter()
            .any(|s| s.starts_with("Skipped RGBTable:") && s.contains("3D") && s.ends_with('.')),
        "{w:?}"
    );
}

#[test]
fn transform_identity_tone_curve_grayscale_hsl_saturation_monotonic() {
    let identity = xmp::parse(identity_xmp().as_bytes()).unwrap();
    let t = Transform::new(&identity).unwrap();
    for rgb in [
        [0.0, 0.0, 0.0],
        [0.5, 0.5, 0.5],
        [1.0, 1.0, 1.0],
        [0.8, 0.2, 0.1],
    ] {
        let out = t.apply(rgb);
        for c in 0..3 {
            assert!(
                (out[c] - rgb[c]).abs() < 2e-3,
                "identity {rgb:?} -> {out:?}"
            );
        }
    }

    let tone = packet(
        r#"   crs:Version="15.0""#,
        r#"   <crs:ToneCurvePV2012>
    <rdf:Seq>
     <rdf:li>0, 0</rdf:li>
     <rdf:li>128, 64</rdf:li>
     <rdf:li>255, 255</rdf:li>
    </rdf:Seq>
   </crs:ToneCurvePV2012>"#,
    );
    let preset = xmp::parse(tone.as_bytes()).unwrap();
    let mid = Transform::new(&preset).unwrap().apply([0.5, 0.5, 0.5]);
    assert!(
        mid[0] < 0.4 && (mid[0] - mid[1]).abs() < 1e-4 && (mid[1] - mid[2]).abs() < 1e-4,
        "{mid:?}"
    );

    let gray = packet(
        r#"   crs:Version="15.0" crs:ConvertToGrayscale="true"
   crs:GrayMixerRed="0" crs:GrayMixerGreen="0" crs:GrayMixerBlue="0""#,
        "",
    );
    let out = Transform::new(&xmp::parse(gray.as_bytes()).unwrap())
        .unwrap()
        .apply([0.8, 0.4, 0.2]);
    assert!(
        (out[0] - out[1]).abs() < 1e-6 && (out[1] - out[2]).abs() < 1e-6,
        "{out:?}"
    );

    let hsl = packet(
        r#"   crs:Version="15.0" crs:SaturationAdjustmentRed="+100"
   crs:SaturationAdjustmentBlue="-100""#,
        "",
    );
    let t = Transform::new(&xmp::parse(hsl.as_bytes()).unwrap()).unwrap();
    let red = t.apply([0.9, 0.1, 0.1]);
    // A sky blue lies at the center of the Blue band; the sRGB primary lies toward Purple.
    let blue = t.apply([0.1, 0.35, 0.9]);
    let sat = |c: [f64; 3]| {
        let max = c[0].max(c[1]).max(c[2]);
        let min = c[0].min(c[1]).min(c[2]);
        if max <= 1e-9 {
            0.0
        } else {
            (max - min) / max
        }
    };
    assert!(sat(red) > 0.85, "{red:?}");
    assert!(sat(blue) < 0.2, "{blue:?}");

    let desat = packet(r#"   crs:Version="15.0" crs:Saturation="-100""#, "");
    let out = Transform::new(&xmp::parse(desat.as_bytes()).unwrap())
        .unwrap()
        .apply([0.9, 0.2, 0.1]);
    assert!(
        (out[0] - out[1]).abs() < 1e-9 && (out[1] - out[2]).abs() < 1e-9,
        "{out:?}"
    );

    // Saturation slider monotonic for a mid-saturated color.
    let base = [0.6, 0.4, 0.35];
    let mut prev_sat = -1.0;
    for amount in [-100i32, -50, 0, 50, 100] {
        let xmp = packet(
            &format!(r#"   crs:Version="15.0" crs:Saturation="{amount}""#),
            "",
        );
        let out = Transform::new(&xmp::parse(xmp.as_bytes()).unwrap())
            .unwrap()
            .apply(base);
        let s = sat(out);
        assert!(s + 1e-9 >= prev_sat, "{amount}: {prev_sat} -> {s}");
        prev_sat = s;
    }

    let contrast = packet(r#"   crs:Version="15.0" crs:Contrast2012="+40""#, "");
    let t = Transform::new(&xmp::parse(contrast.as_bytes()).unwrap()).unwrap();
    assert_mono(|x| t.apply([x, x, x])[0]);
}

#[test]
fn contrast_is_an_s_curve_and_zero_is_the_identity() {
    for amount in [-100.0, -50.0, 50.0, 100.0] {
        assert_mono(|x| contrast2012(x, amount));
    }
    for x in [0.0, 0.2, 0.5, 0.8, 1.0] {
        assert!((contrast2012(x, 0.0) - x).abs() < 1e-12);
        // An amount between two fitted curves lies between them.
        let (low, mid, high) = (
            contrast2012(x, 50.0),
            contrast2012(x, 75.0),
            contrast2012(x, 100.0),
        );
        assert!(mid >= low.min(high) - 1e-12 && mid <= low.max(high) + 1e-12);
    }
    assert!(contrast2012(0.25, 100.0) < 0.25);
    assert!(contrast2012(0.75, 100.0) > 0.75);
    assert!(contrast2012(0.25, -100.0) > 0.25);
    assert!(contrast2012(0.75, -100.0) < 0.75);
    // More than the largest fitted amount holds its curve.
    assert_eq!(contrast2012(0.3, 150.0), contrast2012(0.3, 100.0));
}

#[test]
fn tone_sliders_are_monotonic_and_move_the_right_way() {
    for amount in [-100.0, -50.0, 50.0, 100.0] {
        assert_mono(|x| highlights_curve(x, amount));
        assert_mono(|x| shadows_curve(x, amount));
        assert_mono(|x| whites_curve(x, amount));
        assert_mono(|x| blacks_curve(x, amount));
    }
    for x in [0.1, 0.5, 0.9] {
        assert!((whites_curve(x, 0.0) - x).abs() < 1e-12);
    }
    assert!(highlights_curve(0.8, -100.0) < 0.8 && highlights_curve(0.8, 100.0) > 0.8);
    assert!(shadows_curve(0.2, -100.0) < 0.2 && shadows_curve(0.2, 100.0) > 0.2);
    assert!(whites_curve(0.8, -100.0) < 0.8 && whites_curve(0.8, 100.0) > 0.8);
    assert!(blacks_curve(0.2, -100.0) < 0.2 && blacks_curve(0.2, 100.0) > 0.2);
    // Each slider acts mostly on its own end of the tones.
    assert!(
        (highlights_curve(0.1, -100.0) - 0.1).abs() < (highlights_curve(0.8, -100.0) - 0.8).abs()
    );
    assert!((blacks_curve(0.9, 100.0) - 0.9).abs() < (blacks_curve(0.1, 100.0) - 0.1).abs());
}

#[test]
fn gray_mixer_orange_brightens_orange_not_blue() {
    let orange_xmp = packet(
        r#"   crs:Version="15.0" crs:ConvertToGrayscale="True" crs:GrayMixerOrange="+100""#,
        "",
    );
    let t = Transform::new(&xmp::parse(orange_xmp.as_bytes()).unwrap()).unwrap();
    let base = packet(r#"   crs:Version="15.0" crs:ConvertToGrayscale="True""#, "");
    let t0 = Transform::new(&xmp::parse(base.as_bytes()).unwrap()).unwrap();
    let orange = [0.9, 0.45, 0.1];
    let blue = [0.1, 0.2, 0.9];
    assert!(t.apply(orange)[0] > t0.apply(orange)[0] + 0.02);
    assert!((t.apply(blue)[0] - t0.apply(blue)[0]).abs() < 0.02);
    let _ = HUE_BANDS;
    let _ = hue_weights(30.0);
}

#[test]
fn pchip_exact_at_points_and_smooth() {
    let curve = Pchip::new(&[(0.0, 0.0), (0.5, 0.25), (1.0, 1.0)]).unwrap();
    assert!((curve.eval(0.0)).abs() < 1e-12);
    assert!((curve.eval(0.5) - 0.25).abs() < 1e-12);
    assert!((curve.eval(1.0) - 1.0).abs() < 1e-12);
    let eps = 1e-4;
    let left = (curve.eval(0.5) - curve.eval(0.5 - eps)) / eps;
    let right = (curve.eval(0.5 + eps) - curve.eval(0.5)) / eps;
    assert!((left - right).abs() < 0.05, "slope {left} vs {right}");
    assert_mono(|x| curve.eval(x));
}

#[test]
fn default_sharpening_gives_no_warning_and_masking_is_not_local() {
    let defaults = packet(
        r#"   crs:Version="15.0"
   crs:Sharpness="40" crs:SharpenRadius="1.0" crs:SharpenDetail="25"
   crs:SharpenEdgeMasking="0" crs:ColorNoiseReduction="25"
   crs:ColorNoiseReductionDetail="50" crs:ColorNoiseReductionSmoothness="50""#,
        "",
    );
    let preset = xmp::parse(defaults.as_bytes()).unwrap();
    let w = warn(&preset);
    assert!(w.is_empty(), "{w:?}");

    let masking = packet(r#"   crs:Version="15.0" crs:SharpenEdgeMasking="50""#, "");
    let w = warn(&xmp::parse(masking.as_bytes()).unwrap());
    assert!(!w.iter().any(|s| s.contains("local")), "{w:?}");
    assert!(w.iter().any(|s| s.contains("sharpening")), "{w:?}");

    let local = packet(
        r#"   crs:Version="15.0""#,
        r#"   <crs:MaskGroupBasedCorrections>
    <rdf:Seq><rdf:li>mask</rdf:li></rdf:Seq>
   </crs:MaskGroupBasedCorrections>"#,
    );
    let w = warn(&xmp::parse(local.as_bytes()).unwrap());
    assert!(w.iter().any(|s| s.contains("local")), "{w:?}");

    let wb = packet(
        r#"   crs:Version="15.0" crs:WhiteBalance="Custom" crs:Temperature="5200" crs:Tint="+10""#,
        "",
    );
    let w = warn(&xmp::parse(wb.as_bytes()).unwrap());
    assert!(
        w.iter()
            .any(|s| s == "The film look does not hold the white balance."),
        "{w:?}"
    );
    assert!(
        !w.iter()
            .any(|s| s.contains("Skipped") && s.contains("white balance")),
        "{w:?}"
    );

    let shift = packet(
        r#"   crs:Version="15.0" crs:WhiteBalance="As Shot" crs:IncrementalTemperature="+20""#,
        "",
    );
    let w = warn(&xmp::parse(shift.as_bytes()).unwrap());
    assert!(
        w.iter()
            .any(|s| s == "The film look does not hold the white balance."),
        "{w:?}"
    );
}

#[test]
fn import_synthetic_preset_end_to_end() {
    use encode::*;
    let n = 3u32;
    let samples = identity_rgb_samples(n);
    let good = rgb_table_bytes(3, n, &samples, 0, 1, 0, 0.0, 2.0);
    let fp = "0123456789abcdef0123456789abcdef";
    let encoded = encode_table(&good);

    let xmp = packet(
        &format!(
            r#"   crs:Version="15.0"
   crs:Exposure2012="+0.50"
   crs:GrainAmount="25"
   crs:GrainSize="40"
   crs:GrainFrequency="50"
   crs:IncrementalTemperature="+10"
   crs:IncrementalTint="-5"
   crs:Highlights2012="-30"
   crs:Clarity="20"
   crs:Saturation="+15"
   crs:RGBTable="{fp}"
   crs:Table_{fp}="{encoded}"
   crs:RGBTableAmount="1""#
        ),
        "",
    );

    let icon = json!({"label": "XP", "shape": "filter"});
    let request = Import {
        effect_id: "xmp-preset-v1",
        name: "Preset",
        icon: &icon,
        source: "look.xmp",
        input: None,
        exposure: 0.0,
        options: Options {
            random_samples: 300,
            rounds: 2,
            joint_rounds: 1,
            sector_rounds: 1,
            sector_samples: 200,
            joint_passes: 1,
            ..Options::default()
        },
    };
    let imported =
        fit::import_with_progress(xmp.as_bytes(), &synthetic_base(), &request, &|_| {}).unwrap();
    assert_eq!(imported.input, Encoding::Standard);
    let hints = imported.hints.as_ref().expect("hints");
    assert_eq!(hints.exposure, Some(0.5));
    assert_eq!(hints.grain_amount, Some(25.0));
    assert_eq!(hints.incremental_temperature, Some(10.0));
    assert!(
        imported
            .warnings
            .iter()
            .any(|w| w.contains("tone bridge") && w.contains("Standard")),
        "{:?}",
        imported.warnings
    );
    assert!(
        imported
            .warnings
            .iter()
            .any(|w| w.contains("Approximated") && w.contains("Highlights")),
        "{:?}",
        imported.warnings
    );
    assert!(
        imported
            .warnings
            .iter()
            .any(|w| w.contains("Skipped") && w.contains("Clarity")),
        "{:?}",
        imported.warnings
    );
    let json: serde_json::Value = serde_json::from_str(&imported.text).unwrap();
    assert_eq!(json["attribution"]["origin"], "xmp-preset");
}

#[test]
fn prepare_composes_base_bridge_before_preset() {
    let identity = identity_xmp();
    let with_bridge = xmp::prepare_with(identity.as_bytes(), true).unwrap();
    let without = xmp::prepare_with(identity.as_bytes(), false).unwrap();
    // Without the bridge, the identity preset is the identity LUT.
    let last = (xmp::LUT_SIZE - 1) as f64;
    let mid = xmp::LUT_SIZE / 2;
    let idx =
        |r: usize, g: usize, b: usize| b * xmp::LUT_SIZE * xmp::LUT_SIZE + g * xmp::LUT_SIZE + r;
    let mid_without = without.cube.table[idx(mid, mid, mid)];
    assert!(
        mid_without
            .iter()
            .all(|v| (*v as f64 - mid as f64 / last).abs() < 1e-3),
        "{mid_without:?}"
    );
    // With the bridge, mid gray moves (same as bridge::apply on mid gray).
    let mid_with = with_bridge.cube.table[idx(mid, mid, mid)];
    assert!(
        (mid_with[0] as f64 - mid as f64 / last).abs() > 0.01,
        "{mid_with:?}"
    );
    assert!(with_bridge.warnings[0].contains("tone bridge"));
    assert!(without.warnings.iter().all(|w| !w.contains("tone bridge")));
}

fn invert_r_samples(n: u32) -> Vec<[u16; 3]> {
    let nop = |i: u32| ((i * 0x0FFFF + (n >> 1)) / (n - 1)) as u16;
    let mut samples = vec![[0u16; 3]; (n * n * n) as usize];
    for r in 0..n {
        for g in 0..n {
            for b in 0..n {
                samples[(r * n * n + g * n + b) as usize] = [nop(n - 1 - r), nop(g), nop(b)];
            }
        }
    }
    samples
}

#[test]
fn rgb_table_matrix_direction_invert_red() {
    use encode::*;
    use grkit_core::color::{inverse, mul, product, srgb_decode, srgb_encode, Matrix};

    let n = 5u32;
    let samples = invert_r_samples(n);
    let encoded = encode_table(&rgb_table_bytes(3, n, &samples, 0, 1, 0, 0.0, 2.0));
    let fp = "invred00000000000000000000000001";
    let xmp = packet(
        &format!(
            r#"   crs:Version="15.0" crs:RGBTable="{fp}" crs:Table_{fp}="{encoded}" crs:RGBTableAmount="1""#
        ),
        "",
    );
    let out = Transform::new(&xmp::parse(xmp.as_bytes()).unwrap())
        .unwrap()
        .apply([0.25, 0.5, 0.75]);
    assert!(
        (out[0] - 0.75).abs() < 1e-3 && (out[1] - 0.5).abs() < 1e-3 && (out[2] - 0.75).abs() < 1e-3,
        "{out:?}"
    );

    // Same table tagged Adobe RGB + gamma 2.2: expected from a direct conversion.
    let encoded = encode_table(&rgb_table_bytes(3, n, &samples, 1, 3, 0, 0.0, 2.0));
    let fp = "invred00000000000000000000000002";
    let xmp = packet(
        &format!(
            r#"   crs:Version="15.0" crs:RGBTable="{fp}" crs:Table_{fp}="{encoded}" crs:RGBTableAmount="1""#
        ),
        "",
    );
    let got = Transform::new(&xmp::parse(xmp.as_bytes()).unwrap())
        .unwrap()
        .apply([0.25, 0.5, 0.75]);

    let bradford_d65_to_d50 = || {
        let bradford: Matrix = [
            [0.8951, 0.2664, -0.1614],
            [-0.7502, 1.7135, 0.0367],
            [0.0389, -0.0685, 1.0296],
        ];
        let d65 = [0.95047, 1.0, 1.08883];
        let d50 = [0.96422, 1.0, 0.82521];
        let (from, to) = (mul(&bradford, d65), mul(&bradford, d50));
        let gain: Matrix = std::array::from_fn(|r| {
            std::array::from_fn(|c| if r == c { to[r] / from[r] } else { 0.0 })
        });
        product(&inverse(&bradford).unwrap(), &product(&gain, &bradford))
    };
    let rgb_to_xyz = |primaries: [[f64; 2]; 3], wp: [f64; 2]| {
        let white = |xy: [f64; 2]| [xy[0] / xy[1], 1.0, (1.0 - xy[0] - xy[1]) / xy[1]];
        let columns: Matrix =
            std::array::from_fn(|r| std::array::from_fn(|c| white(primaries[c])[r]));
        let scale = mul(&inverse(&columns).unwrap(), white(wp));
        std::array::from_fn(|r| std::array::from_fn(|c| columns[r][c] * scale[c]))
    };
    let d65_to_prophoto = |primaries: [[f64; 2]; 3]| {
        let adapt = bradford_d65_to_d50();
        let space = rgb_to_xyz(primaries, [0.3127, 0.3290]);
        let prophoto = rgb_to_xyz(
            [[0.7347, 0.2653], [0.1596, 0.8404], [0.0366, 0.0001]],
            [0.3457, 0.3585],
        );
        product(&inverse(&prophoto).unwrap(), &product(&adapt, &space))
    };
    let srgb_to_pp = d65_to_prophoto([[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]]);
    let adobe_to_pp = d65_to_prophoto([[0.64, 0.33], [0.21, 0.71], [0.15, 0.06]]);
    let pp_to_adobe = inverse(&adobe_to_pp).unwrap();
    let pp_to_srgb = inverse(&srgb_to_pp).unwrap();
    let display = [0.25, 0.5, 0.75];
    let linear_pp = mul(&srgb_to_pp, display.map(srgb_decode));
    let linear_adobe = mul(&pp_to_adobe, linear_pp);
    let encoded = linear_adobe.map(|v| v.clamp(0.0, 1.0).powf(1.0 / 2.2));
    let mapped = [1.0 - encoded[0], encoded[1], encoded[2]];
    let linear_out = mapped.map(|v| v.clamp(0.0, 1.0).powf(2.2));
    let linear_pp_out = mul(&adobe_to_pp, linear_out);
    let expected = mul(&pp_to_srgb, linear_pp_out).map(|v| srgb_encode(v.clamp(0.0, 1.0)));
    for c in 0..3 {
        assert!(
            (got[c] - expected[c]).abs() < 2e-2,
            "c={c} got={got:?} exp={expected:?}"
        );
    }
}

#[test]
fn hue_weights_partition_of_unity_and_smooth() {
    for h in 0..360 {
        let w = hue_weights(h as f64);
        let sum: f64 = w.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9, "h={h} sum={sum} {w:?}");
    }
    // Halfway between the centers of Orange and Yellow.
    let halfway = hue_weights((HUE_BANDS[1].1 + HUE_BANDS[2].1) / 2.0);
    assert!(
        (halfway[1] - 0.5).abs() < 1e-9 && (halfway[2] - 0.5).abs() < 1e-9,
        "{halfway:?}"
    );
    for h in 0..359 {
        let a = hue_weights(h as f64);
        let b = hue_weights((h + 1) as f64);
        for i in 0..8 {
            assert!((a[i] - b[i]).abs() < 0.05, "band {i} at {h}");
        }
    }
}

#[test]
fn parametric_curve_monotone_at_extreme_sliders() {
    let cases = [
        r#"crs:ParametricDarks="-100""#,
        r#"crs:ParametricShadows="+100""#,
        r#"crs:ParametricLights="-100""#,
        r#"crs:ParametricHighlights="+100""#,
        r#"crs:ParametricShadows="+100" crs:ParametricDarks="-100" crs:ParametricLights="+100" crs:ParametricHighlights="-100""#,
    ];
    for attrs in cases {
        let xmp = packet(&format!(r#"   crs:Version="15.0" {attrs}"#), "");
        let t = Transform::new(&xmp::parse(xmp.as_bytes()).unwrap()).unwrap();
        assert_mono(|x| t.apply([x, x, x])[0]);
    }
}

#[test]
fn color_grade_ignores_split_toning_when_both_present() {
    let both = packet(
        r#"   crs:Version="15.0"
   crs:ColorGradeShadowHue="30" crs:ColorGradeShadowSat="40" crs:ColorGradeBlending="50"
   crs:SplitToningShadowHue="30" crs:SplitToningShadowSaturation="40" crs:SplitToningBalance="0""#,
        "",
    );
    let only = packet(
        r#"   crs:Version="15.0"
   crs:ColorGradeShadowHue="30" crs:ColorGradeShadowSat="40" crs:ColorGradeBlending="50""#,
        "",
    );
    let a = Transform::new(&xmp::parse(both.as_bytes()).unwrap())
        .unwrap()
        .apply([0.3, 0.25, 0.2]);
    let b = Transform::new(&xmp::parse(only.as_bytes()).unwrap())
        .unwrap()
        .apply([0.3, 0.25, 0.2]);
    for c in 0..3 {
        assert!((a[c] - b[c]).abs() < 1e-9, "{a:?} vs {b:?}");
    }
}

#[test]
fn color_grade_blending_zero_still_affects_shadows() {
    let mid_dark = [0.25, 0.22, 0.2];
    for blending in [0, 100] {
        let xmp = packet(
            &format!(
                r#"   crs:Version="15.0" crs:ColorGradeShadowSat="50" crs:ColorGradeShadowHue="20" crs:ColorGradeBlending="{blending}""#
            ),
            "",
        );
        let out = Transform::new(&xmp::parse(xmp.as_bytes()).unwrap())
            .unwrap()
            .apply(mid_dark);
        let delta: f64 = (0..3).map(|i| (out[i] - mid_dark[i]).abs()).sum();
        assert!(delta > 1e-3, "blending {blending}: {out:?}");
    }
}

#[test]
fn nan_saturation_ignored_nan_curve_errors() {
    let nan_sat = packet(r#"   crs:Version="15.0" crs:Saturation="nan""#, "");
    let preset = xmp::parse(nan_sat.as_bytes()).unwrap();
    assert!(preset.number("Saturation").is_none());
    let out = Transform::new(&preset).unwrap().apply([0.8, 0.2, 0.1]);
    assert!((out[0] - 0.8).abs() < 2e-3);

    let bad_curve = packet(
        r#"   crs:Version="15.0""#,
        r#"   <crs:ToneCurvePV2012>
    <rdf:Seq>
     <rdf:li>nan, 0</rdf:li>
     <rdf:li>255, 255</rdf:li>
    </rdf:Seq>
   </crs:ToneCurvePV2012>"#,
    );
    let err = match Transform::new(&xmp::parse(bad_curve.as_bytes()).unwrap()) {
        Ok(_) => panic!("expected curve error"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("ToneCurvePV2012"), "{err}");
}

#[test]
fn looks_like_rejects_images_and_cubes_accepts_bom() {
    let packet = identity_xmp();
    let mut png = vec![0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    png.extend(packet.as_bytes());
    assert!(!xmp::looks_like(&png));

    let cube = b"TITLE \"x\"\nLUT_3D_SIZE 2\n0 0 0\n";
    assert!(!xmp::looks_like(cube));

    let mut with_bom = vec![0xEF, 0xBB, 0xBF, b'\n', b'\n'];
    with_bom.extend(packet.as_bytes());
    assert!(xmp::looks_like(&with_bom));
    assert!(xmp::looks_like(packet.as_bytes()));
}

#[test]
fn missing_table_and_named_look_warnings() {
    let missing = packet(
        r#"   crs:Version="15.0" crs:RGBTable="missingfingerprint000000000000000""#,
        "",
    );
    let w = warn(&xmp::parse(missing.as_bytes()).unwrap());
    assert!(
        w.iter()
            .any(|s| s == "Skipped the profile table: the preset does not contain it."),
        "{w:?}"
    );

    let named = packet(
        r#"   crs:Version="15.0""#,
        r#"   <crs:Look>
    <rdf:Description crs:Name="Portrait Soft" crs:Amount="1"/>
   </crs:Look>"#,
    );
    let w = warn(&xmp::parse(named.as_bytes()).unwrap());
    assert!(
        w.iter()
            .any(|s| s.contains("Portrait Soft") && s.contains("does not contain")),
        "{w:?}"
    );

    let adobe = packet(
        r#"   crs:Version="15.0""#,
        r#"   <crs:Look>
    <rdf:Description crs:Name="Adobe Color" crs:Amount="1"/>
   </crs:Look>"#,
    );
    let w = warn(&xmp::parse(adobe.as_bytes()).unwrap());
    assert!(!w.iter().any(|s| s.contains("Adobe Color")), "{w:?}");

    // The fields that describe a profile are not color settings.
    let stubbed = packet(
        r#"   crs:Version="15.0""#,
        r#"   <crs:Look>
    <rdf:Description crs:Name="Adobe Color" crs:Amount="1.000000" crs:Stubbed="true"/>
   </crs:Look>"#,
    );
    let w = warn(&xmp::parse(stubbed.as_bytes()).unwrap());
    assert!(w.is_empty(), "{w:?}");

    // An Adobe profile is the base rendering: its table reference without data and its tone
    // curve are not part of the look, and it gives no warning.
    let base = packet(
        r#"   crs:Version="15.0" crs:Saturation="+10""#,
        r#"   <crs:Look>
    <rdf:Description crs:Name="Adobe Landscape" crs:Amount="1">
     <crs:Parameters>
      <rdf:Description crs:Highlights2012="-12" crs:ConvertToGrayscale="False"
       crs:LookTable="0B3BFB5CFB7DBF7FF175E98F24D316B0">
       <crs:ToneCurvePV2012>
        <rdf:Seq><rdf:li>0, 0</rdf:li><rdf:li>64, 60</rdf:li><rdf:li>255, 255</rdf:li></rdf:Seq>
       </crs:ToneCurvePV2012>
      </rdf:Description>
     </crs:Parameters>
    </rdf:Description>
   </crs:Look>"#,
    );
    let parsed = xmp::parse(base.as_bytes()).unwrap();
    assert!(parsed.look.is_none());
    let w = warn(&parsed);
    assert!(w.is_empty(), "{w:?}");

    let unread = packet(
        r#"   crs:Version="15.0""#,
        r#"   <crs:Look>
    <rdf:Description crs:Name="Portrait Soft" crs:Amount="1" crs:Saturation="+20"/>
   </crs:Look>"#,
    );
    let w = warn(&xmp::parse(unread.as_bytes()).unwrap());
    assert!(
        w.iter()
            .any(|s| s == "Skipped some settings of the profile."),
        "{w:?}"
    );
}

#[test]
fn less_saturation_leads_to_the_gray_of_the_same_luminance() {
    let at = |saturation: &str| {
        let xmp = packet(&format!(r#"   crs:Saturation="{saturation}""#), "");
        Transform::new(&xmp::parse(xmp.as_bytes()).unwrap())
            .unwrap()
            .apply([0.0, 0.0, 1.0])
    };
    // Pure blue has the luminance 0.0722, which is the display gray 0.30.
    let gray = at("-100");
    assert!((gray[0] - 0.2977).abs() < 2e-3 && gray[0] == gray[1] && gray[1] == gray[2]);
    let near = at("-99");
    for c in 0..3 {
        assert!((near[c] - gray[c]).abs() < 0.02, "{near:?} {gray:?}");
    }
}

#[test]
fn a_hue_band_does_not_change_a_gray() {
    let transform =
        |attrs: &str| Transform::new(&xmp::parse(packet(attrs, "").as_bytes()).unwrap()).unwrap();
    for (plain, band) in [
        (
            r#"   crs:ConvertToGrayscale="True""#,
            r#"   crs:ConvertToGrayscale="True" crs:GrayMixerRed="+100""#,
        ),
        (
            r#"   crs:Version="15.0""#,
            r#"   crs:LuminanceAdjustmentRed="+100""#,
        ),
    ] {
        let (plain, band) = (transform(plain), transform(band));
        // The gain of the band on a color: 1 is no change.
        let gain = |rgb: [f64; 3]| band.apply(rgb)[0] / plain.apply(rgb)[0];
        assert!((gain([0.5, 0.5, 0.5]) - 1.0).abs() < 1e-6);
        // A weak red changes less than a strong red.
        let (weak, strong) = (gain([0.5, 0.45, 0.45]), gain([0.5, 0.1, 0.1]));
        assert!(weak > 1.0 && weak < strong, "{weak} {strong}");
    }
}

#[test]
fn camera_calibration_turns_colors_and_keeps_grays() {
    let xmp = packet(r#"   crs:BlueHue="-45" crs:BlueSaturation="+35""#, "");
    let preset = xmp::parse(xmp.as_bytes()).unwrap();
    let transform = Transform::new(&preset).unwrap();
    let gray = transform.apply([0.4, 0.4, 0.4]);
    assert!(gray.iter().all(|v| (v - 0.4).abs() < 1e-4), "{gray:?}");
    // A negative blue hue moves a sky blue toward cyan: more green, less red.
    let sky = [0.3, 0.5, 0.8];
    let out = transform.apply(sky);
    assert!(out[1] > sky[1] + 0.02 && out[0] < sky[0], "{out:?}");
    assert!(warnings(&preset, &transform).is_empty());

    // The shadow tint has no model: it stays a warning.
    let tint = packet(r#"   crs:ShadowTint="+20""#, "");
    let preset = xmp::parse(tint.as_bytes()).unwrap();
    let w = warnings(&preset, &Transform::new(&preset).unwrap());
    assert_eq!(w, ["Skipped camera calibration."]);
}

#[test]
fn vibrance_protects_skin_hues_and_keeps_grays() {
    let saturation = |c: [f64; 3]| {
        let (max, min) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
        (max - min) / max
    };
    let more = packet(r#"   crs:Vibrance="+100""#, "");
    let more = Transform::new(&xmp::parse(more.as_bytes()).unwrap()).unwrap();
    let gray = more.apply([0.5, 0.5, 0.5]);
    assert!(gray.iter().all(|v| (v - 0.5).abs() < 1e-3), "{gray:?}");
    // An orange of skin gains less saturation than a green of the same saturation.
    let (skin, green) = ([0.8, 0.6, 0.5], [0.5, 0.8, 0.6]);
    let gain = |c: [f64; 3]| saturation(more.apply(c)) / saturation(c);
    assert!(
        gain(skin) > 1.0 && gain(skin) < gain(green),
        "{} {}",
        gain(skin),
        gain(green)
    );

    let less = packet(r#"   crs:Vibrance="-100""#, "");
    let less = Transform::new(&xmp::parse(less.as_bytes()).unwrap()).unwrap();
    assert!(saturation(less.apply(green)) < 0.5 * saturation(green));
}

#[test]
fn mismatched_table_fingerprint_type_is_error() {
    use encode::*;
    let n = 3u32;
    let samples = identity_rgb_samples(n);
    let encoded = encode_table(&rgb_table_bytes(3, n, &samples, 0, 1, 0, 0.0, 2.0));
    let fp = "mismatch000000000000000000000001";
    let xmp = packet(
        &format!(r#"   crs:Version="15.0" crs:LookTable="{fp}" crs:Table_{fp}="{encoded}""#),
        "",
    );
    let err = match Transform::new(&xmp::parse(xmp.as_bytes()).unwrap()) {
        Ok(_) => panic!("expected type mismatch"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("LookTable") && err.contains("RGB"), "{err}");
}
