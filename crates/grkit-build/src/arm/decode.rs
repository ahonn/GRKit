//! A decoder of the A32 instructions that the firmware readers match
//! (`userdata`), and the immediate and branch fields that the relocation
//! and the tests read.

// Registers and conditions.
pub(crate) const R0: u8 = 0;
pub(crate) const R1: u8 = 1;
pub(crate) const R2: u8 = 2;
pub(crate) const R4: u8 = 4;
pub(crate) const R5: u8 = 5;
pub(crate) const FP: u8 = 11;
pub(crate) const IP: u8 = 12;
pub(crate) const SP: u8 = 13;
pub(crate) const LR: u8 = 14;
pub(crate) const PC: u8 = 15;
pub(crate) const ALWAYS: u8 = 0xe;
pub(crate) const EQ: u8 = 0x0;
pub(crate) const NE: u8 = 0x1;

/// The A32 instructions that the patterns use. `Other` is any other one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
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
    /// VPUSH of D registers.
    VPush,
    /// `vmov.f32 sd, sm`.
    VMov(u8, u8),
    /// VSTR and VLDR of a single register with an immediate offset: Sd, Rn,
    /// offset.
    VStore(u8, u8, i32),
    VLoad(u8, u8, i32),
    /// VCMP or VCMPE of two single registers, and `vmrs APSR_nzcv, fpscr`.
    VCompare,
    VFlags,
    Other,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Insn {
    pub(crate) condition: u8,
    pub(crate) op: Op,
}

/// The value of a data-processing immediate: eight bits rotated right by
/// twice the four bits above them.
pub fn modified_immediate(word: u32) -> u32 {
    (word & 0xff).rotate_right(((word >> 8) & 0xf) * 2)
}

/// The 16-bit immediate of a MOVW or MOVT.
pub fn move_immediate(word: u32) -> u32 {
    ((word >> 4) & 0xf000) | (word & 0xfff)
}

/// The address that the 24-bit offset of a branch at `address` points to,
/// for each opcode that has one (B, BL, and BLX with an immediate).
pub fn branch_destination(address: u32, word: u32) -> u32 {
    let offset = ((word << 8) as i32 >> 6) as u32;
    address.wrapping_add(8).wrapping_add(offset)
}

/// The target of a B or BL at `address`, if `word` is one.
pub fn branch_target(address: u32, word: u32) -> Option<u32> {
    ((word >> 25) & 7 == 5 && word >> 28 != 0xf).then(|| branch_destination(address, word))
}

/// A single-precision register: four bits at `at` and the low bit at `low`.
fn single(word: u32, at: u32, low: u32) -> u8 {
    (((word >> at & 0xf) << 1) | (word >> low & 1)) as u8
}

/// Decode the A32 instruction `word` at `address`.
pub(crate) fn decode(address: u32, word: u32) -> Insn {
    let condition = (word >> 28) as u8;
    let rd = ((word >> 12) & 0xf) as u8;
    let rn = ((word >> 16) & 0xf) as u8;
    let rm = (word & 0xf) as u8;
    let op = if condition == 0xf {
        if word & 0x0e00_0000 == 0x0a00_0000 {
            Op::Exchange(branch_destination(address, word) | ((word >> 23) & 2))
        } else {
            Op::Other
        }
    } else if word & 0x0fff_ffff == 0x012f_ff1e {
        Op::Return
    } else if word & 0x0ff0_0000 == 0x0380_0000 {
        Op::OrrImm(rd, rn, modified_immediate(word))
    } else if word & 0x0ff0_0000 == 0x0300_0000 {
        Op::Movw(rd, move_immediate(word))
    } else if word & 0x0ff0_0000 == 0x0340_0000 {
        Op::Movt(rd, move_immediate(word))
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
            Op::Call(branch_destination(address, word))
        } else {
            Op::Branch(condition, branch_destination(address, word))
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
    } else if word & 0x0fbf_0f00 == 0x0d2d_0b00 {
        Op::VPush
    } else if word & 0x0fbf_0fd0 == 0x0eb0_0a40 {
        Op::VMov(single(word, 12, 22), single(word, 0, 5))
    } else if word & 0x0f20_0f00 == 0x0d00_0a00 {
        // VSTR or VLDR of a single register.
        let offset = 4 * (word & 0xff) as i32;
        let offset = if word & 0x0080_0000 != 0 {
            offset
        } else {
            -offset
        };
        let sd = single(word, 12, 22);
        if word & 0x0010_0000 != 0 {
            Op::VLoad(sd, rn, offset)
        } else {
            Op::VStore(sd, rn, offset)
        }
    } else if word & 0x0fbf_0f50 == 0x0eb4_0a40 {
        Op::VCompare
    } else if word & 0x0fff_ffff == 0x0ef1_fa10 {
        Op::VFlags
    } else {
        Op::Other
    };
    Insn { condition, op }
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
        // The VFP instructions of a float setting (GR III 2.10 CommonXv).
        let vfp = [
            (0xed2d_8b02, Op::VPush),
            (0xeeb0_8a40, Op::VMov(16, 0)),
            (0xeeb0_0a48, Op::VMov(0, 16)),
            (0xedd3_8a08, Op::VLoad(17, 3, 0x20)),
            (0xed83_0a08, Op::VStore(0, 3, 0x20)),
            (0xed83_0a53, Op::VStore(0, 3, 0x14c)),
            (0xeef4_8a48, Op::VCompare),
            (0xeef1_fa10, Op::VFlags),
        ];
        for (word, op) in vfp {
            assert_eq!(decode(0x5324_2218, word).op, op, "{word:#x}");
        }
    }
}
