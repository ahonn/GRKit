//! The code shapes of the UserData Set dispatcher (see `userdata`): the
//! strcmp chains of the dispatcher and of the label handlers, the number
//! handlers, and the store of a setter.

use super::{Field, Kind, Setting};
use crate::arm::decode::*;
use crate::firmware::FirmwareImage;
use anyhow::{bail, ensure, Result};

pub(super) struct Image<'a> {
    pub(super) image: &'a FirmwareImage,
}

impl Image<'_> {
    pub(super) fn word(&self, address: u32) -> Result<u32> {
        Ok(u32::from_le_bytes(self.image.read(address, 4)?.try_into()?))
    }

    pub(super) fn insn(&self, address: u32) -> Result<Insn> {
        Ok(decode(address, self.word(address)?))
    }

    pub(super) fn string(&self, address: u32) -> Result<String> {
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
    pub(super) fn code(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
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
    pub(super) fn find(&self, needle: &[u8]) -> Vec<u32> {
        let body = &self.image.body;
        (0..body.len().saturating_sub(needle.len()))
            .filter(|&i| body[i..i + needle.len()] == *needle)
            .map(|i| crate::target::RTOS_BASE + i as u32)
            .collect()
    }
}

/// A `movw rd, lo` and a `movt rd, hi` at most two words after it: the value
/// and the address after the movt.
pub(super) fn movw_movt(image: &Image, address: u32, rd: u8) -> Result<Option<(u32, u32)>> {
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
pub(super) struct Link {
    pub(super) text: String,
    pub(super) target: u32,
    pub(super) zero: Vec<u8>,
}

/// How a strcmp chain ends: a last `bne` to its error path, or another
/// instruction.
pub(super) enum ChainEnd {
    Mismatch,
    Other,
}

/// Parse a strcmp chain on `[base, #offset]` from `start`.
pub(super) fn strcmp_chain(
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

/// Follow the setter on its fall-through path to the store of r1. A
/// conditional branch is not taken: the fall-through path is the path for a
/// changed value. One call with the object and the value is followed (the
/// inner setter of GR IV).
fn setter_field(image: &Image, setter: u32, depth: u32, float: bool) -> Result<Field> {
    let mut registers = [Value::Unknown; 16];
    registers[0] = Value::Object;
    // The value is in r1, or in s0 for a float setting.
    let mut singles = [false; 32];
    if float {
        singles[0] = true;
    } else {
        registers[1] = Value::Setting;
    }
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
                    depth == 0
                        && registers[0] == Value::Object
                        && if float {
                            singles[0]
                        } else {
                            registers[1] == Value::Setting
                        },
                    "Call before the store at {at:#x}"
                );
                return setter_field(image, target, depth + 1, float);
            }
            Op::VPush | Op::VCompare | Op::VFlags => {}
            Op::VMov(sd, sm) => singles[sd as usize] = singles[sm as usize],
            Op::VLoad(sd, ..) => singles[sd as usize] = false,
            Op::VStore(sd, rn, displacement) => {
                ensure!(
                    singles[sd as usize],
                    "Store at {at:#x} is not the value to the settings"
                );
                ensure!(displacement >= 0, "Store at {at:#x} below the field base");
                return match registers[rn as usize] {
                    Value::Pointer(record, offset) => Ok(Field {
                        record,
                        offset: offset + displacement as u32,
                        width: 4,
                    }),
                    Value::Object => Ok(Field {
                        record: None,
                        offset: displacement as u32,
                        width: 4,
                    }),
                    _ => bail!("Store of the value at {at:#x} to an unknown base"),
                };
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
        field: setter_field(image, setter, 0, false)?,
    })
}

/// A number handler: strtol(argv[3], &end, 0), a check of *end, the getters,
/// a move or an extension of the number into r1, and the setter. Or a float
/// handler: strtof(argv[3], &end) with the number kept in s16, the check, the
/// getters, the number moved back to s0, and the setter.
fn number_handler(image: &Image, handler: u32) -> Result<Setting> {
    let (mut calls, mut r2_zero, mut argument) = (Vec::new(), false, false);
    let (mut parsed, mut converted, mut checked) = (None, false, false);
    let (mut float, mut kept) = (false, None);
    let mut at = handler;
    for _ in 0..40 {
        let insn = image.insn(at)?;
        match insn.op {
            Op::MovImm(R2, 0) => r2_zero = true,
            Op::Load(4, false, R0, R1, 12) => argument = true,
            Op::Call(target) => {
                ensure!(
                    !calls.is_empty() || argument,
                    "Number handler {handler:#x}: the first call does not parse argv[3]"
                );
                calls.push(target);
                if calls.len() >= 2 && converted {
                    break;
                }
            }
            Op::VPush if calls.is_empty() => {}
            // strtof returns in s0: keep it in a register that the getters save.
            Op::VMov(sd, 0) if calls.len() == 1 && kept.is_none() => {
                (float, kept) = (true, Some(sd));
            }
            Op::VMov(0, sm) if calls.len() >= 2 && Some(sm) == kept => converted = true,
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
    ensure!(
        float || r2_zero,
        "Number handler {handler:#x}: the first call is not strtol(argv[3], &end, 0)"
    );
    let setter = *calls.last().unwrap();
    Ok(Setting {
        kind: if float { Kind::Float } else { Kind::Number },
        setter,
        getters: calls[1..calls.len() - 1].to_vec(),
        field: setter_field(image, setter, 0, float)?,
    })
}

/// The setting of a dispatcher stub.
pub(super) fn setting(image: &Image, stub: u32, strcmp: u32) -> Result<Setting> {
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
