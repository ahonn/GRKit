use crate::{
    arm::{branch, elf_text},
    firmware::FirmwareImage,
    target::HEADER_REFERENCE,
    toolchain::{run, Toolchain},
    Patch,
};
use anyhow::{ensure, Context, Result};
use std::{collections::BTreeMap, fs, path::Path};

pub const PROCESSING_BASE: u32 = 0x53002080;
// The translation table starts at 0x53004000. Both components must fit below it.
pub const MENU_BASE: u32 = 0x53002340;
// Framing lives after the decoded RTOS image (0x540b55c0). The RTOS maps this
// tail read-only and executable up to 0x55000000, and no firmware code refers
// to it. BOOT does not reload it, so the installer overwrites any earlier
// GRKit code there instead of requiring zeros.
pub const FRAMING_BASE: u32 = 0x540b6000;
pub const FRAMING_END: u32 = 0x540be000;
const RTOS_EXECUTABLE_END: u32 = 0x55000000;
pub const CONTEXTS: &[(u32, &str)] = &[
    (0x536dc124, "abfdffeb"),
    (0x536dc28c, PROCESSING_ORIGINAL),
    (0x536dc688, "943196e5903084e58b4e00eb"),
    (0x536de1a8, "0e30d4e5"),
    (0x536e8694, "9b30d4e5"),
    (0x536e87c4, "891900eb"),
    // LiveViewController's Image Control widget: load native type, map, assign.
    (
        0x531710bc,
        "e83494e5001ad3e597e204eb0040a0e10500a0e10410a0e131a608eb",
    ),
];
// Instructions replaced by the processing hook and replayed on its stock path.
const PROCESSING_ORIGINAL: &str =
    "940d08e3f0cc08e3010545e301c545e37b9f86e201e190e74c0e08e3010545e3";
const PROCESSING_HOOKS: &[(u32, &str, bool)] = &[
    (0x536dc124, "module_parameters", true),
    (0x536dc28c, "module_processing", false),
    (0x536dc68c, "module_packet", false),
    (0x536de1a8, "module_gamma_dirty", false),
    (0x536e8694, "module_gamma_flag", false),
    (0x536e87c4, "module_gamma_curves", true),
];
pub const MENU_HOOKS: &[(u32, &str, &str)] = &[
    (0x531a48f0, "menu_native_touch_0", "f431d0e5"),
    (0x531a4904, "menu_native_touch_1", "f431d0e5"),
    (0x531a4918, "menu_native_touch_2", "f431d0e5"),
    (0x531a492c, "menu_native_touch_3", "f431d0e5"),
    (0x531a4940, "menu_native_touch_4", "f431d0e5"),
    (0x5319ff24, "menu_destroy", "0dc0a0e1"),
    (0x531a4114, "menu_refresh", "0dc0a0e1"),
    (0x531a4db8, "menu_event", "b030d1e1"),
    (0x531a3e88, "menu_exit", "7022d0e5"),
    (0x531a4cfc, "menu_changed", "0dc0a0e1"),
    (0x533966fc, "menu_text_measure", "077193e7"),
    (0x53397108, "menu_text_draw", "073193e7"),
    (0x53209fb8, "menu_icon_width", "9e3200e3"),
    (0x53209fec, "menu_icon_height", "9e3200e3"),
    (0x5320a7cc, "menu_icon_guard", "9e1200e3"),
    (0x5320a810, "menu_icon_descriptor", "033192e7"),
    (0x531a0ba0, "menu_native_adj_apply", "0dc0a0e1"),
    (0x531ab998, "menu_native_dialog_apply", "0dc0a0e1"),
    // ImageControlController (MENU page) event and exit handlers.
    (0x531ac270, "menu_native_page_event", "0dc0a0e1"),
    (0x531ab1b0, "menu_native_page_exit", "0dc0a0e1"),
];

pub const FRAMING_HOOKS: &[(u32, &str, &str, bool)] = &[
    (0x5317681c, "framing_update", "0dc0a0e1", false),
    (0x53176de4, "framing_exit", "0dc0a0e1", false),
    (0x53170b50, "framing_destroy", "0dc0a0e1", false),
    (0x53391b14, "framing_canvas", "0dc0a0e1", false),
    (0x53391be0, "framing_children", "500800eb", true),
    // Still aspect property (0xd4) handler: replaces `ldrb r1, [r4]`, the
    // requested capture aspect, so a native 3:2 request records the Framing ratio.
    (0x535423f4, "framing_capture_request_hook", "0010d4e5", true),
    // GetResolution id 2 heights (L, then each smaller size or crop step).
    (0x536a2e48, "framing_height_0", "d33ea0e3", true),
    (0x536a2e34, "framing_height_1", "a93ea0e3", true),
    (0x536a2e20, "framing_height_2", "763ea0e3", true),
    (0x536a2e08, "framing_height_3", "383400e3", true),
    // Id 2 top insets of the screennail and the thumbnail.
    (0x536a2f98, "framing_screennail_inset", "2610a0e3", true),
    (0x536a3050, "framing_thumbnail_inset", "1010a0e3", true),
    // Playback size classifier and image aspect getter.
    (0x532b39b8, "framing_classify", "0dfcffeb", true),
    (0x532b3cac, "framing_image_aspect", "50fbffea", false),
];

/// Data and single-instruction changes that let capture use the hidden GR IV
/// aspect ids 1 (4:3) and 2 (16:9) while the live view stays 3:2. GR III
/// implements only live-view modes 0 (3:2) and 3 (1:1).
pub const FRAMING_PATCHES: &[(u32, &str, &str)] = &[
    // Live-view mode table for capture ids 1, 2, 3 (read at 0x53669424): 0, 0, 3.
    (0x53d553c8, "02010300", "00000300"),
    // Second mode lookup at 0x53437170: `bhi` -> `bne`, so only id 3 reads its
    // table. Without it the live view times out after a 4:3 or 16:9 capture.
    (0x534371d8, "1500008a", "1500001a"),
    // Pipeline row 0x536a3200: return row 0 for ids 1 and 2 without the
    // ModelDefs.cpp:115 assertion, as the stock fallback does.
    (0x536a3228, "f83b0ae3", "080000ea"),
    // AF and touch image rectangle 0x535ee890: ids 1 and 2 use the 3:2
    // rectangle. Stock writes zeros for id 2 and leaves id 1 stale.
    (0x535ee8b4, "0c00000a", "0300000a"),
    (0x535ee8c0, "000052e3", "020052e1"),
];

pub const VIEWFINDER_HOOKS: &[(u32, &str, &str, bool)] =
    &[(0x531710c4, "menu_viewfinder_icon", "97e204eb", true)];

/// Image Control tag for GRKit photos: capture writes the effect code, and the
/// playback DISP page shows the effect icon.
pub const METADATA_HOOKS: &[(u32, &str, &str, bool)] = &[
    // PentaxMakernoteEntryDataSetter, tag 0x4f (ImageTone).
    (0x53608080, "menu_capture_tone", "e1fdfeeb", true),
    // Playback DISP page Image Control icon.
    (0x53179b7c, "menu_playback_icon", "daf404eb", true),
    // Playback parameter detail: render GRKit photos as Standard.
    (0x532b7cfc, "menu_playback_detail", "eef1ffeb", true),
];

/// In-place instruction changes: (address, before, after).
pub const INSTRUCTION_PATCHES: &[(u32, &str, &str)] = &[
    // ImageTone extraction stores 0x10000 | (tag - 0x100) instead of 0 for
    // non-native tags, so playback can match a GRKit code. Native getters still
    // accept only 1..12.
    (0x535c251c, "0020a083", "01288083"),
];

pub struct Component {
    pub patches: Vec<Patch>,
    pub relocations: Vec<usize>,
    pub symbols: BTreeMap<String, u32>,
    pub code_size: usize,
}

fn code_component(
    code: Vec<u8>,
    symbols: BTreeMap<String, u32>,
    body: &FirmwareImage,
    base: u32,
    end: u32,
) -> Result<Component> {
    ensure!(
        u64::from(base) + code.len() as u64 <= u64::from(end),
        "Private component exceeds its reserved region ({} bytes, limit {})",
        code.len(),
        end - base
    );
    let image_end = crate::target::RTOS_BASE + body.body.len() as u32;
    if base >= image_end {
        // The decoded image does not contain the tail. The installer checks
        // that the running camera has zeros there.
        ensure!(
            end <= RTOS_EXECUTABLE_END,
            "Private component is outside the RTOS executable mapping"
        );
    } else {
        ensure!(
            body.read(base, code.len())?.iter().all(|b| *b == 0),
            "Private component reservation is occupied"
        );
    }
    let relocations: Vec<_> = code
        .chunks_exact(4)
        .enumerate()
        .filter_map(|(i, word)| (word == HEADER_REFERENCE.to_le_bytes()).then_some(i * 4))
        .collect();
    ensure!(!relocations.is_empty(), "Missing private header literal");
    let code_size = code.len();
    Ok(Component {
        patches: vec![Patch {
            address: base,
            before: vec![0; code_size],
            after: code,
        }],
        relocations,
        symbols,
        code_size,
    })
}

pub fn processing(
    directory: &Path,
    runtime: &Path,
    body: &FirmwareImage,
    tools: &Toolchain,
) -> Result<Component> {
    for &(address, expected) in CONTEXTS {
        ensure!(
            body.read(address, expected.len() / 2)? == hex::decode(expected)?,
            "Private processing instructions changed at {address:#x}"
        );
    }
    let original = hex::decode(PROCESSING_ORIGINAL)?
        .chunks_exact(4)
        .map(|bytes| {
            format!(
                ".word 0x{:08x}",
                u32::from_le_bytes(bytes.try_into().unwrap())
            )
        })
        .collect::<Vec<_>>()
        .join("\n    ");
    let assembly = format!(
        ".equ HEADER, 0x{HEADER_REFERENCE:x}\n{}",
        include_str!("../../../runtime/processing.S").replace("@ ORIGINAL_PROCESSING", &original)
    );
    let source = directory.join("module-processing.S");
    let object = directory.join("module-processing.o");
    fs::write(&source, assembly)?;
    run(
        &tools.clang,
        [
            "--target=arm-none-eabi",
            "-march=armv7-a",
            "-marm",
            "-I",
            path(runtime)?,
            "-c",
            path(&source)?,
            "-o",
            path(&object)?,
        ],
    )?;
    let (code, symbols) = elf_text(&fs::read(object)?)?;
    let mut component = code_component(code, symbols, body, PROCESSING_BASE, MENU_BASE)?;
    for &(address, name, link) in PROCESSING_HOOKS {
        let target = PROCESSING_BASE
            .checked_add(
                *component
                    .symbols
                    .get(name)
                    .with_context(|| format!("Missing symbol {name}"))?,
            )
            .context("Symbol overflow")?;
        let mut instruction = u32::from_le_bytes(branch(address, target, 14)?);
        if link {
            instruction |= 0x01000000;
        }
        component.patches.push(Patch {
            address,
            before: body.read(address, 4)?.to_vec(),
            after: instruction.to_le_bytes().to_vec(),
        });
    }
    Ok(component)
}

pub fn menu(
    directory: &Path,
    runtime: &Path,
    body: &FirmwareImage,
    tools: &Toolchain,
) -> Result<Component> {
    let source = directory.join("module-menu.S");
    let object = directory.join("module-menu.o");
    let assembly_object = directory.join("module-menu-asm.o");
    fs::write(&source, include_str!("../../../runtime/menu.S"))?;
    run(
        &tools.clang,
        [
            "--target=arm-none-eabi",
            "-march=armv7-a",
            // Thumb-2 for size. menu.c keeps its firmware entry points in A32.
            "-mthumb",
            "-Oz",
            "-ffreestanding",
            "-fno-builtin",
            "-fno-stack-protector",
            "-fno-unwind-tables",
            "-fno-asynchronous-unwind-tables",
            "-c",
            path(&runtime.join("menu.c"))?,
            "-o",
            path(&object)?,
        ],
    )?;
    run(
        &tools.clang,
        [
            "--target=arm-none-eabi",
            "-march=armv7-a",
            "-marm",
            "-c",
            path(&source)?,
            "-o",
            path(&assembly_object)?,
        ],
    )?;
    let mut component = linked_component(
        directory,
        "module-menu",
        &[object, assembly_object],
        body,
        tools,
        MENU_BASE,
        0x53004000,
    )?;
    let hooks = MENU_HOOKS
        .iter()
        .map(|&(a, n, b)| (a, n, b, false))
        .chain(VIEWFINDER_HOOKS.iter().copied())
        .chain(METADATA_HOOKS.iter().copied());
    add_patches(&mut component, body, MENU_BASE, INSTRUCTION_PATCHES, hooks)?;
    Ok(component)
}

pub fn framing(
    directory: &Path,
    runtime: &Path,
    body: &FirmwareImage,
    tools: &Toolchain,
) -> Result<Component> {
    let object = directory.join("framing.o");
    let assembly_object = directory.join("framing-asm.o");
    for (source, object) in [("framing.c", &object), ("framing.S", &assembly_object)] {
        run(
            &tools.clang,
            [
                "--target=arm-none-eabi",
                "-march=armv7-a",
                "-marm",
                "-Os",
                "-ffreestanding",
                "-fno-builtin",
                "-fno-stack-protector",
                "-fno-unwind-tables",
                "-fno-asynchronous-unwind-tables",
                "-I",
                path(runtime)?,
                "-c",
                path(&runtime.join(source))?,
                "-o",
                path(object)?,
            ],
        )?;
    }
    let mut component = linked_component(
        directory,
        "module-framing",
        &[object, assembly_object],
        body,
        tools,
        FRAMING_BASE,
        FRAMING_END,
    )?;
    // The capture hook changes nothing until a Framing ratio is selected, so
    // the data patches can follow it.
    add_patches(
        &mut component,
        body,
        FRAMING_BASE,
        FRAMING_PATCHES,
        FRAMING_HOOKS.iter().copied(),
    )?;
    Ok(component)
}

fn linked_component(
    directory: &Path,
    name: &str,
    objects: &[std::path::PathBuf],
    body: &FirmwareImage,
    tools: &Toolchain,
    base: u32,
    end: u32,
) -> Result<Component> {
    let script = directory.join(format!("{name}.ld"));
    fs::write(&script, format!("SECTIONS {{ . = {base:#x}; .text : {{ *(.text*) *(.rodata*) }} /DISCARD/ : {{ *(.ARM.exidx*) *(.comment*) }} }}"))?;
    let elf = directory.join(format!("{name}.elf"));
    let mut arguments = vec![
        "-T".to_owned(),
        path(&script)?.to_owned(),
        "-o".to_owned(),
        path(&elf)?.to_owned(),
    ];
    for object in objects {
        arguments.push(path(object)?.to_owned());
    }
    run(&tools.lld, arguments.iter().map(String::as_str))?;
    let (code, absolute) = elf_text(&fs::read(elf)?)?;
    let symbols = absolute
        .into_iter()
        .map(|(symbol, value)| {
            Ok((
                symbol,
                value
                    .checked_sub(base)
                    .context("Component symbol outside code")?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    code_component(code, symbols, body, base, end)
}

/// Append entry branches into the component code, then in-place changes.
fn add_patches<'a>(
    component: &mut Component,
    body: &FirmwareImage,
    base: u32,
    instructions: &[(u32, &str, &str)],
    hooks: impl Iterator<Item = (u32, &'a str, &'a str, bool)>,
) -> Result<()> {
    for (address, name, expected, link) in hooks {
        let before = hex::decode(expected)?;
        ensure!(
            body.read(address, 4)? == before,
            "Private hook instruction differs at {address:#x}"
        );
        let target = base
            .checked_add(
                *component
                    .symbols
                    .get(name)
                    .with_context(|| format!("Missing symbol {name}"))?,
            )
            .context("Symbol overflow")?;
        let mut instruction = u32::from_le_bytes(branch(address, target, 14)?);
        if link {
            instruction |= 0x01000000;
        }
        component.patches.push(Patch {
            address,
            before,
            after: instruction.to_le_bytes().to_vec(),
        });
    }
    for &(address, before, after) in instructions {
        let before = hex::decode(before)?;
        ensure!(
            body.read(address, before.len())? == before,
            "Private instruction differs at {address:#x}"
        );
        component.patches.push(Patch {
            address,
            before,
            after: hex::decode(after)?,
        });
    }
    Ok(())
}

pub(crate) fn path(path: &Path) -> Result<&str> {
    path.to_str().context("Toolchain path is not UTF-8")
}
