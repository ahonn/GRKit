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

use crate::firmware::FirmwareImage;
use anyhow::{bail, ensure, Result};
use grkit_core::native;
use std::collections::BTreeMap;

/// The failure message of the shell command, which the reader finds first.
const FAILURE: &[u8] = b"UserDataDebug::shell_UserData() faild";

// Registers.
const R0: u8 = 0;
const R1: u8 = 1;
const R2: u8 = 2;
const R4: u8 = 4;
const R5: u8 = 5;
const FP: u8 = 11;
const IP: u8 = 12;
const SP: u8 = 13;
const LR: u8 = 14;
const PC: u8 = 15;
const ALWAYS: u8 = 0xe;
const EQ: u8 = 0x0;
const NE: u8 = 0x1;

/// The A32 instructions that the patterns use. `Other` is any other one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    Movw(u8, u32),
    Movt(u8, u32),
    MovReg(u8, u8),
    MovImm(u8, u32),
    MvnImm(u8, u32),
    /// The register, the destination, the source, and the immediate. `S` set.
    SubImm(u8, u8, u32, bool),
    AddImm(u8, u8, u32),
    CmpImm(u8, u32),
    CmpReg,
    /// A load with an immediate offset: size, signed, Rt, Rn, offset.
    Load(u8, bool, u8, u8, i32),
    /// A load with a register offset.
    LoadIndexed(u8),
    /// A word or byte load with an immediate offset and writeback (`[rn,
    /// #offset]!`): size, Rt, Rn, offset.
    LoadUpdate(u8, u8, u8, i32),
    /// A store with an immediate offset: size, Rt, Rn, offset.
    Store(u8, u8, u8, i32),
    /// A store with a register offset, without a shift: size, Rt, Rn, Rm.
    StoreIndexed(u8, u8, u8, u8),
    /// A store with a shifted register offset.
    StoreShifted,
    /// Branch: condition and target. `link` for BL.
    Branch(u8, u32),
    Call(u32),
    /// BLX with an immediate target.
    Exchange(u32),
    /// `orr rd, rn, #imm` without `S`.
    OrrImm(u8, u8, u32),
    /// `bx lr`.
    Return,
    /// STMDB sp! (push).
    Push,
    /// LDM sp (`writeback` for pop) with the registers.
    LoadMultiple(bool, u16),
    Extend(u8, u8),
    Other,
}

#[derive(Clone, Copy, Debug)]
struct Insn {
    condition: u8,
    op: Op,
}

fn modified_immediate(word: u32) -> u32 {
    (word & 0xff).rotate_right(((word >> 8) & 0xf) * 2)
}

fn branch_target(address: u32, word: u32) -> u32 {
    let offset = ((word << 8) as i32 >> 6) as u32;
    address.wrapping_add(8).wrapping_add(offset)
}

fn decode(address: u32, word: u32) -> Insn {
    let condition = (word >> 28) as u8;
    let rd = ((word >> 12) & 0xf) as u8;
    let rn = ((word >> 16) & 0xf) as u8;
    let rm = (word & 0xf) as u8;
    let op = if condition == 0xf {
        if word & 0x0e00_0000 == 0x0a00_0000 {
            Op::Exchange(branch_target(address, word) | ((word >> 23) & 2))
        } else {
            Op::Other
        }
    } else if word & 0x0fff_ffff == 0x012f_ff1e {
        Op::Return
    } else if word & 0x0ff0_0000 == 0x0380_0000 {
        Op::OrrImm(rd, rn, modified_immediate(word))
    } else if word & 0x0ff0_0000 == 0x0300_0000 {
        Op::Movw(rd, ((word >> 4) & 0xf000) | (word & 0xfff))
    } else if word & 0x0ff0_0000 == 0x0340_0000 {
        Op::Movt(rd, ((word >> 4) & 0xf000) | (word & 0xfff))
    } else if word & 0x0fef_0ff0 == 0x01a0_0000 && word & 0x0010_0000 == 0 {
        Op::MovReg(rd, rm)
    } else if word & 0x0fef_0000 == 0x03a0_0000 && word & 0x0010_0000 == 0 {
        Op::MovImm(rd, modified_immediate(word))
    } else if word & 0x0fef_0000 == 0x03e0_0000 && word & 0x0010_0000 == 0 {
        Op::MvnImm(rd, modified_immediate(word))
    } else if word & 0x0fe0_0000 == 0x0240_0000 {
        Op::SubImm(rd, rn, modified_immediate(word), word & 0x0010_0000 != 0)
    } else if word & 0x0ff0_0000 == 0x0280_0000 {
        Op::AddImm(rd, rn, modified_immediate(word))
    } else if word & 0x0ff0_f000 == 0x0350_0000 {
        Op::CmpImm(rn, modified_immediate(word))
    } else if word & 0x0ff0_f010 == 0x0150_0000 {
        Op::CmpReg
    } else if word & 0x0f20_0000 == 0x0500_0000 {
        // LDR, LDRB, STR, STRB with an immediate offset and no writeback.
        let size = if word & 0x0040_0000 != 0 { 1 } else { 4 };
        let offset = (word & 0xfff) as i32;
        let offset = if word & 0x0080_0000 != 0 {
            offset
        } else {
            -offset
        };
        if word & 0x0010_0000 != 0 {
            Op::Load(size, false, rd, rn, offset)
        } else {
            Op::Store(size, rd, rn, offset)
        }
    } else if word & 0x0f30_0000 == 0x0530_0000 {
        let size = if word & 0x0040_0000 != 0 { 1 } else { 4 };
        let offset = (word & 0xfff) as i32;
        let offset = if word & 0x0080_0000 != 0 {
            offset
        } else {
            -offset
        };
        Op::LoadUpdate(size, rd, rn, offset)
    } else if word & 0x0f20_0010 == 0x0700_0000 {
        // The same with a register offset.
        let size = if word & 0x0040_0000 != 0 { 1 } else { 4 };
        if word & 0x0010_0000 != 0 {
            Op::LoadIndexed(rd)
        } else if word & 0x0000_0ff0 == 0 && word & 0x0080_0000 != 0 {
            Op::StoreIndexed(size, rd, rn, rm)
        } else {
            Op::StoreShifted
        }
    } else if word & 0x0e00_0090 == 0x0000_0090
        && word & 0x0000_0060 != 0
        && word & 0x0120_0000 == 0x0100_0000
    {
        // LDRH, LDRSB, LDRSH, STRH with an offset and no writeback.
        let kind = (word >> 5) & 3;
        let load = word & 0x0010_0000 != 0;
        let immediate = word & 0x0040_0000 != 0;
        let offset = (((word >> 4) & 0xf0) | (word & 0xf)) as i32;
        let offset = if word & 0x0080_0000 != 0 {
            offset
        } else {
            -offset
        };
        match (load, immediate, kind) {
            (true, true, 1) => Op::Load(2, false, rd, rn, offset),
            (true, true, 2) => Op::Load(1, true, rd, rn, offset),
            (true, true, 3) => Op::Load(2, true, rd, rn, offset),
            (true, false, _) => Op::LoadIndexed(rd),
            (false, true, 1) => Op::Store(2, rd, rn, offset),
            (false, false, 1) if word & 0x0000_0f00 == 0 && word & 0x0080_0000 != 0 => {
                Op::StoreIndexed(2, rd, rn, rm)
            }
            _ => Op::Other,
        }
    } else if word & 0x0e00_0000 == 0x0a00_0000 {
        if word & 0x0100_0000 != 0 {
            Op::Call(branch_target(address, word))
        } else {
            Op::Branch(condition, branch_target(address, word))
        }
    } else if word & 0x0fff_0000 == 0x092d_0000 {
        Op::Push
    } else if word & 0x0fdf_0000 == 0x089d_0000 {
        Op::LoadMultiple(word & 0x0020_0000 != 0, word as u16)
    } else if matches!(
        word & 0x0fff_03f0,
        0x06ef_0070 | 0x06af_0070 | 0x06ff_0070 | 0x06bf_0070
    ) {
        Op::Extend(rd, rm)
    } else {
        Op::Other
    };
    Insn { condition, op }
}

struct Image<'a> {
    image: &'a FirmwareImage,
}

impl Image<'_> {
    fn word(&self, address: u32) -> Result<u32> {
        Ok(u32::from_le_bytes(self.image.read(address, 4)?.try_into()?))
    }

    fn insn(&self, address: u32) -> Result<Insn> {
        Ok(decode(address, self.word(address)?))
    }

    fn string(&self, address: u32) -> Result<String> {
        let mut bytes = Vec::new();
        for at in address.. {
            let byte = self.image.read(at, 1)?[0];
            if byte == 0 {
                break;
            }
            ensure!(bytes.len() < 256, "String at {address:#x} has no end");
            bytes.push(byte);
        }
        Ok(bytes.iter().map(|&b| b as char).collect())
    }

    /// The code words of the image, with their addresses.
    fn code(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        let target = self.image.target;
        let end = (target.data_source - crate::target::RTOS_BASE) as usize;
        self.image.body[..end]
            .chunks_exact(4)
            .enumerate()
            .map(|(index, word)| {
                (
                    crate::target::RTOS_BASE + index as u32 * 4,
                    u32::from_le_bytes(word.try_into().unwrap()),
                )
            })
    }

    /// The address of a byte string in the image.
    fn find(&self, needle: &[u8]) -> Vec<u32> {
        let body = &self.image.body;
        (0..body.len().saturating_sub(needle.len()))
            .filter(|&i| body[i..i + needle.len()] == *needle)
            .map(|i| crate::target::RTOS_BASE + i as u32)
            .collect()
    }
}

/// A `movw rd, lo` and a `movt rd, hi` at most two words after it: the value
/// and the address after the movt.
fn movw_movt(image: &Image, address: u32, rd: u8) -> Result<Option<(u32, u32)>> {
    let Op::Movw(register, low) = image.insn(address)?.op else {
        return Ok(None);
    };
    if register != rd {
        return Ok(None);
    }
    for at in [address + 4, address + 8] {
        if let Op::Movt(register, high) = image.insn(at)?.op {
            if register == rd {
                return Ok(Some((high << 16 | low, at + 4)));
            }
        }
    }
    Ok(None)
}

/// A link of a strcmp chain: the string, where a match continues, and the
/// registers that are 0 there (the strcmp result and a `subs` copy of it).
struct Link {
    text: String,
    target: u32,
    zero: Vec<u8>,
}

/// How a strcmp chain ends: a last `bne` to its error path, or another
/// instruction.
enum ChainEnd {
    Mismatch,
    Other,
}

/// Parse a strcmp chain on `[base, #offset]` from `start`.
fn strcmp_chain(
    image: &Image,
    start: u32,
    base: u8,
    offset: i32,
    limit: usize,
) -> Result<(Vec<Link>, Option<u32>, ChainEnd)> {
    let (mut links, mut strcmp) = (Vec::new(), None);
    let (mut low, mut string, mut argument) = (None::<u32>, None::<u32>, false);
    let mut at = start;
    for _ in 0..limit {
        let insn = image.insn(at)?;
        match insn.op {
            Op::Load(4, false, R0, register, displacement)
                if register == base && displacement == offset =>
            {
                argument = true;
            }
            Op::Movw(R1, value) => {
                low = Some(value);
                string = None;
            }
            Op::Movt(R1, value) => {
                let Some(low) = low.take() else {
                    bail!("movt r1 without movw at {at:#x}");
                };
                string = Some(value << 16 | low);
            }
            Op::SubImm(FP, IP, 4, false) => {}
            Op::MovReg(rd, rm) if rd != R0 && rd != R1 && (rm == R0 || rm == R1) => {}
            Op::Exchange(target) if string.is_some() => {
                match strcmp {
                    None => strcmp = Some(target),
                    Some(known) => {
                        ensure!(known == target, "strcmp changed at {at:#x}: {target:#x}")
                    }
                }
                ensure!(
                    argument,
                    "strcmp at {at:#x} without r0 = [r{base}, #{offset}]"
                );
                let compare = image.insn(at + 4)?;
                let mut zero = vec![R0];
                match compare.op {
                    Op::CmpImm(R0, 0) => {}
                    Op::SubImm(rd, R0, 0, true) => zero.push(rd),
                    _ => bail!("No compare after strcmp at {:#x}", at + 4),
                }
                let text = image.string(string.take().unwrap())?;
                match image.insn(at + 8)?.op {
                    Op::Branch(EQ, target) => {
                        links.push(Link { text, target, zero });
                        at += 12;
                        argument = false;
                        continue;
                    }
                    Op::Branch(NE, _) => {
                        links.push(Link {
                            text,
                            target: at + 12,
                            zero,
                        });
                        return Ok((links, strcmp, ChainEnd::Mismatch));
                    }
                    _ => bail!("Unexpected branch after the compare at {:#x}", at + 8),
                }
            }
            _ => return Ok((links, strcmp, ChainEnd::Other)),
        }
        at += 4;
    }
    bail!("strcmp chain from {start:#x} is too long")
}

/// The handler of a dispatcher stub: `mov r0, r5; mov r1, r4; sub sp, fp, #N;
/// ldm sp, {...}; b HANDLER`.
fn stub_handler(image: &Image, stub: u32) -> Result<u32> {
    let ops: Vec<Op> = (0..5)
        .map(|k| image.insn(stub + 4 * k).map(|insn| insn.op))
        .collect::<Result<_>>()?;
    match ops[..] {
        [Op::MovReg(R0, R5), Op::MovReg(R1, R4), Op::SubImm(SP, FP, _, false), Op::LoadMultiple(false, registers), Op::Branch(ALWAYS, handler)]
            if registers & 1 << PC == 0 =>
        {
            Ok(handler)
        }
        _ => bail!("Unexpected dispatcher stub at {stub:#x}"),
    }
}

/// A value of the abstract interpreter of a match block or a setter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Value {
    Constant(u32),
    /// The return value of a call.
    Returned(u32),
    /// The setter object, the setter value.
    Object,
    Setting,
    /// A pointer that the setter loads from the object at the offset, plus a
    /// displacement; or the object plus a displacement.
    Pointer(Option<u32>, u32),
    Stack,
    Unknown,
}

/// A call of a match block: its target and r0 and r1.
struct BlockCall {
    target: u32,
    r0: Value,
    r1: Value,
}

/// Run a match block: its calls, up to the return or a tail call.
fn match_block(image: &Image, start: u32, zero: &[u8]) -> Result<Vec<BlockCall>> {
    let mut registers = [Value::Unknown; 16];
    for &register in zero {
        registers[register as usize] = Value::Constant(0);
    }
    let mut calls = Vec::new();
    let mut at = start;
    for _ in 0..64 {
        let insn = image.insn(at)?;
        ensure!(
            insn.condition == ALWAYS,
            "Conditional instruction in the match block at {at:#x}"
        );
        match insn.op {
            Op::Call(target) => {
                calls.push(BlockCall {
                    target,
                    r0: registers[0],
                    r1: registers[1],
                });
                registers[0] = Value::Returned(target);
                for register in [1, 2, 3, IP, LR] {
                    registers[register as usize] = Value::Unknown;
                }
            }
            Op::MovImm(rd, value) => registers[rd as usize] = Value::Constant(value),
            Op::MvnImm(rd, value) => registers[rd as usize] = Value::Constant(!value),
            Op::MovReg(rd, rm) => registers[rd as usize] = registers[rm as usize],
            Op::Movw(rd, value) => registers[rd as usize] = Value::Constant(value),
            Op::Movt(rd, value) => {
                let Value::Constant(low) = registers[rd as usize] else {
                    bail!("movt without movw at {at:#x}");
                };
                registers[rd as usize] = Value::Constant(value << 16 | (low & 0xffff));
            }
            Op::LoadMultiple(_, list) if list & 1 << PC != 0 => return Ok(calls),
            Op::SubImm(SP, FP, _, false) => {}
            Op::LoadMultiple(false, list) if list & 1 << LR != 0 => {
                let Op::Branch(ALWAYS, target) = image.insn(at + 4)?.op else {
                    bail!("ldm without a tail branch at {at:#x}");
                };
                calls.push(BlockCall {
                    target,
                    r0: registers[0],
                    r1: registers[1],
                });
                return Ok(calls);
            }
            Op::Branch(ALWAYS, target) => {
                at = target;
                continue;
            }
            _ => bail!("Unexpected instruction in the match block at {at:#x}"),
        }
        at += 4;
    }
    bail!("Match block at {start:#x} is too long")
}

/// The field that a setter stores to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field {
    /// The offset in the setter object of the pointer to the settings
    /// record (GR III), or `None` for a field of the object itself (GR IV).
    pub record: Option<u32>,
    pub offset: u32,
    pub width: u32,
}

/// Follow the setter on its fall-through path to the store of r1. A
/// conditional branch is not taken: the fall-through path is the path for a
/// changed value. One call with the object and the value is followed (the
/// inner setter of GR IV).
fn setter_field(image: &Image, setter: u32, depth: u32) -> Result<Field> {
    let mut registers = [Value::Unknown; 16];
    registers[0] = Value::Object;
    registers[1] = Value::Setting;
    for k in 0..40 {
        let at = setter + 4 * k;
        let insn = image.insn(at)?;
        if let Op::Branch(condition, _) = insn.op {
            if condition != ALWAYS {
                continue;
            }
        }
        ensure!(
            insn.condition == ALWAYS,
            "Conditional instruction before the store at {at:#x}"
        );
        match insn.op {
            Op::Push | Op::MovReg(IP, SP) | Op::SubImm(FP | SP, _, _, false) => {}
            Op::SubImm(rd, FP, _, _) => registers[rd as usize] = Value::Stack,
            Op::MovImm(rd, value) => registers[rd as usize] = Value::Constant(value),
            Op::MovReg(rd, rm) => registers[rd as usize] = registers[rm as usize],
            Op::Movw(rd, value) => registers[rd as usize] = Value::Constant(value),
            Op::Movt(rd, _) => registers[rd as usize] = Value::Unknown,
            Op::Extend(rd, rm) => {
                registers[rd as usize] = if registers[rm as usize] == Value::Setting {
                    Value::Setting
                } else {
                    Value::Unknown
                }
            }
            Op::CmpImm(..) | Op::CmpReg => {}
            Op::LoadIndexed(rd) => registers[rd as usize] = Value::Unknown,
            Op::Load(size, signed, rd, rn, displacement) => {
                registers[rd as usize] = if registers[rn as usize] == Value::Object
                    && size == 4
                    && !signed
                    && displacement >= 0
                {
                    Value::Pointer(Some(displacement as u32), 0)
                } else {
                    Value::Unknown
                }
            }
            Op::LoadUpdate(size, rd, rn, displacement) => {
                registers[rd as usize] =
                    if registers[rn as usize] == Value::Object && size == 4 && displacement >= 0 {
                        Value::Pointer(Some(displacement as u32), 0)
                    } else {
                        Value::Unknown
                    };
                // The base moved by the displacement.
                registers[rn as usize] = Value::Unknown;
            }
            Op::AddImm(rd, rn, value) => {
                registers[rd as usize] = match registers[rn as usize] {
                    Value::Pointer(record, offset) => Value::Pointer(record, offset + value),
                    Value::Object => Value::Pointer(None, value),
                    _ => Value::Unknown,
                }
            }
            Op::Call(target) => {
                ensure!(
                    depth == 0 && registers[0] == Value::Object && registers[1] == Value::Setting,
                    "Call before the store at {at:#x}"
                );
                return setter_field(image, target, depth + 1);
            }
            Op::Store(size, rt, rn, _) | Op::StoreIndexed(size, rt, rn, _) => {
                // A register offset must be a constant.
                let displacement = match insn.op {
                    Op::StoreIndexed(_, _, _, rm) => match registers[rm as usize] {
                        Value::Constant(value) => value as i32,
                        _ => bail!("Store at {at:#x} with a register offset that is not constant"),
                    },
                    Op::Store(_, _, _, displacement) => displacement,
                    _ => unreachable!(),
                };
                let (source, base) = (registers[rt as usize], registers[rn as usize]);
                if source == Value::Setting {
                    ensure!(displacement >= 0, "Store at {at:#x} below the field base");
                    let width = u32::from(size);
                    return match base {
                        Value::Pointer(record, offset) => Ok(Field {
                            record,
                            offset: offset + displacement as u32,
                            width,
                        }),
                        Value::Object => Ok(Field {
                            record: None,
                            offset: displacement as u32,
                            width,
                        }),
                        _ => bail!("Store of the value at {at:#x} to an unknown base"),
                    };
                }
                ensure!(
                    rn == FP || rn == SP || base == Value::Stack,
                    "Store at {at:#x} is not the value to the settings"
                );
            }
            _ => bail!("No store of the value before {at:#x}"),
        }
    }
    bail!("No store within 40 instructions of the setter {setter:#x}")
}

/// What a handler accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Each label and the value that it stores.
    Labels(Vec<(String, i32)>),
    /// A number that strtol parses.
    Number,
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

/// A label handler: `mov r5, r1`, then a chain of strcmp on argv[3].
fn label_handler(image: &Image, handler: u32, strcmp: u32) -> Result<Setting> {
    ensure!(
        (0..8).any(|k| matches!(
            image.insn(handler + 4 * k).map(|i| i.op),
            Ok(Op::MovReg(R5, R1))
        )),
        "Handler {handler:#x} has no mov r5, r1"
    );
    let mut start = handler;
    loop {
        match image.insn(start)?.op {
            Op::Load(4, false, R0, R5, 12) | Op::Movw(R1, _) => break,
            _ => {}
        }
        start += 4;
        ensure!(
            start - handler <= 0x40,
            "Handler {handler:#x}: no chain start"
        );
    }
    let (links, chain_strcmp, end) = strcmp_chain(image, start, R5, 12, 20000)?;
    ensure!(
        chain_strcmp == Some(strcmp),
        "Handler {handler:#x} uses another strcmp"
    );
    ensure!(
        matches!(end, ChainEnd::Mismatch),
        "Handler {handler:#x}: the chain does not end with a mismatch branch"
    );
    let mut labels = Vec::new();
    let mut found: Option<(u32, Vec<u32>)> = None;
    for link in &links {
        let calls = match_block(image, link.target, &link.zero)?;
        let Some((setter, getters)) = calls.split_last() else {
            bail!("Handler {handler:#x}, label {}: no call", link.text);
        };
        let Value::Constant(value) = setter.r1 else {
            bail!(
                "Handler {handler:#x}, label {}: r1 is not constant",
                link.text
            );
        };
        let getters: Vec<u32> = getters.iter().map(|call| call.target).collect();
        ensure!(
            getters.last().map(|&g| Value::Returned(g)) == Some(setter.r0),
            "Handler {handler:#x}, label {}: the setter object is not the last getter",
            link.text
        );
        match &found {
            None => found = Some((setter.target, getters)),
            Some((known, known_getters)) => ensure!(
                *known == setter.target && *known_getters == getters,
                "Handler {handler:#x}: the labels use other setters or getters"
            ),
        }
        labels.push((link.text.clone(), value as i32));
    }
    let (setter, getters) = found.unwrap();
    Ok(Setting {
        kind: Kind::Labels(labels),
        setter,
        getters,
        field: setter_field(image, setter, 0)?,
    })
}

/// A number handler: strtol(argv[3], &end, 0), a check of *end, the getters,
/// a move or an extension of the number into r1, and the setter.
fn number_handler(image: &Image, handler: u32) -> Result<Setting> {
    let (mut calls, mut r2_zero, mut argument) = (Vec::new(), false, false);
    let (mut parsed, mut converted, mut checked) = (None, false, false);
    let mut at = handler;
    for _ in 0..40 {
        let insn = image.insn(at)?;
        match insn.op {
            Op::MovImm(R2, 0) => r2_zero = true,
            Op::Load(4, false, R0, R1, 12) => argument = true,
            Op::Call(target) => {
                ensure!(
                    !calls.is_empty() || (argument && r2_zero),
                    "Number handler {handler:#x}: the first call is not strtol(argv[3], &end, 0)"
                );
                calls.push(target);
                if calls.len() >= 2 && converted {
                    break;
                }
            }
            Op::MovReg(rd, R0) if calls.len() == 1 && parsed.is_none() => parsed = Some(rd),
            Op::Load(1, ..) => {}
            Op::CmpImm(..) | Op::CmpReg if calls.len() == 1 => checked = true,
            Op::Branch(NE, _) if calls.len() == 1 => {}
            Op::Extend(R1, rm) | Op::MovReg(R1, rm) if Some(rm) == parsed => converted = true,
            Op::Push
            | Op::Movw(..)
            | Op::Movt(..)
            | Op::Store(..)
            | Op::SubImm(..)
            | Op::Load(4, false, ..)
            | Op::LoadUpdate(4, ..)
            | Op::MovReg(IP, SP) => {}
            _ => bail!("Number handler {handler:#x}: unexpected instruction at {at:#x}"),
        }
        at += 4;
    }
    ensure!(
        checked,
        "Number handler {handler:#x}: no check of the end pointer"
    );
    ensure!(
        calls.len() >= 2 && converted,
        "Number handler {handler:#x}: no setter call"
    );
    let setter = *calls.last().unwrap();
    Ok(Setting {
        kind: Kind::Number,
        setter,
        getters: calls[1..calls.len() - 1].to_vec(),
        field: setter_field(image, setter, 0)?,
    })
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
/// type and the parameters of set No0, the white balance, and the correction
/// and noise reduction settings of still pictures.
pub fn recipe_setting(name: &str) -> bool {
    const PREFIXES: [&str; 5] = [
        "StillActiveImageControlSet",
        "StillImageControlNo0",
        "AfterReleaseStillImageControlNo0Param",
        "StillFwupImageControlNo0Param",
        "StillWhiteBalance",
    ];
    const NAMES: [&str; 4] = [
        "StillHighlightCorrect",
        "StillShadowCorrect",
        "StillLensShadingCorrection",
        "StillHighIsoNR",
    ];
    // The names of a custom set are text settings.
    (PREFIXES.iter().any(|prefix| name.starts_with(prefix)) && !name.ends_with("UserCustomName"))
        || NAMES.contains(&name)
}

/// The setting of a dispatcher stub.
fn setting(image: &Image, stub: u32, strcmp: u32) -> Result<Setting> {
    let handler = stub_handler(image, stub)?;
    let head: Vec<Op> = (0..12)
        .map(|k| image.insn(handler + 4 * k).map(|insn| insn.op))
        .collect::<Result<_>>()?;
    if head.contains(&Op::Load(4, false, R1, R1, 12)) {
        bail!("a text setting is not supported")
    } else if head.contains(&Op::Load(4, false, R0, R1, 12)) {
        number_handler(image, handler)
    } else {
        label_handler(image, handler, strcmp)
    }
}

/// The labels of a setting as a sorted list of (label, stored value).
fn label_values(setting: &Setting) -> Option<Vec<(String, i64)>> {
    let Kind::Labels(labels) = &setting.kind else {
        return None;
    };
    let mut values: Vec<(String, i64)> = labels
        .iter()
        .map(|(label, value)| (label.clone(), i64::from(*value)))
        .collect();
    values.sort();
    Some(values)
}

fn model_values(labels: &native::Labels) -> Vec<(String, i64)> {
    let mut values: Vec<(String, i64)> = labels
        .values()
        .map(|(label, value)| (label.to_string(), i64::from(value)))
        .collect();
    values.sort();
    values
}

/// The stored type value of each type whose parameter fields `fields` gives,
/// from the functions that pass a changed parameter on to the picture
/// pipeline. Each one runs only while the current type is its own:
///
/// ```text
/// ldrb r3, [r0, #FIELD]        ; the parameter
/// ...                          ; at most 10 words
/// movw r3, #CURRENT
/// ldrh r3, [r0, r3]            ; the current type << 8
/// cmp  r3, #TYPE << 8
/// ```
///
/// A type has its value only if each such function of its fields compares
/// with one value.
fn synced_types(image: &Image, fields: &BTreeMap<u32, usize>) -> Result<BTreeMap<usize, u32>> {
    let code: Vec<(u32, u32)> = image.code().collect();
    let words: Vec<u32> = code.iter().map(|&(_, word)| word).collect();
    let mut found: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
    for (index, &word) in words.iter().enumerate() {
        // ldrb r3, [r0, #imm12]
        if word & 0xffff_f000 != 0xe5d0_3000 {
            continue;
        }
        let Some(&image_type) = fields.get(&(word & 0xfff)) else {
            continue;
        };
        for at in index + 1..(index + 12).min(words.len().saturating_sub(2)) {
            let (movw, load, compare) = (words[at], words[at + 1], words[at + 2]);
            // movw r3, #imm16; ldrh r3, [r0, rM]; cmp r3, #imm
            if movw & 0xfff0_f000 == 0xe300_3000
                && load & 0xffff_fff0 == 0xe190_30b0
                && compare & 0xffff_f000 == 0xe353_0000
            {
                let value = modified_immediate(compare);
                ensure!(
                    value & 0xff == 0,
                    "A type check at {:#x} is not a type",
                    code[at + 2].0
                );
                found.entry(image_type).or_default().push(value >> 8);
                break;
            }
        }
    }
    let mut types = BTreeMap::new();
    for (image_type, values) in found {
        ensure!(
            values.iter().all(|&value| value == values[0]),
            "The parameters of type {} check several types: {values:?}",
            native::TYPES[image_type].label
        );
        types.insert(image_type, values[0]);
    }
    Ok(types)
}

/// The model check of GR IV (`FW_MODEL_RECORD`): the address of the
/// pointer to the current model record, and the offset of the byte that
/// `SystemConfig_GetImageSensorColorType` returns, 1 on GR IV Monochrome.
///
/// ```text
/// movw/movt rX, POINTER
/// ldr  rY, [rX]
/// ldrb r0, [rY, #OFFSET]       ; conditional: the record is not 0
/// ...
/// movw/movt rZ, "SystemConfig_GetImageSensorColorType"   ; its error path
/// ```
fn sensor_color(image: &Image) -> Result<(u32, u32)> {
    let names = image.find(b"SystemConfig_GetImageSensorColorType\0");
    ensure!(
        names.len() == 1,
        "{} names of the sensor color check",
        names.len()
    );
    let name = names[0];
    let mut references = Vec::new();
    for (address, word) in image.code() {
        if let Op::Movw(rd, low) = decode(address, word).op {
            if low == name & 0xffff
                && movw_movt(image, address, rd)?.map(|(value, _)| value) == Some(name)
            {
                references.push(address);
            }
        }
    }
    ensure!(
        references.len() == 1,
        "{} uses of the sensor color check name",
        references.len()
    );
    let reference = references[0];
    for start in (reference - 0x40..reference).step_by(4) {
        let Op::Movw(pointer_register, _) = image.insn(start)?.op else {
            continue;
        };
        let Some((pointer, after)) = movw_movt(image, start, pointer_register)? else {
            continue;
        };
        let mut record = None;
        for at in (after..reference).step_by(4) {
            match image.insn(at)?.op {
                Op::Load(4, false, rt, rn, 0) if rn == pointer_register => record = Some(rt),
                Op::Load(1, false, R0, rn, offset) if Some(rn) == record && offset > 0 => {
                    return Ok((pointer, offset as u32));
                }
                _ => {}
            }
        }
    }
    bail!("No model record in the sensor color check at {reference:#x}")
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

/// The hook site whose BL calls the native icon mapper of the types.
const ICON_MAPPER_SITE: &str = "menu_viewfinder_icon";

/// Whether condition `condition` holds for the flags N, Z, C, and V.
fn condition_holds(condition: u8, [n, z, c, v]: [bool; 4]) -> Result<bool> {
    Ok(match condition {
        0x0 => z,
        0x1 => !z,
        0x2 => c,
        0x3 => !c,
        0x4 => n,
        0x5 => !n,
        0x6 => v,
        0x7 => !v,
        0x8 => c && !z,
        0x9 => !c || z,
        0xa => n == v,
        0xb => n != v,
        0xc => !z && n == v,
        0xd => z || n != v,
        ALWAYS => true,
        _ => bail!("Condition {condition:#x} is not a condition"),
    })
}

/// The icon that the native icon mapper returns for type code `code`.
///
/// The live-view Image Control widget calls the mapper at the
/// `menu_viewfinder_icon` site with the type code in r1, and the native Image
/// Control list calls it for the icon of each row. GR IV enters it through a
/// thunk that adds to r0 and sets r2 to 0, where a nonzero r2 asks for no
/// icon. The mapper is a chain of `cmp r1, #CODE` and branches to blocks that
/// put the icon in r0 and return:
///
/// ```text
/// cmp   r1, #0xa
/// bls   NONE            ; mov r0, #0; bx lr
/// cmp   r1, #0xb
/// beq   STANDARD        ; cmp r2, #0; movw r0, #ICON; movne r0, #0; bx lr
/// ...
/// ```
///
/// The reader runs the code from the BL target with only r1 known, and only
/// for the instructions of this shape: each other instruction, or a read of
/// an unknown register, is an error.
fn type_icon(image: &Image, mapper: u32, code: u32) -> Result<u32> {
    let mut registers: [Option<u32>; 16] = [None; 16];
    registers[R1 as usize] = Some(code);
    let mut flags = None;
    let mut at = mapper;
    for _ in 0..128 {
        let insn = image.insn(at)?;
        let read = |register: u8| {
            registers[register as usize]
                .ok_or_else(|| anyhow::anyhow!("The icon mapper reads r{register} at {at:#x}"))
        };
        if insn.condition != ALWAYS {
            let flags =
                flags.ok_or_else(|| anyhow::anyhow!("The icon mapper has no flags at {at:#x}"))?;
            if !condition_holds(insn.condition, flags)? {
                at += 4;
                continue;
            }
        }
        match insn.op {
            Op::CmpImm(rn, value) => {
                let left = read(rn)?;
                let result = left.wrapping_sub(value);
                flags = Some([
                    result >> 31 != 0,
                    result == 0,
                    left >= value,
                    ((left ^ value) & (left ^ result)) >> 31 != 0,
                ]);
            }
            Op::MovImm(rd, value) | Op::Movw(rd, value) => registers[rd as usize] = Some(value),
            Op::OrrImm(rd, rn, value) => registers[rd as usize] = Some(read(rn)? | value),
            // The thunk moves r0, which the mapper does not read.
            Op::AddImm(rd, rn, value) => {
                registers[rd as usize] = registers[rn as usize].map(|x| x.wrapping_add(value))
            }
            Op::Branch(_, target) => {
                at = target;
                continue;
            }
            Op::Return => return read(R0),
            _ => bail!("Unexpected instruction in the icon mapper at {at:#x}"),
        }
        at += 4;
    }
    bail!("The icon mapper at {mapper:#x} does not return for type {code}")
}

/// The native icon of each portable type of `codes` (`NATIVE_TYPE_CODES`),
/// or 0 for a type that the camera does not have. Each type that the camera
/// has must have an icon of the native range.
fn type_icons(image: &FirmwareImage, codes: &[u32]) -> Result<Vec<u32>> {
    let target = image.target;
    let &(_, site, _) = target
        .layout
        .sites
        .iter()
        .find(|(name, _, _)| *name == ICON_MAPPER_SITE)
        .ok_or_else(|| anyhow::anyhow!("{}: no {ICON_MAPPER_SITE} site", target.name))?;
    let image = Image { image };
    let Op::Call(mapper) = image.insn(site)?.op else {
        bail!("{}: no BL at the {ICON_MAPPER_SITE} site", target.name);
    };
    codes
        .iter()
        .zip(native::TYPES.iter())
        .map(|(&code, image_type)| {
            if code == 0xff {
                return Ok(0);
            }
            let icon = type_icon(&image, mapper, code)?;
            ensure!(
                icon != 0 && icon < 0x7000,
                "{}: the icon of {} is {icon:#x}",
                target.name,
                image_type.label
            );
            Ok(icon)
        })
        .collect()
}

/// `runtime/targets/<target>-native.h`: the stored values and the field
/// offsets of the native settings of recipes (`grkit_core::native`) in the
/// firmware of `image`. Each name, label, and stored value of the model must
/// be those of the firmware, and the firmware may have no parameter of a
/// type that the model leaves out.
pub fn native_header(image: &FirmwareImage) -> Result<String> {
    let target = image.target;
    let family = native::Family::of_target(target.name)
        .ok_or_else(|| anyhow::anyhow!("{} has no native family", target.name))?;
    let table = extract(image, recipe_setting)?;
    let get = |name: &str| -> Result<&Setting> {
        table
            .settings
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("{}: no setting {name}", target.name))
    };
    let byte_field = |name: &str| -> Result<u32> {
        let setting = get(name)?;
        ensure!(
            setting.field.width == 1,
            "{}: {name} is not one byte",
            target.name
        );
        Ok(setting.field.offset)
    };
    let check_labels = |name: &str, labels: &native::Labels| -> Result<u32> {
        let setting = get(name)?;
        ensure!(
            label_values(setting) == Some(model_values(labels)),
            "{}: the labels of {name} differ from the model",
            target.name
        );
        byte_field(name)
    };
    // One record shape for each setting.
    let first = table.settings.values().next().unwrap();
    for (name, setting) in &table.settings {
        ensure!(
            setting.field.record == first.field.record && setting.getters == first.getters,
            "{}: {name} has another record or getter",
            target.name
        );
    }
    // The active set and the type: the label and the stored value of each.
    let active = get(native::ACTIVE_SET_SETTING)?;
    ensure!(
        matches!(&active.kind, Kind::Labels(labels) if labels.iter().any(|(l, v)| l == native::MAIN_SET && *v == 0)),
        "{}: no main set No0 = 0",
        target.name
    );
    let type_setting = get(native::TYPE_SETTING)?;
    let Kind::Labels(type_labels) = &type_setting.kind else {
        bail!("{}: the type is not a label setting", target.name);
    };
    // A type is in the build if its family has it, but a build can lack a
    // Monochrome type that a later build adds: then the firmware has none of
    // its parameters.
    let present: Vec<bool> = native::TYPES
        .iter()
        .enumerate()
        .map(|(index, image_type)| {
            image_type.families[family as usize].is_some()
                && (!image_type.monochrome
                    || (0..native::PARAMETERS.len()).any(|parameter| {
                        native::parameter_name(family, index, parameter)
                            .is_some_and(|name| table.settings.contains_key(&name))
                    }))
        })
        .collect();
    // The parameters of each type.
    let mut fields = Vec::new();
    for (index, image_type) in native::TYPES.iter().enumerate() {
        for parameter in 0..native::PARAMETERS.len() {
            match native::parameter_name(family, index, parameter).filter(|_| present[index]) {
                Some(name) => {
                    let labels = native::type_parameter_labels(family, index, parameter).unwrap();
                    fields.push(check_labels(&name, &labels)?);
                }
                None => {
                    for prefix in native::Prefix::ALL {
                        let name = format!(
                            "{}{}{}",
                            prefix.text(),
                            image_type.label,
                            native::PARAMETERS[parameter].key
                        );
                        ensure!(
                            !table.settings.contains_key(&name)
                                || image_type.families[family as usize].is_none(),
                            "{}: the firmware has {name}, which the model does not",
                            target.name
                        );
                    }
                    fields.push(0);
                }
            }
        }
    }
    // The stored value of each type: the label of StillImageControlNo0, or
    // for a Monochrome type, the value that the functions of its parameters
    // check. The other types check that the two agree.
    let parameters = native::PARAMETERS.len();
    let type_fields: BTreeMap<u32, usize> = fields
        .iter()
        .enumerate()
        .filter(|&(_, &field)| field != 0)
        .map(|(at, &field)| (field, at / parameters))
        .collect();
    let synced = if native::TYPES
        .iter()
        .any(|t| t.monochrome && t.families[family as usize].is_some())
    {
        synced_types(&Image { image }, &type_fields)?
    } else {
        BTreeMap::new()
    };
    let mut codes = Vec::new();
    for (index, image_type) in native::TYPES.iter().enumerate() {
        let code = if !present[index] {
            0xff
        } else if image_type.monochrome {
            *synced.get(&index).ok_or_else(|| {
                anyhow::anyhow!("{}: no stored value of {}", target.name, image_type.label)
            })?
        } else {
            let (_, value) = type_labels
                .iter()
                .find(|(l, _)| l == image_type.label)
                .ok_or_else(|| anyhow::anyhow!("{}: no type {}", target.name, image_type.label))?;
            let value = u32::try_from(*value)?;
            if let Some(&checked) = synced.get(&index) {
                ensure!(
                    checked == value,
                    "{}: {} is {value}, but its parameters check {checked}",
                    target.name,
                    image_type.label
                );
            }
            value
        };
        codes.push(code);
    }
    let labelled = codes
        .iter()
        .zip(native::TYPES.iter())
        .filter(|(&code, image_type)| code != 0xff && !image_type.monochrome)
        .count();
    ensure!(
        type_labels.len() == labelled,
        "{}: the firmware has a type that the model does not",
        target.name
    );
    let mut values: Vec<u32> = codes.iter().copied().filter(|&code| code != 0xff).collect();
    values.sort_unstable();
    values.dedup();
    ensure!(
        values.len() == codes.iter().filter(|&&code| code != 0xff).count(),
        "{}: two types have one stored value",
        target.name
    );
    let icons = type_icons(image, &codes)?;
    // The white balance modes, their shifts, and the color temperature.
    let mode_setting = get(native::WHITE_BALANCE_SETTING)?;
    let Kind::Labels(mode_labels) = &mode_setting.kind else {
        bail!("{}: the white balance is not a label setting", target.name);
    };
    let (mut mode_codes, mut shifts) = (Vec::new(), Vec::new());
    for (index, label) in native::mode_labels(family).into_iter().enumerate() {
        mode_codes.push(match label {
            Some(label) => {
                let (_, value) = mode_labels
                    .iter()
                    .find(|(l, _)| l == label)
                    .ok_or_else(|| anyhow::anyhow!("{}: no white balance {label}", target.name))?;
                u32::try_from(*value)?
            }
            None => 0xffff,
        });
        match native::shift_names(family, index) {
            Some([ba, gm]) => {
                shifts.push(check_labels(&ba, &native::SHIFT_BA)?);
                shifts.push(check_labels(&gm, &native::SHIFT_GM)?);
            }
            None => shifts.extend([0, 0]),
        }
    }
    ensure!(
        mode_labels.len() == mode_codes.iter().filter(|&&code| code != 0xffff).count(),
        "{}: the firmware has a white balance mode that the model does not",
        target.name
    );
    let temperature = get(native::COLOR_TEMPERATURE)?;
    ensure!(
        temperature.kind == Kind::Number && temperature.field.width == 2,
        "{}: the color temperature is not a number of two bytes",
        target.name
    );
    let corrections: Vec<u32> = native::CORRECTIONS
        .iter()
        .map(|correction| check_labels(correction.name, &correction.labels))
        .collect::<Result<_>>()?;
    let list = |values: &[u32]| {
        values
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    let mut out = format!(
        "/* Generated from the firmware of {} (crates/grkit-build/src/userdata.rs). Do\n   \
         not edit: with the firmware files of the ignored firmware tests, run\n   \
         GRKIT_UPDATE_GENERATED=1 cargo test -p grkit-build -- --ignored\n   \
         native_headers_match_each_firmware.\n\n   \
         The stored values and the field offsets of the native settings of recipes\n   \
         (runtime/native.h). FW_SETTINGS gives the settings object. A field is at\n   \
         its offset in the settings record, which is the word at NATIVE_RECORD in\n   \
         the object, or the object itself without NATIVE_RECORD. The fields are one\n   \
         byte, the color temperature two. Each offset of a parameter that a type\n   \
         does not have is 0, and each code of a type or mode that the camera does\n   \
         not have is all ones. NATIVE_TYPE_ICONS gives the native icon ID of each\n   \
         type, as the native Image Control list shows it, or 0. On GR IV, the byte\n   \
         at NATIVE_SENSOR_COLOR_FIELD of the model record (FW_MODEL_RECORD) is 1\n   \
         on GR IV Monochrome. */\n",
        target.name
    );
    out.push_str("#ifndef GRKIT_TARGET_NATIVE_H\n#define GRKIT_TARGET_NATIVE_H\n");
    // The family of the names and labels in runtime/native.h.
    out.push_str(&format!(
        "#define NATIVE_FAMILY_{} 1\n",
        family.key().to_ascii_uppercase()
    ));
    if let Some(record) = first.field.record {
        out.push_str(&format!("#define NATIVE_RECORD {record}U\n"));
    }
    out.push_str(&format!(
        "#define NATIVE_ACTIVE_SET_FIELD {}U\n#define NATIVE_TYPE_FIELD {}U\n",
        byte_field(native::ACTIVE_SET_SETTING)?,
        type_setting.field.offset
    ));
    out.push_str(&format!("#define NATIVE_TYPE_CODES {}\n", list(&codes)));
    out.push_str(&format!("#define NATIVE_TYPE_ICONS {}\n", list(&icons)));
    out.push_str(&format!(
        "#define NATIVE_PARAMETER_FIELDS {}\n",
        list(&fields)
    ));
    out.push_str(&format!(
        "#define NATIVE_MODE_FIELD {}U\n#define NATIVE_MODE_CODES {}\n",
        byte_field(native::WHITE_BALANCE_SETTING)?,
        list(&mode_codes)
    ));
    out.push_str(&format!("#define NATIVE_SHIFT_FIELDS {}\n", list(&shifts)));
    out.push_str(&format!(
        "#define NATIVE_TEMPERATURE_FIELD {}U\n",
        temperature.field.offset
    ));
    out.push_str(&format!(
        "#define NATIVE_CORRECTION_FIELDS {}\n",
        list(&corrections)
    ));
    if native::TYPES
        .iter()
        .any(|t| t.monochrome && t.families[family as usize].is_some())
    {
        // The build header gives the model record (FW_MODEL_RECORD) and the
        // type setter (FW_IMAGE_CONTROL_SET), which monochrome_check reads.
        let (_, offset) = sensor_color(&Image { image })?;
        out.push_str(&format!("#define NATIVE_SENSOR_COLOR_FIELD {offset:#x}U\n"));
    }
    out.push_str("#endif\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Words of GR III 2.10 and GR IV 1.20 at their addresses.
    #[test]
    fn decodes_the_instructions_of_the_patterns() {
        let cases = [
            (0x5334_6894, 0xe1a0_4001, Op::MovReg(R4, R1)),
            (0x5334_6898, 0xe303_16e8, Op::Movw(R1, 0x36e8)),
            (0x5334_68a4, 0xe345_13b2, Op::Movt(R1, 0x53b2)),
            (0x5334_68a8, 0xe594_0008, Op::Load(4, false, R0, R4, 8)),
            (0x5330_797c, 0xe250_4000, Op::SubImm(R4, R0, 0, true)),
            (0x5325_2aac, 0xe590_3004, Op::Load(4, false, 3, R0, 4)),
            (0x5325_2ab4, 0xe5c3_14d6, Op::Store(1, R1, 3, 0x4d6)),
            (0x5325_2ab8, 0xebff_ffe1, Op::Call(0x5325_2a44)),
            (0x5325_2abc, 0xe350_0000, Op::CmpImm(R0, 0)),
            (0x532f_ddd4, 0xe150_0001, Op::CmpReg),
            (0x532f_dde0, 0xe5c5_14a9, Op::Store(1, R1, R5, 0x4a9)),
            (0x532f_dea4, 0xe50b_302c, Op::Store(4, 3, FP, -0x2c)),
            (0x5327_3628, 0xe5b3_2004, Op::LoadUpdate(4, R2, 3, 4)),
            (0x532f_de84, 0xe3a0_3000, Op::MovImm(3, 0)),
            (0x532f_de88, 0xe92d_d870, Op::Push),
            (0x532f_dddc, 0x0a00_0022, Op::Branch(EQ, 0x532f_de6c)),
            (0x5339_068c, 0x1382_2001, Op::OrrImm(R2, R2, 1)),
            (0x5339_069c, 0xe12f_ff1e, Op::Return),
        ];
        for (address, word, op) in cases {
            assert_eq!(decode(address, word).op, op, "{address:#x}");
        }
        // blx #0x531416e8 from 0x533468ac.
        assert_eq!(
            decode(0x5334_68ac, 0xfaf7_eb8d).op,
            Op::Exchange(0x5314_16e8)
        );
        // A conditional instruction keeps its condition.
        assert_eq!(decode(0x532f_dddc, 0x0a00_0022).condition, EQ);
    }

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
