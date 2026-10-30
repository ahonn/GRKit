//! Ports the firmware addresses of a target to another build of the firmware:
//! a firmware version whose code moved by more than the relocation radius of
//! the installer, or whose code changed (`docs/architecture.md`, Porting).
//!
//! The port takes each address that the target names: its hook and patch
//! sites (`components::hooks`, `components::Layout`), the `FIRMWARE()`
//! addresses of its runtime header, and the data addresses of `Target`. It
//! finds each one in the other build with the windows of the installer
//! (`crate::relocation`), within a larger radius, and gives data that code
//! loads that code as a second source. The family header has no addresses (a
//! test checks it). An address whose code or data differs in the other build
//! has no port, and a person must port it.
//!
//! The port does not check values that are not addresses: object offsets, text
//! IDs, enum values, and the offsets that header expressions add to an
//! address. The draft header marks each expression with an offset. The draft
//! target module (`Port::draft_target`) is the module of the new build in
//! `crate::target`, with a comment on each value to check.

use crate::{
    components,
    firmware::FirmwareImage,
    loader::firmware_symbol,
    relocation::{self, ImageLayout, Planner},
    runtime_source,
    target::{Target, RTOS_BASE},
};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fmt::Write};

/// The default search range on each side of an address. GR IV 1.20 moved
/// code by up to 0x29000 and `.data` by up to 0x37000 from 1.11. A larger
/// range needs larger windows, which more often include changed code.
pub const RADIUS: u32 = 0x80000;
/// The largest radius: more than the RTOS image, and twice it fits in a word.
pub const MAX_RADIUS: u32 = 0x0400_0000;

/// Where an address of a target comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// A hook or patch site of the layout. Its stock bytes change with it.
    Site,
    /// A `FIRMWARE()` address in the runtime header of the target.
    Header,
    /// A data address of `Target` or of its Image Control.
    Target,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Site => "site",
            Kind::Header => "header",
            Kind::Target => "target",
        }
    }
}

/// An address that a target names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub kind: Kind,
    /// `hook:<symbol>` or `<table>:<index>` for a site, the definition name
    /// for a header address (with `#<n>` after the first one in a definition),
    /// or the field of `Target`.
    pub name: String,
    pub address: u32,
    /// The stock bytes at a site.
    pub stock: Option<String>,
    /// For a site: the symbol of its hook, if it is one, and the Rust of the
    /// fields after the stock bytes in its table row (the target module).
    pub symbol: Option<&'static str>,
    pub rest: String,
}

/// The sites of a target: its hooks, then the contexts, the patches, and the
/// guards of its layout, each in table order.
fn sites(target: &Target) -> Result<Vec<Address>> {
    let layout = target.layout;
    let mut sites = Vec::new();
    let mut add =
        |name: String, symbol: Option<&'static str>, address: u32, stock: &str, rest: String| {
            sites.push(Address {
                kind: Kind::Site,
                name,
                address,
                stock: Some(stock.to_string()),
                symbol,
                rest,
            });
        };
    for hook in components::hooks(target)? {
        let symbol = hook.spec.symbol;
        add(
            format!("hook:{symbol}"),
            Some(symbol),
            hook.address,
            hook.stock,
            String::new(),
        );
    }
    for (index, &(address, stock)) in layout.contexts.iter().enumerate() {
        add(
            format!("context:{index}"),
            None,
            address,
            stock,
            String::new(),
        );
    }
    for (table, patches) in [
        ("instruction", layout.instruction_patches),
        ("framing_patch", layout.framing_patches),
    ] {
        for (index, &(address, stock, after)) in patches.iter().enumerate() {
            add(
                format!("{table}:{index}"),
                None,
                address,
                stock,
                format!("{after:?}"),
            );
        }
    }
    // The startup literals stay at their address in each build.
    for (index, &(address, stock)) in layout.guards.iter().enumerate() {
        if address != relocation::STARTUP_LITERALS {
            add(
                format!("guard:{index}"),
                None,
                address,
                stock,
                String::new(),
            );
        }
    }
    Ok(sites)
}

/// Each definition of a runtime header, as (name, value), with continuation
/// lines joined.
fn definitions(header: &str) -> Vec<(String, String)> {
    header
        .replace("\\\n", " ")
        .lines()
        .filter_map(|line| {
            let definition = line.strip_prefix("#define ")?;
            let (name, value) = definition.split_once(' ').unwrap_or((definition, ""));
            Some((name.to_string(), value.trim().to_string()))
        })
        .collect()
}

/// The `FIRMWARE()` addresses of a definition value, in order.
fn firmware_addresses(value: &str) -> Vec<u32> {
    value
        .split("FIRMWARE(")
        .skip(1)
        .filter_map(|part| {
            let number = part.split(')').next()?.trim();
            u32::from_str_radix(number.strip_prefix("0x")?, 16).ok()
        })
        .collect()
}

/// The runtime header of a target, without the family header that it includes.
fn header(target: &Target) -> Result<&'static str> {
    let path = target.header();
    let bytes = runtime_source(&path).with_context(|| format!("No runtime/{path}"))?;
    Ok(std::str::from_utf8(bytes)?)
}

/// Every address that `target` names: its sites, its header addresses, then
/// its data addresses.
pub fn addresses(target: &Target) -> Result<Vec<Address>> {
    let mut addresses = sites(target)?;
    for (name, value) in definitions(header(target)?) {
        for (index, address) in firmware_addresses(&value).into_iter().enumerate() {
            addresses.push(Address {
                kind: Kind::Header,
                name: if index == 0 {
                    name.clone()
                } else {
                    format!("{name}#{index}")
                },
                address,
                stock: None,
                symbol: None,
                rest: String::new(),
            });
        }
    }
    let mut data = vec![
        ("array_table".to_string(), target.array_table),
        ("array_count".to_string(), target.array_count),
        ("heap_min".to_string(), target.heap_min),
        ("english_slot".to_string(), target.english_slot),
        ("pixel_base_slot".to_string(), target.pixel_base_slot),
    ];
    if let Some(native) = target.image_control {
        for (index, &table) in native.tables.iter().enumerate() {
            data.push((format!("image_control.tables[{index}]"), table));
        }
        data.push(("image_control.record".into(), native.record));
        data.push(("image_control.coefficients".into(), native.coefficients));
        for (index, &address) in native.record_tail.iter().enumerate() {
            data.push((format!("image_control.record_tail[{index}]"), address));
        }
        data.push(("image_control.base_gamma".into(), native.base_gamma));
        data.push((
            "image_control.settings_domain".into(),
            native.settings_domain,
        ));
    }
    for (name, address) in data {
        addresses.push(Address {
            kind: Kind::Target,
            name,
            address,
            stock: None,
            symbol: None,
            rest: String::new(),
        });
    }
    Ok(addresses)
}

/// An address of the base target in the other build.
#[derive(Debug, Clone)]
pub struct Ported {
    pub address: Address,
    /// The address in the other build, or why the port did not find it.
    pub port: std::result::Result<u32, String>,
    /// The number of sources that agree on the port (`relocation::Resolution`).
    pub sources: usize,
    /// Why a person must check a port that the sources agree on.
    pub review: Option<String>,
    /// For a site: the bytes at the port, as many as the stock bytes.
    pub stock: Option<String>,
}

#[derive(Debug)]
pub struct Port {
    pub base: &'static Target,
    pub radius: u32,
    /// The layout of the other build.
    pub layout: ImageLayout,
    pub addresses: Vec<Ported>,
    /// Whether the installer relocates the base target to the other build
    /// (`relocatable`).
    pub relocatable: bool,
    /// The `RTOS_IDENTITY` word of the other build.
    pub identity: Option<u32>,
}

/// Adds the sources of an address to the plan. A window of data can match a
/// moved copy of the same words, such as the start of a table that grew, so a
/// data address that code loads also gets that code as a second source; the
/// sources must agree. A site keeps the window of its function, like the
/// installer.
fn plan(planner: &mut Planner, address: &Address, planned: u32) -> Result<usize> {
    let first = installer_symbol(planner, address, planned);
    if address.kind == Kind::Site || planned == planner.target().heap_min {
        return first;
    }
    let second = planner.referenced(planned);
    first.or(second)
}

/// The sources of an address as the installer plans them
/// (`loader::relocation_tables`): data of `Target` in the image by its
/// content, the others by the region that holds them.
fn installer_symbol(planner: &mut Planner, address: &Address, planned: u32) -> Result<usize> {
    if address.kind == Kind::Target && planned < planner.target().data_source {
        planner.content(planned)
    } else {
        firmware_symbol(planner, planned)
    }
}

/// The address that the plan takes for `address`, and the offset to add. An
/// address in .data that no code loads directly keeps its offset from the
/// address that code loads (`Target::relocation_bases`).
fn planned(target: &Target, address: &Address) -> (u32, u32) {
    target
        .relocation_bases
        .iter()
        .find(|(field, _)| *field == address.address)
        .map_or((address.address, 0), |&(field, base)| (base, field - base))
}

/// Why a port with only weak sources needs a check.
const WEAK: &str = "only windows of mostly addresses agree: a table that grew or moved \
    matches at another start";

/// Why a port in a run of equal functions needs a check
/// (`relocation::Planner::repeated`). GR IV 1.20 has one more parameter
/// setter than 1.11, and the port gave the next setter for two rows.
const REPEATED: &str = "the function occurs another number of times nearby: a function \
    added to or removed from a run of equal functions moves the others";

/// A source in a window of mostly address words. A table of pointers masks
/// to equal words, so its window can match where the table grew or moved,
/// and a literal in it can then hold another entry. The code windows of
/// GR IV 1.11 hold at most a fifth of address words.
fn weak(plan: &relocation::Plan, source: &relocation::Source) -> bool {
    use relocation::Source;
    let (Source::Window { window, .. }
    | Source::Literal { window, .. }
    | Source::Pair { window, .. }
    | Source::Branch { window, .. }) = *source
    else {
        return false;
    };
    let words = &plan.windows[window].words;
    let addresses = words
        .iter()
        .filter(|&&word| word == relocation::ADDRESS_MASK)
        .count();
    addresses * 2 > words.len()
}

/// Entry 11 of the first Image Control type table points to the Standard
/// record (`FirmwareImage::stock_record`). A ported table start without it
/// needs a check.
fn check_image_control(addresses: &mut [Ported], other: &[u8], layout: &ImageLayout) {
    let port = |addresses: &[Ported], name: &str| {
        addresses
            .iter()
            .find(|ported| ported.address.kind == Kind::Target && ported.address.name == name)
            .and_then(|ported| ported.port.clone().ok())
    };
    let (Some(table), Some(record)) = (
        port(addresses, "image_control.tables[0]"),
        port(addresses, "image_control.record"),
    ) else {
        return;
    };
    let entry = table + 11 * 4;
    let holder = entry
        .checked_sub(relocation::DATA_START)
        .map(|offset| layout.data_source + offset - RTOS_BASE);
    let value = holder.and_then(|at| {
        other
            .get(at as usize..at as usize + 4)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap()))
    });
    if value != Some(record) {
        if let Some(ported) = addresses.iter_mut().find(|ported| {
            ported.address.kind == Kind::Target && ported.address.name == "image_control.tables[0]"
        }) {
            ported.review = Some(format!("entry 11 is not the Standard record {record:#x}"));
        }
    }
}

/// Ports each address of `base.target` to `other`, the RTOS image of another
/// build, loaded at `RTOS_BASE`.
pub fn port(base: &FirmwareImage, other: &[u8], radius: u32) -> Result<Port> {
    ensure!(
        radius.is_multiple_of(4) && (relocation::RADIUS..=MAX_RADIUS).contains(&radius),
        "The radius must be a multiple of 4 from {:#x} to {MAX_RADIUS:#x}",
        relocation::RADIUS
    );
    let target = base.target;
    let addresses = addresses(target)?;
    let mut planner = Planner::with_radius(base, radius)?;
    let mut symbols = Vec::new();
    let mut repeated = Vec::new();
    for address in &addresses {
        let (planned, offset) = planned(target, address);
        symbols.push(
            plan(&mut planner, address, planned)
                .map(|symbol| (symbol, offset))
                .map_err(|error| format!("no window in {}: {error}", target.firmware)),
        );
        repeated.push(if address.kind == Kind::Target {
            None
        } else {
            planner.repeated(planned)
        });
    }
    let plan = planner.finish();
    let resolution = relocation::resolve(&plan, other, radius)?;
    let read = |address: u32, size: usize| -> Option<String> {
        let start = address.checked_sub(RTOS_BASE)? as usize;
        Some(hex::encode(other.get(start..start + size)?))
    };
    let masked: Vec<u32> = other
        .chunks_exact(4)
        .map(|word| relocation::mask(u32::from_le_bytes(word.try_into().unwrap())))
        .collect();
    // How often `words` occur within twice the radius of `at` in `other`.
    let occurrences = |at: u32, words: &[u32]| -> usize {
        let index = (at.saturating_sub(RTOS_BASE) / 4) as usize;
        let span = (2 * radius / 4) as usize;
        let low = index.saturating_sub(span);
        let high = (index + span).min(masked.len().saturating_sub(words.len()));
        (low..=high)
            .filter(|&at| masked[at..at + words.len()] == *words)
            .count()
    };
    let addresses = addresses
        .into_iter()
        .zip(symbols)
        .zip(repeated)
        .map(|((address, symbol), repeated)| {
            let resolved = symbol.and_then(|(symbol, offset)| {
                resolution.symbols[symbol]
                    .clone()
                    .map(|(value, sources)| (value + offset, sources))
            });
            let review = match &resolved {
                Ok((_, sources)) if sources.iter().all(|source| weak(&plan, source)) => {
                    Some(WEAK.to_string())
                }
                Ok((value, _))
                    if repeated
                        .as_ref()
                        .is_some_and(|(words, count)| occurrences(*value, words) != *count) =>
                {
                    Some(REPEATED.to_string())
                }
                _ => None,
            };
            let sources = resolved.as_ref().map_or(0, |(_, sources)| sources.len());
            let port = resolved.map(|(value, _)| value);
            let stock = match (&port, &address.stock) {
                (Ok(at), Some(stock)) => read(*at, stock.len() / 2),
                _ => None,
            };
            Ported {
                address,
                port,
                sources,
                review,
                stock,
            }
        })
        .collect();
    let mut addresses: Vec<Ported> = addresses;
    check_image_control(&mut addresses, other, &resolution.layout);
    let relocatable = relocatable(base, other, &addresses)?;
    let identity = (crate::target::RTOS_IDENTITY - RTOS_BASE) as usize;
    let identity = other
        .get(identity..identity + 4)
        .map(|word| u32::from_le_bytes(word.try_into().unwrap()));
    Ok(Port {
        base: target,
        radius,
        layout: resolution.layout,
        addresses,
        relocatable,
        identity,
    })
}

/// Whether the installer relocates the base target to the other build: each
/// address has a port without a review, and the plan of the installer, within
/// `relocation::RADIUS`, resolves each source of each address to that port,
/// with room for the private tail. The plan of a build has more symbols, such
/// as branch targets, so a test of the build on the other firmware must still
/// confirm it.
fn relocatable(base: &FirmwareImage, other: &[u8], addresses: &[Ported]) -> Result<bool> {
    if addresses
        .iter()
        .any(|ported| ported.port.is_err() || ported.review.is_some())
    {
        return Ok(false);
    }
    let target = base.target;
    let mut planner = Planner::new(base)?;
    let mut symbols = Vec::new();
    for ported in addresses {
        let (planned, offset) = planned(target, &ported.address);
        match installer_symbol(&mut planner, &ported.address, planned) {
            Ok(symbol) => symbols.push((symbol, offset)),
            Err(_) => return Ok(false),
        }
    }
    let resolution = relocation::resolve(&planner.finish(), other, relocation::RADIUS)?;
    let (tail, end) = target.layout.tail;
    Ok(resolution.complete
        && relocation::DATA_START - resolution.layout.tail >= end - tail
        && symbols
            .iter()
            .zip(addresses)
            .all(|(&(symbol, offset), ported)| {
                resolution.symbols[symbol]
                    .as_ref()
                    .is_ok_and(|&(value, _)| Ok(value + offset) == ported.port)
            }))
}

/// Compares a port with the addresses of an existing target of the other
/// build: the same names must have the ported addresses.
pub fn check(port: &Port, target: &Target) -> Result<Value> {
    let expected: BTreeMap<(Kind, String), u32> = addresses(target)?
        .into_iter()
        .map(|address| ((address.kind, address.name), address.address))
        .collect();
    let mut equal = 0;
    let mut differ = Vec::new();
    let mut unported = Vec::new();
    let mut review = Vec::new();
    for ported in &port.addresses {
        let key = (ported.address.kind, ported.address.name.clone());
        let Some(&expected) = expected.get(&key) else {
            continue;
        };
        let name = format!("{}:{}", key.0.name(), key.1);
        let entry = |address: u32| {
            json!({"name": name, "port": format!("{address:#x}"),
                "target": format!("{expected:#x}"), "review": ported.review})
        };
        match &ported.port {
            Ok(address) if ported.review.is_some() => review.push(entry(*address)),
            Ok(address) if *address == expected => equal += 1,
            Ok(address) => differ.push(entry(*address)),
            Err(_) => unported.push(name),
        }
    }
    // Ports that need a check do not count as equal or different.
    Ok(json!({
        "target": target.name,
        "equal": equal,
        "differ": differ,
        "review": review,
        "unported": unported,
    }))
}

impl Port {
    /// The addresses that have no port.
    pub fn unported(&self) -> impl Iterator<Item = &Ported> {
        self.addresses.iter().filter(|ported| ported.port.is_err())
    }

    /// The largest distance that a ported address moved.
    pub fn largest_move(&self) -> u32 {
        self.addresses
            .iter()
            .filter_map(|ported| Some(ported.port.as_ref().ok()?.abs_diff(ported.address.address)))
            .max()
            .unwrap_or(0)
    }

    /// Each address with its port, for a report file.
    pub fn report(&self) -> Value {
        let addresses: Vec<Value> = self
            .addresses
            .iter()
            .map(|ported| {
                json!({
                    "kind": ported.address.kind.name(),
                    "name": ported.address.name,
                    "base": format!("{:#x}", ported.address.address),
                    "port": ported.port.as_ref().map(|at| format!("{at:#x}")).ok(),
                    "error": ported.port.as_ref().err(),
                    "sources": ported.sources,
                    "review": ported.review,
                    "stock": ported.address.stock,
                    "port_stock": ported.stock,
                })
            })
            .collect();
        json!({
            "base": self.base.name,
            "radius": format!("{:#x}", self.radius),
            "data_source": format!("{:#x}", self.layout.data_source),
            "data_size": format!("{:#x}", self.layout.data_size),
            "heap_min": format!("{:#x}", self.layout.heap_min),
            "tail": format!("{:#x}", self.layout.tail),
            "addresses": addresses,
        })
    }

    fn find(&self, kind: Kind, name: &str) -> Option<&Ported> {
        self.addresses
            .iter()
            .find(|ported| ported.address.kind == kind && ported.address.name == name)
    }

    /// The runtime header of the base target with each `FIRMWARE()` address
    /// ported. A comment marks each address without a port and each
    /// expression that adds an offset, which the port does not check.
    pub fn draft_header(&self) -> Result<String> {
        let mut out = String::from(
            "/* Draft from `grkit port`. Comments still describe the base build, and\n   \
             values that are not addresses are those of the base build. */\n",
        );
        let source = header(self.base)?.replace("\\\n", "\u{0}");
        for line in source.lines() {
            let line = line.replace('\u{0}', "\\\n");
            let Some(definition) = line.strip_prefix("#define ") else {
                out.push_str(&line);
                out.push('\n');
                continue;
            };
            let (name, value) = definition.split_once(' ').unwrap_or((definition, ""));
            let mut ported = line.clone();
            let mut notes = Vec::new();
            for (index, address) in firmware_addresses(value).into_iter().enumerate() {
                let key = if index == 0 {
                    name.to_string()
                } else {
                    format!("{name}#{index}")
                };
                let old = format!("FIRMWARE({address:#x})");
                let found = self.find(Kind::Header, &key);
                match found.map(|p| &p.port) {
                    Some(Ok(at)) => {
                        ported = ported.replacen(&old, &format!("FIRMWARE({at:#x})"), 1)
                    }
                    _ => notes.push(format!("no port for {address:#x}")),
                }
                if let Some(reason) = found.and_then(|p| p.review.as_ref()) {
                    notes.push(format!("check {address:#x}: {reason}"));
                }
            }
            if value.contains("FIRMWARE(") && value.contains(") +") {
                notes.push("check the offset".into());
            }
            out.push_str(&ported);
            if !notes.is_empty() {
                write!(out, " /* PORT: {} */", notes.join("; "))?;
            }
            out.push('\n');
        }
        Ok(out)
    }

    /// The port of an address and the note of its line. An address without a
    /// port keeps its base value.
    fn value(&self, kind: Kind, name: &str) -> Result<(u32, String)> {
        let ported = self
            .find(kind, name)
            .with_context(|| format!("No address {name}"))?;
        Ok(match &ported.port {
            Ok(at) => (
                *at,
                ported
                    .review
                    .as_ref()
                    .map_or(String::new(), |reason| format!("PORT: check: {reason}")),
            ),
            Err(error) => (ported.address.address, format!("PORT: {error}")),
        })
    }

    /// An array field of `ImageControl`, from its ported entries.
    fn draft_list(&self, out: &mut String, field: &str, count: usize) -> Result<()> {
        let mut values = Vec::new();
        let mut notes = Vec::new();
        for index in 0..count {
            let (value, note) =
                self.value(Kind::Target, &format!("image_control.{field}[{index}]"))?;
            values.push(format!("{value:#x}"));
            if !note.is_empty() {
                notes.push(format!("[{index}] {note}"));
            }
        }
        push(
            out,
            4,
            &format!("{field}: [{}],", values.join(", ")),
            &notes.join("; "),
        );
        Ok(())
    }

    /// A site table: the sites whose name starts with `prefix`, at their
    /// ports, with the stock bytes of the other build.
    fn draft_table(&self, out: &mut String, name: &str, kind: &str, prefix: &str) -> Result<()> {
        let rows: Vec<&Ported> = self
            .addresses
            .iter()
            .filter(|ported| {
                ported.address.kind == Kind::Site && ported.address.name.starts_with(prefix)
            })
            .collect();
        push(out, 0, "", "");
        if rows.is_empty() {
            push(out, 0, &format!("const {name}: {kind} = &[];"), "");
            return Ok(());
        }
        push(out, 0, &format!("const {name}: {kind} = &["), "");
        for ported in rows {
            let site = &ported.address;
            let (address, mut note) = self.value(Kind::Site, &site.name)?;
            let stock = if ported.port.is_ok() {
                ported.stock.clone()
            } else {
                site.stock.clone()
            }
            .unwrap_or_default();
            // The new bytes of a patch follow the stock bytes.
            if !site.rest.is_empty() && ported.port.is_ok() && ported.stock != site.stock {
                note = [
                    note.as_str(),
                    "PORT: the stock bytes changed; check the new bytes",
                ]
                .iter()
                .filter(|part| !part.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join("; ");
            }
            let fields: Vec<String> = site
                .symbol
                .map(|symbol| format!("{symbol:?}"))
                .into_iter()
                .chain([format!("{address:#x}"), format!("{stock:?}")])
                .chain((!site.rest.is_empty()).then(|| site.rest.clone()))
                .collect();
            let single = format!("({}),", fields.join(", "));
            if single.len() + 4 <= 100 {
                push(out, 4, &single, &note);
            } else {
                // As rustfmt wraps a long tuple.
                push(out, 4, "(", "");
                for field in &fields {
                    push(out, 8, &format!("{field},"), "");
                }
                push(out, 4, "),", &note);
            }
        }
        push(out, 0, "];", "");
        Ok(())
    }

    /// The module of the new build in `crate::target`: the data of the base
    /// target with each address ported, and the sites at the ported sites
    /// with the stock bytes of the other build. Values that are not addresses
    /// are those of the base target. A `PORT` comment marks each value to
    /// check and each address without a port, which keeps its base value. The
    /// module has the items of the checked-in target modules (tests).
    pub fn draft_target(&self) -> Result<String> {
        let base = self.base;
        let layout = base.layout;
        let out = &mut String::new();
        let base_note = |what: &str| format!("PORT: {what} ({} in the base)", base.name);
        push(
            out,
            0,
            &format!(
                "//! Draft from `grkit port`: {} in another build. Values that are not",
                base.name
            ),
            "",
        );
        push(
            out,
            0,
            "//! addresses are those of the base; check each `PORT` comment.",
            "",
        );
        push(out, 0, "", "");
        push(
            out,
            0,
            &format!(
                "use super::{{{}, Download, ImageControl, Source, Target}};",
                base.family.name
            ),
            "",
        );
        push(
            out,
            0,
            "use crate::components::{InstructionPatch, Layout, Site};",
            "",
        );

        push(out, 0, "", "");
        push(out, 0, "pub const TARGET: Target = Target {", "");
        push(
            out,
            4,
            &format!("name: {:?},", base.name),
            &base_note("the name of the new target"),
        );
        push(
            out,
            4,
            &format!("family: &{}::FAMILY,", base.family.name),
            "",
        );
        push(
            out,
            4,
            &format!("firmware: {:?},", base.firmware),
            &base_note("the model and version"),
        );
        push(
            out,
            4,
            &format!("firmware_sha256: {:?},", base.firmware_sha256),
            &base_note("the SHA-256 of the firmware file"),
        );
        push(
            out,
            4,
            &format!("decoded_sha256: {:?},", base.decoded_sha256),
            &base_note("the SHA-256 of the decoded firmware"),
        );
        push(out, 4, "download: Download {", "");
        push(
            out,
            8,
            &format!("zip: {:?},", base.download.zip),
            &base_note("the zip on the Ricoh download page"),
        );
        push(
            out,
            8,
            &format!("zip_sha256: {:?},", base.download.zip_sha256),
            &base_note("the SHA-256 of the zip"),
        );
        push(
            out,
            8,
            &format!("file: {:?},", base.download.file),
            &base_note("the firmware file in the zip"),
        );
        push(
            out,
            8,
            &format!("source: Source::{:?},", base.download.source),
            &base_note("Ricoh, or Dump for an RTOS image of a camera"),
        );
        push(out, 4, "},", "");
        let identities: Vec<_> = base
            .identities
            .iter()
            .map(|word| format!("{word:#010x}"))
            .collect();
        let identity = self
            .identity
            .map_or("none".to_string(), |word| format!("{word:#010x}"));
        push(
            out,
            4,
            &format!("identities: &[{}],", identities.join(", ")),
            &format!(
                "PORT: the identity of the new build is {identity} ({} in the base)",
                base.name
            ),
        );
        push(out, 4, &format!("data_start: {:#x},", base.data_start), "");
        push(
            out,
            4,
            &format!("data_source: {:#x},", self.layout.data_source),
            "",
        );
        push(
            out,
            4,
            &format!("data_size: {:#x},", self.layout.data_size),
            "",
        );
        for field in ["array_table", "array_count", "heap_min"] {
            let (value, note) = self.value(Kind::Target, field)?;
            push(out, 4, &format!("{field}: {value:#x},"), &note);
        }
        push(out, 4, &format!("heap_end: {:#x},", base.heap_end), "");
        for field in ["english_slot", "pixel_base_slot"] {
            let (value, note) = self.value(Kind::Target, field)?;
            push(out, 4, &format!("{field}: {value:#x},"), &note);
        }
        push(
            out,
            4,
            &format!("pixel_base_indirect: {},", base.pixel_base_indirect),
            "",
        );
        let page = base
            .native_tone_page
            .map_or("None".to_string(), |page| format!("Some({page:#x})"));
        push(out, 4, &format!("native_tone_page: {page},"), "");
        push(out, 4, "layout: &LAYOUT,", "");
        let image_control = match base.image_control {
            Some(_) => "image_control: Some(&IMAGE_CONTROL),",
            None => "image_control: None,",
        };
        push(out, 4, image_control, "");
        // A relocation base keeps its offset from its field.
        let mut bases = Vec::new();
        let mut notes = Vec::new();
        for &(field, address) in base.relocation_bases {
            let ported = self
                .addresses
                .iter()
                .find(|ported| {
                    ported.address.kind == Kind::Target && ported.address.address == field
                })
                .with_context(|| format!("No data address {field:#x}"))?;
            let (at, note) = self.value(Kind::Target, &ported.address.name)?;
            bases.push(format!("({at:#x}, {:#x})", at - (field - address)));
            if !note.is_empty() {
                notes.push(format!("{}: {note}", ported.address.name));
            }
        }
        push(
            out,
            4,
            &format!("relocation_bases: &[{}],", bases.join(", ")),
            &notes.join("; "),
        );
        push(out, 0, "};", "");

        if let Some(native) = base.image_control {
            push(out, 0, "", "");
            push(
                out,
                0,
                "pub const IMAGE_CONTROL: ImageControl = ImageControl {",
                "",
            );
            self.draft_list(out, "tables", native.tables.len())?;
            push(
                out,
                4,
                &format!("table_bytes: {},", native.table_bytes),
                &base_note("the bytes of each table"),
            );
            for field in ["record", "coefficients"] {
                let (value, note) = self.value(Kind::Target, &format!("image_control.{field}"))?;
                push(out, 4, &format!("{field}: {value:#x},"), &note);
            }
            self.draft_list(out, "record_tail", native.record_tail.len())?;
            for field in ["base_gamma", "settings_domain"] {
                let (value, note) = self.value(Kind::Target, &format!("image_control.{field}"))?;
                push(out, 4, &format!("{field}: {value:#x},"), &note);
            }
            push(
                out,
                4,
                &format!("settings_type: {:#x},", native.settings_type),
                &base_note("the offset of the type in the settings"),
            );
            push(
                out,
                4,
                &format!("type_count: {},", native.type_count),
                &base_note("the number of native types"),
            );
            push(out, 0, "};", "");
        }

        let (tail, end) = layout.tail;
        push(out, 0, "", "");
        push(out, 0, "pub const LAYOUT: Layout = Layout {", "");
        push(
            out,
            4,
            &format!("define: {:?},", layout.define),
            &base_note("the definition that selects the new header in runtime/firmware.h"),
        );
        push(
            out,
            4,
            &format!(
                "tail: ({:#x}, {:#x}),",
                self.layout.tail,
                self.layout.tail + (end - tail)
            ),
            "",
        );
        for field in [
            "contexts: CONTEXTS,",
            "sites: SITES,",
            "instruction_patches: INSTRUCTION_PATCHES,",
            "framing_patches: FRAMING_PATCHES,",
            "guards: GUARDS,",
        ] {
            push(out, 4, field, "");
        }
        push(out, 0, "};", "");

        self.draft_table(out, "SITES", "&[Site]", "hook:")?;
        self.draft_table(out, "CONTEXTS", "&[(u32, &str)]", "context:")?;
        self.draft_table(
            out,
            "INSTRUCTION_PATCHES",
            "&[InstructionPatch]",
            "instruction:",
        )?;
        self.draft_table(
            out,
            "FRAMING_PATCHES",
            "&[InstructionPatch]",
            "framing_patch:",
        )?;

        // The guards in the order of the base: the startup literals of the
        // other build, and the ported guards.
        push(out, 0, "", "");
        push(out, 0, "const GUARDS: &[(u32, &str)] = &[", "");
        for (index, &(address, _)) in layout.guards.iter().enumerate() {
            if address == relocation::STARTUP_LITERALS {
                let literals: Vec<u8> = [
                    self.layout.data_source,
                    relocation::DATA_START,
                    self.layout.data_size,
                ]
                .into_iter()
                .flat_map(u32::to_le_bytes)
                .collect();
                let literals = hex::encode(literals);
                push(out, 4, &format!("({address:#x}, {literals:?}),"), "");
                continue;
            }
            let name = format!("guard:{index}");
            let ported = self
                .find(Kind::Site, &name)
                .with_context(|| format!("No address {name}"))?;
            let (at, note) = self.value(Kind::Site, &name)?;
            let stock = if ported.port.is_ok() {
                ported.stock.clone()
            } else {
                ported.address.stock.clone()
            }
            .unwrap_or_default();
            push(out, 4, &format!("({at:#x}, {stock:?}),"), &note);
        }
        push(out, 0, "];", "");
        Ok(std::mem::take(out))
    }
}

/// Appends a line of `indent` spaces, `text`, and a `note` comment if any.
fn push(out: &mut String, indent: usize, text: &str, note: &str) {
    out.push_str(&" ".repeat(indent));
    out.push_str(text);
    if !note.is_empty() {
        out.push_str(" // ");
        out.push_str(note);
    }
    out.push('\n');
}
