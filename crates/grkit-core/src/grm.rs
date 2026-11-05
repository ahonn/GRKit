//! The module file format (GRM v1).
//!
//! A module is the code of one feature that the installer links into the
//! running camera at boot. The same file serves every camera: a module refers
//! to the firmware only through the services that the core exports.
//!
//! A file has a header, the code segment (text and read-only data), the data
//! segment (initialized writable data), the relocations of each segment, the
//! imports, the import names, and the artwork tiles. All numbers are
//! little-endian `u32`.
//!
//! The tiles follow the file, not the code: INSTALL copies them to the icon
//! chunks, so they take no module code memory. A module of ABI 2.2 or earlier
//! has its tiles in the code and none after the names.
//!
//! The module is linked with its code segment at address 0 and its data
//! segment at [`DATA_LINK`]. A relocated word holds such a link address. The
//! installer adds the load address of the segment that the word names. An
//! import word holds 0 in the file, and the installer writes the address of the
//! core symbol whose name hash is in the import entry. `runtime/module_link.h`
//! is the installer side of this format.

use anyhow::{ensure, Context, Result};

/// `GRKM`.
pub const MAGIC: u32 = 0x4d4b_5247;
pub const FORMAT_VERSION: u32 = 1;
pub const HEADER_SIZE: usize = 64;
/// The link address of the data segment. A relocated word at or above it
/// refers to the data segment, and a word below it to the code segment.
pub const DATA_LINK: u32 = 0x1000_0000;
/// Bit 31 of an import location: the location is in the data segment.
pub const DATA_LOCATION: u32 = 0x8000_0000;
/// The ABI of the core that the module needs: major in the high half, minor
/// in the low half. The installer loads a module whose major equals its own
/// and whose minor is not larger than its own (`runtime/sdk/grkit.h`).
pub const ABI_MAJOR: u32 = 2;
pub const ABI_MINOR: u32 = 4;
/// The largest code, data, or BSS size of one module that the installer
/// accepts.
pub const MAX_SEGMENT: usize = 0x10000;
/// The bytes of one artwork tile: 40 x 40 RGBA (`GRM_TILE_BYTES`).
pub const TILE_BYTES: usize = 40 * 40 * 4;
/// The most tiles of one module that the installer accepts (`GRM_TILES`).
pub const MAX_TILES: usize = 64;
/// The longest module ID, which is also the file name before `.GRM`.
pub const ID_SIZE: usize = 8;
/// The order of a module without `order` in its manifest. The official
/// modules have smaller orders, so other modules follow them by default.
pub const DEFAULT_ORDER: u32 = 100;
/// The largest order.
pub const MAX_ORDER: u32 = 65535;

/// The services that the core exports to modules: the name that a module
/// imports (`runtime/sdk/grkit.h`) and the C symbol of the core that
/// implements it.
pub const EXPORTS: &[(&str, &str)] = &[
    ("overlay_label", "grkit_export_overlay_label"),
    ("setting_get", "grkit_export_setting_get"),
    ("setting_set", "grkit_export_setting_set"),
    ("local_time", "grkit_export_local_time"),
    ("divide", "grkit_export_divide"),
    ("picture_white", "grkit_export_picture_white"),
    ("picture_shot", "grkit_export_picture_shot"),
    ("timer_start", "grkit_export_timer_start"),
    ("timer_cancel", "grkit_export_timer_cancel"),
    ("ui_toast", "grkit_export_ui_toast"),
    ("ui_message", "grkit_export_ui_message"),
    ("ui_confirm", "grkit_export_ui_confirm"),
    ("ui_progress_open", "grkit_export_ui_progress_open"),
    ("ui_progress_set", "grkit_export_ui_progress_set"),
    ("ui_progress_close", "grkit_export_ui_progress_close"),
    ("ui_open_native", "grkit_export_ui_open_native"),
    ("ui_slider", "grkit_export_ui_slider"),
];
/// The import that the installer fills with the context of each module
/// (`GrkitContext` in `runtime/sdk/grkit.h`).
pub const CONTEXT: &str = "grkit_self";
/// The size of a module descriptor (`GrkitModule`).
pub const DESCRIPTOR_SIZE: u32 = 40;

/// Whether a module can import `name`.
pub fn importable(name: &str) -> bool {
    name == CONTEXT || EXPORTS.iter().any(|&(export, _)| export == name)
}

/// The folder of the package modules in the SD tree. A person adds a module
/// in `GRKIT/MODULES`, which the installer also reads.
pub const FOLDER: &str = crate::card::MODULES_FOLDER;

/// The FNV-1a hash of a symbol name, which an import entry and the export
/// table of the core give.
pub fn symbol_hash(name: &str) -> u32 {
    name.bytes().fold(0x811c_9dc5, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    })
}

/// The state file ID of the module setting `module`.`key`: the CRC-32 of the
/// name, modulo 10^9, moved out of the range of the Fn adjustment entries
/// (`runtime::PARAMETER_KEY_BASE` and up) and away from 0. The installer
/// computes the same ID (`runtime/module_link.h`, `grm_state_id`).
pub fn state_id(module: &str, key: &str) -> u32 {
    crate::runtime::state_key_id(&format!("{module}.{key}"))
}

/// A segment of a module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Segment {
    Code,
    Data,
}

/// A word that the installer fills with the address of a core symbol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Import {
    pub name: String,
    pub segment: Segment,
    pub offset: u32,
}

/// The parts of a module file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Module {
    pub id: String,
    pub version: u32,
    /// The default place of the module in the GRKit root of ADJ and in the
    /// GRKit section of MENU: a smaller order first, then by ID. The
    /// installer reads it from the header before it reads the rest.
    pub order: u32,
    pub code: Vec<u8>,
    pub data: Vec<u8>,
    pub bss_size: u32,
    /// The offset of the module descriptor (`grkit_module`) in the code.
    pub descriptor: u32,
    /// The offsets of the relocated words of each segment.
    pub code_relocations: Vec<u32>,
    pub data_relocations: Vec<u32>,
    pub imports: Vec<Import>,
    /// The artwork tiles, [`TILE_BYTES`] each, after the import names. The
    /// descriptor gives their count and no address (ABI 2.3).
    pub tiles: Vec<u8>,
}

/// The placement declared by a viewfinder indicator. Package verification
/// checks shared capacity; INSTALL validates its settings and artwork.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IndicatorPlacement {
    NativeSlot(u32),
    StatusRow,
}

/// Whether `id` is a valid module ID: 1 to 8 characters `A-Z`, `0-9`, or `_`.
/// The ID is also the 8.3 file name, which the camera lists in upper case.
pub fn valid_id(id: &str) -> bool {
    (1..=ID_SIZE).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn word(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset.checked_add(4).context("Truncated module file")?)
            .context("Truncated module file")?
            .try_into()?,
    ))
}

/// Whether `size` bytes at `offset` lie in `len` bytes, without an overflow
/// on a 32-bit target.
fn inside(offset: u32, size: usize, len: usize) -> bool {
    (offset as usize)
        .checked_add(size)
        .is_some_and(|end| end <= len)
}

impl Module {
    /// Check the rules that the installer checks, so that a tool never writes
    /// a file that the camera rejects.
    pub fn validate(&self) -> Result<()> {
        ensure!(valid_id(&self.id), "Invalid module ID {:?}", self.id);
        for (name, size) in [
            ("code", self.code.len()),
            ("data", self.data.len()),
            ("BSS", self.bss_size as usize),
        ] {
            ensure!(
                size % 4 == 0,
                "The {name} size of {} is not a multiple of 4",
                self.id
            );
            ensure!(
                size <= MAX_SEGMENT,
                "The {name} of {} exceeds {MAX_SEGMENT} bytes",
                self.id
            );
        }
        ensure!(
            self.tiles.len().is_multiple_of(TILE_BYTES)
                && self.tiles.len() / TILE_BYTES <= MAX_TILES,
            "{} has more than {MAX_TILES} tiles or a partial tile",
            self.id
        );
        ensure!(
            self.descriptor.is_multiple_of(4)
                && inside(self.descriptor, DESCRIPTOR_SIZE as usize, self.code.len()),
            "The descriptor of {} is outside its code",
            self.id
        );
        let data_end = DATA_LINK + self.data.len() as u32 + self.bss_size;
        for (segment, relocations, bytes) in [
            (Segment::Code, &self.code_relocations, &self.code),
            (Segment::Data, &self.data_relocations, &self.data),
        ] {
            let mut last = None;
            for &offset in relocations {
                ensure!(
                    offset % 4 == 0 && inside(offset, 4, bytes.len()),
                    "Invalid {segment:?} relocation {offset:#x} in {}",
                    self.id
                );
                ensure!(last < Some(offset), "Unsorted relocations in {}", self.id);
                last = Some(offset);
                let value = word(bytes, offset as usize)?;
                let valid = if value >= DATA_LINK {
                    value <= data_end
                } else {
                    (value & !1) as usize <= self.code.len()
                };
                ensure!(
                    valid,
                    "Relocation {offset:#x} of {} refers outside the module ({value:#x})",
                    self.id
                );
            }
        }
        for import in &self.imports {
            let bytes = match import.segment {
                Segment::Code => &self.code,
                Segment::Data => &self.data,
            };
            ensure!(
                import.offset % 4 == 0
                    && inside(import.offset, 4, bytes.len())
                    && word(bytes, import.offset as usize)? == 0,
                "Invalid import {} in {}",
                import.name,
                self.id
            );
            ensure!(
                !import.name.is_empty() && !import.name.contains('\0'),
                "Invalid import name in {}",
                self.id
            );
        }
        Ok(())
    }

    /// Read only the placement of each indicator from the unrelocated code.
    /// Code pointers in a GRM are offsets from zero. Bound every read before
    /// package verification uses it; do not execute or link module code.
    pub(crate) fn indicator_placements(&self) -> Result<Vec<IndicatorPlacement>> {
        const POINT_INDICATOR: u32 = 5;
        const REGISTRATIONS_MAX: usize = 16;
        let descriptor = self.descriptor as usize;
        let count = word(&self.code, descriptor + 12)? as usize;
        ensure!(count <= REGISTRATIONS_MAX, "Too many module registrations");
        if count == 0 {
            return Ok(Vec::new());
        }
        let list = word(&self.code, descriptor + 8)?;
        ensure!(
            list % 4 == 0 && inside(list, count * 8, self.code.len()),
            "Invalid module registration table"
        );
        let mut placements = Vec::new();
        for index in 0..count {
            let entry = list as usize + index * 8;
            if word(&self.code, entry)? != POINT_INDICATOR {
                continue;
            }
            let address = word(&self.code, entry + 4)?;
            ensure!(
                address % 4 == 0 && inside(address, 24, self.code.len()),
                "Invalid viewfinder indicator descriptor"
            );
            let address = address as usize;
            ensure!(
                word(&self.code, address)? >= 24,
                "Truncated viewfinder indicator descriptor"
            );
            let placement = word(&self.code, address + 4)?;
            let slot = word(&self.code, address + 8)?;
            placements.push(match (placement, slot) {
                (1, 1) => IndicatorPlacement::NativeSlot(slot),
                (2, 0) => IndicatorPlacement::StatusRow,
                _ => anyhow::bail!("Invalid viewfinder placement {placement} with slot {slot}"),
            });
        }
        Ok(placements)
    }

    /// The file bytes.
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut names = Vec::new();
        for import in &self.imports {
            names.extend_from_slice(import.name.as_bytes());
            names.push(0);
        }
        while names.len() % 4 != 0 {
            names.push(0);
        }
        let mut id = [0u8; ID_SIZE];
        id[..self.id.len()].copy_from_slice(self.id.as_bytes());
        let mut out = Vec::new();
        let put = |out: &mut Vec<u8>, value: u32| out.extend_from_slice(&value.to_le_bytes());
        put(&mut out, MAGIC);
        put(&mut out, FORMAT_VERSION);
        put(&mut out, ABI_MAJOR << 16 | ABI_MINOR);
        out.extend_from_slice(&id);
        put(&mut out, self.version);
        put(&mut out, self.code.len() as u32);
        put(&mut out, self.data.len() as u32);
        put(&mut out, self.bss_size);
        put(&mut out, self.descriptor);
        put(&mut out, self.code_relocations.len() as u32);
        put(&mut out, self.data_relocations.len() as u32);
        put(&mut out, self.imports.len() as u32);
        put(&mut out, names.len() as u32);
        put(&mut out, self.order);
        put(&mut out, (self.tiles.len() / TILE_BYTES) as u32);
        debug_assert_eq!(out.len(), HEADER_SIZE);
        out.extend_from_slice(&self.code);
        out.extend_from_slice(&self.data);
        for &offset in self.code_relocations.iter().chain(&self.data_relocations) {
            put(&mut out, offset);
        }
        for import in &self.imports {
            put(&mut out, symbol_hash(&import.name));
            let location = match import.segment {
                Segment::Code => import.offset,
                Segment::Data => import.offset | DATA_LOCATION,
            };
            put(&mut out, location);
        }
        out.extend_from_slice(&names);
        out.extend_from_slice(&self.tiles);
        Ok(out)
    }

    /// Read a module file.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() >= HEADER_SIZE, "Truncated module file");
        ensure!(word(bytes, 0)? == MAGIC, "Not a GRKit module");
        ensure!(
            word(bytes, 4)? == FORMAT_VERSION,
            "Unsupported module format"
        );
        let id_bytes = &bytes[12..20];
        let id_end = id_bytes.iter().position(|&b| b == 0).unwrap_or(ID_SIZE);
        ensure!(
            id_bytes[id_end..].iter().all(|&b| b == 0),
            "Invalid module ID field"
        );
        let id = std::str::from_utf8(&id_bytes[..id_end])?.to_owned();
        let field = |index: usize| word(bytes, 20 + index * 4);
        let (version, code_size, data_size, bss_size, descriptor) =
            (field(0)?, field(1)?, field(2)?, field(3)?, field(4)?);
        let (code_count, data_count, import_count, names_size) =
            (field(5)?, field(6)?, field(7)?, field(8)?);
        let (order, tile_count) = (field(9)?, field(10)?);
        ensure!(tile_count as usize <= MAX_TILES, "Too many module tiles");
        let mut at = HEADER_SIZE;
        let mut take = |length: usize| -> Result<&[u8]> {
            let part = bytes
                .get(at..at.checked_add(length).context("Module size overflow")?)
                .context("Truncated module file")?;
            at += length;
            Ok(part)
        };
        let code = take(code_size as usize)?.to_vec();
        let data = take(data_size as usize)?.to_vec();
        let words = |part: &[u8]| -> Vec<u32> {
            part.chunks_exact(4)
                .map(|w| u32::from_le_bytes(w.try_into().unwrap()))
                .collect()
        };
        let code_relocations = words(take(code_count as usize * 4)?);
        let data_relocations = words(take(data_count as usize * 4)?);
        let entries = words(take(import_count as usize * 8)?);
        let names = take(names_size as usize)?;
        let tiles = take(tile_count as usize * TILE_BYTES)?.to_vec();
        ensure!(at == bytes.len(), "Trailing bytes in module file");
        let mut name_list = names.split(|&b| b == 0);
        let mut imports = Vec::new();
        for entry in entries.chunks_exact(2) {
            let name = std::str::from_utf8(name_list.next().context("Missing import name")?)?;
            ensure!(
                symbol_hash(name) == entry[0],
                "Import hash does not match {name}"
            );
            let (segment, offset) = if entry[1] & DATA_LOCATION != 0 {
                (Segment::Data, entry[1] & !DATA_LOCATION)
            } else {
                (Segment::Code, entry[1])
            };
            imports.push(Import {
                name: name.to_owned(),
                segment,
                offset,
            });
        }
        let module = Self {
            id,
            version,
            order,
            code,
            data,
            bss_size,
            descriptor,
            code_relocations,
            data_relocations,
            imports,
            tiles,
        };
        module.validate()?;
        Ok(module)
    }
}

/// The files of a new module folder (`grkit module new`): a working module
/// that `grkit module build` and `grkit module test` accept. It has one
/// setting, MODE (Off, On), a choice page, and a layer that draws a square
/// while MODE is On. Paths are relative to the folder.
pub fn template(id: &str) -> Result<crate::Files> {
    use crate::language::Language;
    ensure!(
        valid_id(id),
        "Invalid module ID {id:?}: use 1 to 8 characters A-Z, 0-9, or _"
    );
    let mut files = crate::Files::new();
    let order = DEFAULT_ORDER;
    let manifest = format!(
        r#"# {id}: a GRKit module (docs/architecture.md, Modules).
# grkit module build --dir <this folder> makes {id}.GRM, and
# grkit module test --dir <this folder> runs test.c on this computer.
id = "{id}"
version = 1
# The default place in the GRKit root of ADJ and in the GRKit section of
# MENU: a smaller order first, then by ID. The official modules use 10 to 50.
order = {order}
sources = ["module.c"]
# Tile n shows value n of the page setting. Tile 0 is the module tile, which
# the root shows at Off. Each tile is a 40 x 40 PNG.
tiles = ["tiles/0-module.png", "tiles/1-on.png"]

# 0 Off, 1 On. The state file keeps the value of a saved setting.
[[settings]]
key = "MODE"
values = 2
saved = true

# One row for each value. The text is in text/<language>.toml.
[page]
setting = "MODE"
off_row = true

# The layer paints while MODE is not 0. The Framing mask paints at order 10,
# the Framing ratio label at 30.
[[layers]]
paint = "module_paint"
setting = "MODE"
order = 20
follow_disp = true
"#
    );
    files.insert("module.toml".into(), manifest.into_bytes());
    let source = format!(
        r#"/* {id}: draws a white square in the top right corner of the photo area
   while MODE is On. The core calls module_paint for each draw of the live
   view, with the value of MODE. */
#include "grkit.h"
#include "grkit_module.h"

void module_paint(const Surface *s, u32 value)
{{
    const u16 *box = s->box;
    (void)value;
    for (u32 y = box[1] + 16; y < box[1] + 32; ++y)
        for (u32 x = box[2] - 32; x < box[2] - 16; ++x)
            SURFACE_PIXEL(s, x, y) = 0xffffffffU;
}}
"#
    );
    files.insert("module.c".into(), source.into_bytes());
    let test = format!(
        r#"/* The host test of {id} (grkit module test). The harness stands in for
   the core (runtime/sdk/grkit_test.h). */
#include <assert.h>
#include "grkit_module.h"

void module_paint(const Surface *s, u32 value);

static const u16 full[4] = {{0, 0, 720, 480}};

int main(void)
{{
    Surface s = grkit_test_surface(full);
    assert(SETTING_MODE == GRKIT_SETTING(0) && grkit_module.settings[0].values == 2);
    grkit_test_clear();
    module_paint(&s, 1);
    assert(grkit_test_pixel(700, 20) == 0xffffffffU);
    assert(grkit_test_pixel(10, 10) == GRKIT_TEST_CLEAR);
    return 0;
}}
"#
    );
    files.insert("test.c".into(), test.into_bytes());
    for language in Language::ALL {
        let rows = match language {
            Language::English => r#"["Off", "On"]"#,
            Language::SimplifiedChinese => r#"["关闭", "开启"]"#,
            Language::TraditionalChinese => r#"["關閉", "開啟"]"#,
            Language::Japanese => r#"["オフ", "オン"]"#,
            Language::Korean => r#"["꺼짐", "켜짐"]"#,
        };
        files.insert(
            format!("text/{}.toml", language.code()),
            format!("title = \"{id}\"\nrows = {rows}\n").into_bytes(),
        );
    }
    // The module tile shows the first characters of the ID that a tile can
    // show.
    let label: String = id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(4)
        .collect();
    let module_tile =
        crate::artwork::render_tile(if label.is_empty() { "MOD" } else { &label }, "plain")?;
    for (name, tile) in [
        ("tiles/0-module.png", module_tile),
        (
            "tiles/1-on.png",
            crate::artwork::render_tile("ON", "plain")?,
        ),
    ] {
        files.insert(name.into(), crate::png::encode_rgba8(40, 40, &tile)?);
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Module {
        let mut code = vec![0u8; 56];
        // A descriptor word that points at code offset 9 (a Thumb function at 8).
        code[16..20].copy_from_slice(&9u32.to_le_bytes());
        // A word that points into the BSS.
        code[20..24].copy_from_slice(&(DATA_LINK + 8).to_le_bytes());
        Module {
            id: "PROBE".into(),
            version: 1,
            order: 30,
            code,
            data: vec![0; 8],
            bss_size: 8,
            descriptor: 16,
            code_relocations: vec![16, 20],
            data_relocations: vec![],
            imports: vec![Import {
                name: "overlay_label".into(),
                segment: Segment::Code,
                offset: 24,
            }],
            tiles: vec![],
        }
    }

    #[test]
    fn a_module_round_trips() {
        let module = sample();
        let bytes = module.encode().unwrap();
        assert_eq!(&bytes[..4], b"GRKM");
        assert_eq!(Module::decode(&bytes).unwrap(), module);
    }

    #[test]
    fn tiles_follow_the_names_and_take_no_code() {
        let mut module = sample();
        module.tiles = (0..2 * TILE_BYTES).map(|i| i as u8).collect();
        let bytes = module.encode().unwrap();
        assert_eq!(word(&bytes, 60).unwrap(), 2);
        assert_eq!(&bytes[bytes.len() - 2 * TILE_BYTES..], &module.tiles[..]);
        assert_eq!(word(&bytes, 24).unwrap() as usize, module.code.len());
        assert_eq!(Module::decode(&bytes).unwrap(), module);
        // A missing tile byte, or one tile too many in the header.
        assert!(Module::decode(&bytes[..bytes.len() - 1]).is_err());
        let mut more = bytes.clone();
        more[60] = 3;
        assert!(Module::decode(&more).is_err());
        module.tiles.push(0);
        assert!(module.encode().is_err());
        module.tiles = vec![0; (MAX_TILES + 1) * TILE_BYTES];
        assert!(module.encode().is_err());
    }

    #[test]
    fn the_hash_is_fnv1a() {
        assert_eq!(symbol_hash(""), 0x811c_9dc5);
        assert_eq!(symbol_hash("a"), 0xe40c_292c);
    }

    #[test]
    fn invalid_modules_are_rejected() {
        let mut module = sample();
        module.id = "probe".into();
        assert!(module.encode().is_err());
        let mut module = sample();
        module.code[20..24].copy_from_slice(&(DATA_LINK + 20).to_le_bytes());
        assert!(module.encode().is_err());
        let mut module = sample();
        module.code[24] = 1;
        assert!(module.encode().is_err());
        let mut module = sample();
        module.code_relocations = vec![20, 16];
        assert!(module.encode().is_err());
        let bytes = sample().encode().unwrap();
        assert!(Module::decode(&bytes[..bytes.len() - 1]).is_err());
    }

    /// Offsets near 2^32 must fail the bounds checks, not wrap past them.
    #[test]
    fn offsets_near_the_end_of_the_address_space_are_rejected() {
        let mut module = sample();
        module.descriptor = 0xffff_ffdc;
        assert!(module.validate().is_err());
        let mut module = sample();
        module.code_relocations = vec![16, 0xffff_fffc];
        assert!(module.validate().is_err());
        let mut module = sample();
        module.imports[0].offset = 0xffff_fffc;
        assert!(module.validate().is_err());
    }
}
