//! Build a module file (`grkit_core::grm`) from C sources.
//!
//! A module is Thumb code that includes only `runtime/sdk/grkit.h`. It calls
//! the core through literal words (`-mlong-calls`), because the module memory
//! lies far from the core, out of BL range. It loads each address from a
//! literal word (`-mno-movt`). The installer therefore moves each reference
//! with one word: every kept relocation that is not PC-relative inside the code
//! is an `R_ARM_ABS32` word.

use crate::{
    arm::{defined_symbols, elf_module, undefined_symbols},
    components::path,
    toolchain::{run, Toolchain, CAMERA_TARGET},
};
use anyhow::{bail, ensure, Context, Result};
use grkit_core::{
    grm::{Import, Module, Segment, DATA_LINK},
    language::Language,
};
use std::{
    fmt::Write,
    fs,
    path::{Path, PathBuf},
};

/// The symbol of the module descriptor (`GRKIT_MODULE`).
const DESCRIPTOR: &str = "grkit_module";
/// Each import gets a distinct placeholder address in the link, so a word that
/// refers to an import is known by its value too.
const IMPORT_BASE: u32 = 0xf000_0000;
/// The stack frame limit of module code, which runs on the stacks of firmware
/// tasks. It is that of the core code on the same tasks (the picture code of
/// the framing component).
const FRAME_LIMIT: &str = "-Wframe-larger-than=1536";

/// Compile and link `sources` into the module `id`. `directory` receives the
/// objects; `sdk` holds `grkit.h`.
pub fn build(
    id: &str,
    version: u32,
    sources: &[&Path],
    sdk: &Path,
    directory: &Path,
    tools: &Toolchain,
) -> Result<Module> {
    build_with(id, version, sources, &[sdk], directory, tools)
}

/// `build` with more include folders.
fn build_with(
    id: &str,
    version: u32,
    sources: &[&Path],
    includes: &[&Path],
    directory: &Path,
    tools: &Toolchain,
) -> Result<Module> {
    ensure!(
        grkit_core::grm::valid_id(id),
        "Invalid module ID {id:?}: use 1 to 8 characters A-Z, 0-9, or _"
    );
    ensure!(!sources.is_empty(), "A module needs a source file");
    let mut objects = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let object = directory.join(format!("module-{index}.o"));
        let mut include_flags = Vec::new();
        for include in includes {
            include_flags.push("-I".to_owned());
            include_flags.push(path(include)?.to_owned());
        }
        run(
            &tools.clang,
            CAMERA_TARGET
                .into_iter()
                .chain([
                    "-mthumb",
                    "-Os",
                    "-ffreestanding",
                    "-fno-builtin",
                    "-fno-common",
                    "-fno-stack-protector",
                    "-fno-unwind-tables",
                    "-fno-asynchronous-unwind-tables",
                    "-mlong-calls",
                    "-mno-movt",
                    FRAME_LIMIT,
                    "-Werror=frame-larger-than",
                    "-c",
                    path(source)?,
                    "-o",
                    path(&object)?,
                ])
                .chain(include_flags.iter().map(String::as_str)),
        )?;
        objects.push(object);
    }
    // The imports: the symbols that no object of the module defines.
    let (mut imports, mut defined) = (
        std::collections::BTreeSet::new(),
        std::collections::BTreeSet::new(),
    );
    for object in &objects {
        let bytes = fs::read(object)?;
        imports.extend(undefined_symbols(&bytes)?);
        defined.extend(defined_symbols(&bytes)?);
    }
    imports.retain(|name| !defined.contains(name));
    // A compiler helper, such as a division, would be an import that the core
    // does not export.
    if let Some(name) = imports
        .iter()
        .find(|name| !grkit_core::grm::importable(name))
    {
        bail!("The module uses {name}, which the core does not export (runtime/sdk/grkit.h)");
    }
    let mut script = format!(
        "SECTIONS {{\n . = 0;\n .text : {{ *(.text*) *(.rodata*) *(.data.rel.ro*) . = ALIGN(4); }}\n . = {DATA_LINK:#x};\n .data : {{ *(.data*) . = ALIGN(4); }}\n .bss : {{ *(.bss*) *(COMMON) . = ALIGN(4); }}\n /DISCARD/ : {{ *(.ARM.exidx*) *(.ARM.extab*) *(.comment*) }}\n}}\n"
    );
    let placeholders: Vec<(String, u32)> = imports
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), IMPORT_BASE + index as u32 * 16))
        .collect();
    for (name, address) in &placeholders {
        writeln!(script, "{name} = {address:#x};")?;
    }
    let script_path = directory.join("module.ld");
    fs::write(&script_path, script)?;
    let elf = directory.join("module.elf");
    let mut arguments = vec![
        "--emit-relocs".to_owned(),
        "-T".to_owned(),
        path(&script_path)?.to_owned(),
        "-o".to_owned(),
        path(&elf)?.to_owned(),
    ];
    for object in &objects {
        arguments.push(path(object)?.to_owned());
    }
    run(&tools.lld, arguments.iter().map(String::as_str))?;
    convert(id, version, &fs::read(elf)?, &placeholders)
}

/// The module of a linked ELF file.
fn convert(id: &str, version: u32, elf: &[u8], placeholders: &[(String, u32)]) -> Result<Module> {
    let mut linked = elf_module(elf)?;
    let descriptor = match linked.symbols.get(DESCRIPTOR) {
        Some(&(Segment::Code, offset)) => offset,
        Some(_) => bail!("{DESCRIPTOR} must be constant: define it with GRKIT_MODULE"),
        None => bail!("The module has no {DESCRIPTOR}: define it with GRKIT_MODULE"),
    };
    let mut module = Module {
        id: id.to_owned(),
        version,
        bss_size: linked.bss_size,
        descriptor,
        ..Module::default()
    };
    for relocation in &linked.relocations {
        let word = matches!(relocation.kind, 2 | 38);
        let bytes = match relocation.segment {
            Segment::Code => &mut linked.code,
            Segment::Data => &mut linked.data,
        };
        let offset = relocation.offset as usize;
        if !word {
            // A PC-relative reference moves with the code, if both ends are in
            // it. R_ARM_NONE marks nothing.
            ensure!(
                relocation.kind == 0
                    || (relocation.segment == Segment::Code
                        && relocation.target == Some(Segment::Code)
                        && !matches!(relocation.kind, 43 | 44 | 47 | 48)),
                "Unsupported relocation type {} of {} in the module: build with -mlong-calls and -mno-movt",
                relocation.kind,
                relocation.symbol
            );
            continue;
        }
        ensure!(
            offset.is_multiple_of(4),
            "Unaligned word relocation of {}",
            relocation.symbol
        );
        if relocation.target.is_some() {
            match relocation.segment {
                Segment::Code => module.code_relocations.push(relocation.offset),
                Segment::Data => module.data_relocations.push(relocation.offset),
            }
            continue;
        }
        let (name, address) = placeholders
            .iter()
            .find(|(name, _)| *name == relocation.symbol)
            .with_context(|| format!("Unexpected absolute symbol {}", relocation.symbol))?;
        let value = u32::from_le_bytes(bytes[offset..offset + 4].try_into()?);
        ensure!(
            value == *address,
            "The module refers to {name} with an offset: an import must be a function or a whole object"
        );
        bytes[offset..offset + 4].fill(0);
        module.imports.push(Import {
            name: name.clone(),
            segment: relocation.segment,
            offset: relocation.offset,
        });
    }
    module.code = linked.code;
    module.data = linked.data;
    module.validate()?;
    Ok(module)
}

/// `module.toml`: what a module is, which `grkit module build` turns into its
/// descriptor (`GrkitModule` in `runtime/sdk/grkit.h`).
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub version: u32,
    /// The default place in the GRKit root of ADJ and in the GRKit section
    /// of MENU: a smaller order first, then by ID (`grkit_core::grm`).
    #[serde(default = "default_order")]
    pub order: u32,
    /// The C sources, relative to the manifest. A module with only data,
    /// such as a page of a capability, has none.
    #[serde(default)]
    pub sources: Vec<String>,
    /// The 40 x 40 PNG tiles, relative to the manifest. A choice page shows
    /// tile n for value n.
    #[serde(default)]
    pub tiles: Vec<String>,
    /// Full names of settings of other modules or of the core, MODULE.KEY,
    /// with '?' first for an optional one.
    #[serde(default)]
    pub uses: Vec<String>,
    #[serde(default)]
    pub settings: Vec<SettingSpec>,
    pub page: Option<PageSpec>,
    #[serde(default)]
    pub layers: Vec<LayerSpec>,
    #[serde(default)]
    pub filters: Vec<FilterSpec>,
    pub events: Option<EventsSpec>,
}

fn default_order() -> u32 {
    grkit_core::grm::DEFAULT_ORDER
}

/// The event subscriber of a module (`GrkitSubscriber`).
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventsSpec {
    /// The C function `void handle(u32 event, u32 a, u32 b)`.
    pub handle: String,
    /// The events: `system_ready`, `setting_changed`.
    pub on: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingSpec {
    pub key: String,
    pub values: u32,
    #[serde(default)]
    pub saved: bool,
    #[serde(default)]
    pub public: bool,
}

/// A choice page of one setting: a key of `settings`, or a name of `uses`.
/// Its text is in `text/<language>.toml`.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageSpec {
    pub setting: String,
    /// Row 0 shows the Off icon instead of tile 0.
    #[serde(default)]
    pub off_row: bool,
    /// The rows of the native settings page: 1 to 5 keys of `settings`, the
    /// first the page setting. The text names each in `[settings.<KEY>]`.
    #[serde(default)]
    pub settings: Vec<String>,
    /// The settings page of the module in MENU has an Options row, which turns
    /// each value of the page on or off for ADJ; for a page of CORE.TONE, its
    /// effects (`GRKIT_CHOICE_OPTIONS`).
    #[serde(default)]
    pub options: bool,
}

/// A `picture.plane` filter.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilterSpec {
    /// The C function `void apply(PictureSet *, u32 value)`.
    pub apply: String,
    /// The setting whose value the filter applies: a key of `settings`, or a
    /// name of `uses`.
    pub setting: String,
    /// `frame` or `mark` (runtime/sdk/grkit.h: GRKIT_STAGE_*).
    pub stage: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerSpec {
    /// The C function `void paint(const Surface *, u32 value)`.
    pub paint: String,
    /// The setting whose value the layer draws: a key of `settings`, or a
    /// name of `uses`. Without it, the value is 1.
    pub setting: Option<String>,
    pub order: u32,
    /// Off while DISP hides the native information.
    #[serde(default)]
    pub follow_disp: bool,
    /// On only while DISP shows the full information.
    #[serde(default)]
    pub full_disp: bool,
    /// Not on the ADJ canvas.
    #[serde(default)]
    pub live_only: bool,
}

/// `text/<language>.toml`: the text of the page in one menu language.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PageText {
    title: String,
    /// One label for each value of the page setting. A page whose rows the
    /// core gives (`CORE_ROWS`) has none.
    #[serde(default)]
    rows: Vec<String>,
    /// The value in each root row summary, `<title><separator><value>`, if it
    /// is not the label. A summary has less room than a row.
    summaries: Option<Vec<String>>,
    /// The rows of the native settings page (`PageSpec::settings`) by key.
    #[serde(default)]
    settings: std::collections::BTreeMap<String, RowText>,
}

/// The text of a row of the native settings page.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RowText {
    label: String,
    /// The text of each value. The page setting uses the page rows instead.
    values: Option<Vec<String>>,
}

/// The capabilities whose pages get their rows from the core
/// (`runtime/sdk/grkit.h`: GRKIT_CHOICE_CORE_ROWS).
const CORE_ROWS: [&str; 2] = ["CORE.TONE", "CORE.RECIPE"];

/// The most UTF-16 units of one text (`runtime/module_link.h`: GRM_TEXT_UNITS).
const TEXT_UNITS: usize = 64;
/// The width and height of a tile in pixels; a tile is RGBA
/// (`runtime/module_link.h`: GRM_TILE_BYTES).
const TILE_SIDE: usize = 40;
/// The most rows of a choice page (`runtime/module_link.h`: GRM_PAGE_ROWS).
const PAGE_ROWS: usize = 64;
/// The most values of a page with options: CORE.ROWS has a bit for each
/// (`runtime/manage.h`: MANAGE_OPTION_VALUES).
const OPTION_VALUES: usize = 16;
/// The largest layer order (`runtime/module_link.h`: GRM_MAX_LAYER_ORDER).
const MAX_LAYER_ORDER: u32 = 0xffff;

fn c_name(name: &str) -> Result<&str> {
    ensure!(
        !name.is_empty()
            && !name.starts_with(|c: char| c.is_ascii_digit())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "{name:?} is not a C name"
    );
    Ok(name)
}

fn valid_key(key: &str) -> bool {
    (1..=15).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

/// A UTF-16 C array with its NUL.
fn utf16(text: &str) -> Result<String> {
    let units: Vec<u16> = text.encode_utf16().collect();
    ensure!(
        units.len() <= TEXT_UNITS,
        "The text {text:?} is longer than {TEXT_UNITS} UTF-16 units"
    );
    let mut out = String::from("{");
    for unit in units.iter().chain([&0]) {
        write!(out, "{unit},")?;
    }
    out.push('}');
    Ok(out)
}

fn read_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    toml::from_str(&fs::read_to_string(path).with_context(|| format!("Read {}", path.display()))?)
        .with_context(|| format!("Read {}", path.display()))
}

fn text_path(directory: &Path, language: Language) -> PathBuf {
    directory.join(format!("text/{}.toml", language.code()))
}

/// The page text of `language` and its path. Each text must be one that the
/// camera's font of that language draws.
fn read_text(directory: &Path, language: Language) -> Result<(PathBuf, PageText)> {
    let path = text_path(directory, language);
    let text: PageText = read_toml(&path)?;
    let rows = text
        .settings
        .values()
        .flat_map(|row| std::iter::once(&row.label).chain(row.values.iter().flatten()));
    for item in std::iter::once(&text.title)
        .chain(&text.rows)
        .chain(text.summaries.iter().flatten())
        .chain(rows)
    {
        if let Some(c) = language.undrawable(item) {
            bail!(
                "{}: the camera cannot draw {c:?} of {item:?}",
                path.display()
            );
        }
    }
    Ok((path, text))
}

/// The names `<prefix>_<language index>` of a text in each language, in the
/// order of `Language::ALL`.
fn per_language(prefix: String) -> impl Iterator<Item = String> {
    (0..Language::ALL.len()).map(move |l| format!("{prefix}_{l}"))
}

/// A C array of pointers to UTF-16 texts.
fn text_table(c: &mut String, name: &str, texts: impl Iterator<Item = String>) -> Result<()> {
    writeln!(
        c,
        "static const u16 *const {name}[] = {{{}}};",
        texts.collect::<Vec<_>>().join(",")
    )?;
    Ok(())
}

/// An entry of `GrkitModule.registrations`.
fn registration(point: &str, implementation: &str) -> String {
    format!("{{.point = {point}, .impl = &{implementation}}}")
}

/// Read `module.toml` in `directory`, check it, and generate the C file of the
/// descriptor and the header of the setting handles. Return the manifest,
/// the generated C, and the header.
pub fn generate(directory: &Path) -> Result<(Manifest, String, String)> {
    let manifest: Manifest = read_toml(&directory.join("module.toml"))?;
    let m = &manifest;
    validate(m)?;
    let tiles = read_tiles(directory, m)?;
    let header = header(m)?;
    let mut c = String::from(
        "/* Generated by grkit module build from module.toml. Do not edit. */\n#include \"grkit.h\"\n",
    );
    let mut registrations = Vec::new();
    if let Some(page) = &m.page {
        if CORE_ROWS.contains(&page.setting.as_str()) {
            core_page(directory, m, page, &mut c)?;
        } else {
            choice_page(directory, m, page, &mut c)?;
        }
        registrations.push(registration("GRKIT_POINT_MENU_PAGE", "page"));
    }
    registrations.extend(layers(m, &mut c)?);
    registrations.extend(filters(m, &mut c)?);
    registrations.extend(subscriber(m, &mut c)?);
    ensure!(
        !registrations.is_empty(),
        "module.toml has no page, no layer, no filter, and no events"
    );
    descriptor(m, &registrations, &tiles, &mut c)?;
    Ok((manifest, c, header))
}

/// Check the fields of the manifest that refer to no other file.
fn validate(m: &Manifest) -> Result<()> {
    ensure!(
        grkit_core::grm::valid_id(&m.id),
        "Invalid module ID {:?}",
        m.id
    );
    ensure!(
        m.order <= grkit_core::grm::MAX_ORDER,
        "order must be 0 to {}",
        grkit_core::grm::MAX_ORDER
    );
    for (i, setting) in m.settings.iter().enumerate() {
        ensure!(
            valid_key(&setting.key),
            "Invalid setting key {:?}",
            setting.key
        );
        ensure!(
            (1..=grkit_core::runtime::MODULE_SETTING_VALUES as u32).contains(&setting.values),
            "Setting {} needs 1 to {} values",
            setting.key,
            grkit_core::runtime::MODULE_SETTING_VALUES
        );
        ensure!(
            m.settings[..i].iter().all(|s| s.key != setting.key),
            "Setting {} is twice in module.toml",
            setting.key
        );
    }
    for name in &m.uses {
        let full = name.strip_prefix('?').unwrap_or(name);
        let (module, key) = full
            .split_once('.')
            .context("A used setting is MODULE.KEY")?;
        ensure!(
            grkit_core::grm::valid_id(module) && valid_key(key),
            "Invalid used setting {name:?}"
        );
    }
    ensure!(
        m.tiles.len() <= grkit_core::runtime::MODULE_TILES_MAX,
        "More than {} tiles",
        grkit_core::runtime::MODULE_TILES_MAX
    );
    Ok(())
}

/// The RGBA pixels of the tiles, one tile after the other.
fn read_tiles(directory: &Path, m: &Manifest) -> Result<Vec<u8>> {
    let mut tiles = Vec::new();
    for tile in &m.tiles {
        let path = directory.join(tile);
        let (width, height, rgba) = grkit_core::png::decode_rgba8(
            &fs::read(&path).with_context(|| format!("Read {}", path.display()))?,
        )
        .with_context(|| format!("Read {}", path.display()))?;
        ensure!(
            width == TILE_SIDE && height == TILE_SIDE,
            "{tile} is not {TILE_SIDE} x {TILE_SIDE} pixels"
        );
        tiles.extend(rgba);
    }
    Ok(tiles)
}

/// The header of the setting handles, which the module sources include.
fn header(m: &Manifest) -> Result<String> {
    let mut header = format!(
        "/* Generated by grkit module build from module.toml. Do not edit. */\n#ifndef GRKIT_MODULE_{id}_H\n#define GRKIT_MODULE_{id}_H\n#include \"grkit.h\"\n",
        id = m.id
    );
    for (i, setting) in m.settings.iter().enumerate() {
        writeln!(header, "#define SETTING_{} GRKIT_SETTING({i})", setting.key)?;
    }
    for (i, name) in m.uses.iter().enumerate() {
        let full = name.strip_prefix('?').unwrap_or(name);
        writeln!(
            header,
            "#define USED_{} GRKIT_USED({i})",
            full.replace('.', "_")
        )?;
    }
    header.push_str("#endif\n");
    Ok(header)
}

/// The C reference of a setting of a page, a layer, or a filter (GRKIT_USE in
/// runtime/sdk/grkit.h): its own setting by key, or a used setting by its
/// full name. Also the number of values of an own setting.
fn setting_reference(m: &Manifest, name: &str) -> Result<(String, Option<usize>)> {
    if let Some(index) = m.settings.iter().position(|s| s.key == name) {
        return Ok((index.to_string(), Some(m.settings[index].values as usize)));
    }
    let index = m
        .uses
        .iter()
        .position(|used| used.strip_prefix('?').unwrap_or(used) == name)
        .with_context(|| format!("module.toml has no setting {name} and does not use it"))?;
    Ok((format!("GRKIT_USE({index})"), None))
}

/// The page of a capability in `CORE_ROWS`: the core gives the rows, so the
/// page has only its title.
fn core_page(directory: &Path, m: &Manifest, page: &PageSpec, c: &mut String) -> Result<()> {
    let (setting, _) = setting_reference(m, &page.setting)?;
    ensure!(
        !page.off_row && page.settings.is_empty(),
        "A page of {} has no off_row and no settings",
        page.setting
    );
    // The options turn effects on or off: the recipes have none.
    ensure!(
        !page.options || page.setting == "CORE.TONE",
        "A page of {} has no options",
        page.setting
    );
    for (l, language) in Language::ALL.into_iter().enumerate() {
        let (path, text) = read_text(directory, language)?;
        ensure!(
            text.rows.is_empty() && text.summaries.is_none() && text.settings.is_empty(),
            "{}: the core gives the rows of {}, so the text has only the title",
            path.display(),
            page.setting
        );
        writeln!(c, "static const u16 title_{l}[] = {};", utf16(&text.title)?)?;
    }
    text_table(c, "page_title", per_language("title".to_owned()))?;
    let flags = if page.options {
        "GRKIT_CHOICE_CORE_ROWS | GRKIT_CHOICE_OPTIONS"
    } else {
        "GRKIT_CHOICE_CORE_ROWS"
    };
    writeln!(
        c,
        "static const GrkitPage page = {{.size = sizeof(GrkitPage), .kind = GRKIT_PAGE_CHOICE, .setting = {setting}, .flags = {flags}, .title = page_title}};"
    )?;
    Ok(())
}

/// A row of the native settings page of a choice page.
struct SettingRow {
    key: String,
    /// The C reference of the setting (`setting_reference`).
    reference: String,
    values: usize,
}

/// A choice page: a row with a tile and a label for each value of its
/// setting, and the rows of its native settings page.
fn choice_page(directory: &Path, m: &Manifest, page: &PageSpec, c: &mut String) -> Result<()> {
    // A used setting has one value for each row of the English text. The
    // installer checks that number.
    let (setting, own_values) = setting_reference(m, &page.setting)?;
    let values = match own_values {
        Some(values) => values,
        None => read_toml::<PageText>(&text_path(directory, Language::ALL[0]))?
            .rows
            .len(),
    };
    ensure!(
        (1..=PAGE_ROWS).contains(&values),
        "A choice page has 1 to {PAGE_ROWS} rows"
    );
    ensure!(
        m.tiles.len() >= values,
        "A choice page needs a tile for each value"
    );
    ensure!(
        !page.options || page.off_row,
        "A page with options has an Off row, which stays on"
    );
    ensure!(
        !page.options || values <= OPTION_VALUES,
        "A page with options has at most {OPTION_VALUES} values"
    );
    let rows = setting_rows(directory, m, page, &setting, values)?;
    for (l, language) in Language::ALL.into_iter().enumerate() {
        choice_text(directory, page, values, &rows, l, language, c)?;
    }
    text_table(c, "page_title", per_language("title".to_owned()))?;
    text_table(
        c,
        "page_summaries",
        (0..values).flat_map(|row| per_language(format!("summary_{row}"))),
    )?;
    text_table(
        c,
        "page_labels",
        (0..values).flat_map(|row| per_language(format!("label_{row}"))),
    )?;
    let settings = setting_row_tables(&rows, c)?;
    let flags = match (page.off_row, page.options) {
        (true, true) => "GRKIT_CHOICE_OFF_ROW | GRKIT_CHOICE_OPTIONS",
        (true, false) => "GRKIT_CHOICE_OFF_ROW",
        _ => "0",
    };
    writeln!(
        c,
        "static const GrkitPage page = {{.size = sizeof(GrkitPage), .kind = GRKIT_PAGE_CHOICE, .setting = {setting}, .flags = {flags}, .title = page_title, .summaries = page_summaries, .labels = page_labels, .rows = {values}{settings}}};"
    )?;
    Ok(())
}

/// The rows of the native settings page. Row 0 is the page setting, also a
/// used one such as CORE.ASPECT: its values are the rows of the page. The
/// other rows are settings of the module or used settings. INSTALL writes
/// the handle of each row, and leaves out the row of an optional used
/// setting that the camera does not have.
fn setting_rows(
    directory: &Path,
    m: &Manifest,
    page: &PageSpec,
    setting: &str,
    values: usize,
) -> Result<Vec<SettingRow>> {
    ensure!(
        page.settings.len() <= grkit_core::runtime::NATIVE_SETTING_ROWS,
        "A page has at most {} rows of a native settings page",
        grkit_core::runtime::NATIVE_SETTING_ROWS
    );
    let mut rows = Vec::new();
    for (i, key) in page.settings.iter().enumerate() {
        let (reference, count) = if i == 0 {
            ensure!(
                key == &page.setting,
                "The first native settings row is the page setting"
            );
            (setting.to_owned(), values)
        } else if let Some(index) = m.settings.iter().position(|s| &s.key == key) {
            (index.to_string(), m.settings[index].values as usize)
        } else {
            let (reference, _) = setting_reference(m, key).with_context(|| {
                format!("A native settings row names {key}, which the module neither has nor uses")
            })?;
            (reference, used_values(directory, key)?)
        };
        ensure!(
            count <= grkit_core::runtime::NATIVE_SETTING_VALUES,
            "A native settings row has at most {} values",
            grkit_core::runtime::NATIVE_SETTING_VALUES
        );
        rows.push(SettingRow {
            key: key.clone(),
            reference,
            values: count,
        });
    }
    Ok(rows)
}

/// The number of values of a used setting in a settings row: that of a
/// capability of the core, else the number of its value texts in English.
/// INSTALL checks it against the setting.
fn used_values(directory: &Path, key: &str) -> Result<usize> {
    if let Some(capability) = grkit_core::capabilities::CAPABILITIES
        .iter()
        .find(|capability| capability.name() == *key)
    {
        ensure!(
            capability.values > 0,
            "{key} cannot be a native settings row"
        );
        return Ok(capability.values as usize);
    }
    let path = text_path(directory, Language::ALL[0]);
    let text: PageText = read_toml(&path)?;
    text.settings
        .get(key)
        .and_then(|row| row.values.as_ref())
        .map(Vec::len)
        .with_context(|| format!("{} gives no values of {key}", path.display()))
}

/// The texts of a choice page in the language with index `l`: the title, a
/// label and a root row summary for each value, and the label and the values
/// of each native settings row.
fn choice_text(
    directory: &Path,
    page: &PageSpec,
    values: usize,
    rows: &[SettingRow],
    l: usize,
    language: Language,
    c: &mut String,
) -> Result<()> {
    let (path, text) = read_text(directory, language)?;
    ensure!(
        text.rows.len() == values,
        "{} has {} rows, and {} has {values} values",
        path.display(),
        text.rows.len(),
        page.setting
    );
    let summary_values = text.summaries.as_ref().unwrap_or(&text.rows);
    ensure!(
        summary_values.len() == values,
        "{} has {} summaries, and {} has {values} values",
        path.display(),
        summary_values.len(),
        page.setting
    );
    writeln!(c, "static const u16 title_{l}[] = {};", utf16(&text.title)?)?;
    let separator = grkit_core::menu_text::separator(language);
    for (row, (label, value)) in text.rows.iter().zip(summary_values).enumerate() {
        writeln!(c, "static const u16 label_{row}_{l}[] = {};", utf16(label)?)?;
        writeln!(
            c,
            "static const u16 summary_{row}_{l}[] = {};",
            utf16(&format!("{}{separator}{value}", text.title))?
        )?;
    }
    ensure!(
        text.settings.len() == rows.len(),
        "{} names the text of {} native settings rows, and the page has {}",
        path.display(),
        text.settings.len(),
        rows.len()
    );
    for (r, row) in rows.iter().enumerate() {
        let key = &row.key;
        let row_text = text
            .settings
            .get(key)
            .with_context(|| format!("{} has no [settings.{key}]", path.display()))?;
        writeln!(
            c,
            "static const u16 row_{r}_{l}[] = {};",
            utf16(&row_text.label)?
        )?;
        let texts = match (&row_text.values, r) {
            (Some(texts), _) => texts,
            (None, 0) => &text.rows,
            (None, _) => bail!("{} gives no values of {key}", path.display()),
        };
        ensure!(
            texts.len() == row.values,
            "{} has {} values of {key}, and {key} has {}",
            path.display(),
            texts.len(),
            row.values
        );
        for (v, value) in texts.iter().enumerate() {
            writeln!(c, "static const u16 row_{r}_{v}_{l}[] = {};", utf16(value)?)?;
        }
    }
    Ok(())
}

/// The tables of the native settings rows (`GrkitSettingRow`). Return the
/// fields of `GrkitPage` that refer to them, empty without rows.
fn setting_row_tables(rows: &[SettingRow], c: &mut String) -> Result<String> {
    for (r, row) in rows.iter().enumerate() {
        text_table(
            c,
            &format!("row_{r}_label"),
            per_language(format!("row_{r}")),
        )?;
        text_table(
            c,
            &format!("row_{r}_values"),
            (0..row.values).flat_map(move |v| per_language(format!("row_{r}_{v}"))),
        )?;
    }
    if rows.is_empty() {
        return Ok(String::new());
    }
    let entries: Vec<_> = rows
        .iter()
        .enumerate()
        .map(|(r, row)| {
            format!(
                "{{.size = sizeof(GrkitSettingRow), .setting = {}, .label = row_{r}_label, .values = row_{r}_values}}",
                row.reference
            )
        })
        .collect();
    writeln!(
        c,
        "static const GrkitSettingRow page_settings[] = {{{}}};",
        entries.join(", ")
    )?;
    Ok(format!(
        ", .setting_rows = {}, .settings = page_settings",
        rows.len()
    ))
}

/// The overlay layers (`GrkitLayer`). Return their registrations.
fn layers(m: &Manifest, c: &mut String) -> Result<Vec<String>> {
    let mut registrations = Vec::new();
    for (i, layer) in m.layers.iter().enumerate() {
        let paint = c_name(&layer.paint)?;
        ensure!(
            layer.order <= MAX_LAYER_ORDER,
            "A layer order is at most {MAX_LAYER_ORDER}"
        );
        let setting = match &layer.setting {
            Some(name) => setting_reference(m, name)?.0,
            None => "GRKIT_NO_SETTING".to_owned(),
        };
        ensure!(
            !(layer.follow_disp && layer.full_disp),
            "A layer has follow_disp or full_disp, not both"
        );
        let flags: Vec<_> = [
            (layer.follow_disp, "GRKIT_LAYER_FOLLOW_DISP"),
            (layer.full_disp, "GRKIT_LAYER_FULL_DISP"),
            (layer.live_only, "GRKIT_LAYER_LIVE_ONLY"),
        ]
        .into_iter()
        .filter_map(|(set, flag)| set.then_some(flag))
        .collect();
        let flags = if flags.is_empty() {
            "0".to_owned()
        } else {
            flags.join(" | ")
        };
        writeln!(c, "void {paint}(const Surface *s, u32 value);")?;
        writeln!(
            c,
            "static const GrkitLayer layer_{i} = {{.size = sizeof(GrkitLayer), .flags = {flags}, .paint = {paint}, .setting = {setting}, .order = {}}};",
            layer.order
        )?;
        registrations.push(registration(
            "GRKIT_POINT_OVERLAY_LAYER",
            &format!("layer_{i}"),
        ));
    }
    Ok(registrations)
}

/// The picture plane filters (`GrkitPlaneFilter`). Return their
/// registrations.
fn filters(m: &Manifest, c: &mut String) -> Result<Vec<String>> {
    let mut registrations = Vec::new();
    for (i, filter) in m.filters.iter().enumerate() {
        let apply = c_name(&filter.apply)?;
        let stage = match filter.stage.as_str() {
            "frame" => "GRKIT_STAGE_FRAME",
            "mark" => "GRKIT_STAGE_MARK",
            other => bail!("Unknown filter stage {other:?}: use frame or mark"),
        };
        let setting = setting_reference(m, &filter.setting)?.0;
        writeln!(c, "void {apply}(PictureSet *set, u32 value);")?;
        writeln!(
            c,
            "static const GrkitPlaneFilter filter_{i} = {{.size = sizeof(GrkitPlaneFilter), .stage = {stage}, .setting = {setting}, .apply = {apply}}};"
        )?;
        registrations.push(registration(
            "GRKIT_POINT_PICTURE_PLANE",
            &format!("filter_{i}"),
        ));
    }
    Ok(registrations)
}

/// The event subscriber (`GrkitSubscriber`). Return its registration.
fn subscriber(m: &Manifest, c: &mut String) -> Result<Option<String>> {
    let Some(events) = &m.events else {
        return Ok(None);
    };
    let handle = c_name(&events.handle)?;
    ensure!(!events.on.is_empty(), "[events] names no event");
    let mut bits = Vec::new();
    for event in &events.on {
        let bit = match event.as_str() {
            "system_ready" => "GRKIT_EVENT_SYSTEM_READY",
            "setting_changed" => "GRKIT_EVENT_SETTING_CHANGED",
            other => bail!("Unknown event {other:?}: use system_ready or setting_changed"),
        };
        ensure!(!bits.contains(&bit), "[events] names {event} twice");
        bits.push(bit);
    }
    writeln!(c, "void {handle}(u32 event, u32 a, u32 b);")?;
    writeln!(
        c,
        "static const GrkitSubscriber subscriber = {{.size = sizeof(GrkitSubscriber), .events = {}, .handle = {handle}}};",
        bits.join(" | ")
    )?;
    Ok(Some(registration("GRKIT_POINT_EVENT", "subscriber")))
}

/// The module descriptor (`GrkitModule`) with its registrations, settings,
/// used settings, and tiles.
fn descriptor(m: &Manifest, registrations: &[String], tiles: &[u8], c: &mut String) -> Result<()> {
    writeln!(
        c,
        "static const GrkitRegistration registrations[] = {{{}}};",
        registrations.join(", ")
    )?;
    let mut fields = format!(
        ".size = sizeof(GrkitModule), .abi = GRKIT_ABI, .registrations = registrations, .registration_count = {}",
        registrations.len()
    );
    if !m.settings.is_empty() {
        let entries: Vec<_> = m
            .settings
            .iter()
            .map(|s| {
                let flags = match (s.saved, s.public) {
                    (true, true) => "GRKIT_SETTING_SAVED | GRKIT_SETTING_PUBLIC",
                    (true, false) => "GRKIT_SETTING_SAVED",
                    (false, true) => "GRKIT_SETTING_PUBLIC",
                    (false, false) => "0",
                };
                format!(
                    "{{.size = sizeof(GrkitSetting), .key = \"{}\", .values = {}, .flags = {flags}}}",
                    s.key, s.values
                )
            })
            .collect();
        writeln!(
            c,
            "static const GrkitSetting settings[] = {{{}}};",
            entries.join(", ")
        )?;
        write!(
            fields,
            ", .settings = settings, .setting_count = {}",
            m.settings.len()
        )?;
    }
    if !m.uses.is_empty() {
        let names: Vec<_> = m.uses.iter().map(|name| format!("\"{name}\"")).collect();
        writeln!(
            c,
            "static const char *const uses[] = {{{}}};",
            names.join(", ")
        )?;
        write!(fields, ", .uses = uses, .use_count = {}", m.uses.len())?;
    }
    if !tiles.is_empty() {
        c.push_str("static const u8 tiles[] __attribute__((aligned(4))) = {");
        for (i, byte) in tiles.iter().enumerate() {
            if i % 32 == 0 {
                c.push('\n');
            }
            write!(c, "{byte},")?;
        }
        c.push_str("\n};\n");
        write!(fields, ", .tiles = tiles, .tile_count = {}", m.tiles.len())?;
    }
    writeln!(
        c,
        "__attribute__((used)) const GrkitModule grkit_module = {{{fields}}};"
    )?;
    Ok(())
}

/// Build the module of the manifest in `directory` (`module.toml`).
/// `objects` receives the generated files and the objects.
pub fn build_manifest(
    directory: &Path,
    sdk: &Path,
    objects: &Path,
    tools: &Toolchain,
) -> Result<Module> {
    let (manifest, c, header) = generate(directory)?;
    let generated = objects.join("grkit_module.c");
    fs::write(&generated, c)?;
    fs::write(objects.join("grkit_module.h"), header)?;
    let mut sources = vec![generated];
    for source in &manifest.sources {
        sources.push(
            directory
                .join(source)
                .canonicalize()
                .with_context(|| format!("Read {source}"))?,
        );
    }
    let sources: Vec<&Path> = sources.iter().map(|p| p.as_path()).collect();
    let mut module = build_with(
        &manifest.id,
        manifest.version,
        &sources,
        &[sdk, objects],
        objects,
        tools,
    )?;
    module.order = manifest.order;
    Ok(module)
}

/// Run the host test of the module in `directory`: its `test.c`, with the
/// module sources, the generated descriptor, and the harness of the SDK
/// (`runtime/sdk/grkit_test.h`), built with the host compiler `cc`. A module
/// without sources needs no `test.c`: then only its manifest is checked.
/// Return whether a test ran.
pub fn test_manifest(directory: &Path, sdk: &Path, objects: &Path, cc: &Path) -> Result<bool> {
    let (manifest, c, header) = generate(directory)?;
    let test = directory.join("test.c");
    if manifest.sources.is_empty() && !test.is_file() {
        return Ok(false);
    }
    ensure!(test.is_file(), "{} has no test.c", directory.display());
    let generated = objects.join("grkit_module.c");
    fs::write(&generated, c)?;
    fs::write(objects.join("grkit_module.h"), header)?;
    let executable = objects.join("module-test");
    let mut arguments: Vec<String> = [
        "-std=c11",
        "-Wall",
        "-Werror",
        "-Wno-unused-function",
        "-include",
    ]
    .map(String::from)
    .to_vec();
    arguments.push(path(&sdk.join("grkit_test.h"))?.to_owned());
    for include in [sdk, objects] {
        arguments.push("-I".into());
        arguments.push(path(include)?.to_owned());
    }
    arguments.push(path(&generated)?.to_owned());
    for source in &manifest.sources {
        arguments.push(path(&directory.join(source))?.to_owned());
    }
    arguments.push(path(&test)?.to_owned());
    arguments.push(path(&sdk.join("grkit_test.c"))?.to_owned());
    arguments.push("-o".into());
    arguments.push(path(&executable)?.to_owned());
    run(cc, arguments.iter().map(String::as_str))?;
    let output = std::process::Command::new(&executable)
        .output()
        .with_context(|| format!("Run {}", executable.display()))?;
    ensure!(
        output.status.success(),
        "The test of {} failed\n{}{}",
        manifest.id,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The value of `#define <name>` in `runtime/<file>`.
    fn define(file: &str, name: &str) -> u64 {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../runtime")
            .join(file);
        let text = fs::read_to_string(&path).unwrap();
        let value = text
            .lines()
            .find_map(|line| line.strip_prefix(&format!("#define {name} ")))
            .unwrap_or_else(|| panic!("{file} has no {name}"))
            .trim()
            .trim_end_matches('U');
        match value.strip_prefix("0x") {
            Some(hex) => u64::from_str_radix(hex, 16),
            None => value.parse(),
        }
        .unwrap_or_else(|_| panic!("{file}: {name} is not a number"))
    }

    /// The builder checks the limits that the installer and the core check.
    #[test]
    fn limits_match_the_runtime() {
        let tile_bytes = (TILE_SIDE * TILE_SIDE * 4) as u64;
        assert_eq!(define("module_link.h", "GRM_TILE_BYTES"), tile_bytes);
        assert_eq!(define("sdk/grkit.h", "GRKIT_TILE_BYTES"), tile_bytes);
        assert_eq!(define("module_link.h", "GRM_PAGE_ROWS"), PAGE_ROWS as u64);
        assert_eq!(define("module_link.h", "GRM_TEXT_UNITS"), TEXT_UNITS as u64);
        assert_eq!(
            define("module_link.h", "GRM_MAX_LAYER_ORDER"),
            u64::from(MAX_LAYER_ORDER)
        );
        assert_eq!(
            define("manage.h", "MANAGE_OPTION_VALUES"),
            OPTION_VALUES as u64
        );
    }
}
