//! Install-time relocation for targets whose cameras can run other builds of
//! the pinned firmware version.
//!
//! The build links the private code against the addresses of the pinned image.
//! For each firmware address that the code or the installer uses, it records
//! how the installer finds that address in the running image: a window of
//! masked words that occurs once near the pinned address, the target of a
//! branch in such a window, or a literal in such a window. The installer finds
//! each window in the running image, computes the new address of each symbol,
//! and moves the references by the difference (`runtime/relocation.h`).
//!
//! A mask clears the bits that differ between builds of the same source when
//! code or data moves: branch offsets, absolute RTOS addresses, and the
//! immediates of MOVW and MOVT. The other bits of each window must be equal,
//! so a window also checks the instructions, offsets, and constants around a
//! hook or in a called function.

use crate::firmware::FirmwareImage;
use crate::target::RTOS_BASE;
use anyhow::{bail, ensure, Context, Result};
use std::collections::BTreeMap;

/// The search range on each side of a pinned window start.
pub const RADIUS: u32 = 0x10000;
/// The masked value of an absolute RTOS address.
pub const ADDRESS_MASK: u32 = 0x5a5a5a5a;
/// The RTOS startup code copies `.data` with the literals at this address:
/// source, destination, and size, then the BSS start and end.
pub const STARTUP_LITERALS: u32 = 0x53000450;
/// The startup code before the literals, which must be equal in each build.
const STARTUP_CODE: u32 = 0x530003f0;
/// The `.data` destination in each build.
pub const DATA_START: u32 = 0x55000000;
/// The largest window, in words.
const MAX_WINDOW: usize = 768;
/// The words of a content window before and after its address.
const CONTENT_BEFORE: usize = 16;
const CONTENT_AFTER: usize = 48;
/// The most witnesses that the plan keeps for one symbol.
const MAX_WITNESSES: usize = 3;

/// Clears the bits of an ARM word that change when code or data moves.
pub fn mask(word: u32) -> u32 {
    if (word >> 25) & 7 == 5 {
        // B, BL, and BLX with a 24-bit offset.
        word & 0xff00_0000
    } else if (0x5300_0000..0x5600_0000).contains(&word) {
        ADDRESS_MASK
    } else if word & 0x0ff0_0000 == 0x0300_0000 || word & 0x0ff0_0000 == 0x0340_0000 {
        // MOVW and MOVT: keep the condition, the opcode, and the register.
        word & 0xfff0_f000
    } else {
        word
    }
}

/// The target of an ARM B or BL at `address`, if `word` is one.
pub fn branch_target(address: u32, word: u32) -> Option<u32> {
    ((word >> 25) & 7 == 5 && word >> 28 != 0xf).then(|| {
        let offset = ((word << 8) as i32 >> 6) as u32;
        address.wrapping_add(8).wrapping_add(offset)
    })
}

/// The 16-bit immediate of an ARM MOVW or MOVT.
pub fn move_immediate(word: u32) -> u32 {
    ((word >> 4) & 0xf000) | (word & 0xfff)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Region {
    /// The RTOS image: code and read-only data.
    Image,
    /// The initial `.data` content, which the image holds at the copy source.
    Data,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub region: Region,
    /// The pinned address of the first word.
    pub start: u32,
    /// The masked words.
    pub words: Vec<u32>,
    /// The installer searches the window. Otherwise it only compares the window
    /// at the address that another source gives.
    pub search: bool,
    /// The word that the search compares first: the least frequent one within
    /// the radius of the start.
    pub probe: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The symbol lies `offset` bytes after the start of a searched window.
    Window { window: usize, offset: u32 },
    /// The target of the B or BL at word `index` of a window. With a `verify`
    /// window, the target is the start of that window, which must match
    /// there, and the symbol keeps its offset from that start.
    Branch {
        window: usize,
        index: u32,
        verify: Option<usize>,
    },
    /// The literal at word `index` of a window holds the symbol.
    Literal { window: usize, index: u32 },
    /// The MOVW at word `movw` and the MOVT at word `movt` of a window hold the
    /// symbol.
    Pair { window: usize, movw: u32, movt: u32 },
    /// Word `index` of the startup literals.
    Startup { index: u32 },
    /// The private tail: the first 4 KiB boundary after the RTOS image.
    Tail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    /// The pinned address.
    pub address: u32,
    /// Each source gives the new address. All must agree.
    pub sources: Vec<Source>,
}

#[derive(Debug, Default)]
pub struct Plan {
    pub windows: Vec<Window>,
    pub symbols: Vec<Symbol>,
}

/// Builds a relocation plan from the pinned image.
pub struct Planner<'a> {
    image: &'a FirmwareImage,
    /// The search range on each side of a pinned window start. A window is
    /// unique within twice this range, so the plan holds while code and data
    /// move by less than it.
    radius: u32,
    /// The masked words of the image and of the initial `.data` content.
    masked: Vec<u32>,
    data: Vec<u32>,
    plan: Plan,
    windows: BTreeMap<(Region, u32, usize, bool), usize>,
    /// The B and BL sites of the image by target, built on first use.
    callers: Option<BTreeMap<u32, Vec<u32>>>,
}

fn words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks_exact(4)
        .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
        .collect()
}

fn prologue(word: u32) -> bool {
    // `mov ip, sp`, or `stmdb sp!, {..., lr}`.
    word == 0xe1a0_c00d || word & 0xffff_4000 == 0xe92d_4000
}

impl<'a> Planner<'a> {
    /// A planner for the installer, which searches within `RADIUS`.
    pub fn new(image: &'a FirmwareImage) -> Result<Self> {
        Self::with_radius(image, RADIUS)
    }

    /// A planner whose windows hold while code and data move by less than
    /// `radius`, such as for a port to another firmware version (`crate::port`).
    pub fn with_radius(image: &'a FirmwareImage, radius: u32) -> Result<Self> {
        let target = image.target;
        ensure!(
            target.data_start == DATA_START,
            "Relocation needs .data at {DATA_START:#x}"
        );
        let code = target
            .data_source
            .checked_sub(RTOS_BASE)
            .context("Invalid data source")? as usize;
        ensure!(code.is_multiple_of(4), "Unaligned data source");
        let masked: Vec<u32> = words(&image.body[..code]).into_iter().map(mask).collect();
        let data = words(image.read(DATA_START, target.data_size as usize)?)
            .into_iter()
            .map(mask)
            .collect();
        let literals = words(image.read(STARTUP_LITERALS, 20)?);
        ensure!(
            literals[0] == target.data_source
                && literals[1] == DATA_START
                && literals[2] == target.data_size
                && literals[4] == target.heap_min,
            "Unexpected startup literals"
        );
        Ok(Self {
            image,
            radius,
            masked,
            data,
            plan: Plan::default(),
            windows: BTreeMap::new(),
            callers: None,
        })
    }

    pub fn target(&self) -> &'static crate::target::Target {
        self.image.target
    }

    pub fn finish(self) -> Plan {
        self.plan
    }

    /// The masked words of the function around the code address `address`,
    /// and how often they occur within twice the radius, if more than once.
    /// Then only the code around the function makes its window unique, such
    /// as the start or the end of a run of equal functions, and the window
    /// keeps the offset of `address` from there. Another build of the same
    /// source keeps that offset. A new version can add a function to the run
    /// and move the others.
    pub fn repeated(&self, address: u32) -> Option<(Vec<u32>, usize)> {
        if !(RTOS_BASE..self.image.target.data_source).contains(&address) {
            return None;
        }
        let (start, end) = self.function(address);
        let words =
            &self.masked[((start - RTOS_BASE) / 4) as usize..((end - RTOS_BASE) / 4) as usize];
        let count = self.occurrences(Region::Image, start, words).len();
        (count > 1).then(|| (words.to_vec(), count))
    }

    fn region(&self, region: Region) -> (&[u32], u32) {
        match region {
            Region::Image => (&self.masked, RTOS_BASE),
            Region::Data => (&self.data, DATA_START),
        }
    }

    /// The pinned positions of `words` within twice the radius of `start`. A
    /// window that occurs once in that range cannot match another copy when
    /// the code moves by less than the radius.
    fn occurrences(&self, region: Region, start: u32, words: &[u32]) -> Vec<u32> {
        let (all, base) = self.region(region);
        let index = ((start - base) / 4) as usize;
        let radius = (2 * self.radius / 4) as usize;
        let low = index.saturating_sub(radius);
        let high = (index + radius).min(all.len().saturating_sub(words.len()));
        (low..=high)
            .filter(|&at| all[at..at + words.len()] == *words)
            .map(|at| base + at as u32 * 4)
            .collect()
    }

    fn window(&mut self, region: Region, start: u32, end: u32, search: bool) -> usize {
        let (all, base) = self.region(region);
        let words = all[((start - base) / 4) as usize..((end - base) / 4) as usize].to_vec();
        let key = (region, start, words.len(), search);
        if let Some(&index) = self.windows.get(&key) {
            return index;
        }
        let probe = if search {
            let index = ((start - base) / 4) as usize;
            let radius = (self.radius / 4) as usize;
            let around =
                &all[index.saturating_sub(radius)..(index + radius + words.len()).min(all.len())];
            let mut counts: BTreeMap<u32, usize> = words.iter().map(|&word| (word, 0)).collect();
            for word in around {
                if let Some(count) = counts.get_mut(word) {
                    *count += 1;
                }
            }
            (0..words.len())
                .min_by_key(|&i| counts[&words[i]])
                .unwrap_or(0)
        } else {
            0
        };
        self.plan.windows.push(Window {
            region,
            start,
            words,
            search,
            probe,
        });
        self.windows.insert(key, self.plan.windows.len() - 1);
        self.plan.windows.len() - 1
    }

    /// Grows `[start, end)` until its masked words occur once within twice
    /// the radius.
    fn unique(&self, region: Region, mut start: u32, mut end: u32) -> Option<(u32, u32)> {
        let (all, base) = self.region(region);
        let limit = base + all.len() as u32 * 4;
        let mut earlier = true;
        loop {
            let words = &all[((start - base) / 4) as usize..((end - base) / 4) as usize];
            if self.occurrences(region, start, words) == [start] {
                return Some((start, end));
            }
            if (end - start) as usize / 4 >= MAX_WINDOW {
                return None;
            }
            // Grow by 8 words, alternately before and after.
            if earlier && start >= base + 32 {
                start -= 32;
            } else if end + 32 <= limit {
                end += 32;
            } else if start >= base + 32 {
                start -= 32;
            } else {
                return None;
            }
            earlier = !earlier;
        }
    }

    /// The function around `address`: from the previous prologue to the next
    /// one, at most 256 words before and 512 words after.
    fn function(&self, address: u32) -> (u32, u32) {
        let all = &self.masked;
        let index = ((address - RTOS_BASE) / 4) as usize;
        let raw =
            |at: usize| u32::from_le_bytes(self.image.body[at * 4..at * 4 + 4].try_into().unwrap());
        let mut start = index;
        while start + 256 > index && start > 0 && !prologue(raw(start)) {
            start -= 1;
        }
        if start > 0 && raw(start - 1) == 0xe1a0_c00d {
            start -= 1;
        }
        let mut end = index + 1;
        while end < index + 512 && end < all.len() && !prologue(raw(end)) {
            end += 1;
        }
        if end > index + 1 && raw(end - 1) == 0xe1a0_c00d {
            end -= 1;
        }
        let end = end.max(start + 8).min(all.len());
        (RTOS_BASE + start as u32 * 4, RTOS_BASE + end as u32 * 4)
    }

    fn symbol(&mut self, address: u32) -> usize {
        match self.plan.symbols.iter().position(|s| s.address == address) {
            Some(index) => index,
            None => {
                self.plan.symbols.push(Symbol {
                    address,
                    sources: Vec::new(),
                });
                self.plan.symbols.len() - 1
            }
        }
    }

    fn add(&mut self, symbol: usize, source: Source) {
        let sources = &mut self.plan.symbols[symbol].sources;
        if !sources.contains(&source) {
            sources.push(source);
        }
    }

    fn in_image(&self, address: u32) -> bool {
        (RTOS_BASE..self.image.target.data_source).contains(&address)
    }

    /// A code address: a function, a hook site, or an address in a function.
    pub fn code(&mut self, address: u32) -> Result<usize> {
        ensure!(
            self.in_image(address) && address.is_multiple_of(4),
            "Code address {address:#x} is outside the RTOS code"
        );
        let symbol = self.symbol(address);
        let (start, end) = self.function(address);
        if let Some((start, end)) = self.unique(Region::Image, start, end) {
            let window = self.window(Region::Image, start, end, true);
            self.add(
                symbol,
                Source::Window {
                    window,
                    offset: address - start,
                },
            );
            return Ok(symbol);
        }
        // Equal functions, such as the getters of one template, follow each
        // other. Each is found through the callers of its entry, the window
        // start, and keeps its offset from the entry.
        let verify = self.window(Region::Image, start, end, false);
        let callers = self.callers().get(&start).cloned().unwrap_or_default();
        let mut found = false;
        for site in callers.into_iter().take(MAX_WITNESSES) {
            let (start, end) = self.function(site);
            if let Some((start, end)) = self.unique(Region::Image, start, end) {
                let window = self.window(Region::Image, start, end, true);
                self.add(
                    symbol,
                    Source::Branch {
                        window,
                        index: (site - start) / 4,
                        verify: Some(verify),
                    },
                );
                found = true;
            }
        }
        ensure!(found, "Code at {address:#x} has no unique window or caller");
        Ok(symbol)
    }

    fn callers(&mut self) -> &BTreeMap<u32, Vec<u32>> {
        let image = &self.image.body;
        let code = self.masked.len();
        self.callers.get_or_insert_with(|| {
            let mut callers: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
            for index in 0..code {
                let address = RTOS_BASE + index as u32 * 4;
                let word = u32::from_le_bytes(image[index * 4..index * 4 + 4].try_into().unwrap());
                if let Some(target) = branch_target(address, word) {
                    callers.entry(target).or_default().push(address);
                }
            }
            callers
        })
    }

    /// The target of the branch that a hook replaces at `site`.
    pub fn branch_target(&mut self, site: u32) -> Result<usize> {
        let word = u32::from_le_bytes(self.image.read(site, 4)?.try_into()?);
        let target = branch_target(site, word)
            .with_context(|| format!("No branch at hook site {site:#x}"))?;
        let symbol = self.symbol(target);
        let site_symbol = self.code(site)?;
        let sources = self.plan.symbols[site_symbol].sources.clone();
        let mut found = false;
        for source in sources {
            if let Source::Window { window, offset } = source {
                self.add(
                    symbol,
                    Source::Branch {
                        window,
                        index: offset / 4,
                        verify: None,
                    },
                );
                found = true;
            }
        }
        ensure!(found, "Hook site {site:#x} has no unique window");
        Ok(symbol)
    }

    /// Read-only data in the image, or initial `.data` content.
    pub fn content(&mut self, address: u32) -> Result<usize> {
        let target = self.image.target;
        let region = if self.in_image(address) {
            Region::Image
        } else if (DATA_START..DATA_START + target.data_size).contains(&address) {
            Region::Data
        } else {
            bail!("Content address {address:#x} is outside the image and .data")
        };
        let (all, base) = self.region(region);
        let index = ((address & !3) - base) / 4;
        let start = base + index.saturating_sub(CONTENT_BEFORE as u32) * 4;
        let end = (base + (index + CONTENT_AFTER as u32) * 4).min(base + all.len() as u32 * 4);
        let (start, end) = self
            .unique(region, start, end)
            .with_context(|| format!("Content at {address:#x} has no unique window"))?;
        let window = self.window(region, start, end, true);
        let symbol = self.symbol(address);
        self.add(
            symbol,
            Source::Window {
                window,
                offset: address - start,
            },
        );
        Ok(symbol)
    }

    /// An address through the code that loads it: literal words and MOVW and
    /// MOVT pairs that hold exactly this address. The installer uses it for
    /// data; a port also uses it as a second witness of image addresses.
    pub fn referenced(&mut self, address: u32) -> Result<usize> {
        let target = self.image.target;
        ensure!(
            (DATA_START..target.heap_min).contains(&address) || self.in_image(address),
            "Referenced address {address:#x} is outside the image, .data, and the BSS"
        );
        let symbol = self.symbol(address);
        let raw = words(&self.image.body[..self.masked.len() * 4]);
        let mut sites = Vec::new();
        for (index, &word) in raw.iter().enumerate() {
            if word == address {
                sites.push((index, None));
            } else if word & 0x0ff0_0000 == 0x0300_0000 && move_immediate(word) == address & 0xffff
            {
                // The MOVT of the same register within the next five words.
                let register = word & 0xf000;
                for (distance, &next) in raw.iter().enumerate().skip(index + 1).take(5) {
                    if next & 0x0ff0_0000 == 0x0340_0000 && next & 0xf000 == register {
                        if move_immediate(next) == address >> 16 {
                            sites.push((index, Some(distance)));
                        }
                        break;
                    }
                }
            }
        }
        let mut found = 0;
        for (index, movt) in sites {
            if found == MAX_WITNESSES {
                break;
            }
            let address_of = |at: usize| RTOS_BASE + at as u32 * 4;
            let site = address_of(index);
            let (start, end) = self.function(site);
            let last = address_of(movt.unwrap_or(index)) + 4;
            let Some((start, end)) = self.unique(Region::Image, start, end.max(last)) else {
                continue;
            };
            let window = self.window(Region::Image, start, end, true);
            let first = (site - start) / 4;
            let source = match movt {
                None => Source::Literal {
                    window,
                    index: first,
                },
                Some(movt) => Source::Pair {
                    window,
                    movw: first,
                    movt: (address_of(movt) - start) / 4,
                },
            };
            self.add(symbol, source);
            found += 1;
        }
        ensure!(found > 0, "Address {address:#x} has no unique reference");
        Ok(symbol)
    }

    /// Word `index` of the startup literals. The startup code before them must
    /// be equal.
    pub fn startup(&mut self, index: u32) -> Result<usize> {
        ensure!(index < 5, "Startup literal {index} does not exist");
        self.window(Region::Image, STARTUP_CODE, STARTUP_LITERALS, false);
        let value = u32::from_le_bytes(
            self.image
                .read(STARTUP_LITERALS + index * 4, 4)?
                .try_into()?,
        );
        let symbol = self.symbol(value);
        self.add(symbol, Source::Startup { index });
        Ok(symbol)
    }

    /// The private tail at its pinned base.
    pub fn tail(&mut self, base: u32) -> usize {
        let symbol = self.symbol(base);
        self.add(symbol, Source::Tail);
        symbol
    }
}

/// The startup code window, which the installer compares at its pinned address.
pub fn startup_window(plan: &Plan) -> Option<usize> {
    plan.windows.iter().position(|window| {
        window.region == Region::Image && window.start == STARTUP_CODE && !window.search
    })
}

/// The layout of another build of the RTOS image, from its startup literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageLayout {
    /// Where the image holds the initial `.data` content, and its size.
    pub data_source: u32,
    pub data_size: u32,
    /// The BSS end, where the script heap starts.
    pub heap_min: u32,
    /// The first 4 KiB boundary after the image, where the private tail goes.
    pub tail: u32,
}

/// Each symbol of a plan in another build of the RTOS image (`image`, loaded
/// at `RTOS_BASE`), with the sources that give it. The installer
/// (`runtime/relocation.h`) fails when any source fails. Here a source that
/// fails gives no value, the sources that give a value must agree, and a
/// symbol fails when no source gives a value. `radius` must be the radius that
/// the plan was made with.
pub struct Resolution {
    pub layout: ImageLayout,
    pub symbols: Vec<std::result::Result<(u32, Vec<Source>), String>>,
    /// Whether the installer would resolve each symbol: each source gave the
    /// value, and the startup window, if the plan has one, matches.
    pub complete: bool,
}

/// Finds the symbols of `plan` in `image`. See `Resolution`.
pub fn resolve(plan: &Plan, image: &[u8], radius: u32) -> Result<Resolution> {
    let raw = words(&image[..image.len() & !3]);
    let read = |address: u32| -> Option<u32> {
        raw.get((address.checked_sub(RTOS_BASE)? / 4) as usize)
            .copied()
    };
    let literal = |index: u32| {
        read(STARTUP_LITERALS + index * 4).context("The image has no startup literals")
    };
    let (data_source, data_start, data_size, heap_min) =
        (literal(0)?, literal(1)?, literal(2)?, literal(4)?);
    // The checks of runtime/relocation.h, and the image must hold `.data`.
    ensure!(
        data_start == DATA_START
            && data_source.is_multiple_of(4)
            && data_size.is_multiple_of(4)
            && data_size != 0
            && (RTOS_BASE..DATA_START).contains(&data_source)
            && DATA_START - data_source >= data_size
            && read(data_source + data_size - 4).is_some(),
        "Unexpected startup literals"
    );
    let layout = ImageLayout {
        data_source,
        data_size,
        heap_min,
        tail: (data_source + data_size + 0xfff) & !0xfff,
    };
    // A region address of a window and the image address that holds it.
    let limits = |region: Region| match region {
        Region::Image => (RTOS_BASE, data_source),
        Region::Data => (DATA_START, DATA_START + data_size),
    };
    let holder = |region: Region, address: u32| match region {
        Region::Image => address,
        Region::Data => address - DATA_START + data_source,
    };
    let matches = |window: &Window, at: u32| -> bool {
        let (low, high) = limits(window.region);
        at >= low
            && at.is_multiple_of(4)
            && (high - at.min(high)) / 4 >= window.words.len() as u32
            && window.words.iter().enumerate().all(|(index, &word)| {
                read(holder(window.region, at) + index as u32 * 4).map(mask) == Some(word)
            })
    };
    let found: Vec<std::result::Result<u32, String>> = plan
        .windows
        .iter()
        .map(|window| {
            if !window.search {
                return Err("not searched".into());
            }
            let (low, high) = limits(window.region);
            let from = window.start.saturating_sub(radius).max(low);
            let to = window.start.saturating_add(radius).min(high);
            let probe = window.words[window.probe];
            let mut hits = (from..=to)
                .step_by(4)
                .filter(|&at| {
                    read(holder(window.region, at) + window.probe as u32 * 4).map(mask)
                        == Some(probe)
                        && matches(window, at)
                })
                .take(2);
            match (hits.next(), hits.next()) {
                (Some(at), None) => Ok(at),
                (None, _) => Err(format!("window {:#x} not found", window.start)),
                (Some(_), Some(_)) => Err(format!("window {:#x} is ambiguous", window.start)),
            }
        })
        .collect();
    let value = |symbol: &Symbol, source: &Source| -> std::result::Result<u32, String> {
        let at = |window: usize| -> std::result::Result<u32, String> { found[window].clone() };
        let word = |window: usize, index: u32| -> std::result::Result<u32, String> {
            let region = plan.windows[window].region;
            read(holder(region, at(window)? + index * 4)).ok_or_else(|| "outside the image".into())
        };
        match *source {
            Source::Window { window, offset } => Ok(at(window)? + offset),
            Source::Literal { window, index } => word(window, index),
            Source::Pair { window, movw, movt } => {
                Ok(move_immediate(word(window, movt)?) << 16 | move_immediate(word(window, movw)?))
            }
            Source::Branch {
                window,
                index,
                verify,
            } => {
                let site = at(window)? + index * 4;
                let target = branch_target(site, word(window, index)?)
                    .ok_or_else(|| format!("no branch at {site:#x}"))?;
                match verify {
                    None => Ok(target),
                    Some(verify) if matches(&plan.windows[verify], target) => {
                        Ok(target + (symbol.address - plan.windows[verify].start))
                    }
                    Some(_) => Err(format!("the branch target {target:#x} differs")),
                }
            }
            Source::Startup { index } => literal(index).map_err(|error| error.to_string()),
            Source::Tail => Ok(layout.tail),
        }
    };
    let startup = startup_window(plan)
        .is_none_or(|window| matches(&plan.windows[window], plan.windows[window].start));
    let mut complete = startup;
    let symbols = plan
        .symbols
        .iter()
        .map(|symbol| {
            let (values, errors): (Vec<_>, Vec<_>) = symbol
                .sources
                .iter()
                .map(|source| value(symbol, source).map(|value| (value, *source)))
                .partition(std::result::Result::is_ok);
            let given: Vec<(u32, Source)> = values
                .into_iter()
                .map(std::result::Result::unwrap)
                .collect();
            complete &= given.len() == symbol.sources.len();
            let mut values: Vec<u32> = given.iter().map(|&(value, _)| value).collect();
            values.sort_unstable();
            values.dedup();
            complete &= values.len() == 1;
            match values[..] {
                [value] => Ok((value, given.into_iter().map(|(_, source)| source).collect())),
                [] => Err(errors
                    .into_iter()
                    .map(std::result::Result::unwrap_err)
                    .collect::<Vec<_>>()
                    .join("; ")),
                _ => Err(format!(
                    "the sources disagree: {}",
                    values
                        .iter()
                        .map(|v| format!("{v:#x}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            }
        })
        .collect();
    Ok(Resolution {
        layout,
        symbols,
        complete,
    })
}
