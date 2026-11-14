//! `runtime/framing_ratios.h`: the tables of the ratios and the sizes of each
//! family for the runtime, from the catalog and the geometry.

use std::fmt::Write;

use super::geometry::{
    area_inset, area_of_value, areas, classified, file_of, file_ratios, files, mode_of, modes,
    photo_box, size, Preview, Shape, CLASSES, CROP_SHORTEST, CROP_SLACK, FAMILIES, NATIVE_CROPS,
};
use super::ratios::{label_index, rows, Geometry, RATIOS};
use crate::abi::comment;

/// The path of the header from the repository root.
pub const PATH: &str = "runtime/framing_ratios.h";

fn list<T: std::fmt::Display>(items: impl IntoIterator<Item = T>) -> String {
    items
        .into_iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// The name of a geometry in a comment.
fn name(value: usize, geometry: Geometry) -> String {
    let earlier = RATIOS[value].geometry != geometry;
    format!(
        "{}{}",
        RATIOS[value].row,
        if earlier { " before 1.6.1" } else { "" }
    )
}

pub fn header() -> String {
    let mut out = String::new();
    comment(
        &mut out,
        "Generated from crates/grkit-core/src/aspect/. Do not edit: change the \
         tables there, then run GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core \
         --test generated.",
    );
    out.push_str("#ifndef GRKIT_FRAMING_RATIOS_H\n#define GRKIT_FRAMING_RATIOS_H\n\n");
    ratio_tables(&mut out);
    file_tables(&mut out);
    out.push('\n');
    for (index, family) in FAMILIES.iter().enumerate() {
        writeln!(
            out,
            "#{} defined(FRAMING_FAMILY_{})",
            if index == 0 { "if" } else { "elif" },
            family.name.to_uppercase()
        )
        .unwrap();
        out.push_str("/* Off, then the values of the rows, from the widest ratio. */\n");
        writeln!(
            out,
            "#define FRAMING_RATIO_ROWS {}",
            list(rows(family.name))
        )
        .unwrap();
        out.push_str(
            "/* GetResolution of each size class, L to XS, by mode: the height of id 2,\n   \
             the width of id 1, and the XS height of id 1. */\n",
        );
        let by_mode = |wide: bool, class: usize, height: bool| {
            list(modes(wide).iter().map(|&(value, geometry)| {
                let (w, h) = size(family, value, geometry, class);
                if height {
                    h
                } else {
                    w
                }
            }))
        };
        for class in 0..CLASSES {
            let line = by_mode(true, class, true);
            writeln!(out, "#define FRAMING_WIDE_HEIGHT_{class} {line}").unwrap();
        }
        for class in 0..CLASSES {
            let line = by_mode(false, class, false);
            writeln!(out, "#define FRAMING_NARROW_WIDTH_{class} {line}").unwrap();
        }
        let line = by_mode(false, CLASSES - 1, true);
        writeln!(out, "#define FRAMING_NARROW_HEIGHT_3 {line}").unwrap();
        out.push_str("/* The stock width of id 2 and height of id 1 of each size class. */\n");
        writeln!(out, "#define FRAMING_STOCK_WIDTHS {}", list(family.widths)).unwrap();
        writeln!(
            out,
            "#define FRAMING_STOCK_HEIGHTS {}",
            list(family.heights_4x3)
        )
        .unwrap();
        out.push_str("/* The 3:2 height of each size class. */\n");
        writeln!(
            out,
            "#define FRAMING_NATIVE_HEIGHTS {}",
            list(family.heights_3x2)
        )
        .unwrap();
        out.push_str(
            "/* CORE.CROP (runtime/crop.h): the crop factor of each value as float bits,\n   \
             and the camera's own factors of 35 and 50 mm. */\n",
        );
        let factors = family.crops.iter().map(|bits| format!("{bits:#010x}"));
        writeln!(out, "#define CROP_FACTORS {}", list(factors)).unwrap();
        for (name, crop) in ["CROP_NATIVE_35", "CROP_NATIVE_50"]
            .iter()
            .zip(NATIVE_CROPS)
        {
            writeln!(out, "#define {name} {:#010x}", family.crops[crop]).unwrap();
        }
        out.push_str(
            "/* The sizes {long side, short side} of each file from 2, L to XS, which\n   \
             the playback classifier compares. */\n",
        );
        let sizes = classified(family)
            .into_iter()
            .flat_map(|(long, short)| [long, short]);
        writeln!(out, "#define FRAMING_SIZES {}", list(sizes)).unwrap();
    }
    out.push_str(
        "#else\n#error \"Define FRAMING_FAMILY_<family> before runtime/framing_ratios.h\"\n\
         #endif\n\n#endif\n",
    );
    out
}

/// The tables by CORE.ASPECT value.
fn ratio_tables(out: &mut String) {
    let values: Vec<String> = RATIOS
        .iter()
        .enumerate()
        .map(|(value, ratio)| format!("{value} {}", ratio.row))
        .collect();
    comment(
        out,
        &format!(
            "The Framing ratios by CORE.ASPECT value: {}. Each table here has \
             an entry for each value, in value order.",
            values.join(", ")
        ),
    );
    let by_value = |out: &mut String, name: &str, f: &dyn Fn(usize) -> u32| {
        writeln!(out, "#define {name} {}", list((0..RATIOS.len()).map(f))).unwrap();
    };
    out.push_str("/* The capture aspect id of a shot and of a RAW development. */\n");
    by_value(out, "FRAMING_RATIO_CAPTURES", &|v| {
        RATIOS[v].geometry.capture()
    });
    by_value(out, "FRAMING_RATIO_DEVELOP", &|v| {
        RATIOS[v].geometry.develop()
    });
    comment(
        out,
        "GRKIT_HOST_FRAME_FILE of a RAW development to the ratio: 1 for the \
         stock sizes of its capture aspect id.",
    );
    by_value(out, "FRAMING_RATIO_FILES", &file_of);
    comment(
        out,
        "The visible-area id (runtime/picture.c), and the live-view label \
         (grkit_core::artwork::OVERLAY_LABELS).",
    );
    by_value(out, "FRAMING_RATIO_AREAS", &|v| area_of_value(v) as u32);
    by_value(out, "FRAMING_RATIO_LABELS", &|v| label_index(v) as u32);
    comment(
        out,
        "The photo area {left, top, right, bottom} on the 720 x 480 live view.",
    );
    let boxes: Vec<String> = (0..RATIOS.len())
        .map(|v| format!("{{{}}}", list(photo_box(v))))
        .collect();
    writeln!(out, "#define FRAMING_PHOTO_BOXES {}", boxes.join(",")).unwrap();
}

/// The tables of GRKIT_HOST_FRAME_FILE, of the modes, and of the areas.
fn file_tables(out: &mut String) {
    let files = files();
    let names: Vec<String> = files
        .iter()
        .enumerate()
        .skip(2)
        .map(|(file, shape)| {
            let shape = shape.unwrap();
            format!("{file} {}", name(shape.value, shape.geometry))
        })
        .collect();
    comment(
        out,
        &format!(
            "GRKIT_HOST_FRAME_FILE: 0 while shooting, 1 for the stock sizes, then \
             each Framing geometry: {}. A geometry before 1.6.1 keeps the sizes \
             that the releases before 1.6.1 recorded.",
            names.join(", ")
        ),
    );
    writeln!(out, "#define FRAMING_FILES {}", files.len()).unwrap();
    out.push_str("/* The capture aspect id and the visible-area id of each file. */\n");
    let file_list =
        |f: &dyn Fn(Shape) -> u32| list(files.iter().map(|file| file.map(f).unwrap_or(0)));
    let captures = file_list(&|shape| shape.geometry.capture());
    writeln!(out, "#define FRAMING_FILE_CAPTURES {captures}").unwrap();
    let file_areas = file_list(&|shape| area_of_value(shape.value) as u32);
    writeln!(out, "#define FRAMING_FILE_AREAS {file_areas}").unwrap();
    comment(
        out,
        "The ratio {p, q} of each file that a crop of a ratio has (runtime/crop.h), \
         or {0, 0}: a crop has the ratio within FRAMING_CROP_SLACK (p + q), and \
         a short side of at least FRAMING_CROP_SHORTEST.",
    );
    let ratios: Vec<String> = file_ratios()
        .iter()
        .map(|(p, q)| format!("{{{p},{q}}}"))
        .collect();
    writeln!(out, "#define FRAMING_FILE_RATIOS {}", ratios.join(",")).unwrap();
    writeln!(out, "#define FRAMING_CROP_SLACK {CROP_SLACK}").unwrap();
    writeln!(out, "#define FRAMING_CROP_SHORTEST {CROP_SHORTEST}").unwrap();

    let indexes = files.len().max(RATIOS.len());
    for (wide, kind) in [(true, "WIDE"), (false, "NARROW")] {
        let modes = modes(wide);
        let names: Vec<String> = modes
            .iter()
            .enumerate()
            .map(|(mode, &(value, geometry))| match mode {
                0 if wide => "0 16:9".to_owned(),
                0 => "0 4:3".to_owned(),
                _ => format!("{mode} {}", name(value, geometry)),
            })
            .collect();
        comment(
            out,
            &format!(
                "The geometries of capture aspect id {} (runtime/framing.S): {}. \
                 The mode of each file, padded to FRAMING_MODE_INDEXES, and of \
                 each value of GRKIT_HOST_FRAME_CROP.",
                if wide { 2 } else { 1 },
                names.join(", ")
            ),
        );
        writeln!(out, "#define FRAMING_{kind}_MODES {}", modes.len()).unwrap();
        let by_file = (0..indexes).map(|index| match files.get(index).copied().flatten() {
            Some(shape) if shape.geometry.wide() == wide => {
                mode_of(wide, shape.value, shape.geometry)
            }
            _ => 0,
        });
        writeln!(out, "#define FRAMING_{kind}_BY_FILE {}", list(by_file)).unwrap();
        let by_crop = (0..indexes).map(|value| match RATIOS.get(value) {
            Some(ratio)
                if !matches!(ratio.geometry, Geometry::Native | Geometry::Stock(_))
                    && ratio.geometry.wide() == wide =>
            {
                mode_of(wide, value, ratio.geometry)
            }
            _ => 0,
        });
        writeln!(out, "#define FRAMING_{kind}_BY_CROP {}", list(by_crop)).unwrap();
        out.push_str(
            "/* Whether each mode has sizes that no native aspect has (runtime/crop.h:\n   \
             crop_resolution_size). */\n",
        );
        let framed = modes
            .iter()
            .map(|&(_, geometry)| u32::from(geometry.framed()));
        writeln!(out, "#define FRAMING_{kind}_FRAMED {}", list(framed)).unwrap();
        let insets = |preview: Preview| {
            list(modes.iter().map(|&(value, geometry)| {
                let area = match geometry {
                    Geometry::Stock(1) => 1,
                    Geometry::Stock(_) => 2,
                    _ => area_of_value(value),
                };
                let (left, top) = area_inset(area, preview);
                if wide {
                    top
                } else {
                    left
                }
            }))
        };
        out.push_str(if wide {
            "/* The top inset of the screennail and of the thumbnail by mode. */\n"
        } else {
            "/* The left inset of the screennail and of the thumbnail by mode. */\n"
        });
        let line = insets(Preview::Screennail);
        writeln!(out, "#define FRAMING_{kind}_SCREENNAIL {line}").unwrap();
        let line = insets(Preview::Thumbnail);
        writeln!(out, "#define FRAMING_{kind}_THUMBNAIL {line}").unwrap();
    }
    writeln!(out, "#define FRAMING_MODE_INDEXES {indexes}").unwrap();

    comment(
        out,
        "The insets {left, top} of each visible area in the 720 x 480 screennail \
         and the 160 x 120 thumbnail: 3:2, 4:3, 16:9, 1:1, then each Framing \
         geometry.",
    );
    writeln!(out, "#define FRAMING_AREAS {}", areas()).unwrap();
    for (name, preview) in [
        ("SCREENNAILS", Preview::Screennail),
        ("THUMBNAILS", Preview::Thumbnail),
    ] {
        let insets: Vec<String> = (0..areas())
            .map(|area| {
                let (left, top) = area_inset(area, preview);
                format!("{{{left},{top}}}")
            })
            .collect();
        writeln!(out, "#define FRAMING_AREA_{name} {}", insets.join(",")).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tables of the releases before 1.6.1 keep their entries, and the
    /// lookups of runtime/framing.S give the size of each ratio: GetResolution
    /// of the mode of GRKIT_HOST_FRAME_CROP, and of the mode of each file.
    #[test]
    fn released_tables_stay_and_modes_give_each_size() {
        let header = header();
        for line in [
            "#define FRAMING_AREA_SCREENNAILS {0,0},{40,0},{0,38},{120,0},{0,107},{0,87},{60,0},{80,0},",
            "#define FRAMING_AREA_THUMBNAILS {0,8},{0,0},{0,16},{20,0},{0,30},{0,26},{6,0},{10,0},",
            "#define FRAMING_WIDE_SCREENNAIL 38,107,87\n",
            "#define FRAMING_WIDE_THUMBNAIL 16,30,26\n",
            "#define FRAMING_RATIO_CAPTURES 0,2,2,1,2,1,1,1,",
            "#define FRAMING_RATIO_DEVELOP 0,2,2,1,2,3,1,1,",
            "#define FRAMING_RATIO_FILES 1,2,1,1,3,1,4,5,",
            "#define FRAMING_WIDE_BY_CROP 0,1,0,0,2,0,0,0,0\n",
            "#define FRAMING_NARROW_BY_CROP 0,0,0,0,0,1,2,3,4\n",
        ] {
            assert!(header.contains(line), "{line}");
        }
        let numbers = |name: &str| -> Vec<usize> {
            let line = header
                .lines()
                .find_map(|line| line.strip_prefix(&format!("#define {name} ")))
                .unwrap();
            line.split(',').map(|n| n.parse().unwrap()).collect()
        };
        let (wide_crop, narrow_crop) = (
            numbers("FRAMING_WIDE_BY_CROP"),
            numbers("FRAMING_NARROW_BY_CROP"),
        );
        let (wide_file, narrow_file) = (
            numbers("FRAMING_WIDE_BY_FILE"),
            numbers("FRAMING_NARROW_BY_FILE"),
        );
        for family in &FAMILIES {
            for class in 0..CLASSES {
                // The lookup of a mode: the stock size for mode 0.
                let wide =
                    |mode: usize| size(family, modes(true)[mode].0, modes(true)[mode].1, class);
                let narrow =
                    |mode: usize| size(family, modes(false)[mode].0, modes(false)[mode].1, class);
                for (value, ratio) in RATIOS.iter().enumerate() {
                    let expected = size(family, value, ratio.geometry, class);
                    match ratio.geometry {
                        Geometry::Wide(_) => assert_eq!(wide(wide_crop[value]), expected),
                        Geometry::Narrow(_) | Geometry::Square => {
                            assert_eq!(narrow(narrow_crop[value]), expected)
                        }
                        _ => assert!(wide_crop[value] == 0 && narrow_crop[value] == 0),
                    }
                }
                for (file, shape) in files().into_iter().enumerate() {
                    let Some(shape) = shape else { continue };
                    let expected = size(family, shape.value, shape.geometry, class);
                    if shape.geometry.wide() {
                        assert_eq!(wide(wide_file[file]), expected);
                    } else {
                        assert_eq!(narrow(narrow_file[file]), expected);
                    }
                }
            }
        }
    }
}
