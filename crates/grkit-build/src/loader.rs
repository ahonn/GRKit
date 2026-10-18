use crate::{firmware::FirmwareImage, target, PatchPlan};
use anyhow::{ensure, Context, Result};
use grkit_core::{
    artwork::{module_tiles, off_pixels, plugins_pixels},
    runtime::{
        icon_stride, HOST_ICON_COUNT, ICONS_PER_CHUNK, LINUX_DIRECTORY, MAX_CHUNKS, MAX_COUNT,
    },
};
use std::fmt::Write;

fn array(output: &mut String, name: &str, bytes: &[u8], is_const: bool) {
    let qualifier = if is_const { "const " } else { "" };
    write!(
        output,
        "static {qualifier}u8 {name}[] __attribute__((aligned(4))) = {{"
    )
    .unwrap();
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write!(output, "{byte}").unwrap();
    }
    output.push_str("};\n");
}
fn words(output: &mut String, name: &str, values: impl IntoIterator<Item = u32>) {
    write!(output, "static const u32 {name}[] = {{").unwrap();
    for (index, value) in values.into_iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write!(output, "{value:#x}").unwrap();
    }
    output.push_str("};\n");
}

pub fn header(plan: &PatchPlan, body: &FirmwareImage) -> Result<String> {
    let target = body.target;
    let modules = target.layout.is_some();
    // A layout with processing hooks has Image Control and its native data.
    let image_control = match target.layout {
        Some(layout) if layout.processing.is_some() => Some(
            target
                .image_control
                .with_context(|| format!("{} lacks native Image Control data", target.firmware))?,
        ),
        _ => None,
    };
    let mut constants = vec![
        ("MAX_COUNT", MAX_COUNT as u32),
        ("MAX_CHUNKS", MAX_CHUNKS as u32),
        ("ICONS_PER_CHUNK", ICONS_PER_CHUNK as u32),
        ("ICON_STRIDE", icon_stride(target.name)? as u32),
        ("HOST_ICON_COUNT", HOST_ICON_COUNT as u32),
        ("MAP_BASE", 0x51000000),
        ("MAP_SIZE", 0xf000000),
        ("ARRAY_COUNT", target.array_count),
        ("ARRAY_TABLE", target.array_table),
        ("HEAP_MIN", target.heap_min),
        ("HEAP_END", target.heap_end),
    ];
    let mut stock_record = None;
    if modules {
        let english = u32::from_le_bytes(body.read(target.english_slot, 4)?.try_into()?);
        constants.extend([
            ("ENGLISH_SLOT", target.english_slot),
            ("ORIGINAL_ENGLISH", english),
        ]);
        if let Some(native) = image_control {
            let (record_address, record) = body.stock_record()?;
            stock_record = Some(record);
            constants.extend([
                ("STOCK_RECORD", record_address),
                ("TABLE_BYTES", native.table_bytes),
                ("SETTINGS_DOMAIN", native.settings_domain),
                ("SETTINGS_IC_TYPE", native.settings_type),
                ("IC_TYPE_COUNT", native.type_count),
            ]);
        }
        let name = if target.pixel_base_indirect {
            "ICON_DRAWER_SLOT"
        } else {
            "ICON_PIXEL_BASE_SLOT"
        };
        constants.push((name, target.pixel_base_slot));
    }
    constants.extend([
        ("PATCH_COUNT", plan.patches.len() as u32),
        ("CODE_RELOCATION_COUNT", plan.relocations.len() as u32),
    ]);
    let mut result = String::new();
    if !modules {
        // The installer then prepares only the private header.
        result.push_str("#define GRKIT_NO_MODULES 1\n");
    } else if image_control.is_none() {
        // The installer prepares the menu artwork but no effect records.
        result.push_str("#define GRKIT_NO_IMAGE_CONTROL 1\n");
    }
    if let Some(layout) = target.layout.filter(|layout| layout.hidden != 0) {
        writeln!(result, "#define TARGET_HIDDEN {:#x}U", layout.hidden)?;
    }
    if target.name == target::GR4_111.name {
        // GR IV icon descriptors: width and a valid flag, height, pixel offset.
        result.push_str("#define GRKIT_ICON_DESCRIPTOR_12 1\n");
        // GR IV also records the native ImageTone values 0x8100..0x8105.
        result.push_str("#define GRKIT_NATIVE_TONE_PAGE 0x81\n");
    }
    for (name, value) in constants {
        writeln!(result, "#define {name} {value:#x}U")?;
    }
    for (name, suffix) in [
        ("EFFECTS_PATH", "EFFECTS"),
        ("PLAN_PATH", "MODULE.BIN"),
        ("PLAN_TMP_PATH", "MODULE.TMP"),
        ("LOCK_PATH", "INSTALL.LCK"),
        ("DONE_PATH", "INSTALL.OK"),
        ("FAILED_PATH", "INSTALL.ERR"),
        ("MODULES_PATH", "MODULES.TXT"),
    ] {
        writeln!(
            result,
            "#define {name} {}",
            serde_json::to_string(&format!("{LINUX_DIRECTORY}/{suffix}"))?
        )?;
    }
    if modules {
        if let Some(record) = stock_record {
            array(&mut result, "stock_record", record, true);
        }
        array(&mut result, "off_pixels", &off_pixels(), true);
        array(&mut result, "plugins_pixels", &plugins_pixels(), true);
        array(&mut result, "module_pixels", &module_tiles().concat(), true);
    }
    if let Some(native) = image_control {
        let tail: Vec<_> = native
            .record_tail
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        array(&mut result, "initialized_tail", &tail, true);
        let mut tables = Vec::new();
        for address in native.tables {
            tables.extend_from_slice(body.read(address, native.table_bytes as usize)?);
        }
        array(&mut result, "stock_tables", &tables, true);
        words(&mut result, "table_addresses", native.tables);
    }
    words(
        &mut result,
        "code_relocations",
        plan.relocations
            .iter()
            .flat_map(|(index, offset)| [*index as u32, *offset as u32]),
    );
    for (i, patch) in plan.patches.iter().enumerate() {
        array(&mut result, &format!("before_{i}"), &patch.before, true);
        array(&mut result, &format!("after_{i}"), &patch.after, false);
    }
    if let Some(layout) = target.layout.filter(|layout| !layout.guards.is_empty()) {
        let mut entries = String::new();
        for (i, &(address, expected)) in layout.guards.iter().enumerate() {
            let bytes = hex::decode(expected)?;
            ensure!(
                body.read(address, bytes.len())? == bytes,
                "Firmware guard differs at {address:#x}"
            );
            array(&mut result, &format!("guard_{i}"), &bytes, true);
            writeln!(entries, "{{{address:#x},{},guard_{i}}},", bytes.len())?;
        }
        writeln!(result, "#define GUARD_COUNT {}U", layout.guards.len())?;
        result.push_str("static const struct guard guards[] = {\n");
        result.push_str(&entries);
        result.push_str("};\n");
    }
    // Kind 1 marks a patch after the decoded RTOS image. The installer does
    // not require its stock bytes, because a restart keeps earlier GRKit code.
    let image_end = target::RTOS_BASE + body.body.len() as u32;
    result.push_str("static struct patch patches[] = {\n");
    for (i, patch) in plan.patches.iter().enumerate() {
        writeln!(
            result,
            "{{{:#x},{},{},before_{i},after_{i}}},",
            patch.address,
            patch.after.len(),
            u32::from(patch.address >= image_end)
        )?;
    }
    result.push_str("};\n");
    Ok(result)
}
