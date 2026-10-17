use crate::{firmware::FirmwareImage, target, PatchPlan};
use anyhow::Result;
use grkit_core::{
    artwork::{module_tiles, off_pixels, plugins_pixels},
    runtime::{
        HOST_ICON_COUNT, ICONS_PER_CHUNK, ICON_STRIDE, LINUX_DIRECTORY, MAX_CHUNKS, MAX_COUNT,
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
    let (record_address, record) = body.stock_record()?;
    let constants = [
        ("MAX_COUNT", MAX_COUNT as u32),
        ("MAX_CHUNKS", MAX_CHUNKS as u32),
        ("ICONS_PER_CHUNK", ICONS_PER_CHUNK as u32),
        ("ICON_STRIDE", ICON_STRIDE as u32),
        ("HOST_ICON_COUNT", HOST_ICON_COUNT as u32),
        ("MAP_BASE", 0x51000000),
        ("MAP_SIZE", 0xf000000),
        ("ARRAY_COUNT", target::ARRAY_COUNT),
        ("ARRAY_TABLE", target::ARRAY_TABLE),
        ("HEAP_MIN", target::HEAP_MIN),
        ("HEAP_END", target::HEAP_END),
        ("ENGLISH_SLOT", target::ENGLISH_SLOT),
        ("ORIGINAL_ENGLISH", 0x55004490),
        ("STOCK_RECORD", record_address),
        ("ICON_PIXEL_BASE_SLOT", target::PIXEL_BASE_SLOT),
        ("PATCH_COUNT", plan.patches.len() as u32),
        ("CODE_RELOCATION_COUNT", plan.relocations.len() as u32),
    ];
    let mut result = String::new();
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
    ] {
        writeln!(
            result,
            "#define {name} {}",
            serde_json::to_string(&format!("{LINUX_DIRECTORY}/{suffix}"))?
        )?;
    }
    array(&mut result, "stock_record", record, true);
    array(&mut result, "off_pixels", &off_pixels(), true);
    array(&mut result, "plugins_pixels", &plugins_pixels(), true);
    array(&mut result, "module_pixels", &module_tiles().concat(), true);
    let tail: Vec<_> = [0x53dc27b4u32, 0x53dc2938, 0x53dc29dc, 0x53dc2444]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
    array(&mut result, "initialized_tail", &tail, true);
    let mut tables = Vec::new();
    for address in target::TABLES {
        tables.extend_from_slice(body.read(address, 104)?);
    }
    array(&mut result, "stock_tables", &tables, true);
    words(&mut result, "table_addresses", target::TABLES);
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
