//! The native settings of the RTOS shell command `UserData Set <name> <label>`.
//!
//! Camera recipes write native settings through the Set dispatcher of the
//! command (`FW_USERDATA_SET`) and read them back from the settings record
//! (`runtime/recipes.h`). This module reads, from the bytes of an RTOS image
//! and with no emulation, the dispatcher, the setter of each setting name, the
//! stored value of each label, and the field that the setter stores to.
//!
//! The code has a fixed shape, so the reader accepts only these patterns:
//!
//! 1. The shell command finds "Set" with `strcmp(argv[1], "Set")` before its
//!    failure message, then calls the Set dispatcher.
//! 2. The dispatcher is a chain of `ldr r0, [r4, #8]; movw/movt r1, NAME;
//!    blx STRCMP; cmp r0, #0; beq STUB`, and each stub is a tail call to the
//!    handler of the setting.
//! 3. A label handler is a chain of `strcmp(argv[3], LABEL)`. Each match
//!    block calls the getters, puts the stored value in r1, and calls the
//!    setter. A number handler parses argv[3] with strtol.
//! 4. The setter stores r1 on its path for a changed value: GR III through
//!    `ldr rX, [r0, #4]` (the settings record), GR IV in an inner function on
//!    the object itself.
//!
//! Each other shape is an error. A setting that does not match is never
//! guessed.

use crate::arm::decode::*;
use crate::firmware::FirmwareImage;
use anyhow::{bail, ensure, Result};
use grkit_core::native;
use header::sensor_color;
use patterns::{movw_movt, setting, strcmp_chain, Image};
use std::collections::BTreeMap;

mod header;
mod patterns;

pub use header::native_header;

/// The failure message of the shell command, which the reader finds first.
const FAILURE: &[u8] = b"UserDataDebug::shell_UserData() faild";

/// The field that a setter stores to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field {
    /// The offset in the setter object of the pointer to the settings
    /// record (GR III), or `None` for a field of the object itself (GR IV).
    pub record: Option<u32>,
    pub offset: u32,
    pub width: u32,
}

/// What a handler accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Each label and the value that it stores.
    Labels(Vec<(String, i32)>),
    /// A number that strtol parses.
    Number,
    /// A number that strtof parses, which the setter takes in s0.
    Float,
}

/// A setting of the dispatcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Setting {
    pub kind: Kind,
    pub setter: u32,
    /// The calls before the setter. The setter object is the result of the
    /// last one.
    pub getters: Vec<u32>,
    pub field: Field,
}

/// The dispatcher and its settings.
#[derive(Clone, Debug)]
pub struct Table {
    pub dispatcher: u32,
    /// Each setting name in the order of the dispatcher.
    pub names: Vec<String>,
    /// The settings that `select` chose.
    pub settings: BTreeMap<String, Setting>,
}

/// The Set dispatcher: `strcmp(argv[1], "Set")` before the failure message
/// of the shell command, then `mov r0, r4; mov r1, r5; bl DISPATCHER`.
pub fn find_dispatcher(image: &FirmwareImage) -> Result<u32> {
    let image = Image { image };
    let failures = image.find(FAILURE);
    ensure!(
        failures.len() == 1,
        "The UserData failure message is not unique"
    );
    let message = failures[0];
    let references: Vec<u32> = image
        .code()
        .filter(|&(address, word)| {
            matches!(decode(address, word).op, Op::Movw(_, low) if low == message & 0xffff)
        })
        .filter_map(|(address, word)| {
            let rd = ((word >> 12) & 0xf) as u8;
            movw_movt(&image, address, rd)
                .ok()
                .flatten()
                .filter(|(value, _)| *value == message)
                .map(|_| address)
        })
        .collect();
    ensure!(
        references.len() == 1,
        "The UserData failure message has {} references",
        references.len()
    );
    let sets = image.find(b"Set\0");
    for at in (references[0] - 0x100..references[0]).step_by(4).rev() {
        let Some((value, next)) = movw_movt(&image, at, R1)? else {
            continue;
        };
        if !sets.contains(&value) {
            continue;
        }
        let ops: Vec<Op> = (0..6)
            .map(|k| image.insn(next + 4 * k).map(|insn| insn.op))
            .collect::<Result<_>>()?;
        return match ops[..] {
            [Op::Exchange(_), Op::CmpImm(R0, 0), Op::Branch(NE, _), Op::MovReg(..), Op::MovReg(..), Op::Call(dispatcher)] => {
                Ok(dispatcher)
            }
            _ => bail!("Unexpected code after strcmp(argv[1], \"Set\") at {next:#x}"),
        };
    }
    bail!("strcmp(argv[1], \"Set\") not found before the UserData failure message")
}

/// Read the dispatcher of `image` and each setting whose name `select`
/// chooses. A chosen setting that does not match the patterns is an error.
pub fn extract(image: &FirmwareImage, select: impl Fn(&str) -> bool) -> Result<Table> {
    let (dispatcher, names, each) = extract_each(image, select)?;
    let mut settings = BTreeMap::new();
    for (name, setting) in each {
        settings.insert(
            name.clone(),
            setting.map_err(|error| anyhow::anyhow!("{name}: {error}"))?,
        );
    }
    Ok(Table {
        dispatcher,
        names,
        settings,
    })
}

/// The dispatcher, its names in order, and the result of each chosen
/// setting.
#[allow(clippy::type_complexity)]
pub fn extract_each(
    image: &FirmwareImage,
    select: impl Fn(&str) -> bool,
) -> Result<(u32, Vec<String>, BTreeMap<String, Result<Setting>>)> {
    let dispatcher = find_dispatcher(image)?;
    let image = Image { image };
    let mut start = dispatcher;
    ensure!(
        (0..12).any(|k| matches!(
            image.insn(dispatcher + 4 * k).map(|i| i.op),
            Ok(Op::MovReg(R4, R1))
        )),
        "Dispatcher {dispatcher:#x} has no mov r4, r1"
    );
    loop {
        match image.insn(start)?.op {
            Op::Load(4, false, R0, R4, 8) | Op::Movw(R1, _) => break,
            _ => {}
        }
        start += 4;
        ensure!(
            start - dispatcher <= 0x40,
            "Dispatcher {dispatcher:#x}: no chain start"
        );
    }
    let (links, strcmp, _) = strcmp_chain(&image, start, R4, 8, 200_000)?;
    let strcmp =
        strcmp.ok_or_else(|| anyhow::anyhow!("Dispatcher {dispatcher:#x} has no names"))?;
    let names: Vec<String> = links.iter().map(|link| link.text.clone()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    ensure!(
        sorted.len() == names.len(),
        "The dispatcher has a name twice"
    );
    let mut settings = BTreeMap::new();
    for link in links.iter().filter(|link| select(&link.text)) {
        settings.insert(link.text.clone(), setting(&image, link.target, strcmp));
    }
    Ok((dispatcher, names, settings))
}

/// Whether recipes use a setting (`runtime/recipes.h`): the active set, the
/// type and the parameters of set No0, the white balance, the correction and
/// noise reduction settings, and the exposure settings of still pictures.
pub fn recipe_setting(name: &str) -> bool {
    const PREFIXES: [&str; 5] = [
        "StillActiveImageControlSet",
        "StillImageControlNo0",
        "AfterReleaseStillImageControlNo0Param",
        "StillFwupImageControlNo0Param",
        "StillWhiteBalance",
    ];
    const NAMES: [&str; 11] = [
        "StillHighlightCorrect",
        "StillShadowCorrect",
        "StillLensShadingCorrection",
        "StillHighIsoNR",
        native::EXPOSURE_SETTING,
        native::EXPOSURE_STEP_SETTING,
        native::ISO_UPPER_SETTING,
        native::ISO_LOWER_SETTING,
        native::ISO_STEP_SETTING,
        native::METERING.name,
        native::ND_FILTER.name,
    ];
    // The names of a custom set are text settings.
    (PREFIXES.iter().any(|prefix| name.starts_with(prefix)) && !name.ends_with("UserCustomName"))
        || NAMES.contains(&name)
}

/// GR IV: the type setter of set No0 and the model record pointer, which the
/// build header names `FW_IMAGE_CONTROL_SET` and `FW_MODEL_RECORD`.
pub fn monochrome_check(image: &FirmwareImage) -> Result<(u32, u32)> {
    let table = extract(image, |name| name == native::TYPE_SETTING)?;
    let setter = table
        .settings
        .get(native::TYPE_SETTING)
        .ok_or_else(|| anyhow::anyhow!("No {}", native::TYPE_SETTING))?
        .setter;
    let (pointer, _) = sensor_color(&Image { image })?;
    Ok((setter, pointer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipes_use_the_look_and_white_balance_settings() {
        assert!(recipe_setting("StillImageControlNo0"));
        assert!(recipe_setting(
            "StillImageControlNo0ParamPositiveFilmSaturation"
        ));
        assert!(recipe_setting(
            "AfterReleaseStillImageControlNo0ParamNegativeFilmContrast"
        ));
        assert!(recipe_setting("StillWhiteBalanceDaylightAdjustBA"));
        assert!(recipe_setting("StillHighIsoNR"));
        assert!(!recipe_setting("StillImageControlNo1"));
        assert!(!recipe_setting(
            "StillImageControlNo1ParamStandardSaturation"
        ));
        assert!(recipe_setting(
            "AfterReleaseStillImageControlNo0ParamMonochromeGrainyParticle"
        ));
        assert!(!recipe_setting(
            "AfterReleaseMovieImageControlNo0ParamMonochromeGrainyParticle"
        ));
        assert!(!recipe_setting("MovieWhiteBalance"));
        assert!(!recipe_setting("StillHighIsoNRCustomIso100"));
    }
}
