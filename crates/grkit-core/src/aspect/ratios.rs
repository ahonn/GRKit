//! The catalog of the ratios of CORE.ASPECT: each value with its names, its
//! live-view label, its width : height, how the camera captures it, and the
//! families that offer it.

/// How a computed side of a size is rounded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rounding {
    /// To the nearest multiple of 8, as the releases before 1.6.1 recorded.
    Nearest8,
    /// Down to a multiple of 16, so that the crop stays inside the stock size.
    /// The JPEG codec of the quick view asserts on a width that is not a
    /// multiple of 16 (GR III).
    Down16,
}

impl Rounding {
    /// `numerator / denominator`, rounded.
    pub fn apply(self, numerator: u64, denominator: u64) -> u32 {
        let value = match self {
            Rounding::Nearest8 => (numerator + 4 * denominator) / (8 * denominator) * 8,
            Rounding::Down16 => numerator / (16 * denominator) * 16,
        };
        value as u32
    }
}

/// How a ratio is captured from native 3:2 (`runtime/framing.S`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Geometry {
    /// The native 3:2 capture: Off.
    Native,
    /// The stock sizes of a hidden capture aspect id: 1 (4:3) or 2 (16:9).
    Stock(u32),
    /// Capture aspect id 1 with the native 1:1 sizes, so a photo plays back
    /// as a native one.
    Square,
    /// Capture aspect id 2 with the width of 16:9 and the height of the ratio.
    Wide(Rounding),
    /// Capture aspect id 1 with the height of 4:3 and the width of the ratio.
    Narrow(Rounding),
}

impl Geometry {
    /// The capture aspect id of a shot: 0 3:2, 1 4:3, 2 16:9.
    pub fn capture(self) -> u32 {
        match self {
            Geometry::Native => 0,
            Geometry::Stock(id) => id,
            Geometry::Square | Geometry::Narrow(_) => 1,
            Geometry::Wide(_) => 2,
        }
    }
    /// The capture aspect id of a RAW development: 1:1 develops the native
    /// 1:1 (3).
    pub fn develop(self) -> u32 {
        match self {
            Geometry::Square => 3,
            other => other.capture(),
        }
    }
    /// Whether the runtime switches GetResolution to sizes that no native
    /// aspect has.
    pub fn framed(self) -> bool {
        matches!(self, Geometry::Wide(_) | Geometry::Narrow(_))
    }
    /// Whether it uses capture aspect id 2.
    pub fn wide(self) -> bool {
        matches!(self, Geometry::Stock(2) | Geometry::Wide(_))
    }
}

/// A ratio, by CORE.ASPECT value.
#[derive(Clone, Copy, Debug)]
pub struct Ratio {
    /// The row of the Framing page, the same text in each language. Off has
    /// the text of each language in `modules/framing/text`.
    pub row: &'static str,
    /// The value in the root row, which has less room.
    pub summary: &'static str,
    /// The live-view label (`artwork::OVERLAY_LABELS`).
    pub label: &'static str,
    /// The tile of the Framing module (`modules/framing/tiles`).
    pub tile: &'static str,
    /// Width : height.
    pub aspect: (u32, u32),
    pub geometry: Geometry,
    /// The roundings of the sizes that earlier releases recorded. Playback
    /// still classifies them.
    pub legacy: &'static [Rounding],
    /// The families that offer it (`geometry::Family::name`).
    pub families: &'static [&'static str],
}

const ALL: &[&str] = &["gr3", "gr4"];

/// The ratios by CORE.ASPECT value.
pub const RATIOS: [Ratio; 9] = [
    Ratio {
        row: "Off",
        summary: "Off",
        label: "",
        tile: "0-module.png",
        aspect: (3, 2),
        geometry: Geometry::Native,
        legacy: &[],
        families: ALL,
    },
    Ratio {
        row: "XPan 65:24",
        summary: "XPan",
        label: "XPan",
        tile: "1-xpan.png",
        aspect: (65, 24),
        geometry: Geometry::Wide(Rounding::Nearest8),
        legacy: &[],
        families: ALL,
    },
    Ratio {
        row: "16:9",
        summary: "16:9",
        label: "16:9",
        tile: "2-16-9.png",
        aspect: (16, 9),
        geometry: Geometry::Stock(2),
        legacy: &[],
        families: ALL,
    },
    Ratio {
        row: "4:3",
        summary: "4:3",
        label: "4:3",
        tile: "3-4-3.png",
        aspect: (4, 3),
        geometry: Geometry::Stock(1),
        legacy: &[],
        families: ALL,
    },
    Ratio {
        row: "2.35:1",
        summary: "2.35:1",
        label: "2.35",
        tile: "4-2-35.png",
        aspect: (47, 20),
        geometry: Geometry::Wide(Rounding::Nearest8),
        legacy: &[],
        families: ALL,
    },
    Ratio {
        row: "1:1",
        summary: "1:1",
        label: "1:1",
        tile: "5-1-1.png",
        aspect: (1, 1),
        geometry: Geometry::Square,
        legacy: &[],
        families: ALL,
    },
    Ratio {
        row: "5:4",
        summary: "5:4",
        label: "5:4",
        tile: "6-5-4.png",
        aspect: (5, 4),
        geometry: Geometry::Narrow(Rounding::Down16),
        legacy: &[Rounding::Nearest8],
        families: ALL,
    },
    Ratio {
        row: "7:6",
        summary: "7:6",
        label: "7:6",
        tile: "7-7-6.png",
        aspect: (7, 6),
        geometry: Geometry::Narrow(Rounding::Down16),
        legacy: &[Rounding::Nearest8],
        families: ALL,
    },
    Ratio {
        row: "3:4",
        summary: "3:4",
        label: "3:4",
        tile: "8-3-4.png",
        aspect: (3, 4),
        geometry: Geometry::Narrow(Rounding::Down16),
        legacy: &[],
        families: ALL,
    },
];

/// The values that a family offers, Off first, then from the widest ratio
/// to the narrowest: the rows of the Framing page (FRAMING_RATIO_ROWS).
pub fn rows(family: &str) -> Vec<usize> {
    let mut rows: Vec<usize> = (1..RATIOS.len())
        .filter(|&value| RATIOS[value].families.contains(&family))
        .collect();
    rows.sort_by(|&a, &b| {
        let ((ap, aq), (bp, bq)) = (RATIOS[a].aspect, RATIOS[b].aspect);
        (bp * aq).cmp(&(ap * bq)).then(a.cmp(&b))
    });
    rows.insert(0, 0);
    rows
}

/// The label of each value: the index of its label in
/// `artwork::OVERLAY_LABELS`, 0 for Off.
pub fn label_index(value: usize) -> usize {
    if value == 0 {
        return 0;
    }
    crate::artwork::OVERLAY_LABELS
        .iter()
        .position(|label| *label == RATIOS[value].label)
        .unwrap_or_else(|| panic!("No label {:?}", RATIOS[value].label))
}

/// The ratios of the rows of the family `gr3` after Off, joined as a list in a
/// language: `separator` between two, `last` before the last one.
pub fn names(separator: &str, last: &str) -> String {
    let rows = rows("gr3");
    let names: Vec<&str> = rows[1..]
        .iter()
        .map(|&value| RATIOS[value].summary)
        .collect();
    let (end, rest) = names.split_last().unwrap();
    format!("{}{last}{end}", rest.join(separator))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::Language;
    use std::path::Path;

    fn root() -> &'static Path {
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
    }

    #[test]
    fn rows_go_from_the_widest_ratio_and_list_each_value_once() {
        for family in ["gr3", "gr4"] {
            let rows = rows(family);
            assert_eq!(rows, [0, 1, 4, 2, 3, 6, 7, 5, 8]);
            let mut unique = rows.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(unique.len(), rows.len());
        }
        let mut texts: Vec<&str> = RATIOS.iter().map(|ratio| ratio.row).collect();
        texts.sort();
        texts.dedup();
        assert_eq!(texts.len(), RATIOS.len(), "each row has its own text");
    }

    /// The Framing module has a tile for each ratio, in value order, and its
    /// row and summary in each language. Off has the text of the language.
    #[test]
    fn the_framing_module_lists_each_ratio() {
        let module = root().join("modules/framing");
        let manifest: toml::Table = std::fs::read_to_string(module.join("module.toml"))
            .unwrap()
            .parse()
            .unwrap();
        let tiles: Vec<&str> = manifest["tiles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tile| tile.as_str().unwrap())
            .collect();
        let expected: Vec<String> = RATIOS
            .iter()
            .map(|ratio| format!("tiles/{}", ratio.tile))
            .collect();
        assert_eq!(tiles, expected);
        for language in Language::ALL {
            let text: toml::Table =
                std::fs::read_to_string(module.join(format!("text/{}.toml", language.code())))
                    .unwrap()
                    .parse()
                    .unwrap();
            for key in ["rows", "summaries"] {
                let items: Vec<&str> = text[key]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|item| item.as_str().unwrap())
                    .collect();
                assert_eq!(items.len(), RATIOS.len(), "{key} of {}", language.code());
                for (item, ratio) in items.iter().zip(&RATIOS).skip(1) {
                    let expected = if key == "rows" {
                        ratio.row
                    } else {
                        ratio.summary
                    };
                    assert_eq!(*item, expected, "{key} of {}", language.code());
                }
            }
        }
    }

    /// The docs and the site list the ratios of the Framing page in its order.
    #[test]
    fn docs_list_the_ratios() {
        for (file, list) in [
            ("README.md", names(", ", ", or ")),
            ("README.zh-Hans.md", names("、", " 或 ")),
            ("README.zh-Hant.md", names("、", " 或 ")),
            ("README.ja.md", names("、", "、")),
            ("README.ko.md", names(", ", ", ")),
        ] {
            let text = std::fs::read_to_string(root().join(file)).unwrap();
            assert!(text.contains(&list), "{file} lists {list}");
        }
        let values: Vec<String> = RATIOS
            .iter()
            .enumerate()
            .map(|(value, ratio)| format!("{value} {}", ratio.row))
            .collect();
        let reference = std::fs::read_to_string(root().join("docs/module-reference.md")).unwrap();
        let row = format!(
            "| `CORE.ASPECT` | {}: {} |",
            RATIOS.len(),
            values.join(", ")
        );
        assert!(
            reference.contains(&row),
            "docs/module-reference.md has {row}"
        );
        let sdk = std::fs::read_to_string(root().join("runtime/sdk/grkit.h")).unwrap();
        let sdk: String = sdk.split_whitespace().collect::<Vec<_>>().join(" ");
        let listed = format!(
            "CORE.ASPECT {} values: the composition ratio, {}.",
            RATIOS.len(),
            values.join(", ")
        );
        assert!(sdk.contains(&listed), "runtime/sdk/grkit.h has {listed}");
    }
}
