use crate::{
    arm::{branch, elf_text_relocations, undefined_symbols},
    firmware::FirmwareImage,
    target::{Target, HEADER_REFERENCE},
    toolchain::{run, Toolchain, CAMERA_TARGET},
    Anchor, Patch, Reference,
};
use anyhow::{bail, ensure, Context, Result};
use std::{collections::BTreeMap, fs, path::Path};

/// The RTOS maps its tail read-only and executable up to here.
pub const RTOS_EXECUTABLE_END: u32 = 0x55000000;

pub use grkit_core::capabilities::Point;

/// The code component that holds the code of a hook and installs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// Image Control processing (`Family::processing_source`).
    Processing,
    /// The private menu (`runtime/menu.*`).
    Menu,
    /// Framing, Guides, and picture code (`runtime/framing.*`, `runtime/picture.*`).
    Framing,
}

/// A hook that each build of a family installs: the symbol of its code, the
/// component that holds that code, the extension points that need it, and
/// whether its site gets a BL instead of a B.
#[derive(Debug, PartialEq, Eq)]
pub struct HookSpec {
    pub symbol: &'static str,
    pub part: Part,
    pub point: Point,
    pub also: Option<Point>,
    pub link: bool,
}

impl HookSpec {
    /// Install the hook when either extension point is active.
    pub fn point_mask(&self) -> u32 {
        self.point.bit() | self.also.map_or(0, Point::bit)
    }
}

/// A hook whose site gets a B to its code.
pub const fn b(symbol: &'static str, part: Part, point: Point) -> HookSpec {
    HookSpec {
        symbol,
        part,
        point,
        also: None,
        link: false,
    }
}

/// A hook whose site gets a BL to its code.
pub const fn bl(symbol: &'static str, part: Part, point: Point) -> HookSpec {
    HookSpec {
        link: true,
        ..b(symbol, part, point)
    }
}

/// A B hook shared by two extension points.
pub const fn b_shared(symbol: &'static str, part: Part, point: Point, also: Point) -> HookSpec {
    HookSpec {
        also: Some(also),
        ..b(symbol, part, point)
    }
}

/// A BL hook shared by two extension points.
pub const fn bl_shared(symbol: &'static str, part: Part, point: Point, also: Point) -> HookSpec {
    HookSpec {
        also: Some(also),
        ..bl(symbol, part, point)
    }
}

/// The site of a hook in one build: its symbol, address, and stock instruction.
pub type Site = (&'static str, u32, &'static str);
/// An in-place change: (address, stock bytes, new bytes).
pub type InstructionPatch = (u32, &'static str, &'static str);

/// A hook of one build: the hook of the family at its site in the build.
#[derive(Debug, Clone, Copy)]
pub struct Hook {
    pub spec: &'static HookSpec,
    pub address: u32,
    pub stock: &'static str,
}

/// Each hook of a target in installation order. The layout gives exactly one
/// site for each hook of the family.
pub fn hooks(target: &Target) -> Result<Vec<Hook>> {
    let layout = target.layout;
    let family = target.family;
    let hooks = family
        .hooks
        .iter()
        .map(|spec| {
            let mut sites = layout.sites.iter().filter(|site| site.0 == spec.symbol);
            let &(_, address, stock) = sites
                .next()
                .with_context(|| format!("{} has no site for {}", target.name, spec.symbol))?;
            ensure!(
                sites.next().is_none(),
                "{} has two sites for {}",
                target.name,
                spec.symbol
            );
            Ok(Hook {
                spec,
                address,
                stock,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if let Some((symbol, ..)) = layout
        .sites
        .iter()
        .find(|site| !family.hooks.iter().any(|spec| spec.symbol == site.0))
    {
        bail!(
            "{} has a site for {symbol}, which is no hook of the {} family",
            target.name,
            family.name
        );
    }
    Ok(hooks)
}

/// The first 256 bytes of the tail stay unused. Components follow their
/// predecessors at 256-byte boundaries and share the executable mapping.
const PROCESSING_START: u32 = 0x100;
pub(crate) const REGION_ALIGN: u32 = 0x100;

pub(crate) fn aligned_code_end(end: u32) -> Result<u32> {
    let aligned = end
        .checked_add(REGION_ALIGN - 1)
        .context("Private code alignment overflows")?
        & !(REGION_ALIGN - 1);
    ensure!(
        aligned <= RTOS_EXECUTABLE_END,
        "Private code exceeds the executable mapping"
    );
    Ok(aligned)
}

/// Where a target's components live and where they hook the firmware.
#[derive(Debug, PartialEq, Eq)]
pub struct Layout {
    /// The preprocessor definition that selects `runtime/targets/<target>.h`
    /// in `runtime/firmware.h`.
    pub define: &'static str,
    /// The base of the private tail after the decoded RTOS image. Components
    /// share its read-only executable mapping through RTOS_EXECUTABLE_END.
    /// No firmware code refers to it. BOOT does not reload it, and nothing clears it: after
    /// power-on, a GR III held the noise of uninitialized DRAM there. The
    /// installer overwrites it without a stock-byte check. The guards detect a
    /// firmware that extends the image into it.
    pub tail: u32,
    /// Stock instructions around the processing hooks that their code relies
    /// on, which the build checks.
    pub contexts: &'static [(u32, &'static str)],
    /// The site of each hook of the family in this build.
    pub sites: &'static [Site],
    /// In-place changes of the menu, for tuning.
    pub instruction_patches: &'static [InstructionPatch],
    /// In-place changes of Framing, for capture.aspect.
    pub framing_patches: &'static [InstructionPatch],
    /// Stock bytes that a statically patched firmware changes when it grows the
    /// RTOS image into the private tail regions. The installer checks them
    /// before it writes anything.
    pub guards: &'static [(u32, &'static str)],
}

impl Layout {
    /// The first component starts after the reserved header space.
    pub fn processing(&self) -> Result<(u32, u32)> {
        self.region(self.tail)
    }

    /// The menu follows the actual Processing payload, if present.
    pub fn menu(&self, processing_end: u32) -> Result<(u32, u32)> {
        self.region(processing_end)
    }

    /// Framing follows the actual menu payload.
    pub fn framing(&self, menu_end: u32) -> Result<(u32, u32)> {
        self.region(menu_end)
    }

    fn region(&self, previous_end: u32) -> Result<(u32, u32)> {
        let first = self
            .tail
            .checked_add(PROCESSING_START)
            .context("Private tail start overflows")?;
        let start = aligned_code_end(previous_end.max(first))?;
        ensure!(
            start < RTOS_EXECUTABLE_END,
            "No executable memory remains for a component"
        );
        Ok((start, RTOS_EXECUTABLE_END))
    }
}

/// `hooks.h` for the assembly sources: for each hook, its address, the address
/// after it, the replaced instruction, and the target of a replaced branch.
pub fn hooks_header(body: &FirmwareImage) -> Result<String> {
    use std::fmt::Write;
    let mut result = format!("/* Generated for {}. */\n", body.target.name);
    // The installer relocates the private code to the running build of the
    // firmware version (`crate::relocation`), so the code names each firmware
    // address with `FIRMWARE()` (runtime/firmware.h).
    let value = |address: u32| format!("FIRMWARE({address:#x})");
    for hook in hooks(body.target)? {
        let address = hook.address;
        let word = u32::from_le_bytes(body.read(address, 4)?.try_into()?);
        let name = hook.spec.symbol.to_ascii_uppercase();
        writeln!(result, "#define HOOK_{name} {address:#x}")?;
        writeln!(result, "#define HOOK_{name}_RESUME {}", value(address + 4))?;
        writeln!(result, "#define HOOK_{name}_STOCK {word:#010x}")?;
        if let Some(target) = crate::relocation::branch_target(address, word) {
            writeln!(result, "#define HOOK_{name}_TARGET {}", value(target))?;
        }
    }
    Ok(result)
}

/// Stack frame limits of the private C code, which runs on the stacks of
/// firmware tasks. The largest frame is `blur_line` of `picture.c` (1344 bytes).
const MENU_FRAME_LIMIT: &str = "-Wframe-larger-than=256";
const FRAMING_FRAME_LIMIT: &str = "-Wframe-larger-than=1536";
/// A static function that a shared header declares and a component uses but
/// does not define, such as `aspect_native` (runtime/aspect.h), fails the build.
const UNDEFINED_INTERNAL: &str = "-Werror=undefined-internal";

pub struct Component {
    pub patches: Vec<Patch>,
    pub relocations: Vec<usize>,
    /// The firmware and tail references. Patch 0 is the payload.
    pub references: Vec<Reference>,
    pub symbols: BTreeMap<String, u32>,
    pub code_size: usize,
    /// Bytes from this component base to the shared executable mapping limit.
    pub limit: usize,
}

fn code_component(
    code: Vec<u8>,
    symbols: BTreeMap<String, u32>,
    body: &FirmwareImage,
    (base, end): (u32, u32),
    references: Vec<Reference>,
) -> Result<Component> {
    ensure!(
        u64::from(base) + code.len() as u64 <= u64::from(end),
        "Private component exceeds the executable mapping ({} bytes, available {})",
        code.len(),
        end - base
    );
    // The decoded image does not contain the tail (see `TAIL_BASE`).
    let image_end = crate::target::RTOS_BASE + body.body.len() as u32;
    ensure!(
        base >= image_end && end <= RTOS_EXECUTABLE_END,
        "Private component is outside the RTOS tail"
    );
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
            points: 0,
            before: vec![0; code_size],
            after: code,
            // The payload moves with the tail.
            site: Some(Anchor::Tail),
        }],
        relocations,
        references,
        symbols,
        code_size,
        limit: (end - base) as usize,
    })
}

/// The hooks of the target of `body` whose code `part` holds.
fn part_hooks(body: &FirmwareImage, part: Part) -> Result<Vec<Hook>> {
    Ok(hooks(body.target)?
        .into_iter()
        .filter(|hook| hook.spec.part == part)
        .collect())
}

pub fn processing(
    directory: &Path,
    runtime: &Path,
    body: &FirmwareImage,
    tools: &Toolchain,
) -> Result<Component> {
    let layout = body.target.layout;
    let assembly = body
        .target
        .family
        .processing_source
        .with_context(|| format!("The {} family has no processing", body.target.family.name))?;
    let (base, end) = layout.processing()?;
    for &(address, expected) in layout.contexts {
        ensure!(
            body.read(address, expected.len() / 2)? == hex::decode(expected)?,
            "Private processing instructions changed at {address:#x}"
        );
    }
    let source_path = directory.join("module-processing.S");
    let object = directory.join("module-processing.o");
    fs::write(&source_path, assembly)?;
    run(
        &tools.clang,
        CAMERA_TARGET
            .into_iter()
            .chain([
                "-marm",
                "-I",
                path(runtime)?,
                "-c",
                path(&source_path)?,
                "-o",
                path(&object)?,
            ])
            .chain([layout.define]),
    )?;
    // The firmware addresses are symbols, which the link resolves.
    let mut component = linked_component(
        directory,
        "module-processing",
        &[object],
        body,
        tools,
        (base, end),
    )?;
    add_patches(
        &mut component,
        body,
        base,
        (Point::Tuning, &[]),
        part_hooks(body, Part::Processing)?,
    )?;
    Ok(component)
}

pub fn menu(
    directory: &Path,
    runtime: &Path,
    body: &FirmwareImage,
    tools: &Toolchain,
    processing_end: u32,
) -> Result<Component> {
    let layout = body.target.layout;
    let (base, end) = layout.menu(processing_end)?;
    let source = directory.join("module-menu.S");
    let object = directory.join("module-menu.o");
    let assembly_object = directory.join("module-menu-asm.o");
    fs::write(&source, include_str!("../../../runtime/menu.S"))?;
    run(
        &tools.clang,
        CAMERA_TARGET
            .into_iter()
            .chain([
                // Thumb-2 for size. menu.c keeps its firmware entry points in A32.
                "-mthumb",
                "-Oz",
                "-ffreestanding",
                "-fno-builtin",
                "-fno-stack-protector",
                "-fno-unwind-tables",
                "-fno-asynchronous-unwind-tables",
                MENU_FRAME_LIMIT,
                "-Werror=frame-larger-than",
                UNDEFINED_INTERNAL,
                "-c",
                path(&runtime.join("menu.c"))?,
                "-o",
                path(&object)?,
            ])
            .chain(target_flags(layout)),
    )?;
    run(
        &tools.clang,
        CAMERA_TARGET
            .into_iter()
            .chain([
                "-marm",
                "-I",
                path(runtime)?,
                "-c",
                path(&source)?,
                "-o",
                path(&assembly_object)?,
            ])
            .chain([layout.define]),
    )?;
    let mut component = linked_component(
        directory,
        "module-menu",
        &[object, assembly_object],
        body,
        tools,
        (base, end),
    )?;
    add_patches(
        &mut component,
        body,
        base,
        (Point::Tuning, layout.instruction_patches),
        part_hooks(body, Part::Menu)?,
    )?;
    Ok(component)
}

/// Framing, linked after a menu whose code ends at `menu_end`.
pub fn framing(
    directory: &Path,
    runtime: &Path,
    body: &FirmwareImage,
    tools: &Toolchain,
    menu_end: u32,
) -> Result<Component> {
    let layout = body.target.layout;
    let (base, end) = layout.framing(menu_end)?;
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
            CAMERA_TARGET
                .into_iter()
                .chain([
                    "-marm",
                    "-Os",
                    "-ffreestanding",
                    "-fno-builtin",
                    "-fno-stack-protector",
                    "-fno-unwind-tables",
                    "-fno-asynchronous-unwind-tables",
                    FRAMING_FRAME_LIMIT,
                    "-Werror=frame-larger-than",
                    UNDEFINED_INTERNAL,
                    "-I",
                    path(runtime)?,
                    "-c",
                    path(&runtime.join(source))?,
                    "-o",
                    path(object)?,
                ])
                .chain(target_flags(layout)),
        )?;
    }
    let mut component = linked_component(
        directory,
        "module-framing",
        &[object, assembly_object, picture, picture_assembly],
        body,
        tools,
        (base, end),
    )?;
    // The capture hook changes nothing until a Framing ratio is selected, so
    // the data patches can follow it.
    add_patches(
        &mut component,
        body,
        base,
        (Point::CaptureAspect, layout.framing_patches),
        part_hooks(body, Part::Framing)?,
    )?;
    Ok(component)
}

/// The preprocessor definition of the target, and no MOVW and MOVT pairs for
/// addresses: a literal word holds each one, so the installer moves it with
/// one addition.
fn target_flags<'a>(layout: &Layout) -> impl Iterator<Item = &'a str> {
    [layout.define, "-mno-movt"].into_iter()
}

/// The prefix of the symbol that `FIRMWARE(address)` names (runtime/firmware.h).
const FIRMWARE_SYMBOL: &str = "fw_0x";

fn linked_component(
    directory: &Path,
    name: &str,
    objects: &[std::path::PathBuf],
    body: &FirmwareImage,
    tools: &Toolchain,
    (base, end): (u32, u32),
) -> Result<Component> {
    use std::fmt::Write;
    let mut script = format!("SECTIONS {{ . = {base:#x}; .text : {{ *(.text*) *(.rodata*) }} /DISCARD/ : {{ *(.ARM.exidx*) *(.comment*) }} }}\n");
    // Each firmware symbol has its address in the pinned image. Other
    // undefined symbols are those of another object, or LLD reports them.
    let mut firmware = std::collections::BTreeSet::new();
    for object in objects {
        for symbol in undefined_symbols(&fs::read(object)?)? {
            if let Some(hex) = symbol.strip_prefix(FIRMWARE_SYMBOL) {
                let address = u32::from_str_radix(hex, 16)
                    .with_context(|| format!("Invalid firmware symbol {symbol}"))?;
                firmware.insert(address);
            }
        }
    }
    for address in firmware {
        writeln!(script, "{FIRMWARE_SYMBOL}{address:x} = {address:#x};")?;
    }
    let script_path = directory.join(format!("{name}.ld"));
    fs::write(&script_path, script)?;
    let elf = directory.join(format!("{name}.elf"));
    // A branch between ARM and Thumb code that cannot interwork goes through
    // a thunk that LLD adds. A position-independent thunk moves with the
    // private code. An absolute one holds its target address in MOVW and MOVT
    // without a relocation record, so the installer cannot move it.
    let mut arguments = vec![
        "--pic-veneer".to_owned(),
        // The relocation records give the references that the installer moves.
        "--emit-relocs".to_owned(),
        "-T".to_owned(),
        path(&script_path)?.to_owned(),
        "-o".to_owned(),
        path(&elf)?.to_owned(),
    ];
    for object in objects {
        arguments.push(path(object)?.to_owned());
    }
    run(&tools.lld, arguments.iter().map(String::as_str))?;
    let (code, absolute, relocations) = elf_text_relocations(&fs::read(elf)?)?;
    if let Some(thunk) = absolute
        .keys()
        .find(|symbol| symbol.contains("ABSLongThunk"))
    {
        anyhow::bail!("{name} has the absolute thunk {thunk}, which the installer cannot move");
    }
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
    let mut references = Vec::new();
    for relocation in relocations {
        // R_ARM_ABS32 and R_ARM_TARGET1 hold an address. Other types against
        // the private code are PC-relative, so they move with it.
        let word = matches!(relocation.kind, 2 | 38);
        let anchor = if relocation.internal {
            if !word {
                ensure!(
                    !matches!(relocation.kind, 5 | 6 | 8 | 43 | 44 | 47 | 48),
                    "Unsupported relocation type {} of {} in {name}",
                    relocation.kind,
                    relocation.symbol
                );
                continue;
            }
            Anchor::Tail
        } else {
            let address = relocation
                .symbol
                .strip_prefix(FIRMWARE_SYMBOL)
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .filter(|address| *address == relocation.value)
                .with_context(|| format!("Unexpected symbol {} in {name}", relocation.symbol))?;
            // R_ARM_CALL and R_ARM_JUMP24: an ARM B or BL to the firmware.
            ensure!(
                word || matches!(relocation.kind, 28 | 29),
                "Unsupported relocation type {} of {} in {name}",
                relocation.kind,
                relocation.symbol
            );
            Anchor::Firmware(address)
        };
        ensure!(relocation.offset % 4 == 0, "Unaligned relocation in {name}");
        references.push(Reference {
            patch: 0,
            offset: relocation.offset as usize,
            branch: !word,
            anchor,
        });
    }
    code_component(code, symbols, body, (base, end), references)
}

/// Append the entry branch of a hook into the component code. The branch
/// moves with its site and with the tail.
fn push_hook(component: &mut Component, base: u32, hook: &Hook, before: Vec<u8>) -> Result<()> {
    let (address, name) = (hook.address, hook.spec.symbol);
    let target = base
        .checked_add(
            *component
                .symbols
                .get(name)
                .with_context(|| format!("Missing symbol {name}"))?,
        )
        .context("Symbol overflow")?;
    let mut instruction = u32::from_le_bytes(branch(address, target, 14)?);
    if hook.spec.link {
        instruction |= 0x01000000;
    }
    component.references.push(Reference {
        patch: component.patches.len(),
        offset: 0,
        branch: true,
        anchor: Anchor::Tail,
    });
    component.patches.push(Patch {
        address,
        points: hook.spec.point_mask(),
        before,
        after: instruction.to_le_bytes().to_vec(),
        site: Some(Anchor::Firmware(address)),
    });
    Ok(())
}

/// Append entry branches into the component code, then in-place changes.
fn add_patches(
    component: &mut Component,
    body: &FirmwareImage,
    base: u32,
    instructions: (Point, &[InstructionPatch]),
    hooks: Vec<Hook>,
) -> Result<()> {
    for hook in &hooks {
        let address = hook.address;
        let before = hex::decode(hook.stock)?;
        ensure!(
            before.len() == 4 && body.read(address, 4)? == before,
            "Private hook instruction differs at {address:#x}"
        );
        push_hook(component, base, hook, before)?;
    }
    let (point, instructions) = instructions;
    for &(address, before, after) in instructions {
        let before = hex::decode(before)?;
        ensure!(
            body.read(address, before.len())? == before,
            "Private instruction differs at {address:#x}"
        );
        component.patches.push(Patch {
            address,
            points: point.bit(),
            before,
            after: hex::decode(after)?,
            site: Some(Anchor::Firmware(address)),
        });
    }
    Ok(())
}

pub(crate) fn path(path: &Path) -> Result<&str> {
    path.to_str().context("Toolchain path is not UTF-8")
}
