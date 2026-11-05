//! The generated C header of the portable model, `runtime/native.h`.

use super::*;
use std::fmt::Write;

fn c_string(text: &str) -> String {
    format!("\"{text}\"")
}

fn c_strings<'a>(texts: impl Iterator<Item = Option<&'a str>>) -> String {
    texts
        .map(|text| text.map(c_string).unwrap_or_else(|| "0".into()))
        .collect::<Vec<_>>()
        .join(",")
}

/// The label table of a label set: a label for each stored value from 0 to
/// the last one, 0 for a value without a label.
fn label_table(labels: &Labels) -> String {
    let last = labels.first + labels.labels.len() as u32;
    c_strings((0..last).map(|value| {
        value
            .checked_sub(labels.first)
            .and_then(|n| labels.labels.get(n as usize).copied())
            .filter(|label| !label.is_empty())
    }))
}

/// The contents of `runtime/native.h`.
pub fn header() -> String {
    let mut out = String::new();
    out.push_str(
        "/* Generated from crates/grkit-core/src/native/. Do not edit: change the\n   \
         tables there, then run GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-core\n   \
         --test generated.\n\n   \
         The native picture settings of camera recipes (runtime/recipes.h): the\n   \
         portable index of each Image Control type, parameter, white balance mode,\n   \
         and correction setting, and the names and labels of each family for\n   \
         UserData Set. A label table gives the label of each stored value, 0 for a\n   \
         value without one. runtime/targets/<target>-native.h gives the stored\n   \
         values and the field offsets of a build. */\n",
    );
    out.push_str("#ifndef GRKIT_NATIVE_H\n#define GRKIT_NATIVE_H\n\n");
    writeln!(out, "#define NATIVE_TYPES {}U", TYPES.len()).unwrap();
    writeln!(out, "#define NATIVE_FIRST_MONOCHROME {FIRST_MONOCHROME}U").unwrap();
    writeln!(out, "#define NATIVE_PARAMETERS {}U", PARAMETERS.len()).unwrap();
    writeln!(out, "#define NATIVE_LEVELS {LEVEL_COUNT}U").unwrap();
    writeln!(out, "#define NATIVE_MODES {}U", WHITE_BALANCE.len()).unwrap();
    writeln!(out, "#define NATIVE_CORRECTIONS {}U", CORRECTIONS.len()).unwrap();
    writeln!(out, "#define NATIVE_TEMPERATURE_MIN {TEMPERATURE_MIN}U").unwrap();
    writeln!(out, "#define NATIVE_TEMPERATURE_MAX {TEMPERATURE_MAX}U").unwrap();
    writeln!(out, "#define NATIVE_TEMPERATURE_STEP {TEMPERATURE_STEP}U").unwrap();
    writeln!(
        out,
        "#define NATIVE_MODE_TEMPERATURE {COLOR_TEMPERATURE_MODE}U"
    )
    .unwrap();
    for (index, parameter) in PARAMETERS.iter().enumerate() {
        writeln!(
            out,
            "#define NATIVE_PARAMETER_{} {index}U",
            parameter.key.to_ascii_uppercase()
        )
        .unwrap();
    }
    out.push('\n');
    writeln!(
        out,
        "#define NATIVE_TYPE_SETTING {}",
        c_string(TYPE_SETTING)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_ACTIVE_SET_SETTING {}",
        c_string(ACTIVE_SET_SETTING)
    )
    .unwrap();
    writeln!(out, "#define NATIVE_MAIN_SET {}", c_string(MAIN_SET)).unwrap();
    writeln!(
        out,
        "#define NATIVE_WHITE_BALANCE_SETTING {}",
        c_string(WHITE_BALANCE_SETTING)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_COLOR_TEMPERATURE_SETTING {}",
        c_string(COLOR_TEMPERATURE)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_PREFIXES {}",
        c_strings(Prefix::ALL.iter().map(|prefix| Some(prefix.text())))
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_PARAMETER_KEYS {}",
        c_strings(PARAMETERS.iter().map(|parameter| Some(parameter.key)))
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_CORRECTION_NAMES {}",
        c_strings(CORRECTIONS.iter().map(|correction| Some(correction.name)))
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_EXPOSURE_SETTING {}",
        c_string(EXPOSURE_SETTING)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_ISO_UPPER_SETTING {}",
        c_string(ISO_UPPER_SETTING)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_METERING_SETTING {}",
        c_string(METERING.name)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_ND_SETTING {}",
        c_string(ND_FILTER.name)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_METERING_LABELS {}",
        label_table(&METERING.labels)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_ND_LABELS {}",
        label_table(&ND_FILTER.labels)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_EXPOSURE_THIRDS_MAX {EXPOSURE_THIRDS_MAX}U"
    )
    .unwrap();
    writeln!(out, "#define NATIVE_ISO_SV_BASE {ISO_SV_BASE}U").unwrap();
    writeln!(
        out,
        "#define NATIVE_SHIFT_BA_LABELS {}",
        label_table(&SHIFT_BA)
    )
    .unwrap();
    writeln!(
        out,
        "#define NATIVE_SHIFT_GM_LABELS {}",
        label_table(&SHIFT_GM)
    )
    .unwrap();
    for (index, correction) in CORRECTIONS.iter().enumerate() {
        writeln!(
            out,
            "#define NATIVE_CORRECTION_{index}_LABELS {}",
            label_table(&correction.labels)
        )
        .unwrap();
    }
    out.push('\n');
    for family in Family::ALL {
        let key = family.key().to_ascii_uppercase();
        writeln!(out, "/* {}. */", family.key()).unwrap();
        // The third stops of the Auto ISO upper limit: from ISO 100 to the
        // highest ISO of the family.
        writeln!(
            out,
            "#define NATIVE_{key}_ISO_STEPS {}U",
            ISO_STEPS
                .iter()
                .position(|&iso| iso == family.iso_max())
                .unwrap()
                + 1
        )
        .unwrap();
        writeln!(
            out,
            "#define NATIVE_{key}_TYPE_LABELS {}",
            c_strings(type_labels(family).into_iter())
        )
        .unwrap();
        writeln!(
            out,
            "#define NATIVE_{key}_MODE_LABELS {}",
            c_strings(mode_labels(family).into_iter())
        )
        .unwrap();
        writeln!(
            out,
            "#define NATIVE_{key}_MODE_SHIFTS {}",
            WHITE_BALANCE
                .iter()
                .map(
                    |mode| u32::from(mode.labels[family.index()].is_some() && mode.shifts)
                        .to_string()
                )
                .collect::<Vec<_>>()
                .join(",")
        )
        .unwrap();
        // The prefix of the parameter names of each type, an index of
        // NATIVE_PREFIXES, 0 for a type that the family does not have.
        writeln!(
            out,
            "#define NATIVE_{key}_TYPE_PREFIXES {}",
            TYPES
                .iter()
                .map(|image_type| {
                    image_type.families[family.index()]
                        .map(|(prefix, _)| Prefix::ALL.iter().position(|&p| p == prefix).unwrap())
                        .unwrap_or(0)
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(",")
        )
        .unwrap();
        // The labels of each parameter in one list: those of parameter p from
        // NATIVE_<FAMILY>_LABEL_STARTS[p], NATIVE_<FAMILY>_LABEL_COUNTS[p]
        // entries, one for each stored value from 0.
        let (mut flat, mut starts, mut counts) = (Vec::new(), Vec::new(), Vec::new());
        for parameter in PARAMETERS.iter() {
            starts.push(flat.len().to_string());
            let entries: Vec<Option<&str>> = match parameter.labels[family.index()] {
                Some(labels) => {
                    let last = labels.first + labels.labels.len() as u32;
                    (0..last)
                        .map(|value| {
                            value
                                .checked_sub(labels.first)
                                .and_then(|n| labels.labels.get(n as usize).copied())
                        })
                        .collect()
                }
                None => Vec::new(),
            };
            counts.push(entries.len().to_string());
            flat.extend(entries);
        }
        writeln!(
            out,
            "#define NATIVE_{key}_LABELS {}",
            c_strings(flat.into_iter())
        )
        .unwrap();
        writeln!(
            out,
            "#define NATIVE_{key}_LABEL_STARTS {}",
            starts.join(",")
        )
        .unwrap();
        writeln!(
            out,
            "#define NATIVE_{key}_LABEL_COUNTS {}",
            counts.join(",")
        )
        .unwrap();
    }
    out.push_str("\n#endif\n");
    out
}
