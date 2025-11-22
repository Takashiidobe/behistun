use super::{
    AddrReg, AddressingMode, BitOp, BitOpImm, BitOpReg, DataReg, EffectiveAddress, Immediate,
    ImmOp, Instruction, InstructionKind, QuickOp, RightOrLeft, Shift, ShiftCount, ShiftEa,
    ShiftReg, Size, Sub, Subx, UnaryOp,
};
use crate::decoder::Add;
use std::fmt;

impl fmt::Display for ShiftCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShiftCount::Immediate(value) => write!(f, "#{}", value),
            ShiftCount::Register(reg) => write!(f, "{reg}"),
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

impl fmt::Display for InstructionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstructionKind::Reset => write!(f, "reset"),
            InstructionKind::Nop => write!(f, "nop"),
            InstructionKind::Illegal => write!(f, "illegal"),
            InstructionKind::Rte => write!(f, "rte"),
            InstructionKind::Rts => write!(f, "rts"),
            InstructionKind::Rtr => write!(f, "rtr"),
            InstructionKind::TrapV => write!(f, "trapv"),
            InstructionKind::Negx(unary) => write!(f, "negx {}", unary),
            InstructionKind::Clr(unary) => write!(f, "clr {}", unary),
            InstructionKind::Neg(unary) => write!(f, "neg {}", unary),
            InstructionKind::Not(unary) => write!(f, "not {}", unary),
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
