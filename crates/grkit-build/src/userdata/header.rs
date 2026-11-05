//! The build header of the native settings of a target
//! (`runtime/targets/<target>-native.h`), and the other firmware facts that it
//! needs: the stored type values, the model check, and the type icons.

use super::patterns::{movw_movt, Image};
use super::{extract, recipe_setting, Kind, Setting};
use crate::arm::decode::*;
use crate::firmware::FirmwareImage;
use anyhow::{bail, ensure, Result};
use grkit_core::native;
use std::collections::BTreeMap;

/// The labels of a setting as a sorted list of (label, stored value).
fn label_values(setting: &Setting) -> Option<Vec<(String, i64)>> {
    let Kind::Labels(labels) = &setting.kind else {
        return None;
    };
    let mut values: Vec<(String, i64)> = labels
        .iter()
        .map(|(label, value)| (label.clone(), i64::from(*value)))
        .collect();
    values.sort();
    Some(values)
}

fn model_values(labels: &native::Labels) -> Vec<(String, i64)> {
    let mut values: Vec<(String, i64)> = labels
        .values()
        .map(|(label, value)| (label.to_string(), i64::from(value)))
        .collect();
    values.sort();
    values
}

/// The stored type value of each type whose parameter fields `fields` gives,
/// from the functions that pass a changed parameter on to the picture
/// pipeline. Each one runs only while the current type is its own:
///
/// ```text
/// ldrb r3, [r0, #FIELD]        ; the parameter
/// ...                          ; at most 10 words
/// movw r3, #CURRENT
/// ldrh r3, [r0, r3]            ; the current type << 8
/// cmp  r3, #TYPE << 8
/// ```
///
/// A type has its value only if each such function of its fields compares
/// with one value.
fn synced_types(image: &Image, fields: &BTreeMap<u32, usize>) -> Result<BTreeMap<usize, u32>> {
    let code: Vec<(u32, u32)> = image.code().collect();
    let words: Vec<u32> = code.iter().map(|&(_, word)| word).collect();
    let mut found: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
    for (index, &word) in words.iter().enumerate() {
        // ldrb r3, [r0, #imm12]
        if word & 0xffff_f000 != 0xe5d0_3000 {
            continue;
        }
        let Some(&image_type) = fields.get(&(word & 0xfff)) else {
            continue;
        };
        for at in index + 1..(index + 12).min(words.len().saturating_sub(2)) {
            let (movw, load, compare) = (words[at], words[at + 1], words[at + 2]);
            // movw r3, #imm16; ldrh r3, [r0, rM]; cmp r3, #imm
            if movw & 0xfff0_f000 == 0xe300_3000
                && load & 0xffff_fff0 == 0xe190_30b0
                && compare & 0xffff_f000 == 0xe353_0000
            {
                let value = modified_immediate(compare);
                ensure!(
                    value & 0xff == 0,
                    "A type check at {:#x} is not a type",
                    code[at + 2].0
                );
                found.entry(image_type).or_default().push(value >> 8);
                break;
            }
        }
    }
    let mut types = BTreeMap::new();
    for (image_type, values) in found {
        ensure!(
            values.iter().all(|&value| value == values[0]),
            "The parameters of type {} check several types: {values:?}",
            native::TYPES[image_type].label
        );
        types.insert(image_type, values[0]);
    }
    Ok(types)
}

/// The model check of GR IV (`FW_MODEL_RECORD`): the address of the
/// pointer to the current model record, and the offset of the byte that
/// `SystemConfig_GetImageSensorColorType` returns, 1 on GR IV Monochrome.
///
/// ```text
/// movw/movt rX, POINTER
/// ldr  rY, [rX]
/// ldrb r0, [rY, #OFFSET]       ; conditional: the record is not 0
/// ...
/// movw/movt rZ, "SystemConfig_GetImageSensorColorType"   ; its error path
/// ```
pub(super) fn sensor_color(image: &Image) -> Result<(u32, u32)> {
    let names = image.find(b"SystemConfig_GetImageSensorColorType\0");
    ensure!(
        names.len() == 1,
        "{} names of the sensor color check",
        names.len()
    );
    let name = names[0];
    let mut references = Vec::new();
    for (address, word) in image.code() {
        if let Op::Movw(rd, low) = decode(address, word).op {
            if low == name & 0xffff
                && movw_movt(image, address, rd)?.map(|(value, _)| value) == Some(name)
            {
                references.push(address);
            }
        }
    }
    ensure!(
        references.len() == 1,
        "{} uses of the sensor color check name",
        references.len()
    );
    let reference = references[0];
    for start in (reference - 0x40..reference).step_by(4) {
        let Op::Movw(pointer_register, _) = image.insn(start)?.op else {
            continue;
        };
        let Some((pointer, after)) = movw_movt(image, start, pointer_register)? else {
            continue;
        };
        let mut record = None;
        for at in (after..reference).step_by(4) {
            match image.insn(at)?.op {
                Op::Load(4, false, rt, rn, 0) if rn == pointer_register => record = Some(rt),
                Op::Load(1, false, R0, rn, offset) if Some(rn) == record && offset > 0 => {
                    return Ok((pointer, offset as u32));
                }
                _ => {}
            }
        }
    }
    bail!("No model record in the sensor color check at {reference:#x}")
}

/// The hook site whose BL calls the native icon mapper of the types.
const ICON_MAPPER_SITE: &str = "menu_viewfinder_icon";

/// Whether condition `condition` holds for the flags N, Z, C, and V.
fn condition_holds(condition: u8, [n, z, c, v]: [bool; 4]) -> Result<bool> {
    Ok(match condition {
        0x0 => z,
        0x1 => !z,
        0x2 => c,
        0x3 => !c,
        0x4 => n,
        0x5 => !n,
        0x6 => v,
        0x7 => !v,
        0x8 => c && !z,
        0x9 => !c || z,
        0xa => n == v,
        0xb => n != v,
        0xc => !z && n == v,
        0xd => z || n != v,
        ALWAYS => true,
        _ => bail!("Condition {condition:#x} is not a condition"),
    })
}

/// The icon that the native icon mapper returns for type code `code`.
///
/// The live-view Image Control widget calls the mapper at the
/// `menu_viewfinder_icon` site with the type code in r1, and the native Image
/// Control list calls it for the icon of each row. GR IV enters it through a
/// thunk that adds to r0 and sets r2 to 0, where a nonzero r2 asks for no
/// icon. The mapper is a chain of `cmp r1, #CODE` and branches to blocks that
/// put the icon in r0 and return:
///
/// ```text
/// cmp   r1, #0xa
/// bls   NONE            ; mov r0, #0; bx lr
/// cmp   r1, #0xb
/// beq   STANDARD        ; cmp r2, #0; movw r0, #ICON; movne r0, #0; bx lr
/// ...
/// ```
///
/// The reader runs the code from the BL target with only r1 known, and only
/// for the instructions of this shape: each other instruction, or a read of
/// an unknown register, is an error.
fn type_icon(image: &Image, mapper: u32, code: u32) -> Result<u32> {
    let mut registers: [Option<u32>; 16] = [None; 16];
    registers[R1 as usize] = Some(code);
    let mut flags = None;
    let mut at = mapper;
    for _ in 0..128 {
        let insn = image.insn(at)?;
        let read = |register: u8| {
            registers[register as usize]
                .ok_or_else(|| anyhow::anyhow!("The icon mapper reads r{register} at {at:#x}"))
        };
        if insn.condition != ALWAYS {
            let flags =
                flags.ok_or_else(|| anyhow::anyhow!("The icon mapper has no flags at {at:#x}"))?;
            if !condition_holds(insn.condition, flags)? {
                at += 4;
                continue;
            }
        }
        match insn.op {
            Op::CmpImm(rn, value) => {
                let left = read(rn)?;
                let result = left.wrapping_sub(value);
                flags = Some([
                    result >> 31 != 0,
                    result == 0,
                    left >= value,
                    ((left ^ value) & (left ^ result)) >> 31 != 0,
                ]);
            }
            Op::MovImm(rd, value) | Op::Movw(rd, value) => registers[rd as usize] = Some(value),
            Op::OrrImm(rd, rn, value) => registers[rd as usize] = Some(read(rn)? | value),
            // The thunk moves r0, which the mapper does not read.
            Op::AddImm(rd, rn, value) => {
                registers[rd as usize] = registers[rn as usize].map(|x| x.wrapping_add(value))
            }
            Op::Branch(_, target) => {
                at = target;
                continue;
            }
            Op::Return => return read(R0),
            _ => bail!("Unexpected instruction in the icon mapper at {at:#x}"),
        }
        at += 4;
    }
    bail!("The icon mapper at {mapper:#x} does not return for type {code}")
}

/// The native icon of each portable type of `codes` (`NATIVE_TYPE_CODES`),
/// or 0 for a type that the camera does not have. Each type that the camera
/// has must have an icon of the native range.
fn type_icons(image: &FirmwareImage, codes: &[u32]) -> Result<Vec<u32>> {
    let target = image.target;
    let &(_, site, _) = target
        .layout
        .sites
        .iter()
        .find(|(name, _, _)| *name == ICON_MAPPER_SITE)
        .ok_or_else(|| anyhow::anyhow!("{}: no {ICON_MAPPER_SITE} site", target.name))?;
    let image = Image { image };
    let Op::Call(mapper) = image.insn(site)?.op else {
        bail!("{}: no BL at the {ICON_MAPPER_SITE} site", target.name);
    };
    codes
        .iter()
        .zip(native::TYPES.iter())
        .map(|(&code, image_type)| {
            if code == 0xff {
                return Ok(0);
            }
            let icon = type_icon(&image, mapper, code)?;
            ensure!(
                icon != 0 && icon < 0x7000,
                "{}: the icon of {} is {icon:#x}",
                target.name,
                image_type.label
            );
            Ok(icon)
        })
        .collect()
}

/// `runtime/targets/<target>-native.h`: the stored values and the field
/// offsets of the native settings of recipes (`grkit_core::native`) in the
/// firmware of `image`. Each name, label, and stored value of the model must
/// be those of the firmware, and the firmware may have no parameter of a
/// type that the model leaves out.
pub fn native_header(image: &FirmwareImage) -> Result<String> {
    let target = image.target;
    let family = native::Family::of_target(target.name)
        .ok_or_else(|| anyhow::anyhow!("{} has no native family", target.name))?;
    let table = extract(image, recipe_setting)?;
    let get = |name: &str| -> Result<&Setting> {
        table
            .settings
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("{}: no setting {name}", target.name))
    };
    let byte_field = |name: &str| -> Result<u32> {
        let setting = get(name)?;
        ensure!(
            setting.field.width == 1,
            "{}: {name} is not one byte",
            target.name
        );
        Ok(setting.field.offset)
    };
    let check_labels = |name: &str, labels: &native::Labels| -> Result<u32> {
        let setting = get(name)?;
        ensure!(
            label_values(setting) == Some(model_values(labels)),
            "{}: the labels of {name} differ from the model",
            target.name
        );
        byte_field(name)
    };
    // One record shape for each setting.
    let first = table.settings.values().next().unwrap();
    for (name, setting) in &table.settings {
        ensure!(
            setting.field.record == first.field.record && setting.getters == first.getters,
            "{}: {name} has another record or getter",
            target.name
        );
    }
    // The active set and the type: the label and the stored value of each.
    let active = get(native::ACTIVE_SET_SETTING)?;
    ensure!(
        matches!(&active.kind, Kind::Labels(labels) if labels.iter().any(|(l, v)| l == native::MAIN_SET && *v == 0)),
        "{}: no main set No0 = 0",
        target.name
    );
    let type_setting = get(native::TYPE_SETTING)?;
    let Kind::Labels(type_labels) = &type_setting.kind else {
        bail!("{}: the type is not a label setting", target.name);
    };
    // A type is in the build if its family has it, but a build can lack a
    // Monochrome type that a later build adds: then the firmware has none of
    // its parameters.
    let present: Vec<bool> = native::TYPES
        .iter()
        .enumerate()
        .map(|(index, image_type)| {
            image_type.families[family as usize].is_some()
                && (!image_type.monochrome
                    || (0..native::PARAMETERS.len()).any(|parameter| {
                        native::parameter_name(family, index, parameter)
                            .is_some_and(|name| table.settings.contains_key(&name))
                    }))
        })
        .collect();
    // The parameters of each type.
    let mut fields = Vec::new();
    for (index, image_type) in native::TYPES.iter().enumerate() {
        for parameter in 0..native::PARAMETERS.len() {
            match native::parameter_name(family, index, parameter).filter(|_| present[index]) {
                Some(name) => {
                    let labels = native::type_parameter_labels(family, index, parameter).unwrap();
                    fields.push(check_labels(&name, &labels)?);
                }
                None => {
                    for prefix in native::Prefix::ALL {
                        let name = format!(
                            "{}{}{}",
                            prefix.text(),
                            image_type.label,
                            native::PARAMETERS[parameter].key
                        );
                        ensure!(
                            !table.settings.contains_key(&name)
                                || image_type.families[family as usize].is_none(),
                            "{}: the firmware has {name}, which the model does not",
                            target.name
                        );
                    }
                    fields.push(0);
                }
            }
        }
    }
    // The Custom sets. StillActiveImageControlSet selects set k with label
    // Nok and value k, and set k has the names of the main set with its own
    // number. Each field of set k is the field of the main set plus k times
    // one stride of that field, so a save reads the active set.
    let Kind::Labels(set_labels) = &active.kind else {
        bail!("{}: the active set is not a label setting", target.name);
    };
    for (k, (label, value)) in set_labels.iter().enumerate() {
        ensure!(
            *label == format!("No{k}") && usize::try_from(*value) == Ok(k),
            "{}: set {label} is not No{k} = {k}",
            target.name
        );
    }
    let sets = set_labels.len() as u32;
    let stride = |name: &str| -> Result<u32> {
        let main = get(name)?;
        let mut stride = 0;
        for k in 1..sets {
            let set_name = name.replacen("No0", &format!("No{k}"), 1);
            let other = get(&set_name)?;
            ensure!(
                other.kind == main.kind
                    && other.field.record == main.field.record
                    && other.field.width == main.field.width,
                "{}: {set_name} is not a setting of the shape of {name}",
                target.name
            );
            if k == 1 {
                stride = other.field.offset.saturating_sub(main.field.offset);
            }
            ensure!(
                (1..=0xff).contains(&stride)
                    && other.field.offset == main.field.offset + k * stride,
                "{}: {set_name} is not one stride of one byte after the set before it",
                target.name
            );
        }
        Ok(stride)
    };
    let type_stride = stride(native::TYPE_SETTING)?;
    let mut strides = Vec::new();
    for (index, _) in native::TYPES.iter().enumerate() {
        for parameter in 0..native::PARAMETERS.len() {
            strides.push(
                match native::parameter_name(family, index, parameter).filter(|_| present[index]) {
                    Some(name) => stride(&name)?,
                    None => 0,
                },
            );
        }
    }
    // The stored value of each type: the label of StillImageControlNo0, or
    // for a Monochrome type, the value that the functions of its parameters
    // check. The other types check that the two agree.
    let parameters = native::PARAMETERS.len();
    let type_fields: BTreeMap<u32, usize> = fields
        .iter()
        .enumerate()
        .filter(|&(_, &field)| field != 0)
        .map(|(at, &field)| (field, at / parameters))
        .collect();
    let synced = if native::TYPES
        .iter()
        .any(|t| t.monochrome && t.families[family as usize].is_some())
    {
        synced_types(&Image { image }, &type_fields)?
    } else {
        BTreeMap::new()
    };
    let mut codes = Vec::new();
    for (index, image_type) in native::TYPES.iter().enumerate() {
        let code = if !present[index] {
            0xff
        } else if image_type.monochrome {
            *synced.get(&index).ok_or_else(|| {
                anyhow::anyhow!("{}: no stored value of {}", target.name, image_type.label)
            })?
        } else {
            let (_, value) = type_labels
                .iter()
                .find(|(l, _)| l == image_type.label)
                .ok_or_else(|| anyhow::anyhow!("{}: no type {}", target.name, image_type.label))?;
            let value = u32::try_from(*value)?;
            if let Some(&checked) = synced.get(&index) {
                ensure!(
                    checked == value,
                    "{}: {} is {value}, but its parameters check {checked}",
                    target.name,
                    image_type.label
                );
            }
            value
        };
        codes.push(code);
    }
    let labelled = codes
        .iter()
        .zip(native::TYPES.iter())
        .filter(|(&code, image_type)| code != 0xff && !image_type.monochrome)
        .count();
    ensure!(
        type_labels.len() == labelled,
        "{}: the firmware has a type that the model does not",
        target.name
    );
    let mut values: Vec<u32> = codes.iter().copied().filter(|&code| code != 0xff).collect();
    values.sort_unstable();
    values.dedup();
    ensure!(
        values.len() == codes.iter().filter(|&&code| code != 0xff).count(),
        "{}: two types have one stored value",
        target.name
    );
    let icons = type_icons(image, &codes)?;
    // The white balance modes, their shifts, and the color temperature.
    let mode_setting = get(native::WHITE_BALANCE_SETTING)?;
    let Kind::Labels(mode_labels) = &mode_setting.kind else {
        bail!("{}: the white balance is not a label setting", target.name);
    };
    let (mut mode_codes, mut shifts) = (Vec::new(), Vec::new());
    for (index, label) in native::mode_labels(family).into_iter().enumerate() {
        mode_codes.push(match label {
            Some(label) => {
                let (_, value) = mode_labels
                    .iter()
                    .find(|(l, _)| l == label)
                    .ok_or_else(|| anyhow::anyhow!("{}: no white balance {label}", target.name))?;
                u32::try_from(*value)?
            }
            None => 0xffff,
        });
        match native::shift_names(family, index) {
            Some([ba, gm]) => {
                shifts.push(check_labels(&ba, &native::SHIFT_BA)?);
                shifts.push(check_labels(&gm, &native::SHIFT_GM)?);
            }
            None => shifts.extend([0, 0]),
        }
    }
    ensure!(
        mode_labels.len() == mode_codes.iter().filter(|&&code| code != 0xffff).count(),
        "{}: the firmware has a white balance mode that the model does not",
        target.name
    );
    let temperature = get(native::COLOR_TEMPERATURE)?;
    ensure!(
        temperature.kind == Kind::Number && temperature.field.width == 2,
        "{}: the color temperature is not a number of two bytes",
        target.name
    );
    let corrections: Vec<u32> = native::CORRECTIONS
        .iter()
        .map(|correction| check_labels(correction.name, &correction.labels))
        .collect::<Result<_>>()?;
    // The exposure settings: four-byte floats and two label settings.
    let float_field = |name: &str| -> Result<u32> {
        let setting = get(name)?;
        ensure!(
            setting.kind == Kind::Float && setting.field.width == 4,
            "{}: {name} is not a float",
            target.name
        );
        Ok(setting.field.offset)
    };
    let exposure = [
        ("EXPOSURE", float_field(native::EXPOSURE_SETTING)?),
        ("EXPOSURE_STEP", float_field(native::EXPOSURE_STEP_SETTING)?),
        ("ISO_UPPER", float_field(native::ISO_UPPER_SETTING)?),
        ("ISO_LOWER", float_field(native::ISO_LOWER_SETTING)?),
        ("ISO_STEP", float_field(native::ISO_STEP_SETTING)?),
        (
            "METERING",
            check_labels(native::METERING.name, &native::METERING.labels)?,
        ),
        (
            "ND",
            check_labels(native::ND_FILTER.name, &native::ND_FILTER.labels)?,
        ),
    ];
    let list = |values: &[u32]| {
        values
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    let mut out = format!(
        "/* Generated from the firmware of {} (crates/grkit-build/src/userdata.rs). Do\n   \
         not edit: with the firmware files of the ignored firmware tests, run\n   \
         GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-build -- --ignored\n   \
         native_headers_match_each_firmware.\n\n   \
         The stored values and the field offsets of the native settings of recipes\n   \
         (runtime/native.h). FW_SETTINGS gives the settings object. A field is at\n   \
         its offset in the settings record, which is the word at NATIVE_RECORD in\n   \
         the object, or the object itself without NATIVE_RECORD. The fields are one\n   \
         byte, the color temperature two, and the exposure compensation, the\n   \
         exposure step, the Auto ISO limits, and the ISO step are four-byte\n   \
         floats. Each offset of a parameter that a type\n   \
         does not have is 0, and each code of a type or mode that the camera does\n   \
         not have is all ones. StillActiveImageControlSet (NATIVE_ACTIVE_SET_FIELD)\n   \
         selects one of NATIVE_SETS sets: 0 is the main set, and the others are the\n   \
         Custom sets. Set k keeps its type at NATIVE_TYPE_FIELD + k *\n   \
         NATIVE_TYPE_SET_STRIDE, and each parameter at its offset + k * its stride\n   \
         of NATIVE_PARAMETER_SET_STRIDES. NATIVE_TYPE_ICONS gives the native icon\n   \
         ID of each type, as the native Image Control list shows it, or 0. On GR\n   \
         IV, the byte at NATIVE_SENSOR_COLOR_FIELD of the model record\n   \
         (FW_MODEL_RECORD) is 1 on GR IV Monochrome. */\n",
        target.name
    );
    out.push_str("#ifndef GRKIT_TARGET_NATIVE_H\n#define GRKIT_TARGET_NATIVE_H\n");
    // The family of the names and labels in runtime/native.h.
    out.push_str(&format!(
        "#define NATIVE_FAMILY_{} 1\n",
        family.key().to_ascii_uppercase()
    ));
    if let Some(record) = first.field.record {
        out.push_str(&format!("#define NATIVE_RECORD {record}U\n"));
    }
    out.push_str(&format!(
        "#define NATIVE_ACTIVE_SET_FIELD {}U\n#define NATIVE_TYPE_FIELD {}U\n",
        byte_field(native::ACTIVE_SET_SETTING)?,
        type_setting.field.offset
    ));
    out.push_str(&format!("#define NATIVE_TYPE_CODES {}\n", list(&codes)));
    out.push_str(&format!("#define NATIVE_TYPE_ICONS {}\n", list(&icons)));
    out.push_str(&format!(
        "#define NATIVE_PARAMETER_FIELDS {}\n",
        list(&fields)
    ));
    out.push_str(&format!(
        "#define NATIVE_SETS {sets}U\n#define NATIVE_TYPE_SET_STRIDE {type_stride}U\n\
         #define NATIVE_PARAMETER_SET_STRIDES {}\n",
        list(&strides)
    ));
    out.push_str(&format!(
        "#define NATIVE_MODE_FIELD {}U\n#define NATIVE_MODE_CODES {}\n",
        byte_field(native::WHITE_BALANCE_SETTING)?,
        list(&mode_codes)
    ));
    out.push_str(&format!("#define NATIVE_SHIFT_FIELDS {}\n", list(&shifts)));
    out.push_str(&format!(
        "#define NATIVE_TEMPERATURE_FIELD {}U\n",
        temperature.field.offset
    ));
    out.push_str(&format!(
        "#define NATIVE_CORRECTION_FIELDS {}\n",
        list(&corrections)
    ));
    for (name, offset) in exposure {
        out.push_str(&format!("#define NATIVE_{name}_FIELD {offset}U\n"));
    }
    if native::TYPES
        .iter()
        .any(|t| t.monochrome && t.families[family as usize].is_some())
    {
        // The build header gives the model record (FW_MODEL_RECORD) and the
        // type setter (FW_IMAGE_CONTROL_SET), which monochrome_check reads.
        let (_, offset) = sensor_color(&Image { image })?;
        out.push_str(&format!("#define NATIVE_SENSOR_COLOR_FIELD {offset:#x}U\n"));
    }
    out.push_str("#endif\n");
    Ok(out)
}
