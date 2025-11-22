use crate::memory::MemoryImage;
use anyhow::Result;
use anyhow::bail;

mod display;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum InstructionKind {
    Reset,
    Nop,
    Illegal,
    Rte,
    Rts,
    Rtr,
    Negx(UnaryOp),
    Clr(UnaryOp),
    Neg(UnaryOp),
    Not(UnaryOp),
    Asd(Shift),
    Lsd(Shift),
    Roxd(Shift),
    Rod(Shift),
    Tas {
        mode: AddressingMode,
    },
    Tst {
        size: Size,
        mode: AddressingMode,
    },
    Jsr {
        mode: AddressingMode,
    },
    Jmp {
        mode: AddressingMode,
    },
    Adda {
        addr_reg: AddrReg,
        size: Size,
        mode: AddressingMode,
    },
    Add(Add),
    Addx(Addx),
    TrapV,
    Trap {
        vector: u8,
    },
    Link {
        addr_reg: AddrReg,
        displacement: i16,
    },
    Unlk {
        addr_reg: AddrReg,
    },
    Btst(BitOp),
    Bchg(BitOp),
    Bclr(BitOp),
    Bset(BitOp),
    Addq(QuickOp),
    Subq(QuickOp),
    Suba {
        addr_reg: AddrReg,
        size: Size,
        mode: AddressingMode,
    },
    Sub(Sub),
    Subx(Subx),
    Andi(ImmOp),
    Subi(ImmOp),
    Addi(ImmOp),
    Eori(ImmOp),
    Cmpi(ImmOp),
    EoriToCcr {
        imm: u8,
    },
    EoriToSr {
        imm: u16,
    },
    Ori(ImmOp),
    OriToCcr {
        imm: u8,
    },
    OriToSr {
        imm: u16,
    },
    Move {
        size: Size,
        src: AddressingMode,
        dst: AddressingMode,
    },
    Movea {
        size: Size,
        src: AddressingMode,
        dst: AddrReg,
    },
    Movep(Movep),
    MoveFromSr {
        dst: AddressingMode,
    },
    MoveToCcr {
        src: AddressingMode,
    },
    MoveToSr {
        src: AddressingMode,
    },
    MoveUsp {
        addr_reg: AddrReg,
        direction: UspDirection,
    },
    Ext {
        data_reg: DataReg,
        mode: ExtMode,
    },
    Nbcd {
        mode: AddressingMode,
    },
    Swap {
        data_reg: DataReg,
    },
    Pea {
        mode: AddressingMode,
    },
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum UspDirection {
    RegToUsp, // An -> USP
    UspToReg, // USP -> An
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ExtMode {
    ByteToWord, // EXT.W - sign extend byte to word
    WordToLong, // EXT.L - sign extend word to long
    ByteToLong, // EXTB.L - sign extend byte to long (68020+)
}

// <ea>,Dn
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct EaToDn {
    pub size: Size,
    pub dst: DataReg,        // Dn
    pub src: AddressingMode, // <ea>
}
// Dn,<ea>
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct DnToEa {
    pub size: Size,
    pub src: DataReg,        // Dn
    pub dst: AddressingMode, // <ea>
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Add {
    EaToDn(EaToDn),
    DnToEa(DnToEa),
}
// Dy,Dx
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Dn {
    pub size: Size,
    pub src: DataReg, // Dy
    pub dst: DataReg, // Dx
}
// -(Ay),-(Ax)
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct PreDec {
    pub size: Size,
    pub src: AddrReg, // Ay
    pub dst: AddrReg, // Ax
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Addx {
    Dn(Dn),
    PreDec(PreDec),
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Sub {
    EaToDn(EaToDn),
    DnToEa(DnToEa),
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Subx {
    Dn(Dn),
    PreDec(PreDec),
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Movep {
    pub size: Size,
    pub data_reg: DataReg,
    pub addr_reg: AddrReg,
    pub displacement: i16,
    pub direction: MovepDirection,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum MovepDirection {
    MemToReg,
    RegToMem,
}

// <ea>
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct ShiftEa {
    pub direction: RightOrLeft,
    pub mode: AddressingMode,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ShiftCount {
    Immediate(u8),
    Register(DataReg),
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct ShiftReg {
    pub direction: RightOrLeft,
    pub size: Size,
    pub count: ShiftCount,
    pub dst: DataReg,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Shift {
    Ea(ShiftEa),
    Reg(ShiftReg),
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct UnaryOp {
    pub size: Size,
    pub mode: AddressingMode,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum BitOp {
    Imm(BitOpImm),
    Reg(BitOpReg),
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct BitOpImm {
    pub bit_num: u8,
    pub mode: AddressingMode,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct BitOpReg {
    pub bit_reg: DataReg,
    pub mode: AddressingMode,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct QuickOp {
    pub data: u8,
    pub size: Size,
    pub mode: AddressingMode,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct ImmOp {
    pub imm: Immediate,
    pub size: Size,
    pub mode: AddressingMode,
}

#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub address: usize,
    pub opcode: u16,
    pub bytes: Vec<u8>,
    pub kind: InstructionKind,
}

impl Instruction {
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
}

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum AddressModeData {
    Short(u16),
    Long(u32),
    Imm(Immediate),
}

impl AddressModeData {
    pub fn to_bytes(self) -> Vec<u8> {
        match self {
            AddressModeData::Short(v) => v.to_be_bytes().to_vec(),
            AddressModeData::Long(v) => v.to_be_bytes().to_vec(),
            AddressModeData::Imm(immediate) => match immediate {
                Immediate::Byte(v) => v.to_be_bytes().to_vec(),
                Immediate::Word(v) => v.to_be_bytes().to_vec(),
                Immediate::Long(v) => v.to_be_bytes().to_vec(),
            },
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Immediate {
    Byte(u8),
    Word(u16),
    Long(u32),
}

pub struct Decoder {
    memory: MemoryImage,
}

impl Decoder {
    pub fn new(memory: MemoryImage) -> Self {
        Self { memory }
    }

    pub fn decode_instructions(&self, start: usize, end: usize) -> Result<Vec<Instruction>> {
        let mut instructions = vec![];
        let mut pc = start;
        while pc < end {
            let inst = self.decode_instruction(pc)?;
            let len = inst.len();
            instructions.push(inst);
            pc += len;
        }
        Ok(instructions)
    }

    fn resolve_ea(
        &self,
        mode: AddressingMode,
        offset: usize,
        immediate_size: Option<Size>,
    ) -> Result<AddressingMode> {
        match mode.ea {
            EffectiveAddress::Dr(_)
            | EffectiveAddress::Ar(_)
            | EffectiveAddress::Addr(_)
            | EffectiveAddress::AddrPostIncr(_)
            | EffectiveAddress::AddrPreDecr(_) => Ok(AddressingMode {
                ea: mode.ea,
                data: None,
            }),
            EffectiveAddress::AddrDisplace(_) | EffectiveAddress::PCDisplace => {
                let word = self.memory.read_word(offset)?;
                let value = Immediate::Word(word);
                Ok(AddressingMode {
                    ea: mode.ea,
                    data: Some(AddressModeData::Imm(value)),
                })
            }
            EffectiveAddress::AddrIndex(_) | EffectiveAddress::PCIndex => {
                let word = self.memory.read_word(offset)?;
                Ok(AddressingMode {
                    ea: mode.ea,
                    data: Some(AddressModeData::Short(word)),
                })
            }
            EffectiveAddress::Immediate => {
                let size = immediate_size.unwrap_or(Size::Word);
                let value = match size {
                    Size::Byte => {
                        let byte = self.memory.read_byte(offset)?;
                        Immediate::Byte(byte)
                    }
                    Size::Word => {
                        let word = self.memory.read_word(offset)?;
                        Immediate::Word(word)
                    }
                    Size::Long => {
                        let long = self.memory.read_long(offset)?;
                        Immediate::Long(long)
                    }
                };
                Ok(AddressingMode {
                    ea: mode.ea,
                    data: Some(AddressModeData::Imm(value)),
                })
            }
            EffectiveAddress::AbsShort => {
                let word = self.memory.read_word(offset)?;
                Ok(AddressingMode {
                    ea: mode.ea,
                    data: Some(AddressModeData::Short(word)),
                })
            }
            EffectiveAddress::AbsLong => {
                let long = self.memory.read_long(offset)?;
                Ok(AddressingMode {
                    ea: mode.ea,
                    data: Some(AddressModeData::Long(long)),
                })
            }
        }
    }

    fn resolve_shift(&self, shift: Shift, start: usize, bytes: &mut Vec<u8>) -> Result<Shift> {
        match shift {
            Shift::Ea(ShiftEa { direction, mode }) => {
                let mode = self.resolve_ea(mode, start + 2, Some(Size::Word))?;
                bytes.extend(mode.to_bytes());
                Ok(Shift::Ea(ShiftEa { direction, mode }))
            }
            Shift::Reg(_) => Ok(shift),
        }
    }

    fn resolve_unary(&self, unary: UnaryOp, start: usize, bytes: &mut Vec<u8>) -> Result<UnaryOp> {
        let mode = self.resolve_ea(unary.mode, start + 2, Some(unary.size))?;
        bytes.extend(mode.to_bytes());
        Ok(UnaryOp {
            size: unary.size,
            mode,
        })
    }

    fn resolve_bit_op(&self, bit_op: BitOp, start: usize, bytes: &mut Vec<u8>) -> Result<BitOp> {
        match bit_op {
            BitOp::Imm(BitOpImm { mode, .. }) => {
                let bit_word = self.memory.read_word(start + 2)?;
                bytes.extend(bit_word.to_be_bytes());
                let bit_num = (bit_word & 0xFF) as u8;
                let mode = self.resolve_ea(mode, start + 4, Some(Size::Byte))?;
                bytes.extend(mode.to_bytes());
                Ok(BitOp::Imm(BitOpImm { bit_num, mode }))
            }
            BitOp::Reg(BitOpReg { bit_reg, mode }) => {
                let mode = self.resolve_ea(mode, start + 2, Some(Size::Byte))?;
                bytes.extend(mode.to_bytes());
                Ok(BitOp::Reg(BitOpReg { bit_reg, mode }))
            }
        }
    }

    fn resolve_quick_op(
        &self,
        quick_op: QuickOp,
        start: usize,
        bytes: &mut Vec<u8>,
    ) -> Result<QuickOp> {
        let mode = self.resolve_ea(quick_op.mode, start + 2, Some(quick_op.size))?;
        bytes.extend(mode.to_bytes());
        Ok(QuickOp {
            data: quick_op.data,
            size: quick_op.size,
            mode,
        })
    }

    fn resolve_imm_op(&self, imm_op: ImmOp, start: usize, bytes: &mut Vec<u8>) -> Result<ImmOp> {
        // Read immediate value based on size
        let (imm, imm_len) = match imm_op.size {
            Size::Byte => {
                let word = self.memory.read_word(start + 2)?;
                bytes.extend(word.to_be_bytes());
                (Immediate::Byte(word as u8), 2)
            }
            Size::Word => {
                let word = self.memory.read_word(start + 2)?;
                bytes.extend(word.to_be_bytes());
                (Immediate::Word(word), 2)
            }
            Size::Long => {
                let long = self.memory.read_long(start + 2)?;
                bytes.extend(long.to_be_bytes());
                (Immediate::Long(long), 4)
            }
        };
        // Resolve EA after the immediate value
        let mode = self.resolve_ea(imm_op.mode, start + 2 + imm_len, Some(imm_op.size))?;
        bytes.extend(mode.to_bytes());
        Ok(ImmOp {
            imm,
            size: imm_op.size,
            mode,
        })
    }

    fn decode_instruction(&self, start: usize) -> Result<Instruction> {
        let opcode = self.memory.read_word(start)?;
        let instr_kind = Self::get_op_kind(opcode)?;
        let mut bytes = opcode.to_be_bytes().to_vec();
        let kind = match instr_kind {
            InstructionKind::Reset
            | InstructionKind::Nop
            | InstructionKind::Illegal
            | InstructionKind::Rte
            | InstructionKind::Rts
            | InstructionKind::Rtr
            | InstructionKind::TrapV => instr_kind,
            InstructionKind::Negx(unary) => {
                let unary = self.resolve_unary(unary, start, &mut bytes)?;
                InstructionKind::Negx(unary)
            }
            InstructionKind::Clr(unary) => {
                let unary = self.resolve_unary(unary, start, &mut bytes)?;
                InstructionKind::Clr(unary)
            }
            InstructionKind::Neg(unary) => {
                let unary = self.resolve_unary(unary, start, &mut bytes)?;
                InstructionKind::Neg(unary)
            }
            InstructionKind::Not(unary) => {
                let unary = self.resolve_unary(unary, start, &mut bytes)?;
                InstructionKind::Not(unary)
            }
            InstructionKind::Asd(shift) => {
                let shift = self.resolve_shift(shift, start, &mut bytes)?;
                InstructionKind::Asd(shift)
            }
            InstructionKind::Lsd(shift) => {
                let shift = self.resolve_shift(shift, start, &mut bytes)?;
                InstructionKind::Lsd(shift)
            }
            InstructionKind::Roxd(shift) => {
                let shift = self.resolve_shift(shift, start, &mut bytes)?;
                InstructionKind::Roxd(shift)
            }
            InstructionKind::Rod(shift) => {
                let shift = self.resolve_shift(shift, start, &mut bytes)?;
                InstructionKind::Rod(shift)
            }
            InstructionKind::Tas { mode } => {
                let mode = self.resolve_ea(mode, start + 2, Some(Size::Byte))?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Tas { mode }
            }
            InstructionKind::Tst { size, mode } => {
                let mode = self.resolve_ea(mode, start + 2, Some(size))?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Tst { size, mode }
            }
            InstructionKind::Jsr { mode } => {
                let mode = self.resolve_ea(mode, start + 2, None)?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Jsr { mode }
            }
            InstructionKind::Jmp { mode } => {
                let mode = self.resolve_ea(mode, start + 2, None)?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Jmp { mode }
            }
            InstructionKind::Adda {
                addr_reg,
                size,
                mode,
            } => {
                let mode = self.resolve_ea(mode, start + 2, None)?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Adda {
                    addr_reg,
                    size,
                    mode,
                }
            }
            InstructionKind::Add(add) => match add {
                Add::EaToDn(EaToDn { size, dst, src }) => {
                    let src = self.resolve_ea(src, start + 2, None)?;
                    bytes.extend(src.to_bytes());
                    InstructionKind::Add(Add::EaToDn(EaToDn { size, src, dst }))
                }
                Add::DnToEa(DnToEa { size, src, dst }) => {
                    let dst = self.resolve_ea(dst, start + 2, None)?;
                    bytes.extend(dst.to_bytes());
                    InstructionKind::Add(Add::DnToEa(DnToEa { size, src, dst }))
                }
            },
            InstructionKind::Addx(_) => instr_kind,
            InstructionKind::Trap { vector } => InstructionKind::Trap { vector },
            InstructionKind::Link {
                addr_reg,
                displacement: _,
            } => {
                let disp_word = self.memory.read_word(start + 2)?;
                bytes.extend(disp_word.to_be_bytes());
                InstructionKind::Link {
                    addr_reg,
                    displacement: disp_word as i16,
                }
            }
            InstructionKind::Unlk { addr_reg } => InstructionKind::Unlk { addr_reg },
            InstructionKind::Btst(bit_op) => {
                let bit_op = self.resolve_bit_op(bit_op, start, &mut bytes)?;
                InstructionKind::Btst(bit_op)
            }
            InstructionKind::Bchg(bit_op) => {
                let bit_op = self.resolve_bit_op(bit_op, start, &mut bytes)?;
                InstructionKind::Bchg(bit_op)
            }
            InstructionKind::Bclr(bit_op) => {
                let bit_op = self.resolve_bit_op(bit_op, start, &mut bytes)?;
                InstructionKind::Bclr(bit_op)
            }
            InstructionKind::Bset(bit_op) => {
                let bit_op = self.resolve_bit_op(bit_op, start, &mut bytes)?;
                InstructionKind::Bset(bit_op)
            }
            InstructionKind::Addq(quick_op) => {
                let quick_op = self.resolve_quick_op(quick_op, start, &mut bytes)?;
                InstructionKind::Addq(quick_op)
            }
            InstructionKind::Subq(quick_op) => {
                let quick_op = self.resolve_quick_op(quick_op, start, &mut bytes)?;
                InstructionKind::Subq(quick_op)
            }
            InstructionKind::Suba {
                addr_reg,
                size,
                mode,
            } => {
                let mode = self.resolve_ea(mode, start + 2, None)?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Suba {
                    addr_reg,
                    size,
                    mode,
                }
            }
            InstructionKind::Sub(sub) => match sub {
                Sub::EaToDn(EaToDn { size, dst, src }) => {
                    let src = self.resolve_ea(src, start + 2, None)?;
                    bytes.extend(src.to_bytes());
                    InstructionKind::Sub(Sub::EaToDn(EaToDn { size, src, dst }))
                }
                Sub::DnToEa(DnToEa { size, src, dst }) => {
                    let dst = self.resolve_ea(dst, start + 2, None)?;
                    bytes.extend(dst.to_bytes());
                    InstructionKind::Sub(Sub::DnToEa(DnToEa { size, src, dst }))
                }
            },
            InstructionKind::Subx(_) => instr_kind,
            InstructionKind::Andi(imm_op) => {
                let imm_op = self.resolve_imm_op(imm_op, start, &mut bytes)?;
                InstructionKind::Andi(imm_op)
            }
            InstructionKind::Subi(imm_op) => {
                let imm_op = self.resolve_imm_op(imm_op, start, &mut bytes)?;
                InstructionKind::Subi(imm_op)
            }
            InstructionKind::Addi(imm_op) => {
                let imm_op = self.resolve_imm_op(imm_op, start, &mut bytes)?;
                InstructionKind::Addi(imm_op)
            }
            InstructionKind::Eori(imm_op) => {
                let imm_op = self.resolve_imm_op(imm_op, start, &mut bytes)?;
                InstructionKind::Eori(imm_op)
            }
            InstructionKind::Cmpi(imm_op) => {
                let imm_op = self.resolve_imm_op(imm_op, start, &mut bytes)?;
                InstructionKind::Cmpi(imm_op)
            }
            InstructionKind::EoriToCcr { .. } => {
                let word = self.memory.read_word(start + 2)?;
                bytes.extend(word.to_be_bytes());
                InstructionKind::EoriToCcr { imm: word as u8 }
            }
            InstructionKind::EoriToSr { .. } => {
                let word = self.memory.read_word(start + 2)?;
                bytes.extend(word.to_be_bytes());
                InstructionKind::EoriToSr { imm: word }
            }
            InstructionKind::Ori(imm_op) => {
                let imm_op = self.resolve_imm_op(imm_op, start, &mut bytes)?;
                InstructionKind::Ori(imm_op)
            }
            InstructionKind::OriToCcr { .. } => {
                let word = self.memory.read_word(start + 2)?;
                bytes.extend(word.to_be_bytes());
                InstructionKind::OriToCcr { imm: word as u8 }
            }
            InstructionKind::OriToSr { .. } => {
                let word = self.memory.read_word(start + 2)?;
                bytes.extend(word.to_be_bytes());
                InstructionKind::OriToSr { imm: word }
            }
            InstructionKind::Move { size, src, dst } => {
                let src = self.resolve_ea(src, start + 2, Some(size))?;
                bytes.extend(src.to_bytes());
                let dst = self.resolve_ea(dst, start + 2 + src.to_bytes().len(), None)?;
                bytes.extend(dst.to_bytes());
                InstructionKind::Move { size, src, dst }
            }
            InstructionKind::Movea { size, src, dst } => {
                let src = self.resolve_ea(src, start + 2, Some(size))?;
                bytes.extend(src.to_bytes());
                InstructionKind::Movea { size, src, dst }
            }
            InstructionKind::Movep(Movep {
                size,
                data_reg,
                addr_reg,
                direction,
                ..
            }) => {
                let disp_word = self.memory.read_word(start + 2)?;
                bytes.extend(disp_word.to_be_bytes());
                InstructionKind::Movep(Movep {
                    size,
                    data_reg,
                    addr_reg,
                    displacement: disp_word as i16,
                    direction,
                })
            }
            InstructionKind::MoveFromSr { dst } => {
                let dst = self.resolve_ea(dst, start + 2, Some(Size::Word))?;
                bytes.extend(dst.to_bytes());
                InstructionKind::MoveFromSr { dst }
            }
            InstructionKind::MoveToCcr { src } => {
                let src = self.resolve_ea(src, start + 2, Some(Size::Word))?;
                bytes.extend(src.to_bytes());
                InstructionKind::MoveToCcr { src }
            }
            InstructionKind::MoveToSr { src } => {
                let src = self.resolve_ea(src, start + 2, Some(Size::Word))?;
                bytes.extend(src.to_bytes());
                InstructionKind::MoveToSr { src }
            }
            InstructionKind::MoveUsp { .. }
            | InstructionKind::Ext { .. }
            | InstructionKind::Swap { .. } => instr_kind,
            InstructionKind::Nbcd { mode } => {
                let mode = self.resolve_ea(mode, start + 2, Some(Size::Byte))?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Nbcd { mode }
            }
            InstructionKind::Pea { mode } => {
                let mode = self.resolve_ea(mode, start + 2, None)?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Pea { mode }
            }
        };

        let instruction = Instruction {
            address: start,
            opcode,
            bytes,
            kind,
        };
        Ok(instruction)
    }

    fn get_op_kind(opcode: u16) -> Result<InstructionKind> {
        // handle the easy instructions
        match opcode {
            0x4E70 => return Ok(InstructionKind::Reset),
            0x4E71 => return Ok(InstructionKind::Nop),
            0x4E73 => return Ok(InstructionKind::Rte),
            0x4E75 => return Ok(InstructionKind::Rts),
            0x4E76 => return Ok(InstructionKind::TrapV),
            0x4E77 => return Ok(InstructionKind::Rtr),
            0x4AFC => return Ok(InstructionKind::Illegal),
            _ => {}
        }

        let group = bit_range(opcode, 12, 16); // 12..16
        let top_reg = bit_range(opcode, 9, 12); // 9..12
        let op_nibble = bit_range(opcode, 8, 12); // 8..12
        let eight_nine = bit_range(opcode, 8, 9); // 8..9
        let _seven_nine = bit_range(opcode, 7, 9); // 7..9
        let opmode = bit_range(opcode, 6, 9); //  6..9
        let six_seven = bit_range(opcode, 6, 7); // 6..7
        let size_bits = bit_range(opcode, 6, 8); // 6..8
        let mid = bit_range(opcode, 4, 8); // 4..8
        let ea_bits = bit_range(opcode, 0, 6); // 0..6
        let ea_mode = bit_range(opcode, 3, 6); // 3..6
        let op_field = bit_range(opcode, 3, 5); // 3..5
        let link_unlk_bit = bit_range(opcode, 3, 4); // 3..4
        let ea_reg = bit_range(opcode, 0, 3); // 0..3
        let trap_bits = bit_range(opcode, 0, 4); // 0..4
        match group {
            0b0100 => {
                // Trap
                if op_nibble == 0b1110 && mid == 0b0100 {
                    return Ok(InstructionKind::Trap { vector: trap_bits });
                }

                // Link/Unlk
                if op_nibble == 0b1110 && mid == 0b0101 {
                    let addr_reg = AddrReg::from_bits(ea_reg)?;

                    if link_unlk_bit == 0 {
                        // Link An,#disp
                        return Ok(InstructionKind::Link {
                            addr_reg,
                            displacement: 0,
                        });
                    } else {
                        // UNLK An
                        return Ok(InstructionKind::Unlk { addr_reg });
                    }
                }

                // Jsr/Jmp
                if top_reg == 0b111 && opmode == 0b100 {
                    match six_seven == 0b0 {
                        // Jsr <ea>
                        true => {
                            return Ok(InstructionKind::Jsr {
                                mode: effective_address(ea_bits)?,
                            });
                        }
                        false => {
                            // Jmp <ea>
                            return Ok(InstructionKind::Jmp {
                                mode: effective_address(ea_bits)?,
                            });
                        }
                    }
                }
                // Tas/Tst
                if top_reg == 0b101 {
                    match size_bits == 0b11 {
                        // Tas <ea>
                        true => {
                            return Ok(InstructionKind::Tas {
                                mode: effective_address(ea_bits)?,
                            });
                        }
                        false => {
                            // Tst <ea>
                            return Ok(InstructionKind::Tst {
                                size: Size::from_size_bits(size_bits)?,
                                mode: effective_address(ea_bits)?,
                            });
                        }
                    }
                }
                // MOVE from SR / MOVE to CCR / MOVE to SR
                // MOVE from SR: 0100 0000 11 eeeeee
                // MOVE to CCR:  0100 0100 11 eeeeee
                // MOVE to SR:   0100 0110 11 eeeeee
                if size_bits == 0b11 && matches!(op_nibble, 0b0000 | 0b0100 | 0b0110) {
                    let mode = effective_address(ea_bits)?;
                    return match op_nibble {
                        0b0000 => Ok(InstructionKind::MoveFromSr { dst: mode }),
                        0b0100 => Ok(InstructionKind::MoveToCcr { src: mode }),
                        0b0110 => Ok(InstructionKind::MoveToSr { src: mode }),
                        _ => unreachable!(),
                    };
                }
                // MOVE USP: 0100 1110 0110 d aaa
                if op_nibble == 0b1110 && mid == 0b0110 {
                    let addr_reg = AddrReg::from_bits(ea_reg)?;
                    let direction = if link_unlk_bit == 0 {
                        UspDirection::RegToUsp
                    } else {
                        UspDirection::UspToReg
                    };
                    return Ok(InstructionKind::MoveUsp { addr_reg, direction });
                }
                // NBCD: 0100 1000 00 eeeeee
                // SWAP: 0100 1000 01 000 rrr
                // PEA:  0100 1000 01 eeeeee (ea_mode != 000)
                // EXT:  0100 100 ooo 000 rrr (opmode = 010/011/111)
                if op_nibble == 0b1000 {
                    if size_bits == 0b00 {
                        let mode = effective_address(ea_bits)?;
                        return Ok(InstructionKind::Nbcd { mode });
                    }
                    if size_bits == 0b01 {
                        if ea_mode == 0b000 {
                            let data_reg = DataReg::from_bits(ea_reg)?;
                            return Ok(InstructionKind::Swap { data_reg });
                        } else {
                            let mode = effective_address(ea_bits)?;
                            return Ok(InstructionKind::Pea { mode });
                        }
                    }
                }
                // EXT: 0100 100 ooo 000 rrr (opmode = 010/011/111)
                if top_reg == 0b100 && ea_mode == 0b000 && matches!(opmode, 0b010 | 0b011 | 0b111) {
                    let data_reg = DataReg::from_bits(ea_reg)?;
                    let mode = match opmode {
                        0b010 => ExtMode::ByteToWord,
                        0b011 => ExtMode::WordToLong,
                        0b111 => ExtMode::ByteToLong,
                        _ => unreachable!(),
                    };
                    return Ok(InstructionKind::Ext { data_reg, mode });
                }
                match op_nibble {
                    0b0000 | 0b0010 | 0b0100 | 0b0110 => {
                        let size = Size::from_size_bits(size_bits)?;
                        let mode = effective_address(ea_bits)?;
                        let unary = UnaryOp { size, mode };
                        return match op_nibble {
                            0b0000 => Ok(InstructionKind::Negx(unary)),
                            0b0010 => Ok(InstructionKind::Clr(unary)),
                            0b0100 => Ok(InstructionKind::Neg(unary)),
                            0b0110 => Ok(InstructionKind::Not(unary)),
                            _ => unreachable!(),
                        };
                    }
                    _ => {}
                }
                bail!("Unsupported");
            }
            0b1110 => {
                let direction = RightOrLeft::from_bit(eight_nine)?;
                if size_bits == 0b11 {
                    let mode = effective_address(ea_bits)?;
                    let shift = Shift::Ea(ShiftEa { direction, mode });
                    return match top_reg {
                        0b000 => Ok(InstructionKind::Asd(shift)),
                        0b001 => Ok(InstructionKind::Lsd(shift)),
                        0b010 => Ok(InstructionKind::Roxd(shift)),
                        0b011 => Ok(InstructionKind::Rod(shift)),
                        _ => bail!("Unsupported shift/rotate opmode: {:#05b}", top_reg),
                    };
                }

                let size = Size::from_size_bits(size_bits)?;
                let rotation = Rotation::from_bit(bit_range(opcode, 5, 6))?;
                let count = match rotation {
                    Rotation::Immediate => {
                        let count = match top_reg {
                            0 => 8,
                            other => other,
                        };
                        ShiftCount::Immediate(count)
                    }
                    Rotation::Register => ShiftCount::Register(DataReg::from_bits(top_reg)?),
                };
                let dst = DataReg::from_bits(ea_reg)?;
                let shift = Shift::Reg(ShiftReg {
                    direction,
                    size,
                    count,
                    dst,
                });
                match op_field {
                    0b00 => Ok(InstructionKind::Asd(shift)),
                    0b01 => Ok(InstructionKind::Lsd(shift)),
                    0b10 => Ok(InstructionKind::Roxd(shift)),
                    0b11 => Ok(InstructionKind::Rod(shift)),
                    _ => unreachable!(),
                }
            }
            0b1101 => {
                // Add/Addx/Adda
                match opmode {
                    // Adda <ea>,An
                    0b011 | 0b111 => Ok(InstructionKind::Adda {
                        addr_reg: AddrReg::from_bits(top_reg)?,
                        size: Size::from_wl_bit(eight_nine)?,
                        mode: effective_address(ea_bits)?,
                    }),
                    // Add <ea>, Dn
                    0b000..=0b010 => Ok(InstructionKind::Add(Add::EaToDn(EaToDn {
                        size: Size::from_wl_bit(eight_nine)?,
                        dst: DataReg::from_bits(top_reg)?,
                        src: effective_address(ea_bits)?,
                    }))),
                    0b100..=0b110 => match ea_mode {
                        // Addx Dn, Dn
                        0b000 => Ok(InstructionKind::Addx(Addx::Dn(Dn {
                            size: Size::from_size_bits(size_bits)?,
                            src: DataReg::from_bits(ea_reg)?,
                            dst: DataReg::from_bits(top_reg)?,
                        }))),
                        // Addx -(An), -(An)
                        0b001 => Ok(InstructionKind::Addx(Addx::PreDec(PreDec {
                            size: Size::from_size_bits(size_bits)?,
                            src: AddrReg::from_bits(ea_reg)?,
                            dst: AddrReg::from_bits(top_reg)?,
                        }))),
                        // Add Dn,<ea>
                        _ => Ok(InstructionKind::Add(Add::DnToEa(DnToEa {
                            size: Size::from_size_bits(size_bits)?,
                            src: DataReg::from_bits(top_reg)?,
                            dst: effective_address(ea_bits)?,
                        }))),
                    },
                    _ => bail!("Unsupported opmode: {:#05b}", opmode),
                }
            }
            0b0000 => {
                // EORI to CCR: 0000 1010 0011 1100 (0x0A3C)
                if opcode == 0x0A3C {
                    return Ok(InstructionKind::EoriToCcr { imm: 0 });
                }
                // EORI to SR: 0000 1010 0111 1100 (0x0A7C)
                if opcode == 0x0A7C {
                    return Ok(InstructionKind::EoriToSr { imm: 0 });
                }
                // ORI to CCR: 0000 0000 0011 1100 (0x003C)
                if opcode == 0x003C {
                    return Ok(InstructionKind::OriToCcr { imm: 0 });
                }
                // ORI to SR: 0000 0000 0111 1100 (0x007C)
                if opcode == 0x007C {
                    return Ok(InstructionKind::OriToSr { imm: 0 });
                }
                // MOVEP: 0000 rrr ooo 001 aaa
                // opmode: 100=MOVEP.W Mem->Reg, 101=MOVEP.L Mem->Reg, 110=MOVEP.W Reg->Mem, 111=MOVEP.L Reg->Mem
                if ea_mode == 0b001 && opmode >= 0b100 {
                    let data_reg = DataReg::from_bits(top_reg)?;
                    let addr_reg = AddrReg::from_bits(ea_reg)?;
                    let (size, direction) = match opmode {
                        0b100 => (Size::Word, MovepDirection::MemToReg),
                        0b101 => (Size::Long, MovepDirection::MemToReg),
                        0b110 => (Size::Word, MovepDirection::RegToMem),
                        0b111 => (Size::Long, MovepDirection::RegToMem),
                        _ => unreachable!(),
                    };
                    return Ok(InstructionKind::Movep(Movep {
                        size,
                        data_reg,
                        addr_reg,
                        displacement: 0, // resolved later
                        direction,
                    }));
                }
                // Btst/Bchg/Bclr/Bset #imm
                if op_nibble == 0b1000 {
                    let mode = effective_address(ea_bits)?;
                    let bit_op = BitOp::Imm(BitOpImm { bit_num: 0, mode });
                    return match size_bits {
                        0b00 => Ok(InstructionKind::Btst(bit_op)),
                        0b01 => Ok(InstructionKind::Bchg(bit_op)),
                        0b10 => Ok(InstructionKind::Bclr(bit_op)),
                        0b11 => Ok(InstructionKind::Bset(bit_op)),
                        _ => unreachable!(),
                    };
                }
                // Btst/Bchg/Bclr/Bset Dn
                if (0b100..=0b111).contains(&opmode) {
                    let mode = effective_address(ea_bits)?;
                    let bit_reg = DataReg::from_bits(top_reg)?;
                    let bit_op = BitOp::Reg(BitOpReg { bit_reg, mode });
                    return match opmode {
                        0b100 => Ok(InstructionKind::Btst(bit_op)),
                        0b101 => Ok(InstructionKind::Bchg(bit_op)),
                        0b110 => Ok(InstructionKind::Bclr(bit_op)),
                        0b111 => Ok(InstructionKind::Bset(bit_op)),
                        _ => unreachable!(),
                    };
                }
                // Ori/Andi/Subi/Addi/Eori/Cmpi #imm, <ea>
                // 0000 oooo ss eeeeee (oooo: 0000=ORI, 0010=ANDI, 0100=SUBI, 0110=ADDI, 1010=EORI, 1100=CMPI)
                if matches!(op_nibble, 0b0000 | 0b0010 | 0b0100 | 0b0110 | 0b1010 | 0b1100) {
                    let size = Size::from_size_bits(size_bits)?;
                    let mode = effective_address(ea_bits)?;
                    // Immediate value will be read during resolve
                    let imm_op = ImmOp {
                        imm: Immediate::Byte(0),
                        size,
                        mode,
                    };
                    return match op_nibble {
                        0b0000 => Ok(InstructionKind::Ori(imm_op)),
                        0b0010 => Ok(InstructionKind::Andi(imm_op)),
                        0b0100 => Ok(InstructionKind::Subi(imm_op)),
                        0b0110 => Ok(InstructionKind::Addi(imm_op)),
                        0b1010 => Ok(InstructionKind::Eori(imm_op)),
                        0b1100 => Ok(InstructionKind::Cmpi(imm_op)),
                        _ => unreachable!(),
                    };
                }
                bail!("Unsupported group 0 instruction");
            }
            0b0101 => {
                // Addq/Subq: Dn Size EA
                let data = match top_reg {
                    0 => 8,
                    n => n,
                };
                let size = Size::from_size_bits(size_bits)?;
                let mode = effective_address(ea_bits)?;
                let quick_op = QuickOp { data, size, mode };
                match eight_nine {
                    0 => Ok(InstructionKind::Addq(quick_op)),
                    1 => Ok(InstructionKind::Subq(quick_op)),
                    _ => unreachable!(),
                }
            }
            0b1001 => {
                // Sub/Subx/Suba
                match opmode {
                    // Suba <ea>,An
                    0b011 | 0b111 => Ok(InstructionKind::Suba {
                        addr_reg: AddrReg::from_bits(top_reg)?,
                        size: Size::from_wl_bit(eight_nine)?,
                        mode: effective_address(ea_bits)?,
                    }),
                    // Sub <ea>, Dn
                    0b000..=0b010 => Ok(InstructionKind::Sub(Sub::EaToDn(EaToDn {
                        size: Size::from_size_bits(size_bits)?,
                        dst: DataReg::from_bits(top_reg)?,
                        src: effective_address(ea_bits)?,
                    }))),
                    0b100..=0b110 => match ea_mode {
                        // Subx Dn, Dn
                        0b000 => Ok(InstructionKind::Subx(Subx::Dn(Dn {
                            size: Size::from_size_bits(size_bits)?,
                            src: DataReg::from_bits(ea_reg)?,
                            dst: DataReg::from_bits(top_reg)?,
                        }))),
                        // Subx -(An), -(An)
                        0b001 => Ok(InstructionKind::Subx(Subx::PreDec(PreDec {
                            size: Size::from_size_bits(size_bits)?,
                            src: AddrReg::from_bits(ea_reg)?,
                            dst: AddrReg::from_bits(top_reg)?,
                        }))),
                        // Sub Dn,<ea>
                        _ => Ok(InstructionKind::Sub(Sub::DnToEa(DnToEa {
                            size: Size::from_size_bits(size_bits)?,
                            src: DataReg::from_bits(top_reg)?,
                            dst: effective_address(ea_bits)?,
                        }))),
                    },
                    _ => bail!("Unsupported opmode: {:#05b}", opmode),
                }
            }
            // MOVE/MOVEA: 00ss ddd mmm sss nnn
            // size encoding: 01=byte, 11=word, 10=long (different from normal!)
            // ddd=dest reg (top_reg), mmm=dest mode (opmode), sss=src mode (ea_mode), nnn=src reg (ea_reg)
            0b0001 | 0b0010 | 0b0011 => {
                let size = match group {
                    0b0001 => Size::Byte,
                    0b0011 => Size::Word,
                    0b0010 => Size::Long,
                    _ => unreachable!(),
                };
                let src = effective_address(ea_bits)?;
                // Destination EA is encoded differently: mode in bits 6-8, reg in bits 9-11
                let dst_mode = opmode;
                let dst_reg = top_reg;
                // MOVEA: destination is address register (mode == 001)
                if dst_mode == 0b001 {
                    let dst = AddrReg::from_bits(dst_reg)?;
                    return Ok(InstructionKind::Movea { size, src, dst });
                }
                // Regular MOVE
                let dst_ea_bits = (dst_mode << 3) | dst_reg;
                let dst = effective_address(dst_ea_bits)?;
                Ok(InstructionKind::Move { size, src, dst })
            }
            _ => bail!("Unsupported group: {:#06b}", group),
        }
    }
}

fn bit_range(word: u16, start: u8, end: u8) -> u8 {
    assert!(end >= start);
    let width = end - start;

    let mask = if width == 16 {
        0xFFFF
    } else {
        (1 << width) - 1
    };

    ((word >> start) & mask) as u8
}

fn bit_range_u8(word: u8, start: u8, end: u8) -> u8 {
    assert!(end >= start);
    let width = end - start;

    let mask = if width == 8 { 0xFF } else { (1 << width) - 1 };

    (word >> start) & mask
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DataReg {
    D0,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
}

impl DataReg {
    fn from_bits(value: u8) -> Result<Self> {
        match value {
            0b000 => Ok(DataReg::D0),
            0b001 => Ok(DataReg::D1),
            0b010 => Ok(DataReg::D2),
            0b011 => Ok(DataReg::D3),
            0b100 => Ok(DataReg::D4),
            0b101 => Ok(DataReg::D5),
            0b110 => Ok(DataReg::D6),
            0b111 => Ok(DataReg::D7),
            _ => bail!("Invalid bits, {:#5b}", value),
        }
    }

    fn number(self) -> u8 {
        match self {
            DataReg::D0 => 0,
            DataReg::D1 => 1,
            DataReg::D2 => 2,
            DataReg::D3 => 3,
            DataReg::D4 => 4,
            DataReg::D5 => 5,
            DataReg::D6 => 6,
            DataReg::D7 => 7,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AddrReg {
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
}

impl AddrReg {
    fn from_bits(value: u8) -> Result<Self> {
        match value {
            0b000 => Ok(AddrReg::A0),
            0b001 => Ok(AddrReg::A1),
            0b010 => Ok(AddrReg::A2),
            0b011 => Ok(AddrReg::A3),
            0b100 => Ok(AddrReg::A4),
            0b101 => Ok(AddrReg::A5),
            0b110 => Ok(AddrReg::A6),
            0b111 => Ok(AddrReg::A7),
            _ => bail!("Invalid bits, {:#5b}", value),
        }
    }

    fn number(self) -> u8 {
        match self {
            AddrReg::A0 => 0,
            AddrReg::A1 => 1,
            AddrReg::A2 => 2,
            AddrReg::A3 => 3,
            AddrReg::A4 => 4,
            AddrReg::A5 => 5,
            AddrReg::A6 => 6,
            AddrReg::A7 => 7,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct AddressingMode {
    pub ea: EffectiveAddress,
    pub data: Option<AddressModeData>,
}

impl From<EffectiveAddress> for AddressingMode {
    fn from(value: EffectiveAddress) -> Self {
        Self {
            ea: value,
            data: None,
        }
    }
}

impl AddressingMode {
    pub fn to_bytes(self) -> Vec<u8> {
        if let Some(data) = self.data {
            data.to_bytes()
        } else {
            vec![]
        }
    }

    pub fn short_data(&self) -> Option<u16> {
        match self.data {
            Some(AddressModeData::Short(value)) => Some(value),
            Some(AddressModeData::Imm(Immediate::Word(value))) => Some(value),
            _ => None,
        }
    }

    pub fn long_data(&self) -> Option<u32> {
        match self.data {
            Some(AddressModeData::Long(value)) => Some(value),
            Some(AddressModeData::Imm(Immediate::Long(value))) => Some(value),
            _ => None,
        }
    }

    pub fn immediate(&self) -> Option<Immediate> {
        match self.data {
            Some(AddressModeData::Imm(immediate)) => Some(immediate),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum EffectiveAddress {
    Dr(DataReg),           // Dn            b000 reg
    Ar(AddrReg),           // An            b001 reg
    Addr(AddrReg),         // (An)          b010 reg
    AddrPostIncr(AddrReg), // (An)+         b011 reg
    AddrPreDecr(AddrReg),  // -(An)         b100 reg
    AddrDisplace(AddrReg), // (d, An)       b101 reg
    AddrIndex(AddrReg),    // (d, An, Xn)   b110 reg
    PCDisplace,            // (d, PC)       b111 b010
    PCIndex,               // (d, PC, Xn)   b111 b011
    AbsShort,              // (xxx.W)       b111 b000
    AbsLong,               // (xxx.L)       b111 b001
    Immediate,             // #imm          b111 b100
}

pub fn effective_address(bits: u8) -> Result<AddressingMode> {
    let m = bit_range_u8(bits, 3, 6);
    let xn = bit_range_u8(bits, 0, 3);

    match (m, xn) {
        (0b000, _) => Ok(AddressingMode::from(EffectiveAddress::Dr(
            DataReg::from_bits(xn)?,
        ))),
        (0b001, _) => Ok(AddressingMode::from(EffectiveAddress::Ar(
            AddrReg::from_bits(xn)?,
        ))),
        (0b010, _) => Ok(AddressingMode::from(EffectiveAddress::Addr(
            AddrReg::from_bits(xn)?,
        ))),
        (0b011, _) => Ok(AddressingMode::from(EffectiveAddress::AddrPostIncr(
            AddrReg::from_bits(xn)?,
        ))),
        (0b100, _) => Ok(AddressingMode::from(EffectiveAddress::AddrPreDecr(
            AddrReg::from_bits(xn)?,
        ))),
        (0b101, _) => Ok(AddressingMode::from(EffectiveAddress::AddrDisplace(
            AddrReg::from_bits(xn)?,
        ))),
        (0b110, _) => Ok(AddressingMode::from(EffectiveAddress::AddrIndex(
            AddrReg::from_bits(xn)?,
        ))),
        (0b111, 0b010) => Ok(AddressingMode::from(EffectiveAddress::PCDisplace)),
        (0b111, 0b011) => Ok(AddressingMode::from(EffectiveAddress::PCIndex)),
        (0b111, 0b000) => Ok(AddressingMode::from(EffectiveAddress::AbsShort)),
        (0b111, 0b001) => Ok(AddressingMode::from(EffectiveAddress::AbsLong)),
        (0b111, 0b100) => Ok(AddressingMode::from(EffectiveAddress::Immediate)),
        _ => bail!("m: {m:#05b}, xn: {xn:#05b}"),
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Size {
    Byte, // .b b00 | / | b01
    Word, // .w b01 | 0 | b11
    Long, // .l b10 | 1 | b10
}

impl Size {
    pub fn from_wl_bit(value: u8) -> Result<Self> {
        match value {
            0b0 => Ok(Size::Word),
            0b1 => Ok(Size::Long),
            _ => bail!("Invalid bits: {value}"),
        }
    }

    pub fn from_size_bits(value: u8) -> Result<Self> {
        match value {
            0b00 => Ok(Size::Byte),
            0b01 => Ok(Size::Word),
            0b10 => Ok(Size::Long),
            _ => bail!("Illegal size field"),
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub enum Condition {
    True,           // T   b0000
    False,          // F   b0001
    Higher,         // HI  b0010
    LowerOrSame,    // LS  b0011
    CarryClear,     // CC  b0100
    CarrySet,       // CS  b0101
    NotEqual,       // NE  b0110
    Equal,          // EQ  b0111
    OverflowClear,  // VC  b1000
    OverflowSet,    // VS  b1001
    Plus,           // PL  b1010
    Minus,          // MI  b1011
    GreaterOrEqual, // GE  b1100
    LessThan,       // LT  b1101
    GreaterThan,    // GT  b1110
    LessOrEqual,    // LE  b1111
}

impl From<u8> for Condition {
    fn from(value: u8) -> Self {
        match value {
            0b0000 => Self::True,
            0b0001 => Self::False,
            0b0010 => Self::Higher,
            0b0011 => Self::LowerOrSame,
            0b0100 => Self::CarryClear,
            0b0101 => Self::CarrySet,
            0b0110 => Self::NotEqual,
            0b0111 => Self::Equal,
            0b1000 => Self::OverflowClear,
            0b1001 => Self::OverflowSet,
            0b1010 => Self::Plus,
            0b1011 => Self::Minus,
            0b1100 => Self::GreaterOrEqual,
            0b1101 => Self::LessThan,
            0b1110 => Self::GreaterThan,
            0b1111 => Self::LessOrEqual,
            _ => unreachable!(),
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub enum DataDir {
    RegToMem, // 0 1
    MemToReg, // 1 0
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub enum DnEa {
    DnEa, // Dn, Ea -> Dn 0
    EaDn, // Ea, Dn -> Ea 1
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RightOrLeft {
    Right, // R b0
    Left,  // L b1
}

impl RightOrLeft {
    pub fn from_bit(bit: u8) -> Result<Self> {
        match bit {
            0 => Ok(RightOrLeft::Right),
            1 => Ok(RightOrLeft::Left),
            _ => bail!("Invalid direction bit: {bit}"),
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub enum Mode {
    DataReg,     // Dn  b0
    AddrPreDecr, // -(An) b1
}

#[derive(Debug, Clone, Copy)]
pub enum Rotation {
    Immediate, // 0
    Register,  // 1
}

impl Rotation {
    pub fn from_bit(bit: u8) -> Result<Self> {
        match bit {
            0 => Ok(Rotation::Immediate),
            1 => Ok(Rotation::Register),
            _ => bail!("Invalid rotation bit: {bit}"),
        }
    }
}
