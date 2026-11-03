//! The native tone curves of GR IV firmware: the base curve that the tuning step builds from
//! the tone steps of a picture, and the per-look curve set that some looks apply on it.
//!
//! The base curve maps the stage input (256 points over 12 bits) to 14-bit codes. The firmware
//! keeps three families of five curves: family 0 has the most contrast, family 1 is the normal
//! one, family 2 the least. The five curves of a family put mid gray at different inputs. The
//! key step selects the input `u` of mid gray, and the curve between the two neighbouring
//! curves of each family is mixed so that it gives mid gray at `u`. Each contrast step mixes two
//! neighbouring families. The shadow and highlight steps change the contrast below and above `u`
//! and ease into the overall curve away from `u`.
//!
//! A curve set gives three curves on the 14-bit base codes. Its weight moves the red and blue
//! curves toward green: `(w g + (3 - w) c) / 3`. Each output color is its curve at the base
//! code, read with a 6-bit fraction.
//!
//! All arithmetic follows the firmware's integer steps, so the results equal the camera tables.
use anyhow::{ensure, Result};

pub const POINTS: usize = 256;
pub type Curve = [u16; POINTS];

/// Tables of the tuning step, read from the firmware.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToneTables {
    /// The input gain of each key step -4..=4, 256 = 1.
    pub gains: [u32; 9],
    /// Three families (most contrast first) of five curves each.
    pub families: [[Curve; 5]; 3],
    /// The ease weights (2048 = 1) of the shadow and highlight steps.
    pub ease: [u16; 97],
    /// Mid gray is `118 * unit` in curve codes; `unit / 64` scales the output.
    pub unit: u16,
}

/// The tone steps of a picture, each -4..=4, and the scale of the stage input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToneSteps {
    pub key: i32,
    pub contrast: i32,
    pub shadow: i32,
    pub highlight: i32,
    /// 8192 = 1. The camera computes it for each picture from the sensor white level, the
    /// smallest white balance gain, and the smallest shading gain, so it changes with the
    /// scene. GR III uses about twice the GR IV value for the same scene.
    pub input_scale: u32,
}

/// A per-look curve set. Disabled, the base curve applies to each color.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurveSet {
    pub enabled: bool,
    pub weight: u8,
    /// Red, green, and blue.
    pub curves: [Curve; 3],
}

/// One contrast step: the two families and their weights (512 = 1).
#[derive(Clone, Copy)]
struct Mix {
    first: usize,
    second: usize,
    first_weight: u32,
    second_weight: u32,
}

impl Mix {
    fn new(step: i32) -> Self {
        let (first, second, first_weight) = match step {
            s if s < -12 => (1, 2, 0),
            s if s > 12 => (0, 1, 512),
            s if s >= 1 => (0, 1, (s * 512 / 12) as u32),
            s => (1, 2, ((s + 12) * 512 / 12) as u32),
        };
        Self {
            first,
            second,
            first_weight,
            second_weight: 512 - first_weight,
        }
    }

    fn at(&self, tables: &ToneTables, curve: usize, i: usize) -> u32 {
        let a = u32::from(tables.families[self.first][curve][i]);
        let b = u32::from(tables.families[self.second][curve][i]);
        (self.first_weight * a + self.second_weight * b + 256) >> 9
    }
}

impl ToneTables {
    /// Checks the shape that `base_curve` relies on.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.gains.iter().all(|&g| (1..=1024).contains(&g)),
            "Unexpected tone gains"
        );
        ensure!(
            self.unit >= 64 && self.unit.is_multiple_of(64),
            "Unexpected tone unit"
        );
        ensure!(
            self.ease.windows(2).all(|pair| pair[0] <= pair[1]) && self.ease[96] <= 2048,
            "Unexpected tone ease table"
        );
        for curve in self.families.iter().flatten() {
            ensure!(
                curve.windows(2).all(|pair| pair[0] <= pair[1]) && curve[255] <= 0x4000,
                "Unexpected tone curve"
            );
        }
        Ok(())
    }

    /// The base curve: 256 codes of 14 bits (16384 at most) for the stage input `16 i`.
    pub fn base_curve(&self, steps: &ToneSteps) -> Curve {
        let gain = (self.gains[(steps.key.clamp(-4, 4) + 4) as usize] * 0x200) >> 8;
        let u = steps.input_scale.wrapping_mul(gain) >> 13;
        // The two neighbouring curves of each family, and the point of mid gray.
        let (u, sample, low, high) = match u {
            u if u < 0x80 => (0x80, 8, 0, 1),
            u if u > 0x800 => (0x800, 128, 3, 4),
            u => {
                let (low, high) = match u {
                    u if u >= 0x400 => (3, 4),
                    u if u >= 0x200 => (2, 3),
                    u if u > 0xff => (1, 2),
                    _ => (0, 1),
                };
                (u, (u >> 4) as usize, low, high)
            }
        };
        let contrast = steps.contrast * 2;
        // Highlights, the whole curve, and shadows.
        let mixes = [
            Mix::new(contrast + steps.highlight),
            Mix::new(contrast),
            Mix::new(contrast + steps.shadow),
        ];
        let mid = u32::from(self.unit) * 118;
        // The weight of the low curve (512 = 1) that puts mid gray at `u`.
        let low_weight = mixes.map(|mix| {
            let l = mix.at(self, low, sample);
            let h = mix.at(self, high, sample);
            if l <= h {
                512
            } else if h >= mid {
                0
            } else {
                ((mid - h) << 9) / (l - h)
            }
        });
        let curve = |k: usize, i: usize| {
            let l = mixes[k].at(self, low, i);
            let h = mixes[k].at(self, high, i);
            (low_weight[k] * l + (512 - low_weight[k]) * h + 256) >> 9
        };
        let ease = |ratio: u32| -> u32 {
            u32::from(self.ease[ratio.saturating_sub(0x20).min(0x60) as usize])
        };
        let divisor = 0x3f02 * u32::from(self.unit >> 6);
        std::array::from_fn(|i| {
            let input = 16 * i as u32;
            let side = curve(if u >= input { 2 } else { 0 }, i);
            let mut value = curve(1, i);
            if u < input && steps.highlight != 0 {
                value = if input >= u << 2 {
                    side
                } else {
                    let w = ease(((2 * i as u32) << 8) / u);
                    (value * (0x800 - w) + side * w + 0x400) >> 11
                };
            }
            if u > input && steps.shadow != 0 {
                value = if input <= u >> 2 {
                    side
                } else {
                    let w = ease((u << 5) / input);
                    (value * (0x800 - w) + side * w + 0x400) >> 11
                };
            }
            ((value << 14) / divisor).min(0x4000) as u16
        })
    }
}

impl CurveSet {
    /// The red, green, and blue curves after the weight moves red and blue toward green.
    /// They apply to the base codes as a camera applies its own record curves.
    pub fn mixed(&self) -> [Curve; 3] {
        let weight = u32::from(self.weight.min(3));
        let [red, green, blue] = &self.curves;
        [
            mix_toward(red, green, weight),
            *green,
            mix_toward(blue, green, weight),
        ]
    }

    /// The red, green, and blue curves of `base` through this set.
    pub fn apply(&self, base: &Curve) -> [Curve; 3] {
        if !self.enabled {
            return [*base; 3];
        }
        self.mixed()
            .map(|curve| base.map(|code| lookup(&curve, code)))
    }
}

fn mix_toward(curve: &Curve, green: &Curve, weight: u32) -> Curve {
    std::array::from_fn(|i| {
        ((weight * u32::from(green[i]) + (3 - weight) * u32::from(curve[i])) / 3) as u16
    })
}

/// `curve` at a 14-bit code: point `code / 64`, then the step to the next point times the
/// 6-bit fraction, rounded toward zero. Codes past point 254 give the last point.
fn lookup(curve: &Curve, code: u16) -> u16 {
    let i = usize::from(code >> 6);
    if i > 254 {
        return curve[255];
    }
    let step = (i32::from(curve[i + 1]) - i32::from(curve[i])) * i32::from(code & 63);
    (i32::from(curve[i]) + step / 64) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A family curve that reaches mid gray (7552) at point `center` and `top` at point 255.
    /// `power` bends both sides; a larger power gives more contrast.
    fn line(center: usize, top: u32, power: f64) -> Curve {
        std::array::from_fn(|i| {
            let (i, c) = (i as f64, center as f64);
            (if i <= c {
                7552.0 * (i / c).powf(power)
            } else {
                let t = (i - c) / (255.0 - c);
                7552.0 + (f64::from(top) - 7552.0) * (1.0 - (1.0 - t).powf(power))
            })
            .round() as u16
        })
    }

    fn tables() -> ToneTables {
        let family = |top, power| [8, 16, 32, 64, 128].map(|c| line(c, top, power));
        ToneTables {
            gains: [512, 431, 362, 304, 256, 215, 181, 152, 128],
            families: [family(16384, 1.6), family(16130, 1.2), family(15000, 0.8)],
            ease: std::array::from_fn(|i| (i as u32 * 2048 / 96) as u16),
            unit: 64,
        }
    }

    fn steps() -> ToneSteps {
        ToneSteps {
            key: 0,
            contrast: 0,
            shadow: 0,
            highlight: 0,
            input_scale: 8192,
        }
    }

    #[test]
    fn neutral_steps_put_mid_gray_at_the_key_input() {
        let t = tables();
        t.validate().unwrap();
        // Key 0: gain 512, u = 512, point 32 is the center of curve 2 (low of 2..3).
        let curve = t.base_curve(&steps());
        assert_eq!(u32::from(curve[32]), 7552 * 16384 / 16130);
        assert!(curve.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(curve[0], 0);
    }

    #[test]
    fn key_moves_mid_gray() {
        let t = tables();
        let darker = t.base_curve(&ToneSteps { key: -4, ..steps() });
        let lighter = t.base_curve(&ToneSteps { key: 4, ..steps() });
        let normal = t.base_curve(&steps());
        // Key -4 doubles the gain: mid gray at point 64.
        assert_eq!(u32::from(darker[64]), 7552 * 16384 / 16130);
        assert!(darker[32] < normal[32] && normal[32] < lighter[32]);
    }

    #[test]
    fn shadow_and_highlight_steps_act_on_their_side_only() {
        let t = tables();
        let normal = t.base_curve(&steps());
        let shadow = t.base_curve(&ToneSteps {
            shadow: 4,
            ..steps()
        });
        let highlight = t.base_curve(&ToneSteps {
            highlight: 4,
            ..steps()
        });
        assert_eq!(shadow[32..], normal[32..]);
        assert_ne!(shadow[..32], normal[..32]);
        assert_eq!(highlight[..=32], normal[..=32]);
        assert_ne!(highlight[33..], normal[33..]);
    }

    #[test]
    fn disabled_set_repeats_the_base() {
        let base = tables().base_curve(&steps());
        let set = CurveSet {
            enabled: false,
            weight: 1,
            curves: [line(64, 16383, 1.0); 3],
        };
        assert_eq!(set.apply(&base), [base; 3]);
    }

    #[test]
    fn set_mixes_red_and_blue_toward_green() {
        let identity: Curve = std::array::from_fn(|i| (i * 64) as u16);
        let red: Curve = [3000; POINTS];
        let green: Curve = [6000; POINTS];
        let blue: Curve = [9000; POINTS];
        let base: Curve = std::array::from_fn(|i| (i * 64) as u16);
        let mut set = CurveSet {
            enabled: true,
            weight: 0,
            curves: [red, green, blue],
        };
        assert_eq!(set.apply(&base), [red, green, blue]);
        set.weight = 1;
        assert_eq!(set.apply(&base), [[4000; POINTS], green, [8000; POINTS]]);
        set.curves = [identity; 3];
        assert_eq!(set.apply(&base)[0], identity);
    }

    #[test]
    fn lookup_rounds_the_fraction_toward_zero() {
        let mut curve: Curve = [0; POINTS];
        curve[1] = 100;
        curve[2] = 0;
        assert_eq!(lookup(&curve, 32), 50);
        assert_eq!(lookup(&curve, 64 + 1), 99);
        assert_eq!(lookup(&curve, 64 + 63), 2);
        curve[255] = 7;
        assert_eq!(lookup(&curve, 0x4000), 7);
    }
}
