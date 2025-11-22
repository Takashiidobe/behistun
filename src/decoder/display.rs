use super::{
    Abcd, AddrReg, AddressingMode, And, BitFieldParam, BitOp, BitOpImm, BitOpReg, Condition,
    DataDir, DataReg, EffectiveAddress, Exg, ExtMode, ImmOp, Immediate, Instruction,
    InstructionKind, Movem, Movep, MovepDirection, Or, QuickOp, Register, RightOrLeft, Sbcd,
    Shift, ShiftCount, ShiftEa, ShiftReg, Size, Sub, Subx, UnaryOp, UspDirection,
};
use crate::decoder::Add;
use std::fmt;

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Condition::True => "t",
            Condition::False => "f",
            Condition::Higher => "hi",
            Condition::LowerOrSame => "ls",
            Condition::CarryClear => "cc",
            Condition::CarrySet => "cs",
            Condition::NotEqual => "ne",
            Condition::Equal => "eq",
            Condition::OverflowClear => "vc",
            Condition::OverflowSet => "vs",
            Condition::Plus => "pl",
            Condition::Minus => "mi",
            Condition::GreaterOrEqual => "ge",
            Condition::LessThan => "lt",
            Condition::GreaterThan => "gt",
            Condition::LessOrEqual => "le",
        };
        f.write_str(s)
    }
}

impl fmt::Display for ShiftCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShiftCount::Immediate(value) => write!(f, "#{}", value),
            ShiftCount::Register(reg) => write!(f, "{reg}"),
        }
    }
}

impl fmt::Display for BitFieldParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BitFieldParam::Immediate(value) => write!(f, "{}", value),
            BitFieldParam::Register(reg) => write!(f, "{reg}"),
        }
    }
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Register::Data(reg) => write!(f, "{reg}"),
            Register::Address(reg) => write!(f, "{reg}"),
        }
    }
}

impl fmt::Display for Shift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Shift::Ea(ShiftEa { direction, mode }) => {
                let dir_char = match direction {
                    RightOrLeft::Left => 'l',
                    RightOrLeft::Right => 'r',
                };
                write!(f, "{}.w {}", dir_char, mode)
            }
            Shift::Reg(ShiftReg {
                direction,
                size,
                count,
                dst,
            }) => {
                let dir_char = match direction {
                    RightOrLeft::Left => 'l',
                    RightOrLeft::Right => 'r',
                };
                write!(f, "{}{size} {count}, {dst}", dir_char)
            }
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.size, self.mode)
    }
}

impl fmt::Display for BitOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BitOp::Imm(BitOpImm { bit_num, mode }) => {
                write!(f, "#{}, {}", bit_num, mode)
            }
            BitOp::Reg(BitOpReg { bit_reg, mode }) => {
                write!(f, "{}, {}", bit_reg, mode)
            }
        }
    }
}

impl fmt::Display for QuickOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} #{}, {}", self.size, self.data, self.mode)
    }
}

impl fmt::Display for ImmOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}, {}", self.size, self.imm, self.mode)
    }
}

impl fmt::Display for Movep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let disp_str = format_signed_hex(self.displacement as i32);
        match self.direction {
            MovepDirection::MemToReg => {
                write!(
                    f,
                    "movep{} {}({}), {}",
                    self.size, disp_str, self.addr_reg, self.data_reg
                )
            }
            MovepDirection::RegToMem => {
                write!(
                    f,
                    "movep{} {}, {}({})",
                    self.size, self.data_reg, disp_str, self.addr_reg
                )
            }
        }
    }
}

impl fmt::Display for Movem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reg_list = format_register_mask(self.register_mask, &self.mode, self.direction);
        match self.direction {
            DataDir::RegToMem => write!(f, "movem{} {}, {}", self.size, reg_list, self.mode),
            DataDir::MemToReg => write!(f, "movem{} {}, {}", self.size, self.mode, reg_list),
        }
    }
}

// Format register mask for MOVEM instruction
// For predecrement mode with RegToMem, the mask is reversed
fn format_register_mask(mask: u16, mode: &AddressingMode, direction: DataDir) -> String {
    use super::EffectiveAddress;

    // For predecrement addressing mode with register-to-memory, the mask is reversed
    let mask = match (&mode.ea, direction) {
        (EffectiveAddress::AddrPreDecr(_), DataDir::RegToMem) => mask.reverse_bits(),
        _ => mask,
    };

    let mut parts = Vec::new();

    // Data registers (bits 0-7 -> D0-D7)
    let d_regs = format_reg_range(mask & 0xFF, "d");
    if !d_regs.is_empty() {
        parts.push(d_regs);
    }

    // Address registers (bits 8-15 -> A0-A7)
    let a_regs = format_reg_range((mask >> 8) & 0xFF, "a");
    if !a_regs.is_empty() {
        parts.push(a_regs);
    }

    if parts.is_empty() {
        "#0".to_string()
    } else {
        parts.join("/")
    }
}

// Format a range of registers (e.g., "d0-d3" or "d0/d2/d4")
fn format_reg_range(mask: u16, prefix: &str) -> String {
    let mut parts = Vec::new();
    let mut i = 0;

    while i < 8 {
        if mask & (1 << i) != 0 {
            let start = i;
            while i < 8 && mask & (1 << i) != 0 {
                i += 1;
            }
            let end = i - 1;

            if start == end {
                parts.push(format!("%{}{}", prefix, start));
            } else if end == start + 1 {
                parts.push(format!("%{}{}", prefix, start));
                parts.push(format!("%{}{}", prefix, end));
            } else {
                parts.push(format!("%{}{}-%{}{}", prefix, start, prefix, end));
            }
        } else {
            i += 1;
        }
    }

    parts.join("/")
}

impl fmt::Display for InstructionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstructionKind::Reset => write!(f, "reset"),
            InstructionKind::Nop => write!(f, "nop"),
            InstructionKind::Illegal => write!(f, "illegal"),
            InstructionKind::Rte => write!(f, "rte"),
            InstructionKind::Rts => write!(f, "rts"),
            InstructionKind::Rtd { displacement } => write!(f, "rtd #{displacement}"),
            InstructionKind::Rtr => write!(f, "rtr"),
            InstructionKind::TrapV => write!(f, "trapv"),
            InstructionKind::Negx(unary) => write!(f, "negx{}", unary),
            InstructionKind::Clr(unary) => write!(f, "clr{}", unary),
            InstructionKind::Neg(unary) => write!(f, "neg{}", unary),
            InstructionKind::Not(unary) => write!(f, "not{}", unary),
            InstructionKind::Tas { mode } => write!(f, "tas.b {mode}"),
            InstructionKind::Tst { size, mode } => write!(f, "tst{size} {mode}"),
            InstructionKind::Jsr { mode } => write!(f, "jsr {mode}"),
            InstructionKind::Jmp { mode } => write!(f, "jmp {mode}"),
            InstructionKind::Adda {
                addr_reg,
                size,
                mode,
            } => write!(f, "adda{size} {mode}, {addr_reg}"),
            InstructionKind::Add(add) => match add {
                Add::EaToDn(super::EaToDn { size, dst, src }) => {
                    write!(f, "add{size} {src}, {dst}")
                }
                Add::DnToEa(super::DnToEa { size, src, dst }) => {
                    write!(f, "add{size} {src}, {dst}")
                }
            },
            InstructionKind::Addx(addx) => match addx {
                super::Addx::Dn(super::Dn { size, src, dst }) => {
                    write!(f, "addx{size} {src}, {dst}")
                }
                super::Addx::PreDec(super::PreDec { size, src, dst }) => {
                    write!(f, "addx{size} -({src}), -({dst})")
                }
            },
            InstructionKind::Asd(shift) => write!(f, "as{}", shift),
            InstructionKind::Lsd(shift) => write!(f, "ls{}", shift),
            InstructionKind::Roxd(shift) => write!(f, "rox{}", shift),
            InstructionKind::Rod(shift) => write!(f, "ro{}", shift),
            InstructionKind::Trap { vector } => write!(f, "trap #{vector}"),
            InstructionKind::Trapcc { condition, operand } => {
                let cond_str = condition.to_string().to_lowercase();
                match operand {
                    None => write!(f, "trap{cond_str}"),
                    Some(Immediate::Word(w)) => write!(f, "trap{cond_str}.w #{w}"),
                    Some(Immediate::Long(l)) => write!(f, "trap{cond_str}.l #{l}"),
                    Some(Immediate::Byte(_)) => unreachable!("TRAPcc cannot have byte operand"),
                }
            }
            InstructionKind::Bkpt { vector } => write!(f, "bkpt #{vector}"),
            InstructionKind::Link {
                addr_reg,
                displacement,
            } => write!(f, "link {}, #{}", addr_reg, displacement),
            InstructionKind::Unlk { addr_reg } => write!(f, "unlk {}", addr_reg),
            InstructionKind::Btst(bit_op) => write!(f, "btst {}", bit_op),
            InstructionKind::Bchg(bit_op) => write!(f, "bchg {}", bit_op),
            InstructionKind::Bclr(bit_op) => write!(f, "bclr {}", bit_op),
            InstructionKind::Bset(bit_op) => write!(f, "bset {}", bit_op),
            InstructionKind::Addq(quick_op) => write!(f, "addq{}", quick_op),
            InstructionKind::Subq(quick_op) => write!(f, "subq{}", quick_op),
            InstructionKind::Moveq { data, dst } => {
                write!(f, "moveq #{}, {}", data, dst)
            }
            InstructionKind::Scc { condition, mode } => {
                write!(f, "s{} {}", condition, mode)
            }
            InstructionKind::DBcc {
                condition,
                data_reg,
                displacement,
            } => {
                write!(
                    f,
                    "db{} {}, {}",
                    condition,
                    data_reg,
                    format_signed_hex(*displacement as i32)
                )
            }
            InstructionKind::Bra { displacement } => {
                write!(f, "bra {}", format_signed_hex(*displacement))
            }
            InstructionKind::Bsr { displacement } => {
                write!(f, "bsr {}", format_signed_hex(*displacement))
            }
            InstructionKind::Bcc {
                condition,
                displacement,
            } => {
                write!(f, "b{} {}", condition, format_signed_hex(*displacement))
            }
            InstructionKind::Divu { src, dst } => {
                write!(f, "divu.w {}, {}", src, dst)
            }
            InstructionKind::Divs { src, dst } => {
                write!(f, "divs.w {}, {}", src, dst)
            }
            InstructionKind::Sbcd(sbcd) => match sbcd {
                Sbcd::Dn { src, dst } => write!(f, "sbcd {}, {}", src, dst),
                Sbcd::PreDec { src, dst } => write!(f, "sbcd -({src}), -({dst})"),
            },
            InstructionKind::Or(or) => match or {
                Or::EaToDn(super::EaToDn { size, dst, src }) => {
                    write!(f, "or{size} {src}, {dst}")
                }
                Or::DnToEa(super::DnToEa { size, src, dst }) => {
                    write!(f, "or{size} {src}, {dst}")
                }
            },
            InstructionKind::Cmp(super::EaToDn { size, dst, src }) => {
                write!(f, "cmp{size} {src}, {dst}")
            }
            InstructionKind::Cmpa {
                addr_reg,
                size,
                src,
            } => write!(f, "cmpa{size} {src}, {addr_reg}"),
            InstructionKind::Cmpm { size, src, dst } => {
                write!(f, "cmpm{size} ({src})+, ({dst})+")
            }
            InstructionKind::Eor(super::DnToEa { size, src, dst }) => {
                write!(f, "eor{size} {src}, {dst}")
            }
            InstructionKind::Mulu { src, dst } => {
                write!(f, "mulu.w {}, {}", src, dst)
            }
            InstructionKind::Muls { src, dst } => {
                write!(f, "muls.w {}, {}", src, dst)
            }
            InstructionKind::Abcd(abcd) => match abcd {
                Abcd::Dn { src, dst } => write!(f, "abcd {}, {}", src, dst),
                Abcd::PreDec { src, dst } => write!(f, "abcd -({src}), -({dst})"),
            },
            InstructionKind::Exg(exg) => match exg {
                Exg::DataData { rx, ry } => write!(f, "exg {rx}, {ry}"),
                Exg::AddrAddr { rx, ry } => write!(f, "exg {rx}, {ry}"),
                Exg::DataAddr { data, addr } => write!(f, "exg {data}, {addr}"),
            },
            InstructionKind::And(and) => match and {
                And::EaToDn(super::EaToDn { size, dst, src }) => {
                    write!(f, "and{size} {src}, {dst}")
                }
                And::DnToEa(super::DnToEa { size, src, dst }) => {
                    write!(f, "and{size} {src}, {dst}")
                }
            },
            InstructionKind::Suba {
                addr_reg,
                size,
                mode,
            } => write!(f, "suba{size} {mode}, {addr_reg}"),
            InstructionKind::Sub(sub) => match sub {
                Sub::EaToDn(super::EaToDn { size, dst, src }) => {
                    write!(f, "sub{size} {src}, {dst}")
                }
                Sub::DnToEa(super::DnToEa { size, src, dst }) => {
                    write!(f, "sub{size} {src}, {dst}")
                }
            },
            InstructionKind::Subx(subx) => match subx {
                Subx::Dn(super::Dn { size, src, dst }) => {
                    write!(f, "subx{size} {src}, {dst}")
                }
                Subx::PreDec(super::PreDec { size, src, dst }) => {
                    write!(f, "subx{size} -({src}), -({dst})")
                }
            },
            InstructionKind::Andi(imm_op) => write!(f, "andi{}", imm_op),
            InstructionKind::Subi(imm_op) => write!(f, "subi{}", imm_op),
            InstructionKind::Addi(imm_op) => write!(f, "addi{}", imm_op),
            InstructionKind::Eori(imm_op) => write!(f, "eori{}", imm_op),
            InstructionKind::Cmpi(imm_op) => write!(f, "cmpi{}", imm_op),
            InstructionKind::EoriToCcr { imm } => write!(f, "eori #0x{:02x}, %ccr", imm),
            InstructionKind::EoriToSr { imm } => write!(f, "eori #0x{:04x}, %sr", imm),
            InstructionKind::Ori(imm_op) => write!(f, "ori{}", imm_op),
            InstructionKind::OriToCcr { imm } => write!(f, "ori #0x{:02x}, %ccr", imm),
            InstructionKind::OriToSr { imm } => write!(f, "ori #0x{:04x}, %sr", imm),
            InstructionKind::Move { size, src, dst } => write!(f, "move{size} {src}, {dst}"),
            InstructionKind::Movea { size, src, dst } => write!(f, "movea{size} {src}, {dst}"),
            InstructionKind::Movep(movep) => write!(f, "{}", movep),
            InstructionKind::MoveFromSr { dst } => write!(f, "move.w %sr, {dst}"),
            InstructionKind::MoveToCcr { src } => write!(f, "move.w {src}, %ccr"),
            InstructionKind::MoveToSr { src } => write!(f, "move.w {src}, %sr"),
            InstructionKind::MoveUsp {
                addr_reg,
                direction,
            } => match direction {
                UspDirection::RegToUsp => write!(f, "move.l {addr_reg}, %usp"),
                UspDirection::UspToReg => write!(f, "move.l %usp, {addr_reg}"),
            },
            InstructionKind::Ext { data_reg, mode } => {
                let suffix = match mode {
                    ExtMode::ByteToWord => ".w",
                    ExtMode::WordToLong => ".l",
                    ExtMode::ByteToLong => "b.l",
                };
                write!(f, "ext{suffix} {data_reg}")
            }
            InstructionKind::Nbcd { mode } => write!(f, "nbcd {mode}"),
            InstructionKind::Swap { data_reg } => write!(f, "swap {data_reg}"),
            InstructionKind::Pea { mode } => write!(f, "pea {mode}"),
            InstructionKind::Lea { src, dst } => write!(f, "lea {src}, {dst}"),
            InstructionKind::Chk {
                size,
                src,
                data_reg,
            } => {
                write!(f, "chk{size} {src}, {data_reg}")
            }
            InstructionKind::Movem(movem) => write!(f, "{movem}"),
            InstructionKind::Cas { size, dc, du, mode } => {
                write!(f, "cas{size} {dc}, {du}, {mode}")
            }
            InstructionKind::Cas2 {
                size,
                dc1,
                dc2,
                du1,
                du2,
                rn1,
                rn2,
            } => {
                write!(f, "cas2{size} {dc1}:{dc2}, {du1}:{du2}, ({rn1}):({rn2})")
            }
            InstructionKind::Cmp2 { size, mode, reg } => {
                write!(f, "cmp2{size} {mode}, {reg}")
            }
            InstructionKind::Chk2 { size, mode, reg } => {
                write!(f, "chk2{size} {mode}, {reg}")
            }
            InstructionKind::Bftst {
                mode,
                offset,
                width,
            } => {
                write!(f, "bftst {mode}{{{offset}:{width}}}")
            }
            InstructionKind::Bfchg {
                mode,
                offset,
                width,
            } => {
                write!(f, "bfchg {mode}{{{offset}:{width}}}")
            }
            InstructionKind::Bfclr {
                mode,
                offset,
                width,
            } => {
                write!(f, "bfclr {mode}{{{offset}:{width}}}")
            }
            InstructionKind::Bfset {
                mode,
                offset,
                width,
            } => {
                write!(f, "bfset {mode}{{{offset}:{width}}}")
            }
            InstructionKind::Bfextu {
                src,
                dst,
                offset,
                width,
            } => {
                write!(f, "bfextu {src}{{{offset}:{width}}}, {dst}")
            }
            InstructionKind::Bfexts {
                src,
                dst,
                offset,
                width,
            } => {
                write!(f, "bfexts {src}{{{offset}:{width}}}, {dst}")
            }
            InstructionKind::Bfins {
                src,
                dst,
                offset,
                width,
            } => {
                write!(f, "bfins {src}, {dst}{{{offset}:{width}}}")
            }
            InstructionKind::Bfffo {
                src,
                dst,
                offset,
                width,
            } => {
                write!(f, "bfffo {src}{{{offset}:{width}}}, {dst}")
            }
            InstructionKind::MuluL { src, dl, dh } => match dh {
                Some(dh) => write!(f, "mulu.l {src}, {dh}:{dl}"),
                None => write!(f, "mulu.l {src}, {dl}"),
            },
            InstructionKind::MulsL { src, dl, dh } => match dh {
                Some(dh) => write!(f, "muls.l {src}, {dh}:{dl}"),
                None => write!(f, "muls.l {src}, {dl}"),
            },
            InstructionKind::DivuL {
                src,
                dq,
                dr,
                is_64bit,
            } => {
                if *is_64bit {
                    write!(f, "divu.l {src}, {dr}:{dq}")
                } else {
                    write!(f, "divul.l {src}, {dr}:{dq}")
                }
            }
            InstructionKind::DivsL {
                src,
                dq,
                dr,
                is_64bit,
            } => {
                if *is_64bit {
                    write!(f, "divs.l {src}, {dr}:{dq}")
                } else {
                    write!(f, "divsl.l {src}, {dr}:{dq}")
                }
            }
        }
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#010x}: {}", self.address, self.kind)
    }
}

impl fmt::Display for Immediate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Immediate::Byte(v) => write!(f, "#0x{:02x}", v),
            Immediate::Word(v) => write!(f, "#0x{:04x}", v),
            Immediate::Long(v) => write!(f, "#0x{:08x}", v),
        }
    }
}

impl fmt::Display for DataReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%d{}", self.number())
    }
}

impl fmt::Display for AddrReg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%a{}", self.number())
    }
}

impl fmt::Display for AddressingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ea {
            EffectiveAddress::Dr(reg) => write!(f, "{reg}"),
            EffectiveAddress::Ar(reg) => write!(f, "{reg}"),
            EffectiveAddress::Addr(reg) => write!(f, "({reg})"),
            EffectiveAddress::AddrPostIncr(reg) => write!(f, "({reg})+"),
            EffectiveAddress::AddrPreDecr(reg) => write!(f, "-({reg})"),
            EffectiveAddress::AddrDisplace(reg) => {
                let disp = self.short_data().map(|v| v as i16 as i32).unwrap_or(0);
                write!(f, "{}({reg})", format_signed_hex(disp))
            }
            EffectiveAddress::AddrIndex(reg) => {
                let ext = self.short_data().unwrap_or(0);
                let base = format!("{reg}");
                write!(f, "{}", format_index_operand(&base, ext))
            }
            EffectiveAddress::PCDisplace => {
                let disp = self.short_data().map(|v| v as i16 as i32).unwrap_or(0);
                write!(f, "{}(%pc)", format_signed_hex(disp))
            }
            EffectiveAddress::PCIndex => {
                let ext = self.short_data().unwrap_or(0);
                write!(f, "{}", format_index_operand("%pc", ext))
            }
            EffectiveAddress::AbsShort => {
                let value = self.short_data().unwrap_or(0);
                write!(f, "0x{value:04x}.w")
            }
            EffectiveAddress::AbsLong => {
                let value = self.long_data().unwrap_or(0);
                write!(f, "0x{value:08x}.l")
            }
            EffectiveAddress::Immediate => {
                let immediate = self.immediate().unwrap_or(Immediate::Word(0));
                write!(f, "{immediate}")
            }
        }
    }
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffix = match self {
            Size::Byte => ".b",
            Size::Word => ".w",
            Size::Long => ".l",
        };
        f.write_str(suffix)
    }
}

// Formats a signed 16-bit displacement value into a hexadecimal string with a '0x' prefix.
// Negative values are prefixed with a '-'.
pub fn format_signed_hex(value: i32) -> String {
    if value < 0 {
        format!("-0x{:x}", -value)
    } else {
        format!("0x{:x}", value)
    }
}

// Formats an index operand for M68k assembly.
// ext_word: The extension word containing index register, size, scale, and displacement.
pub fn format_index_operand(base_reg: &str, ext_word: u16) -> String {
    let displacement = super::bit_range(ext_word, 0, 8) as i8 as i32;
    let index_reg_num = super::bit_range(ext_word, 12, 15);
    let is_addr_reg = super::bit_range(ext_word, 11, 12) == 1;
    let index_size_bit = super::bit_range(ext_word, 10, 11);
    let scale_bits = super::bit_range(ext_word, 9, 10);

    let index_reg_str = if is_addr_reg {
        format!("%a{}", index_reg_num)
    } else {
        format!("%d{}", index_reg_num)
    };

    let size_suffix = match index_size_bit {
        0 => ".w",
        1 => ".l",
        _ => ".?", // Should not happen based on bit_range (1 bit)
    };

    let scale = 1 << scale_bits;

    let disp_str = format_signed_hex(displacement);

    if displacement == 0 {
        format!("({base_reg},{index_reg_str}{size_suffix}*{scale})")
    } else {
        format!("{disp_str}({base_reg},{index_reg_str}{size_suffix}*{scale})")
    }
}
