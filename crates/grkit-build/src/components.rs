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
/// The startup literals for the `.data` copy: source, destination, and size.
/// Source plus size is the end of the RTOS image.
const GR3_GUARDS: &[(u32, &str)] = &[(0x53000450, "40b507540000005580a00300")];
// Instructions replaced by the processing hook and replayed on its stock path.
const PROCESSING_ORIGINAL: &str =
    "940d08e3f0cc08e3010545e301c545e37b9f86e201e190e74c0e08e3010545e3";
const PROCESSING_HOOKS: &[(u32, &str, bool)] = &[
    (0x536dc124, "module_parameters", true),
    (0x536dc28c, "module_processing", false),
    (0x536dc68c, "module_packet", false),
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

/// Effect extension and picture processing (`runtime/picture.*`), linked into the Framing region.
pub const PICTURE_HOOKS: &[(u32, &str, &str, bool)] = &[
    // Live-view diff: `ldrb r3, [r4, #0xe]` before the gamma stage flag.
    (0x536de1a8, "module_gamma_dirty", "0e30d4e5", false),
    // YCbCrMatrix Make: the call to its calc. Applies the post-curve matrix.
    (0x536e4774, "picture_post", "e9ffffeb", true),
    // JpegEncodeMacroProcess::Execute entry: the date stamp before the encode.
    (0x535047d4, "picture_encode_entry", "0dc0a0e1", false),
    // PentaxMakernoteEntryDataSetter, tag 0x4f (ImageTone): the effect code of
    // the shot.
    (0x53608080, "picture_capture_tone", "e1fdfeeb", true),
];

pub const VIEWFINDER_HOOKS: &[(u32, &str, &str, bool)] =
    &[(0x531710c4, "menu_viewfinder_icon", "97e204eb", true)];

/// Image Control tag for GRKit photos: the playback DISP page shows the effect
/// icon. Capture writes the effect code (`picture_capture_tone`).
pub const METADATA_HOOKS: &[(u32, &str, &str, bool)] = &[
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

/// A hook: (address, symbol, stock instruction, BL instead of B).
pub type Hook = (u32, &'static str, &'static str, bool);
/// An in-place change: (address, stock bytes, new bytes).
pub type InstructionPatch = (u32, &'static str, &'static str);

/// Where a target's components live and where they hook the firmware.
#[derive(Debug, PartialEq, Eq)]
pub struct Layout {
    /// The preprocessor definition that selects `runtime/targets/<target>.h`
    /// in `runtime/firmware.h`.
    pub define: &'static str,
    /// Module bits (`GRKIT_HOST_HIDDEN`) that the target does not implement.
    pub hidden: u32,
    /// Image Control processing, if the target has it: its region and its
    /// assembly source.
    pub processing: Option<(u32, u32)>,
    pub processing_source: &'static str,
    pub menu: (u32, u32),
    /// Framing, Guides, and picture code (`runtime/framing.*`, `runtime/picture.*`).
    pub framing: Option<(u32, u32)>,
    pub contexts: &'static [(u32, &'static str)],
    pub processing_hooks: &'static [(u32, &'static str, bool)],
    /// Menu entry hooks (B), then the viewfinder and metadata hooks.
    pub menu_hooks: &'static [(u32, &'static str, &'static str)],
    pub viewfinder_hooks: &'static [Hook],
    pub metadata_hooks: &'static [Hook],
    pub instruction_patches: &'static [InstructionPatch],
    pub framing_hooks: &'static [Hook],
    pub picture_hooks: &'static [Hook],
    pub framing_patches: &'static [InstructionPatch],
    /// Stock bytes that a statically patched firmware changes when it grows the
    /// RTOS image into the private tail regions. The installer checks them
    /// before it writes anything.
    pub guards: &'static [(u32, &'static str)],
}

pub const GR3_LAYOUT: Layout = Layout {
    define: "-DGRKIT_TARGET_GR3_210",
    hidden: 0,
    processing: Some((PROCESSING_BASE, MENU_BASE)),
    processing_source: include_str!("../../../runtime/processing.S"),
    menu: (MENU_BASE, 0x53004000),
    framing: Some((FRAMING_BASE, FRAMING_END)),
    contexts: CONTEXTS,
    processing_hooks: PROCESSING_HOOKS,
    menu_hooks: MENU_HOOKS,
    viewfinder_hooks: VIEWFINDER_HOOKS,
    metadata_hooks: METADATA_HOOKS,
    instruction_patches: INSTRUCTION_PATCHES,
    framing_hooks: FRAMING_HOOKS,
    picture_hooks: PICTURE_HOOKS,
    framing_patches: FRAMING_PATCHES,
    guards: GR3_GUARDS,
};

/// GR IV 1.11 menu hooks. The menu code follows the GR IV boot
/// header, which ends at 0x530021a8.
const GR4_MENU_HOOKS: &[(u32, &str, &str)] = &[
    (0x531bf9d8, "menu_native_touch_0", "fc31d0e5"),
    (0x531bf9ec, "menu_native_touch_1", "fc31d0e5"),
    (0x531bfa00, "menu_native_touch_2", "fc31d0e5"),
    (0x531bfa14, "menu_native_touch_3", "fc31d0e5"),
    (0x531bfa28, "menu_native_touch_4", "fc31d0e5"),
    (0x531ba1c0, "menu_destroy", "0dc0a0e1"),
    (0x531bf3c8, "menu_refresh", "0dc0a0e1"),
    (0x531bf834, "menu_event", "b030d1e1"),
    (0x531bf580, "menu_exit", "fc11d0e5"),
    (0x531bf38c, "menu_changed", "0dc0a0e1"),
    (0x53572b00, "menu_text_measure", "077193e7"),
    (0x5357350c, "menu_text_draw", "073193e7"),
    // ADJ Image Control item apply (item 15, +0x28), ImageControlController
    // apply, event, and exit, as on GR III.
    (0x531badd4, "menu_native_adj_apply", "0dc0a0e1"),
    (0x531c5894, "menu_native_dialog_apply", "0dc0a0e1"),
    (0x531c635c, "menu_native_page_event", "0dc0a0e1"),
    (0x531c4fb8, "menu_native_page_exit", "0dc0a0e1"),
    // Icon lookup entry, then the limit check and the table read of the icon draw.
    (0x5323db68, "menu_icon_lookup", "0dc0a0e1"),
    (0x5323e434, "menu_icon_limit", "020057e1"),
    (0x5323e4d8, "menu_icon_entry", "073190e7"),
];

/// GR IV 1.11 Framing and Guides hooks: LiveViewController update, exit, and
/// destroy, then Canvas::Draw entry and its child-draw BL. XPan crops native
/// 16:9, so GR IV needs no capture request hook and no live-view patches.
const GR4_FRAMING_HOOKS: &[Hook] = &[
    (0x5318e350, "framing_update", "0dc0a0e1", false),
    (0x5318e984, "framing_exit", "0dc0a0e1", false),
    (0x531879e0, "framing_destroy", "0dc0a0e1", false),
    (0x5356dde4, "framing_canvas", "0dc0a0e1", false),
    (0x5356deb0, "framing_children", "6b0800eb", true),
    // ImageResolutionTranslatorKb636 (the GR IV sizes) GetResolution id 2
    // heights. The Kb588 class holds the GR III sizes and is not used.
    (0x538cfff8, "framing_height_0", "983d00e3", true),
    (0x538cffe4, "framing_height_1", "ae3ea0e3", true),
    (0x538cffcc, "framing_height_2", "7b3ea0e3", true),
    (0x538cffb8, "framing_height_3", "383400e3", true),
    // Id 2 top insets of the screennail and the thumbnail.
    (0x538d00fc, "framing_screennail_inset", "2610a0e3", true),
    (0x538d01b4, "framing_thumbnail_inset", "1010a0e3", true),
    // Playback size classifier and image aspect getter.
    (0x533a2850, "framing_classify", "6dfcffeb", true),
    (0x533a2c58, "framing_image_aspect", "6bfbffea", false),
];

/// GR IV 1.11 picture hooks, as `PICTURE_HOOKS` on GR III: the live-view diff,
/// the YCbCr matrix calc call, and the JpegEncodeMacroProcess::Execute entry.
const GR4_PICTURE_HOOKS: &[Hook] = &[
    (0x5391410c, "module_gamma_dirty", "0e30d4e5", false),
    (0x5391a6a4, "picture_post", "e9ffffeb", true),
    (0x537034b4, "picture_encode_entry", "0dc0a0e1", false),
    // The ImageTone call of the maker-note setter.
    (0x53810804, "picture_capture_tone", "1a37ffeb", true),
];

const GR4_VIEWFINDER_HOOKS: &[Hook] = &[(0x53187f90, "menu_viewfinder_icon", "2e3d08eb", true)];

/// GR IV 1.11 Image Control tag, as `METADATA_HOOKS` on GR III: the playback
/// DISP icon and the playback parameter detail.
const GR4_METADATA_HOOKS: &[Hook] = &[
    (0x531916c0, "menu_playback_icon", "5a5408eb", true),
    (0x533a7614, "menu_playback_detail", "7af0ffeb", true),
];

/// GR IV ImageTone extraction compares each native tag value. Its default
/// case stores 0x10000 | (tag - 0x100), as on GR III, instead of 0.
const GR4_INSTRUCTION_PATCHES: &[InstructionPatch] = &[(0x537d8a54, "0020a0e3", "ff2c80e2")];

/// GR IV 1.11 Image Control processing (`runtime/targets/gr4-1.11-processing.S`).
const GR4_PROCESSING_HOOKS: &[(u32, &str, bool)] = &[
    (0x53912c70, "module_parameters", true),
    (0x53912d40, "module_processing", true),
    (0x53912d4c, "module_record", true),
    (0x53913188, "module_packet", true),
    (0x5391e360, "module_gamma_flag", true),
    (0x5391e76c, "module_gamma_curves", true),
];
/// The stock instructions around the GR IV processing hooks.
const GR4_CONTEXTS: &[(u32, &str)] = &[
    // Normalizer BL, then the type-table getters and the record store.
    (0x53912c70, "d8f9ffeb"),
    (0x53912d3c, "34a01be50a10a0e10600a0e18d4c00eb080084e5"),
    // Packet: the last field store before the native tail.
    (0x53913184, "943195e5903084e5255000eb"),
    // Gamma stage: the custom-curve flag and the curve getter BL.
    (0x5391e360, "9b10d1e5"),
    (0x5391e76c, "231a00eb"),
    // LiveViewController's Image Control widget: load native type, map, assign.
    (0x53187f88, "ec3494e5401ad3e52e3d08eb"),
];

/// The startup `.data` literals, as on GR III, and the icon catalog getter
/// (`movw r0` / `movt r0`, 0x55013410), which patched firmware moves into the
/// RTOS tail when it adds icons.
const GR4_GUARDS: &[(u32, &str)] = &[
    (0x53000450, "40b637540000005580740500"),
    (0x533ef604, "100403e3010545e3"),
];

pub const GR4_LAYOUT: Layout = Layout {
    define: "-DGRKIT_TARGET_GR4_111",
    hidden: 0,
    // After the Framing region.
    processing: Some((0x543db000, 0x543dc000)),
    processing_source: include_str!("../../../runtime/targets/gr4-1.11-processing.S"),
    menu: (0x530021c0, 0x53004000),
    // After the decoded RTOS image (0x543d2ac0), like GR III.
    framing: Some((0x543d3000, 0x543db000)),
    contexts: GR4_CONTEXTS,
    processing_hooks: GR4_PROCESSING_HOOKS,
    menu_hooks: GR4_MENU_HOOKS,
    viewfinder_hooks: GR4_VIEWFINDER_HOOKS,
    metadata_hooks: GR4_METADATA_HOOKS,
    instruction_patches: GR4_INSTRUCTION_PATCHES,
    framing_hooks: GR4_FRAMING_HOOKS,
    picture_hooks: GR4_PICTURE_HOOKS,
    framing_patches: &[],
    guards: GR4_GUARDS,
};

/// Every hook of a layout: (address, symbol).
fn hook_sites(layout: &Layout) -> impl Iterator<Item = (u32, &'static str)> + '_ {
    layout
        .processing_hooks
        .iter()
        .map(|&(address, name, _)| (address, name))
        .chain(
            layout
                .menu_hooks
                .iter()
                .map(|&(address, name, _)| (address, name)),
        )
        .chain(
            layout
                .viewfinder_hooks
                .iter()
                .chain(layout.metadata_hooks)
                .chain(layout.framing_hooks)
                .chain(layout.picture_hooks)
                .map(|&(address, name, _, _)| (address, name)),
        )
}

/// `hooks.h` for the assembly sources: for each hook, its address, the address
/// after it, the replaced instruction, and the target of a replaced branch.
pub fn hooks_header(body: &FirmwareImage) -> Result<String> {
    use std::fmt::Write;
    let mut result = format!("/* Generated for {}. */\n", body.target.name);
    let Some(layout) = body.target.layout else {
        return Ok(result);
    };
    for (address, name) in hook_sites(layout) {
        let word = u32::from_le_bytes(body.read(address, 4)?.try_into()?);
        let name = name.to_ascii_uppercase();
        writeln!(result, "#define HOOK_{name} {address:#x}")?;
        writeln!(result, "#define HOOK_{name}_RESUME {:#x}", address + 4)?;
        writeln!(result, "#define HOOK_{name}_STOCK {word:#010x}")?;
        if (word >> 25) & 7 == 5 && word >> 28 != 0xf {
            let offset = ((word << 8) as i32 >> 6) as u32;
            let target = address.wrapping_add(8).wrapping_add(offset);
            writeln!(result, "#define HOOK_{name}_TARGET {target:#x}")?;
        }
    }
    Ok(result)
}

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
        // The decoded image does not contain the tail, and the installer does
        // not compare it: BOOT loads only the image, so the tail can hold the
        // previous session's code. This check uses the GR III executable
        // mapping, which is not confirmed on other targets.
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
    layout: &Layout,
    (base, end): (u32, u32),
) -> Result<Component> {
    for &(address, expected) in layout.contexts {
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
        layout
            .processing_source
            .replace("@ ORIGINAL_PROCESSING", &original)
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
        ]
        .into_iter()
        .chain([layout.define]),
    )?;
    let (code, symbols) = elf_text(&fs::read(object)?)?;
    let mut component = code_component(code, symbols, body, base, end)?;
    for &(address, name, link) in layout.processing_hooks {
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
    layout: &Layout,
) -> Result<Component> {
    let (base, end) = layout.menu;
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
        ]
        .into_iter()
        .chain([layout.define]),
    )?;
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
            path(&assembly_object)?,
        ]
        .into_iter()
        .chain([layout.define]),
    )?;
    let mut component = linked_component(
        directory,
        "module-menu",
        &[object, assembly_object],
        body,
        tools,
        base,
        end,
    )?;
    let hooks = layout
        .menu_hooks
        .iter()
        .map(|&(a, n, b)| (a, n, b, false))
        .chain(layout.viewfinder_hooks.iter().copied())
        .chain(layout.metadata_hooks.iter().copied());
    add_patches(
        &mut component,
        body,
        base,
        layout.instruction_patches,
        hooks,
    )?;
    Ok(component)
}

pub fn framing(
    directory: &Path,
    runtime: &Path,
    body: &FirmwareImage,
    tools: &Toolchain,
    layout: &Layout,
    (base, end): (u32, u32),
) -> Result<Component> {
    let object = directory.join("framing.o");
    let assembly_object = directory.join("framing-asm.o");
    let picture = directory.join("picture.o");
    let picture_assembly = directory.join("picture-asm.o");
    for (source, object) in [
        ("framing.c", &object),
        ("framing.S", &assembly_object),
        ("picture.c", &picture),
        ("picture.S", &picture_assembly),
    ] {
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
            ]
            .into_iter()
            .chain([layout.define]),
        )?;
    }
    let mut component = linked_component(
        directory,
        "module-framing",
        &[object, assembly_object, picture, picture_assembly],
        body,
        tools,
        base,
        end,
    )?;
    // The capture hook changes nothing until a Framing ratio is selected, so
    // the data patches can follow it.
    add_patches(
        &mut component,
        body,
        base,
        layout.framing_patches,
        layout
            .framing_hooks
            .iter()
            .chain(layout.picture_hooks)
            .copied(),
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
