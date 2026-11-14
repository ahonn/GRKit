//! The sizes of the ratios: the stock sizes of each camera family, the size
//! of a ratio in each size class, the geometries that the runtime switches
//! GetResolution to, the classification of a photo size, and the preview
//! insets and live-view boxes.

use super::ratios::{Geometry, RATIOS};

/// The size classes of GetResolution: L, M, S, and XS.
pub const CLASSES: usize = 4;

/// The stock sizes of a camera family: the immediates of GetResolution of
/// its firmware (grkit-build `framing_family_sizes_are_those_of_each_firmware`).
#[derive(Clone, Copy, Debug)]
pub struct Family {
    pub name: &'static str,
    /// The width of 3:2 and 16:9 in each size class.
    pub widths: [u32; CLASSES],
    /// The height of 3:2.
    pub heights_3x2: [u32; CLASSES],
    /// The height of 16:9 (capture aspect id 2).
    pub heights_16x9: [u32; CLASSES],
    /// The width and the height of 4:3 (capture aspect id 1).
    pub widths_4x3: [u32; CLASSES],
    pub heights_4x3: [u32; CLASSES],
    /// The side of native 1:1. Only the XS side differs from the 4:3 height:
    /// the runtime switches only the XS height (FRAMING_NARROW_HEIGHT_3).
    pub sides_1x1: [u32; CLASSES],
}

pub const FAMILIES: [Family; 2] = [
    Family {
        name: "gr3",
        widths: [6000, 4800, 3360, 1920],
        heights_3x2: [4000, 3200, 2240, 1280],
        heights_16x9: [3376, 2704, 1888, 1080],
        widths_4x3: [5328, 4272, 2992, 1920],
        heights_4x3: [4000, 3200, 2240, 1440],
        sides_1x1: [4000, 3200, 2240, 1280],
    },
    Family {
        name: "gr4",
        widths: [6192, 4944, 3504, 1920],
        heights_3x2: [4128, 3296, 2336, 1280],
        heights_16x9: [3480, 2784, 1968, 1080],
        widths_4x3: [5504, 4400, 3120, 1920],
        heights_4x3: [4128, 3296, 2336, 1440],
        sides_1x1: [4128, 3296, 2336, 1280],
    },
];

pub fn family(name: &str) -> &'static Family {
    FAMILIES
        .iter()
        .find(|family| family.name == name)
        .unwrap_or_else(|| panic!("No family {name}"))
}

/// The size {width, height} of ratio `value` with `geometry` (its own, or a
/// geometry of an earlier release) in size class `class` of `family`. A wide
/// ratio takes the height from the width, a narrow one the width from the
/// height.
pub fn size(family: &Family, value: usize, geometry: Geometry, class: usize) -> (u32, u32) {
    let (p, q) = RATIOS[value].aspect;
    let (p, q) = (u64::from(p), u64::from(q));
    match geometry {
        Geometry::Native => (family.widths[class], family.heights_3x2[class]),
        Geometry::Stock(2) => (family.widths[class], family.heights_16x9[class]),
        Geometry::Stock(_) => (family.widths_4x3[class], family.heights_4x3[class]),
        Geometry::Square => (family.sides_1x1[class], family.sides_1x1[class]),
        Geometry::Wide(rounding) => (
            family.widths[class],
            rounding.apply(u64::from(family.widths[class]) * q, p),
        ),
        Geometry::Narrow(rounding) => (
            rounding.apply(u64::from(family.heights_4x3[class]) * p, q),
            family.heights_4x3[class],
        ),
    }
}

/// A geometry that GetResolution switches to, of ratio `value`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    pub value: usize,
    pub geometry: Geometry,
    /// A size of an earlier release, which only playback takes.
    pub legacy: bool,
}

/// The geometries of the sizes that no native aspect has: the ratios in
/// value order, then the sizes of earlier releases.
pub fn shapes() -> Vec<Shape> {
    let mut shapes: Vec<Shape> = RATIOS
        .iter()
        .enumerate()
        .filter(|(_, ratio)| ratio.geometry.framed())
        .map(|(value, ratio)| Shape {
            value,
            geometry: ratio.geometry,
            legacy: false,
        })
        .collect();
    for (value, ratio) in RATIOS.iter().enumerate() {
        for &rounding in ratio.legacy {
            let geometry = match ratio.geometry {
                Geometry::Wide(_) => Geometry::Wide(rounding),
                _ => Geometry::Narrow(rounding),
            };
            shapes.push(Shape {
                value,
                geometry,
                legacy: true,
            });
        }
    }
    shapes
}

/// The geometry of a photo (GRKIT_HOST_FRAME_FILE): 0 while shooting, 1 for
/// the stock sizes, then 2 + n for shape n.
pub fn files() -> Vec<Option<Shape>> {
    let mut files = vec![None, None];
    files.extend(shapes().into_iter().map(Some));
    files
}

/// The file of ratio `value` itself, or 1 for a ratio with stock sizes.
pub fn file_of(value: usize) -> u32 {
    files()
        .iter()
        .position(|file| matches!(file, Some(shape) if shape.value == value && !shape.legacy))
        .unwrap_or(1) as u32
}

/// The modes of capture aspect id 2 (`wide`) or 1: the stock geometry first,
/// then each geometry of that id, the ratios in value order, then the sizes
/// of earlier releases.
pub fn modes(wide: bool) -> Vec<(usize, Geometry)> {
    let mut modes = vec![(0, Geometry::Stock(if wide { 2 } else { 1 }))];
    for (value, ratio) in RATIOS.iter().enumerate() {
        let switched = matches!(
            ratio.geometry,
            Geometry::Square | Geometry::Wide(_) | Geometry::Narrow(_)
        );
        if switched && ratio.geometry.wide() == wide {
            modes.push((value, ratio.geometry));
        }
    }
    modes.extend(
        shapes()
            .into_iter()
            .filter(|shape| shape.legacy && shape.geometry.wide() == wide)
            .map(|shape| (shape.value, shape.geometry)),
    );
    modes
}

/// The mode of ratio `value` with `geometry`, or 0 (stock).
pub fn mode_of(wide: bool, value: usize, geometry: Geometry) -> usize {
    modes(wide)
        .iter()
        .position(|&mode| mode == (value, geometry))
        .unwrap_or(0)
}

/// A Framing size: its file (GRKIT_HOST_FRAME_FILE), its geometry, and its
/// size class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Classified {
    pub file: u32,
    pub shape: Shape,
    pub class: usize,
}

/// The sizes {long side, short side} of each size class of each file from 2,
/// which the playback classifier compares: a size matches in either
/// orientation (runtime/framing.c: framing_size_file).
pub fn classified(family: &Family) -> Vec<(u32, u32)> {
    let mut sizes = Vec::new();
    for shape in shapes() {
        for class in 0..CLASSES {
            let (w, h) = size(family, shape.value, shape.geometry, class);
            sizes.push((w.max(h), w.min(h)));
        }
    }
    sizes
}

/// The Framing geometry of a photo size of `family`, as the playback
/// classifier finds it, or None for another size.
pub fn classify(family: &Family, width: u32, height: u32) -> Option<Classified> {
    let key = (width.max(height), width.min(height));
    let index = classified(family).iter().position(|&size| size == key)?;
    Some(Classified {
        file: (index / CLASSES + 2) as u32,
        shape: shapes()[index / CLASSES],
        class: index % CLASSES,
    })
}

/// The previews of a JPEG, with the insets of the native areas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preview {
    /// 720 x 480.
    Screennail,
    /// 160 x 120.
    Thumbnail,
}

impl Preview {
    pub fn size(self) -> (u32, u32) {
        match self {
            Preview::Screennail => (720, 480),
            Preview::Thumbnail => (160, 120),
        }
    }
    /// The insets {left, top} of the native areas 3:2, 4:3, 16:9, and 1:1:
    /// those of the camera.
    pub fn native(self) -> [(u32, u32); 4] {
        match self {
            Preview::Screennail => [(0, 0), (40, 0), (0, 38), (120, 0)],
            Preview::Thumbnail => [(0, 8), (0, 0), (0, 16), (20, 0)],
        }
    }
}

/// The visible-area id of a picture (`runtime/picture.c`): 0 3:2, 1 4:3,
/// 2 16:9, 3 1:1, then 4 + n for the shape n of a ratio.
pub fn area_of_value(value: usize) -> usize {
    match RATIOS[value].geometry {
        Geometry::Native => 0,
        Geometry::Stock(1) => 1,
        Geometry::Stock(_) => 2,
        Geometry::Square => 3,
        _ => {
            4 + shapes()
                .iter()
                .position(|shape| shape.value == value && !shape.legacy)
                .unwrap()
        }
    }
}

/// The number of visible areas: the native ones, then each current shape.
pub fn areas() -> usize {
    4 + shapes().iter().filter(|shape| !shape.legacy).count()
}

/// The inset of ratio `aspect` in a `w` x `h` preview. A wide ratio has bars
/// above and below, rounded to the nearest pixel. A narrow one has bars at
/// the sides, rounded up to an even column: the IIP stores a picture with
/// chroma pairs only from an even column.
fn inset(aspect: (u32, u32), w: u32, h: u32) -> (u32, u32) {
    let (p, q) = (u64::from(aspect.0), u64::from(aspect.1));
    let (w, h) = (u64::from(w), u64::from(h));
    if p * h >= q * w {
        // (h - w q / p) / 2, to the nearest.
        let numerator = h * p - w * q;
        (0, ((numerator + p) / (2 * p)) as u32)
    } else {
        // (w - h p / q) / 2, up to an even number.
        let numerator = w * q - h * p;
        ((numerator.div_ceil(4 * q) * 2) as u32, 0)
    }
}

/// The insets {left, top} of visible area `area` in `preview`.
pub fn area_inset(area: usize, preview: Preview) -> (u32, u32) {
    if area < 4 {
        return preview.native()[area];
    }
    let shape = shapes()
        .into_iter()
        .filter(|shape| !shape.legacy)
        .nth(area - 4)
        .unwrap();
    let (w, h) = preview.size();
    inset(RATIOS[shape.value].aspect, w, h)
}

/// The box {left, top, right, bottom} of each ratio on the 720 x 480 live
/// view: the full width or height, the other side rounded to the nearest
/// pixel and centered.
pub fn photo_box(value: usize) -> [u32; 4] {
    if RATIOS[value].geometry == Geometry::Native {
        return [0, 0, 720, 480];
    }
    let (p, q) = RATIOS[value].aspect;
    if p * 480 >= q * 720 {
        let h = (720 * q + p / 2) / p;
        let top = (480 - h) / 2;
        [0, top, 720, top + h]
    } else {
        let w = (480 * p + q / 2) / q;
        let left = (720 - w) / 2;
        [left, 0, left + w, 480]
    }
}

#[cfg(test)]
mod tests {
    use super::super::ratios::Rounding;
    use super::*;

    fn sizes(family: &str, value: usize, geometry: Geometry) -> Vec<(u32, u32)> {
        (0..CLASSES)
            .map(|class| size(super::family(family), value, geometry, class))
            .collect()
    }

    /// The sizes that the releases before 1.6.1 recorded stay: XPan and
    /// 2.35:1 as they were, 5:4 (from 1.5.0) and 7:6 (from 1.6.0) as their
    /// playback geometries.
    #[test]
    fn released_sizes_stay() {
        let earlier = Geometry::Narrow(Rounding::Nearest8);
        for (family, xpan, scope, five, seven) in [
            (
                "gr3",
                [(6000, 2216), (4800, 1776), (3360, 1240), (1920, 712)],
                [(6000, 2552), (4800, 2040), (3360, 1432), (1920, 816)],
                [(5000, 4000), (4000, 3200), (2800, 2240), (1800, 1440)],
                [(4664, 4000), (3736, 3200), (2616, 2240), (1680, 1440)],
            ),
            (
                "gr4",
                [(6192, 2288), (4944, 1824), (3504, 1296), (1920, 712)],
                [(6192, 2632), (4944, 2104), (3504, 1488), (1920, 816)],
                [(5160, 4128), (4120, 3296), (2920, 2336), (1800, 1440)],
                [(4816, 4128), (3848, 3296), (2728, 2336), (1680, 1440)],
            ),
        ] {
            assert_eq!(sizes(family, 1, RATIOS[1].geometry), xpan);
            assert_eq!(sizes(family, 4, RATIOS[4].geometry), scope);
            assert_eq!(sizes(family, 6, earlier), five);
            assert_eq!(sizes(family, 7, earlier), seven);
        }
    }

    /// The sizes of 5:4, 7:6, and 3:4 from 1.6.1: widths that are multiples
    /// of 16, inside the 4:3 width.
    #[test]
    fn narrow_sizes_are_multiples_of_16() {
        for (family, five, seven, three) in [
            (
                "gr3",
                [(4992, 4000), (4000, 3200), (2800, 2240), (1792, 1440)],
                [(4656, 4000), (3728, 3200), (2608, 2240), (1680, 1440)],
                [(2992, 4000), (2400, 3200), (1680, 2240), (1072, 1440)],
            ),
            (
                "gr4",
                [(5152, 4128), (4112, 3296), (2912, 2336), (1792, 1440)],
                [(4816, 4128), (3840, 3296), (2720, 2336), (1680, 1440)],
                [(3088, 4128), (2464, 3296), (1744, 2336), (1072, 1440)],
            ),
        ] {
            assert_eq!(sizes(family, 6, RATIOS[6].geometry), five);
            assert_eq!(sizes(family, 7, RATIOS[7].geometry), seven);
            assert_eq!(sizes(family, 8, RATIOS[8].geometry), three);
        }
        for family in &FAMILIES {
            for shape in shapes().into_iter().filter(|shape| !shape.legacy) {
                if let Geometry::Narrow(Rounding::Down16) = shape.geometry {
                    for class in 0..CLASSES {
                        let (w, _) = size(family, shape.value, shape.geometry, class);
                        assert!(w % 16 == 0 && w <= family.widths_4x3[class], "{w}");
                    }
                }
            }
        }
    }

    /// The playback classifier tells each Framing size apart: no size of one
    /// geometry is a size of another, except an earlier size that a release
    /// still records, and none is a native size.
    #[test]
    fn classified_sizes_are_unique_and_not_native() {
        for family in &FAMILIES {
            let shapes = shapes();
            let sizes = classified(family);
            let mut native = Vec::new();
            for class in 0..CLASSES {
                for (w, h) in [
                    (family.widths[class], family.heights_3x2[class]),
                    (family.widths[class], family.heights_16x9[class]),
                    (family.widths_4x3[class], family.heights_4x3[class]),
                    (family.sides_1x1[class], family.sides_1x1[class]),
                ] {
                    native.push((w.max(h), w.min(h)));
                }
            }
            for (i, size) in sizes.iter().enumerate() {
                assert!(!native.contains(size), "{} {size:?}", family.name);
                for (j, other) in sizes.iter().enumerate().take(i) {
                    if size == other {
                        let (a, b) = (shapes[i / CLASSES], shapes[j / CLASSES]);
                        assert!(a.value == b.value && a.legacy, "{} {size:?}", family.name);
                    }
                }
            }
            // The runtime switches only the XS height of id 1.
            assert_eq!(family.sides_1x1[..3], family.heights_4x3[..3]);
        }
    }

    /// `classify` finds each size of each geometry in either orientation;
    /// an earlier size equal to a current one is the current geometry.
    #[test]
    fn classify_finds_each_size() {
        for family in &FAMILIES {
            for (n, shape) in shapes().into_iter().enumerate() {
                for class in 0..CLASSES {
                    let (w, h) = size(family, shape.value, shape.geometry, class);
                    for (w, h) in [(w, h), (h, w)] {
                        let found = classify(family, w, h).unwrap();
                        assert_eq!(found.class, class);
                        assert_eq!(found.shape.value, shape.value);
                        if found.shape != shape {
                            assert!(shape.legacy && !found.shape.legacy);
                        } else {
                            assert_eq!(found.file, n as u32 + 2);
                        }
                    }
                }
            }
            assert_eq!(
                classify(family, family.widths[0], family.heights_3x2[0]),
                None
            );
        }
    }

    #[test]
    fn released_boxes_stay() {
        let boxes: Vec<[u32; 4]> = (0..RATIOS.len()).map(photo_box).collect();
        assert_eq!(
            boxes[..8],
            [
                [0, 0, 720, 480],
                [0, 107, 720, 373],
                [0, 37, 720, 442],
                [40, 0, 680, 480],
                [0, 87, 720, 393],
                [120, 0, 600, 480],
                [60, 0, 660, 480],
                [80, 0, 640, 480],
            ]
        );
    }
}
