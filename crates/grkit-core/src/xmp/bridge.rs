//! Tone bridge from the camera Standard JPEG rendering toward a reference raw default.
//!
//! Lightroom presets are built on a raw converter's default rendering. On the camera the LUT
//! sits on Ricoh Standard. This module holds a global luminance curve that moves Standard
//! display values toward that reference default, fitted on six GR III / GR IV Standard camera
//! JPEGs against the same DNGs developed with a permissive raw converter. A leave-one-out check
//! cut the mean CIEDE2000 gap from about 5.2 to about 3.6. A color matrix on top did not help.
//! Per-photo differences (dynamic range, white balance, noise reduction) remain.
//!
//! The curve is pinned to (0, 0) and (1, 1) so the LUT still covers the full display range.
//! It acts as a luminance gain: linear RGB is scaled by
//! `decode(curve(encode(Y))) / Y`, which keeps chromaticity.

use crate::color::{srgb_decode, srgb_encode, Rgb};

const SAMPLES: usize = 256;

/// Display luminance in → out, 256 even steps from 0 to 1. Fitted as above; ends are identity.
#[rustfmt::skip]
const CURVE: [f64; SAMPLES] = [
    0.00000, 0.00295, 0.00590, 0.00884, 0.01179, 0.01474, 0.01769, 0.02064,
    0.02359, 0.02683, 0.03006, 0.03329, 0.03652, 0.03975, 0.04299, 0.04622,
    0.04937, 0.05137, 0.05337, 0.05537, 0.05737, 0.05936, 0.06136, 0.06336,
    0.06544, 0.06831, 0.07117, 0.07404, 0.07691, 0.07978, 0.08264, 0.08551,
    0.08836, 0.09104, 0.09373, 0.09642, 0.09910, 0.10179, 0.10447, 0.10716,
    0.10995, 0.11327, 0.11660, 0.11992, 0.12325, 0.12658, 0.12990, 0.13323,
    0.13667, 0.14064, 0.14461, 0.14857, 0.15254, 0.15651, 0.16047, 0.16444,
    0.16835, 0.17208, 0.17581, 0.17954, 0.18327, 0.18700, 0.19073, 0.19446,
    0.19819, 0.20195, 0.20570, 0.20946, 0.21321, 0.21697, 0.22072, 0.22448,
    0.22809, 0.23132, 0.23455, 0.23778, 0.24101, 0.24425, 0.24748, 0.25071,
    0.25424, 0.25842, 0.26260, 0.26679, 0.27097, 0.27515, 0.27933, 0.28352,
    0.28750, 0.29111, 0.29472, 0.29833, 0.30194, 0.30555, 0.30916, 0.31277,
    0.31610, 0.31896, 0.32182, 0.32468, 0.32755, 0.33041, 0.33327, 0.33614,
    0.33973, 0.34441, 0.34908, 0.35376, 0.35843, 0.36310, 0.36778, 0.37245,
    0.37690, 0.38106, 0.38522, 0.38938, 0.39354, 0.39770, 0.40186, 0.40601,
    0.41055, 0.41550, 0.42045, 0.42541, 0.43036, 0.43532, 0.44027, 0.44523,
    0.44995, 0.45444, 0.45893, 0.46341, 0.46790, 0.47239, 0.47688, 0.48137,
    0.48599, 0.49071, 0.49544, 0.50017, 0.50489, 0.50962, 0.51435, 0.51907,
    0.52389, 0.52876, 0.53364, 0.53852, 0.54340, 0.54827, 0.55315, 0.55803,
    0.56345, 0.56924, 0.57503, 0.58082, 0.58662, 0.59241, 0.59820, 0.60399,
    0.60886, 0.61317, 0.61748, 0.62180, 0.62611, 0.63042, 0.63473, 0.63905,
    0.64340, 0.64778, 0.65216, 0.65654, 0.66092, 0.66530, 0.66968, 0.67406,
    0.67776, 0.68116, 0.68455, 0.68794, 0.69133, 0.69473, 0.69812, 0.70151,
    0.70586, 0.71058, 0.71530, 0.72002, 0.72474, 0.72947, 0.73419, 0.73891,
    0.74338, 0.74778, 0.75217, 0.75657, 0.76096, 0.76535, 0.76975, 0.77414,
    0.77824, 0.78226, 0.78628, 0.79030, 0.79433, 0.79835, 0.80237, 0.80639,
    0.80841, 0.80997, 0.81153, 0.81308, 0.81464, 0.81620, 0.81776, 0.81932,
    0.82172, 0.82427, 0.82682, 0.82937, 0.83193, 0.83448, 0.83703, 0.83958,
    0.84209, 0.84460, 0.84710, 0.84960, 0.85211, 0.85461, 0.85711, 0.85962,
    0.86407, 0.86874, 0.87340, 0.87806, 0.88272, 0.88738, 0.89204, 0.89670,
    0.89736, 0.89776, 0.89815, 0.89855, 0.89894, 0.89934, 0.89973, 0.90013,
    0.91228, 0.92481, 0.93734, 0.94987, 0.96241, 0.97494, 0.98747, 1.00000,
];

/// Warning added when the bridge is composed into a preset LUT.
pub const WARNING: &str = "The look includes a tone bridge from the camera Standard rendering \
    toward a reference raw default, not Adobe Color.";

/// Maps a display sRGB color through the base bridge (luminance gain).
pub fn apply(display: Rgb) -> Rgb {
    let linear = display.map(|v| srgb_decode(v.clamp(0.0, 1.0)));
    let luminance = 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
    if luminance <= 1e-9 {
        let y = srgb_decode(eval(0.0));
        return [srgb_encode(y); 3];
    }
    let gain = srgb_decode(eval(srgb_encode(luminance))) / luminance;
    linear.map(|v| srgb_encode((v * gain).clamp(0.0, 1.0)))
}

fn eval(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0) * (SAMPLES - 1) as f64;
    let i = (x as usize).min(SAMPLES - 2);
    let t = x - i as f64;
    CURVE[i] * (1.0 - t) + CURVE[i + 1] * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_ends_are_identity_and_monotonic() {
        assert!((CURVE[0] - 0.0).abs() < 1e-12);
        assert!((CURVE[SAMPLES - 1] - 1.0).abs() < 1e-12);
        for window in CURVE.windows(2) {
            assert!(window[1] + 1e-12 >= window[0]);
        }
    }

    #[test]
    fn apply_keeps_black_and_white() {
        let black = apply([0.0, 0.0, 0.0]);
        assert!(black.iter().all(|v| *v < 1e-6), "{black:?}");
        let white = apply([1.0, 1.0, 1.0]);
        assert!(white.iter().all(|v| (*v - 1.0).abs() < 1e-6), "{white:?}");
    }

    #[test]
    fn mid_gray_moves() {
        let mid = apply([0.5, 0.5, 0.5]);
        // The bridge is not the identity in the middle of the range.
        assert!((mid[0] - 0.5).abs() > 0.01, "{mid:?}");
        assert!((mid[0] - mid[1]).abs() < 1e-9 && (mid[1] - mid[2]).abs() < 1e-9);
    }
}
