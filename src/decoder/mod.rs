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
    Rtd {
        displacement: i16,
    },
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
    Trapcc {
        condition: Condition,
        operand: Option<Immediate>,
    },
    Bkpt {
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
    Moveq {
        data: i8,
        dst: DataReg,
    },
    Scc {
        condition: Condition,
        mode: AddressingMode,
    },
    DBcc {
        condition: Condition,
        data_reg: DataReg,
        displacement: i16,
    },
    Bra {
        displacement: i32,
    },
    Bsr {
        displacement: i32,
    },
    Bcc {
        condition: Condition,
        displacement: i32,
    },
    Divu {
        src: AddressingMode,
        dst: DataReg,
    },
    Divs {
        src: AddressingMode,
        dst: DataReg,
    },
    DivuL {
        src: AddressingMode,
        dq: DataReg,    // quotient register
        dr: DataReg,    // remainder register
        is_64bit: bool, // true = 64÷32, false = 32÷32
    },
    DivsL {
        src: AddressingMode,
        dq: DataReg,
        dr: DataReg,
        is_64bit: bool,
    },
    Sbcd(Sbcd),
    Or(Or),
    Cmp(EaToDn),
    Cmpa {
        addr_reg: AddrReg,
        size: Size,
        src: AddressingMode,
    },
    Cmpm {
        size: Size,
        src: AddrReg, // (Ay)+
        dst: AddrReg, // (Ax)+
    },
    Eor(DnToEa),
    Mulu {
        src: AddressingMode,
        dst: DataReg,
    },
    Muls {
        src: AddressingMode,
        dst: DataReg,
    },
    MuluL {
        src: AddressingMode,
        dl: DataReg,         // low result register
        dh: Option<DataReg>, // high result register (if 64-bit result)
    },
    MulsL {
        src: AddressingMode,
        dl: DataReg,
        dh: Option<DataReg>,
    },
    Abcd(Abcd),
    Exg(Exg),
    And(And),
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
    Lea {
        src: AddressingMode,
        dst: AddrReg,
    },
    Chk {
        size: Size,
        src: AddressingMode,
        data_reg: DataReg,
    },
    Movem(Movem),
    /// CAS - Compare and Swap (68020+)
    Cas {
        size: Size,
        dc: DataReg, // Compare operand
        du: DataReg, // Update operand
        mode: AddressingMode,
    },
    /// CAS2 - Compare and Swap 2 (68020+)
    /// Dual compare-and-swap on two memory locations
    Cas2 {
        size: Size,
        dc1: DataReg, // First compare operand
        dc2: DataReg, // Second compare operand
        du1: DataReg, // First update operand
        du2: DataReg, // Second update operand
        rn1: AddrReg, // First address register
        rn2: AddrReg, // Second address register
    },
    /// CMP2 - Compare Register Against Bounds (68020+)
    Cmp2 {
        size: Size,
        mode: AddressingMode,
        reg: Register,
    },
    /// CHK2 - Check Register Against Bounds (68020+)
    Chk2 {
        size: Size,
        mode: AddressingMode,
        reg: Register,
    },
    /// BFTST - Test Bit Field (68020+)
    Bftst {
        mode: AddressingMode,
        offset: BitFieldParam,
        width: BitFieldParam,
    },
    /// BFCHG - Change Bit Field (invert bits) (68020+)
    Bfchg {
        mode: AddressingMode,
        offset: BitFieldParam,
        width: BitFieldParam,
    },
    /// BFCLR - Clear Bit Field (68020+)
    Bfclr {
        mode: AddressingMode,
        offset: BitFieldParam,
        width: BitFieldParam,
    },
    /// BFSET - Set Bit Field (68020+)
    Bfset {
        mode: AddressingMode,
        offset: BitFieldParam,
        width: BitFieldParam,
    },
    /// BFEXTU - Extract Bit Field Unsigned (68020+)
    Bfextu {
        src: AddressingMode,
        dst: DataReg,
        offset: BitFieldParam,
        width: BitFieldParam,
    },
    /// BFEXTS - Extract Bit Field Signed (68020+)
    Bfexts {
        src: AddressingMode,
        dst: DataReg,
        offset: BitFieldParam,
        width: BitFieldParam,
    },
    /// BFINS - Insert Bit Field (68020+)
    Bfins {
        src: DataReg,
        dst: AddressingMode,
        offset: BitFieldParam,
        width: BitFieldParam,
    },
    /// BFFFO - Find First One Bit Field (68020+)
    Bfffo {
        src: AddressingMode,
        dst: DataReg,
        offset: BitFieldParam,
        width: BitFieldParam,
    },
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum UspDirection {
    RegToUsp, // An -> USP
    UspToReg, // USP -> An
}

/// Bit field offset or width parameter (68020+)
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum BitFieldParam {
    Immediate(u8),     // 0-31 for offset, 1-32 for width (0 means 32)
    Register(DataReg), // Value taken from data register
}

/// General purpose register - either data or address register
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Register {
    Data(DataReg),
    Address(AddrReg),
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

// SBCD is always byte-sized
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Sbcd {
    Dn { src: DataReg, dst: DataReg },
    PreDec { src: AddrReg, dst: AddrReg },
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Or {
    EaToDn(EaToDn),
    DnToEa(DnToEa),
}

// ABCD is always byte-sized (like SBCD)
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Abcd {
    Dn { src: DataReg, dst: DataReg },
    PreDec { src: AddrReg, dst: AddrReg },
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Exg {
    DataData { rx: DataReg, ry: DataReg },
    AddrAddr { rx: AddrReg, ry: AddrReg },
    DataAddr { data: DataReg, addr: AddrReg },
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum And {
    EaToDn(EaToDn),
    DnToEa(DnToEa),
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

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Movem {
    pub size: Size,
    pub direction: DataDir,
    pub register_mask: u16,
    pub mode: AddressingMode,
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

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum AddressModeData {
    Short(u16),
    Long(u32),
    Imm(Immediate),
    /// 68020+ full extension word format for indexed addressing
    IndexExt {
        ext_word: u16,
        base_disp: i32,
    },
}

impl AddressModeData {
    pub fn to_bytes(self) -> Vec<u8> {
        match self {
            AddressModeData::Short(v) => v.to_be_bytes().to_vec(),
            AddressModeData::Long(v) => v.to_be_bytes().to_vec(),
            AddressModeData::Imm(immediate) => match immediate {
                // Even for byte immediates, the architecture encodes a full word extension.
                Immediate::Byte(v) => (v as u16).to_be_bytes().to_vec(),
                Immediate::Word(v) => v.to_be_bytes().to_vec(),
                Immediate::Long(v) => v.to_be_bytes().to_vec(),
            },
            AddressModeData::IndexExt {
                ext_word,
                base_disp,
            } => {
                let mut bytes = ext_word.to_be_bytes().to_vec();
                // BD size is in bits 5-4: 00=reserved, 01=null, 10=word, 11=long
                let bd_size = (ext_word >> 4) & 0x3;
                match bd_size {
                    0b10 => bytes.extend((base_disp as i16).to_be_bytes()),
                    0b11 => bytes.extend(base_disp.to_be_bytes()),
                    _ => {}
                }
                bytes
            }
        }
    }
}

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
                let ext_word = self.memory.read_word(offset)?;
                // Check bit 8 for full extension word format (68020+)
                if (ext_word & 0x0100) != 0 {
                    // Full extension word format
                    // Bits 5-4: BD size (00=reserved, 01=null, 10=word, 11=long)
                    let bd_size = (ext_word >> 4) & 0x3;
                    let base_disp = match bd_size {
                        0b01 => 0i32, // Null displacement
                        0b10 => {
                            // Word displacement
                            let disp = self.memory.read_word(offset + 2)? as i16;
                            disp as i32
                        }
                        0b11 => {
                            // Long displacement
                            self.memory.read_long(offset + 2)? as i32
                        }
                        _ => 0i32, // Reserved, treat as null
                    };
                    Ok(AddressingMode {
                        ea: mode.ea,
                        data: Some(AddressModeData::IndexExt {
                            ext_word,
                            base_disp,
                        }),
                    })
                } else {
                    // Brief extension word format (68000 compatible)
                    Ok(AddressingMode {
                        ea: mode.ea,
                        data: Some(AddressModeData::Short(ext_word)),
                    })
                }
            }
            EffectiveAddress::Immediate => {
                let size = immediate_size.unwrap_or(Size::Word);
                let value = match size {
                    Size::Byte => {
                        let word = self.memory.read_word(offset)?;
                        // Even for byte-sized immediates the encoding uses a word; keep the byte value.
                        Immediate::Byte(word as u8)
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

    fn resolve_branch_displacement(
        &self,
        disp_byte: i32,
        start: usize,
        bytes: &mut Vec<u8>,
    ) -> Result<i32> {
        match disp_byte {
            0 => {
                // 16-bit displacement follows
                let word = self.memory.read_word(start + 2)?;
                bytes.extend(word.to_be_bytes());
                Ok(word as i16 as i32)
            }
            -1 => {
                // 32-bit displacement follows (68020+)
                let long = self.memory.read_long(start + 2)?;
                bytes.extend(long.to_be_bytes());
                Ok(long as i32)
            }
            _ => {
                // 8-bit displacement is in the opcode
                Ok(disp_byte)
            }
        }
    }

    pub fn decode_instruction(&self, start: usize) -> Result<Instruction> {
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
            InstructionKind::Rtd { displacement: _ } => {
                // Read 16-bit signed displacement
                let disp_word = self.memory.read_word(start + 2)?;
                bytes.extend(disp_word.to_be_bytes());
                InstructionKind::Rtd {
                    displacement: disp_word as i16,
                }
            }
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
                let mode = self.resolve_ea(mode, start + 2, Some(size))?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Adda {
                    addr_reg,
                    size,
                    mode,
                }
            }
            InstructionKind::Add(add) => match add {
                Add::EaToDn(EaToDn { size, dst, src }) => {
                    let src = self.resolve_ea(src, start + 2, Some(size))?;
                    bytes.extend(src.to_bytes());
                    InstructionKind::Add(Add::EaToDn(EaToDn { size, src, dst }))
                }
                Add::DnToEa(DnToEa { size, src, dst }) => {
                    let dst = self.resolve_ea(dst, start + 2, Some(size))?;
                    bytes.extend(dst.to_bytes());
                    InstructionKind::Add(Add::DnToEa(DnToEa { size, src, dst }))
                }
            },
            InstructionKind::Addx(_) => instr_kind,
            InstructionKind::Trap { vector } => InstructionKind::Trap { vector },
            InstructionKind::Trapcc {
                condition,
                operand: _,
            } => {
                // Extract operand size from bits 0-2 of opcode
                let size_code = bit_range(opcode, 0, 3);
                let operand = match size_code {
                    0b010 => {
                        // Word operand
                        let word = self.memory.read_word(start + 2)?;
                        bytes.extend(word.to_be_bytes());
                        Some(Immediate::Word(word))
                    }
                    0b011 => {
                        // Long operand
                        let long = self.memory.read_long(start + 2)?;
                        bytes.extend(long.to_be_bytes());
                        Some(Immediate::Long(long))
                    }
                    0b100 => {
                        // No operand
                        None
                    }
                    _ => bail!("Invalid TRAPcc operand size: {size_code}"),
                };
                InstructionKind::Trapcc { condition, operand }
            }
            InstructionKind::Bkpt { vector } => InstructionKind::Bkpt { vector },
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
            InstructionKind::Moveq { .. } => instr_kind,
            InstructionKind::Scc { condition, mode } => {
                let mode = self.resolve_ea(mode, start + 2, Some(Size::Byte))?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Scc { condition, mode }
            }
            InstructionKind::DBcc {
                condition,
                data_reg,
                ..
            } => {
                let disp_word = self.memory.read_word(start + 2)?;
                bytes.extend(disp_word.to_be_bytes());
                InstructionKind::DBcc {
                    condition,
                    data_reg,
                    displacement: disp_word as i16,
                }
            }
            InstructionKind::Bra { displacement } => {
                let displacement =
                    self.resolve_branch_displacement(displacement, start, &mut bytes)?;
                InstructionKind::Bra { displacement }
            }
            InstructionKind::Bsr { displacement } => {
                let displacement =
                    self.resolve_branch_displacement(displacement, start, &mut bytes)?;
                InstructionKind::Bsr { displacement }
            }
            InstructionKind::Bcc {
                condition,
                displacement,
            } => {
                let displacement =
                    self.resolve_branch_displacement(displacement, start, &mut bytes)?;
                InstructionKind::Bcc {
                    condition,
                    displacement,
                }
            }
            InstructionKind::Divu { src, dst } => {
                let src = self.resolve_ea(src, start + 2, Some(Size::Word))?;
                bytes.extend(src.to_bytes());
                InstructionKind::Divu { src, dst }
            }
            InstructionKind::Divs { src, dst } => {
                let src = self.resolve_ea(src, start + 2, Some(Size::Word))?;
                bytes.extend(src.to_bytes());
                InstructionKind::Divs { src, dst }
            }
            InstructionKind::Sbcd(_) => instr_kind,
            InstructionKind::Or(or) => match or {
                Or::EaToDn(EaToDn { size, dst, src }) => {
                    let src = self.resolve_ea(src, start + 2, Some(size))?;
                    bytes.extend(src.to_bytes());
                    InstructionKind::Or(Or::EaToDn(EaToDn { size, src, dst }))
                }
                Or::DnToEa(DnToEa { size, src, dst }) => {
                    let dst = self.resolve_ea(dst, start + 2, Some(size))?;
                    bytes.extend(dst.to_bytes());
                    InstructionKind::Or(Or::DnToEa(DnToEa { size, src, dst }))
                }
            },
            InstructionKind::Cmp(EaToDn { size, dst, src }) => {
                let src = self.resolve_ea(src, start + 2, Some(size))?;
                bytes.extend(src.to_bytes());
                InstructionKind::Cmp(EaToDn { size, src, dst })
            }
            InstructionKind::Cmpa {
                addr_reg,
                size,
                src,
            } => {
                let src = self.resolve_ea(src, start + 2, Some(size))?;
                bytes.extend(src.to_bytes());
                InstructionKind::Cmpa {
                    addr_reg,
                    size,
                    src,
                }
            }
            InstructionKind::Cmpm { .. } => instr_kind,
            InstructionKind::Eor(DnToEa { size, src, dst }) => {
                let dst = self.resolve_ea(dst, start + 2, Some(size))?;
                bytes.extend(dst.to_bytes());
                InstructionKind::Eor(DnToEa { size, src, dst })
            }
            InstructionKind::Mulu { src, dst } => {
                let src = self.resolve_ea(src, start + 2, Some(Size::Word))?;
                bytes.extend(src.to_bytes());
                InstructionKind::Mulu { src, dst }
            }
            InstructionKind::Muls { src, dst } => {
                let src = self.resolve_ea(src, start + 2, Some(Size::Word))?;
                bytes.extend(src.to_bytes());
                InstructionKind::Muls { src, dst }
            }
            InstructionKind::Abcd(_) => instr_kind,
            InstructionKind::Exg(_) => instr_kind,
            InstructionKind::And(and) => match and {
                And::EaToDn(EaToDn { size, dst, src }) => {
                    let src = self.resolve_ea(src, start + 2, Some(size))?;
                    bytes.extend(src.to_bytes());
                    InstructionKind::And(And::EaToDn(EaToDn { size, src, dst }))
                }
                And::DnToEa(DnToEa { size, src, dst }) => {
                    let dst = self.resolve_ea(dst, start + 2, Some(size))?;
                    bytes.extend(dst.to_bytes());
                    InstructionKind::And(And::DnToEa(DnToEa { size, src, dst }))
                }
            },
            InstructionKind::Suba {
                addr_reg,
                size,
                mode,
            } => {
                let mode = self.resolve_ea(mode, start + 2, Some(size))?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Suba {
                    addr_reg,
                    size,
                    mode,
                }
            }
            InstructionKind::Sub(sub) => match sub {
                Sub::EaToDn(EaToDn { size, dst, src }) => {
                    let src = self.resolve_ea(src, start + 2, Some(size))?;
                    bytes.extend(src.to_bytes());
                    InstructionKind::Sub(Sub::EaToDn(EaToDn { size, src, dst }))
                }
                Sub::DnToEa(DnToEa { size, src, dst }) => {
                    let dst = self.resolve_ea(dst, start + 2, Some(size))?;
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
            InstructionKind::Lea { src, dst } => {
                let src = self.resolve_ea(src, start + 2, None)?;
                bytes.extend(src.to_bytes());
                InstructionKind::Lea { src, dst }
            }
            InstructionKind::Chk {
                size,
                src,
                data_reg,
            } => {
                let src = self.resolve_ea(src, start + 2, Some(size))?;
                bytes.extend(src.to_bytes());
                InstructionKind::Chk {
                    size,
                    src,
                    data_reg,
                }
            }
            InstructionKind::Movem(Movem {
                size,
                direction,
                mode,
                ..
            }) => {
                let register_mask = self.memory.read_word(start + 2)?;
                bytes.extend(register_mask.to_be_bytes());
                let mode = self.resolve_ea(mode, start + 4, None)?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Movem(Movem {
                    size,
                    direction,
                    register_mask,
                    mode,
                })
            }
            InstructionKind::Cas { size, mode, .. } => {
                // Extension word: 0000 000D DD00 0ddd
                // DDD (bits 8-6) = Du (update register)
                // ddd (bits 2-0) = Dc (compare register)
                let ext_word = self.memory.read_word(start + 2)?;
                bytes.extend(ext_word.to_be_bytes());
                let du = DataReg::from_bits(((ext_word >> 6) & 0x7) as u8)?;
                let dc = DataReg::from_bits((ext_word & 0x7) as u8)?;
                let mode = self.resolve_ea(mode, start + 4, Some(size))?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Cas { size, dc, du, mode }
            }
            InstructionKind::Cas2 { size, .. } => {
                // CAS2 has two extension words
                // Extension word 1: 1RRR 0000 00DD D0dd d
                //   Bit 15: 1 (Rn1 is address register)
                //   Bits 14-12: Rn1 address register number
                //   Bit 11: 0
                //   Bits 10-6: 00000
                //   Bits 5-3: Du1 data register number
                //   Bit 2: 0
                //   Bits 2-0: Dc1 data register number
                // Extension word 2: same format for Rn2, Du2, Dc2
                let ext1 = self.memory.read_word(start + 2)?;
                let ext2 = self.memory.read_word(start + 4)?;
                bytes.extend(ext1.to_be_bytes());
                bytes.extend(ext2.to_be_bytes());

                let rn1 = AddrReg::from_bits(((ext1 >> 12) & 0x7) as u8)?;
                let du1 = DataReg::from_bits(((ext1 >> 3) & 0x7) as u8)?;
                let dc1 = DataReg::from_bits((ext1 & 0x7) as u8)?;

                let rn2 = AddrReg::from_bits(((ext2 >> 12) & 0x7) as u8)?;
                let du2 = DataReg::from_bits(((ext2 >> 3) & 0x7) as u8)?;
                let dc2 = DataReg::from_bits((ext2 & 0x7) as u8)?;

                InstructionKind::Cas2 {
                    size,
                    dc1,
                    dc2,
                    du1,
                    du2,
                    rn1,
                    rn2,
                }
            }
            InstructionKind::Cmp2 { size, mode, .. } | InstructionKind::Chk2 { size, mode, .. } => {
                // Extension word: DAAA 1C00 0000 0000
                //   Bit 15: D/A (0=data register, 1=address register)
                //   Bits 14-12: Register number
                //   Bit 11: CHK2/CMP2 selector (1=CHK2, 0=CMP2)
                //   Other bits: reserved
                let ext_word = self.memory.read_word(start + 2)?;
                bytes.extend(ext_word.to_be_bytes());

                let is_address = (ext_word & 0x8000) != 0;
                let reg_num = ((ext_word >> 12) & 0x7) as u8;
                let is_chk2 = (ext_word & 0x0800) != 0;

                let reg = if is_address {
                    Register::Address(AddrReg::from_bits(reg_num)?)
                } else {
                    Register::Data(DataReg::from_bits(reg_num)?)
                };

                let mode = self.resolve_ea(mode, start + 4, Some(size))?;
                bytes.extend(mode.to_bytes());

                if is_chk2 {
                    InstructionKind::Chk2 { size, mode, reg }
                } else {
                    InstructionKind::Cmp2 { size, mode, reg }
                }
            }
            InstructionKind::Bftst { mode, .. } => {
                // Extension word for bit field instructions:
                // Bit 11 (Do): 0=offset is immediate, 1=offset in register
                // Bits 10-6: offset value (if Do=0) or bits 8-6 are register (if Do=1)
                // Bit 5 (Dw): 0=width is immediate, 1=width in register
                // Bits 4-0: width value (if Dw=0) or bits 2-0 are register (if Dw=1)
                // Note: width of 0 means 32 bits
                let ext_word = self.memory.read_word(start + 2)?;
                bytes.extend(ext_word.to_be_bytes());
                let offset = if (ext_word & 0x0800) != 0 {
                    // Offset from register
                    BitFieldParam::Register(DataReg::from_bits(((ext_word >> 6) & 0x7) as u8)?)
                } else {
                    // Immediate offset
                    BitFieldParam::Immediate(((ext_word >> 6) & 0x1f) as u8)
                };
                let width = if (ext_word & 0x0020) != 0 {
                    // Width from register
                    BitFieldParam::Register(DataReg::from_bits((ext_word & 0x7) as u8)?)
                } else {
                    // Immediate width (0 means 32)
                    BitFieldParam::Immediate((ext_word & 0x1f) as u8)
                };
                let mode = self.resolve_ea(mode, start + 4, None)?;
                bytes.extend(mode.to_bytes());
                InstructionKind::Bftst {
                    mode,
                    offset,
                    width,
                }
            }
            InstructionKind::Bfextu { src, .. } => {
                // Extension word for BFEXTU:
                // Bits 14-12: Destination register Dn
                // Bit 11 (Do): 0=offset is immediate, 1=offset in register
                // Bits 10-6: offset value (if Do=0) or bits 8-6 are register (if Do=1)
                // Bit 5 (Dw): 0=width is immediate, 1=width in register
                // Bits 4-0: width value (if Dw=0) or bits 2-0 are register (if Dw=1)
                let ext_word = self.memory.read_word(start + 2)?;
                bytes.extend(ext_word.to_be_bytes());
                let dst = DataReg::from_bits(((ext_word >> 12) & 0x7) as u8)?;
                let offset = if (ext_word & 0x0800) != 0 {
                    BitFieldParam::Register(DataReg::from_bits(((ext_word >> 6) & 0x7) as u8)?)
                } else {
                    BitFieldParam::Immediate(((ext_word >> 6) & 0x1f) as u8)
                };
                let width = if (ext_word & 0x0020) != 0 {
                    BitFieldParam::Register(DataReg::from_bits((ext_word & 0x7) as u8)?)
                } else {
                    BitFieldParam::Immediate((ext_word & 0x1f) as u8)
                };
                let src = self.resolve_ea(src, start + 4, None)?;
                bytes.extend(src.to_bytes());
                InstructionKind::Bfextu {
                    src,
                    dst,
                    offset,
                    width,
                }
            }
            InstructionKind::Bfexts { src, .. } => {
                // Extension word for BFEXTS (same format as BFEXTU):
                // Bits 14-12: Destination register Dn
                // Bit 11 (Do): 0=offset is immediate, 1=offset in register
                // Bits 10-6: offset value (if Do=0) or bits 8-6 are register (if Do=1)
                // Bit 5 (Dw): 0=width is immediate, 1=width in register
                // Bits 4-0: width value (if Dw=0) or bits 2-0 are register (if Dw=1)
                let ext_word = self.memory.read_word(start + 2)?;
                bytes.extend(ext_word.to_be_bytes());
                let dst = DataReg::from_bits(((ext_word >> 12) & 0x7) as u8)?;
                let offset = if (ext_word & 0x0800) != 0 {
                    BitFieldParam::Register(DataReg::from_bits(((ext_word >> 6) & 0x7) as u8)?)
                } else {
                    BitFieldParam::Immediate(((ext_word >> 6) & 0x1f) as u8)
                };
                let width = if (ext_word & 0x0020) != 0 {
                    BitFieldParam::Register(DataReg::from_bits((ext_word & 0x7) as u8)?)
                } else {
                    BitFieldParam::Immediate((ext_word & 0x1f) as u8)
                };
                let src = self.resolve_ea(src, start + 4, None)?;
                bytes.extend(src.to_bytes());
                InstructionKind::Bfexts {
                    src,
                    dst,
                    offset,
                    width,
                }
            }
            InstructionKind::Bfchg { mode, .. }
            | InstructionKind::Bfclr { mode, .. }
            | InstructionKind::Bfset { mode, .. } => {
                // Extension word format like BFTST (no data reg fields)
                let ext_word = self.memory.read_word(start + 2)?;
                bytes.extend(ext_word.to_be_bytes());
                let offset = if (ext_word & 0x0800) != 0 {
                    BitFieldParam::Register(DataReg::from_bits(((ext_word >> 6) & 0x7) as u8)?)
                } else {
                    BitFieldParam::Immediate(((ext_word >> 6) & 0x1f) as u8)
                };
                let width = if (ext_word & 0x0020) != 0 {
                    BitFieldParam::Register(DataReg::from_bits((ext_word & 0x7) as u8)?)
                } else {
                    BitFieldParam::Immediate((ext_word & 0x1f) as u8)
                };
                let mode = self.resolve_ea(mode, start + 4, None)?;
                bytes.extend(mode.to_bytes());
                match instr_kind {
                    InstructionKind::Bfchg { .. } => InstructionKind::Bfchg {
                        mode,
                        offset,
                        width,
                    },
                    InstructionKind::Bfclr { .. } => InstructionKind::Bfclr {
                        mode,
                        offset,
                        width,
                    },
                    InstructionKind::Bfset { .. } => InstructionKind::Bfset {
                        mode,
                        offset,
                        width,
                    },
                    _ => unreachable!(),
                }
            }
            InstructionKind::Bfffo { src, .. } => {
                // Extension word for BFFFO (same format as BFEXTU):
                let ext_word = self.memory.read_word(start + 2)?;
                bytes.extend(ext_word.to_be_bytes());
                let dst = DataReg::from_bits(((ext_word >> 12) & 0x7) as u8)?;
                let offset = if (ext_word & 0x0800) != 0 {
                    BitFieldParam::Register(DataReg::from_bits(((ext_word >> 6) & 0x7) as u8)?)
                } else {
                    BitFieldParam::Immediate(((ext_word >> 6) & 0x1f) as u8)
                };
                let width = if (ext_word & 0x0020) != 0 {
                    BitFieldParam::Register(DataReg::from_bits((ext_word & 0x7) as u8)?)
                } else {
                    BitFieldParam::Immediate((ext_word & 0x1f) as u8)
                };
                let src = self.resolve_ea(src, start + 4, None)?;
                bytes.extend(src.to_bytes());
                InstructionKind::Bfffo {
                    src,
                    dst,
                    offset,
                    width,
                }
            }
            InstructionKind::Bfins { dst, .. } => {
                // Extension word for BFINS (same format as BFEXTU/BFEXTS):
                // Bits 14-12: Source register Dn (contains value to insert)
                // Bit 11 (Do): 0=offset is immediate, 1=offset in register
                // Bits 10-6: offset value (if Do=0) or bits 8-6 are register (if Do=1)
                // Bit 5 (Dw): 0=width is immediate, 1=width in register
                // Bits 4-0: width value (if Dw=0) or bits 2-0 are register (if Dw=1)
                let ext_word = self.memory.read_word(start + 2)?;
                bytes.extend(ext_word.to_be_bytes());
                let src = DataReg::from_bits(((ext_word >> 12) & 0x7) as u8)?;
                let offset = if (ext_word & 0x0800) != 0 {
                    BitFieldParam::Register(DataReg::from_bits(((ext_word >> 6) & 0x7) as u8)?)
                } else {
                    BitFieldParam::Immediate(((ext_word >> 6) & 0x1f) as u8)
                };
                let width = if (ext_word & 0x0020) != 0 {
                    BitFieldParam::Register(DataReg::from_bits((ext_word & 0x7) as u8)?)
                } else {
                    BitFieldParam::Immediate((ext_word & 0x1f) as u8)
                };
                let dst = self.resolve_ea(dst, start + 4, None)?;
                bytes.extend(dst.to_bytes());
                InstructionKind::Bfins {
                    src,
                    dst,
                    offset,
                    width,
                }
            }
            InstructionKind::MuluL { src, .. } | InstructionKind::MulsL { src, .. } => {
                // Extension word: 0hhh hs0f 0000 0lll
                // hhh (bits 14-12) = Dh register (high result, if 64-bit)
                // s (bit 11): 0=MULU.L, 1=MULS.L
                // f (bit 10): 0=32-bit result, 1=64-bit result
                // lll (bits 14-12) = Dl register (low result)
                let ext_word = self.memory.read_word(start + 2)?;
                bytes.extend(ext_word.to_be_bytes());
                let is_signed = (ext_word & 0x0800) != 0;
                let is_64bit = (ext_word & 0x0400) != 0;
                let dest_low = DataReg::from_bits(((ext_word >> 12) & 0x7) as u8)?;
                let dest_high = if is_64bit {
                    Some(DataReg::from_bits((ext_word & 0x7) as u8)?)
                } else {
                    None
                };
                let src = self.resolve_ea(src, start + 4, Some(Size::Long))?;
                bytes.extend(src.to_bytes());
                if is_signed {
                    InstructionKind::MulsL {
                        src,
                        dl: dest_low,
                        dh: dest_high,
                    }
                } else {
                    InstructionKind::MuluL {
                        src,
                        dl: dest_low,
                        dh: dest_high,
                    }
                }
            }
            InstructionKind::DivuL { src, .. } | InstructionKind::DivsL { src, .. } => {
                // Extension word: 0qqq qs0f 0000 0rrr
                // qqq (bits 14-12) = Dq register (quotient)
                // s (bit 11): 0=DIVU.L, 1=DIVS.L
                // f (bit 10): 0=32÷32, 1=64÷32
                // rrr (bits 2-0) = Dr register (remainder)
                let ext_word = self.memory.read_word(start + 2)?;
                bytes.extend(ext_word.to_be_bytes());
                let is_signed = (ext_word & 0x0800) != 0;
                let is_64bit = (ext_word & 0x0400) != 0;
                let dq = DataReg::from_bits(((ext_word >> 12) & 0x7) as u8)?;
                let dr = DataReg::from_bits((ext_word & 0x7) as u8)?;
                let src = self.resolve_ea(src, start + 4, Some(Size::Long))?;
                bytes.extend(src.to_bytes());
                if is_signed {
                    InstructionKind::DivsL {
                        src,
                        dq,
                        dr,
                        is_64bit,
                    }
                } else {
                    InstructionKind::DivuL {
                        src,
                        dq,
                        dr,
                        is_64bit,
                    }
                }
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
            0x4E74 => {
                // RTD #<displacement> (68010+)
                return Ok(InstructionKind::Rtd { displacement: 0 });
            }
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

                // BKPT: 0100 1000 0100 1nnn (68010+)
                if op_nibble == 0b1000 && mid == 0b0100 && ea_mode == 0b001 {
                    let vector = bit_range(opcode, 0, 3);
                    return Ok(InstructionKind::Bkpt { vector });
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
                if top_reg == 0b111 && matches!(opmode, 0b010 | 0b011) {
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
                // Tas/Tst (but not LEA which has opmode=111)
                if top_reg == 0b101 && opmode != 0b111 {
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
                    return Ok(InstructionKind::MoveUsp {
                        addr_reg,
                        direction,
                    });
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
                // LEA: 0100 aaa 111 eeeeee
                if opmode == 0b111 {
                    let dst = AddrReg::from_bits(top_reg)?;
                    let src = effective_address(ea_bits)?;
                    return Ok(InstructionKind::Lea { src, dst });
                }
                // CHK: 0100 ddd ss0 eeeeee (ss: 11=word, 10=long)
                if opmode == 0b110 || opmode == 0b100 {
                    let data_reg = DataReg::from_bits(top_reg)?;
                    let size = match opmode {
                        0b110 => Size::Word,
                        0b100 => Size::Long,
                        _ => unreachable!(),
                    };
                    let src = effective_address(ea_bits)?;
                    return Ok(InstructionKind::Chk {
                        size,
                        src,
                        data_reg,
                    });
                }
                // MULU.L/MULS.L: 0100 1100 00 mmmrrr (68020+)
                // DIVU.L/DIVS.L: 0100 1100 01 mmmrrr (68020+)
                if op_nibble == 0b1100 && size_bits <= 0b01 {
                    let mode = effective_address(ea_bits)?;
                    // Extension word and register selection resolved later
                    if size_bits == 0b00 {
                        // MULU.L/MULS.L
                        return Ok(InstructionKind::MuluL {
                            src: mode,
                            dl: DataReg::D0, // placeholder
                            dh: None,        // placeholder
                        });
                    } else {
                        // DIVU.L/DIVS.L
                        return Ok(InstructionKind::DivuL {
                            src: mode,
                            dq: DataReg::D0, // placeholder
                            dr: DataReg::D0, // placeholder
                            is_64bit: false, // placeholder
                        });
                    }
                }
                // MOVEM: 0100 1d00 1s eeeeee
                // d=0 (reg to mem): op_nibble=1000, d=1 (mem to reg): op_nibble=1100
                if matches!(op_nibble, 0b1000 | 0b1100) && size_bits >= 0b10 {
                    let direction = DataDir::from_bit(bit_range(opcode, 10, 11))?;
                    let size = Size::from_wl_bit(six_seven)?;
                    let mode = effective_address(ea_bits)?;
                    return Ok(InstructionKind::Movem(Movem {
                        size,
                        direction,
                        register_mask: 0, // resolved later
                        mode,
                    }));
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
                    // Check for bit field instructions (68020+)
                    // BFTST:  1110 100 011 eeeeee (top_reg=100, bit8=0)
                    // BFCHG:  1110 101 011 eeeeee (top_reg=101, bit8=0)
                    // BFCLR:  1110 110 011 eeeeee (top_reg=110, bit8=0)
                    // BFSET:  1110 111 011 eeeeee (top_reg=111, bit8=0)
                    // BFEXTU: 1110 100 111 eeeeee (top_reg=100, bit8=1)
                    // BFEXTS: 1110 101 111 eeeeee (top_reg=101, bit8=1)
                    // BFFFO:  1110 110 111 eeeeee (top_reg=110, bit8=1)
                    // BFINS:  1110 111 111 eeeeee (top_reg=111, bit8=1)
                    if top_reg >= 0b100 {
                        let mode = effective_address(ea_bits)?;
                        // Placeholders - extension word resolved later
                        let offset = BitFieldParam::Immediate(0);
                        let width = BitFieldParam::Immediate(0);
                        // dst placeholder for BFEXTU - resolved from extension word later
                        let dst = DataReg::D0;
                        return match (top_reg, eight_nine) {
                            (0b100, 0) => Ok(InstructionKind::Bftst {
                                mode,
                                offset,
                                width,
                            }),
                            (0b101, 0) => Ok(InstructionKind::Bfchg {
                                mode,
                                offset,
                                width,
                            }),
                            (0b110, 0) => Ok(InstructionKind::Bfclr {
                                mode,
                                offset,
                                width,
                            }),
                            (0b111, 0) => Ok(InstructionKind::Bfset {
                                mode,
                                offset,
                                width,
                            }),
                            (0b100, 1) => Ok(InstructionKind::Bfextu {
                                src: mode,
                                dst,
                                offset,
                                width,
                            }),
                            (0b101, 1) => Ok(InstructionKind::Bfexts {
                                src: mode,
                                dst,
                                offset,
                                width,
                            }),
                            (0b110, 1) => Ok(InstructionKind::Bfffo {
                                src: mode,
                                dst,
                                offset,
                                width,
                            }),
                            (0b111, 1) => Ok(InstructionKind::Bfins {
                                src: dst,  // src is actually a DataReg, reusing dst placeholder
                                dst: mode,
                                offset,
                                width,
                            }),
                            _ => bail!(
                                "Unsupported bit field instruction: top_reg={:#05b}, bit8={}",
                                top_reg,
                                eight_nine
                            ),
                        };
                    }
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
                        size: Size::from_size_bits(size_bits)?,
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
                // CHK2/CMP2: 0000 0ss0 11mm mrrr (68020+)
                // op_nibble = 0ss0, size_bits = 11
                // Must check BEFORE immediate operations since 0ss0 can match ADDI pattern
                // Differentiated from CAS by having op_nibble bit 3 = 0
                if (op_nibble & 0b1001) == 0b0000 && size_bits == 0b11 && op_nibble != 0 {
                    let size = match (op_nibble >> 1) & 0b11 {
                        0b00 => Size::Byte,
                        0b01 => Size::Word,
                        0b10 => Size::Long,
                        _ => bail!("Invalid CHK2/CMP2 size"),
                    };
                    let mode = effective_address(ea_bits)?;
                    // CHK2 vs CMP2 and register determined from extension word
                    // Use CMP2 as placeholder, will be resolved in decode_instruction
                    return Ok(InstructionKind::Cmp2 {
                        size,
                        mode,
                        reg: Register::Data(DataReg::D0), // placeholder
                    });
                }
                // Ori/Andi/Subi/Addi/Eori/Cmpi #imm, <ea>
                // 0000 oooo ss eeeeee (oooo: 0000=ORI, 0010=ANDI, 0100=SUBI, 0110=ADDI, 1010=EORI, 1100=CMPI)
                if matches!(
                    op_nibble,
                    0b0000 | 0b0010 | 0b0100 | 0b0110 | 0b1010 | 0b1100
                ) {
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
                // CAS2: 0000 1ss0 11111100 (68020+)
                // ea_bits = 111100 = 0x3C
                if (op_nibble & 0b1001) == 0b1000 && size_bits == 0b11 && ea_bits == 0x3C {
                    let cas_size = match (op_nibble >> 1) & 0b11 {
                        0b01 => Size::Byte,
                        0b10 => Size::Word,
                        0b11 => Size::Long,
                        _ => bail!("Invalid CAS2 size"),
                    };
                    // All registers read from extension words during resolve
                    return Ok(InstructionKind::Cas2 {
                        size: cas_size,
                        dc1: DataReg::D0, // placeholder
                        dc2: DataReg::D0, // placeholder
                        du1: DataReg::D0, // placeholder
                        du2: DataReg::D0, // placeholder
                        rn1: AddrReg::A0, // placeholder
                        rn2: AddrReg::A0, // placeholder
                    });
                }
                // CAS: 0000 1ss0 11mm mrrr (68020+)
                // op_nibble = 1ss0, size_bits = 11
                if (op_nibble & 0b1001) == 0b1000 && size_bits == 0b11 {
                    let cas_size = match (op_nibble >> 1) & 0b11 {
                        0b01 => Size::Byte,
                        0b10 => Size::Word,
                        0b11 => Size::Long,
                        _ => bail!("Invalid CAS size"),
                    };
                    let mode = effective_address(ea_bits)?;
                    // dc and du registers read from extension word during resolve
                    return Ok(InstructionKind::Cas {
                        size: cas_size,
                        dc: DataReg::D0, // placeholder
                        du: DataReg::D0, // placeholder
                        mode,
                    });
                }
                bail!("Unsupported group 0 instruction");
            }
            0b0101 => {
                // Scc/DBcc/TRAPcc when size_bits == 11
                if size_bits == 0b11 {
                    let condition = Condition::from(op_nibble);
                    // TRAPcc: 0101 cccc 1111 1sss (68020+)
                    if ea_mode == 0b111 {
                        // ea_reg encodes the operand size
                        // 010 = word, 011 = long, 100 = no operand
                        return Ok(InstructionKind::Trapcc {
                            condition,
                            operand: None, // resolved later
                        });
                    }
                    // DBcc: 0101 cccc 11 001 rrr
                    if ea_mode == 0b001 {
                        let data_reg = DataReg::from_bits(ea_reg)?;
                        return Ok(InstructionKind::DBcc {
                            condition,
                            data_reg,
                            displacement: 0, // resolved later
                        });
                    }
                    // Scc: 0101 cccc 11 eeeeee
                    let mode = effective_address(ea_bits)?;
                    return Ok(InstructionKind::Scc { condition, mode });
                }
                // Addq/Subq: 0101 ddd ss eeeeee (ss != 11)
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
            // OR/DIVU/DIVS/SBCD: 1000 rrr ooo eeeeee
            0b1000 => {
                match opmode {
                    // OR <ea>,Dn: opmode 000-010
                    0b000..=0b010 => Ok(InstructionKind::Or(Or::EaToDn(EaToDn {
                        size: Size::from_size_bits(size_bits)?,
                        dst: DataReg::from_bits(top_reg)?,
                        src: effective_address(ea_bits)?,
                    }))),
                    // DIVU <ea>,Dn: opmode 011
                    0b011 => Ok(InstructionKind::Divu {
                        src: effective_address(ea_bits)?,
                        dst: DataReg::from_bits(top_reg)?,
                    }),
                    // SBCD/OR Dn,<ea>: opmode 100
                    0b100 => match ea_mode {
                        // SBCD Dy,Dx
                        0b000 => Ok(InstructionKind::Sbcd(Sbcd::Dn {
                            src: DataReg::from_bits(ea_reg)?,
                            dst: DataReg::from_bits(top_reg)?,
                        })),
                        // SBCD -(Ay),-(Ax)
                        0b001 => Ok(InstructionKind::Sbcd(Sbcd::PreDec {
                            src: AddrReg::from_bits(ea_reg)?,
                            dst: AddrReg::from_bits(top_reg)?,
                        })),
                        // OR Dn,<ea>
                        _ => Ok(InstructionKind::Or(Or::DnToEa(DnToEa {
                            size: Size::Byte,
                            src: DataReg::from_bits(top_reg)?,
                            dst: effective_address(ea_bits)?,
                        }))),
                    },
                    // OR Dn,<ea>: opmode 101-110
                    0b101 | 0b110 => Ok(InstructionKind::Or(Or::DnToEa(DnToEa {
                        size: Size::from_size_bits(size_bits)?,
                        src: DataReg::from_bits(top_reg)?,
                        dst: effective_address(ea_bits)?,
                    }))),
                    // DIVS <ea>,Dn: opmode 111
                    0b111 => Ok(InstructionKind::Divs {
                        src: effective_address(ea_bits)?,
                        dst: DataReg::from_bits(top_reg)?,
                    }),
                    _ => bail!("Unsupported group 8 opmode: {:#05b}", opmode),
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
            // CMP/CMPA/CMPM/EOR: 1011 rrr ooo eeeeee
            0b1011 => {
                match opmode {
                    // CMP <ea>,Dn: opmode 000-010
                    0b000..=0b010 => Ok(InstructionKind::Cmp(EaToDn {
                        size: Size::from_size_bits(size_bits)?,
                        dst: DataReg::from_bits(top_reg)?,
                        src: effective_address(ea_bits)?,
                    })),
                    // CMPA <ea>,An: opmode 011 (word) or 111 (long)
                    0b011 | 0b111 => Ok(InstructionKind::Cmpa {
                        addr_reg: AddrReg::from_bits(top_reg)?,
                        size: Size::from_wl_bit(eight_nine)?,
                        src: effective_address(ea_bits)?,
                    }),
                    // CMPM/EOR: opmode 100-110
                    0b100..=0b110 => match ea_mode {
                        // CMPM (Ay)+,(Ax)+
                        0b001 => Ok(InstructionKind::Cmpm {
                            size: Size::from_size_bits(size_bits)?,
                            src: AddrReg::from_bits(ea_reg)?,
                            dst: AddrReg::from_bits(top_reg)?,
                        }),
                        // EOR Dn,<ea>
                        _ => Ok(InstructionKind::Eor(DnToEa {
                            size: Size::from_size_bits(size_bits)?,
                            src: DataReg::from_bits(top_reg)?,
                            dst: effective_address(ea_bits)?,
                        })),
                    },
                    _ => bail!("Unsupported group 11 opmode: {:#05b}", opmode),
                }
            }
            // AND/MULU/MULS/ABCD/EXG: 1100 rrr ooo eeeeee
            0b1100 => {
                match opmode {
                    // AND <ea>,Dn: opmode 000-010
                    0b000..=0b010 => Ok(InstructionKind::And(And::EaToDn(EaToDn {
                        size: Size::from_size_bits(size_bits)?,
                        dst: DataReg::from_bits(top_reg)?,
                        src: effective_address(ea_bits)?,
                    }))),
                    // MULU <ea>,Dn: opmode 011
                    0b011 => Ok(InstructionKind::Mulu {
                        src: effective_address(ea_bits)?,
                        dst: DataReg::from_bits(top_reg)?,
                    }),
                    // ABCD/EXG/AND Dn,<ea>: opmode 100
                    0b100 => match ea_mode {
                        // ABCD Dy,Dx
                        0b000 => Ok(InstructionKind::Abcd(Abcd::Dn {
                            src: DataReg::from_bits(ea_reg)?,
                            dst: DataReg::from_bits(top_reg)?,
                        })),
                        // ABCD -(Ay),-(Ax)
                        0b001 => Ok(InstructionKind::Abcd(Abcd::PreDec {
                            src: AddrReg::from_bits(ea_reg)?,
                            dst: AddrReg::from_bits(top_reg)?,
                        })),
                        // AND Dn,<ea>
                        _ => Ok(InstructionKind::And(And::DnToEa(DnToEa {
                            size: Size::Byte,
                            src: DataReg::from_bits(top_reg)?,
                            dst: effective_address(ea_bits)?,
                        }))),
                    },
                    // EXG/AND Dn,<ea>: opmode 101
                    0b101 => {
                        // EXG Dx,Dy: 1100 xxx 101000 yyy
                        if ea_mode == 0b000 {
                            Ok(InstructionKind::Exg(Exg::DataData {
                                rx: DataReg::from_bits(top_reg)?,
                                ry: DataReg::from_bits(ea_reg)?,
                            }))
                        // EXG Ax,Ay: 1100 xxx 101001 yyy
                        } else if ea_mode == 0b001 {
                            Ok(InstructionKind::Exg(Exg::AddrAddr {
                                rx: AddrReg::from_bits(top_reg)?,
                                ry: AddrReg::from_bits(ea_reg)?,
                            }))
                        } else {
                            // AND Dn,<ea>
                            Ok(InstructionKind::And(And::DnToEa(DnToEa {
                                size: Size::Word,
                                src: DataReg::from_bits(top_reg)?,
                                dst: effective_address(ea_bits)?,
                            })))
                        }
                    }
                    // EXG/AND Dn,<ea>: opmode 110
                    0b110 => {
                        // EXG Dx,Ay: 1100 xxx 110001 yyy
                        if ea_mode == 0b001 {
                            Ok(InstructionKind::Exg(Exg::DataAddr {
                                data: DataReg::from_bits(top_reg)?,
                                addr: AddrReg::from_bits(ea_reg)?,
                            }))
                        } else {
                            // AND Dn,<ea>
                            Ok(InstructionKind::And(And::DnToEa(DnToEa {
                                size: Size::Long,
                                src: DataReg::from_bits(top_reg)?,
                                dst: effective_address(ea_bits)?,
                            })))
                        }
                    }
                    // MULS <ea>,Dn: opmode 111
                    0b111 => Ok(InstructionKind::Muls {
                        src: effective_address(ea_bits)?,
                        dst: DataReg::from_bits(top_reg)?,
                    }),
                    _ => bail!("Unsupported group 12 opmode: {:#05b}", opmode),
                }
            }
            // Bcc/BRA/BSR: 0110 cccc dddddddd
            0b0110 => {
                let condition = Condition::from(op_nibble);
                let disp_byte = (opcode & 0xFF) as i8 as i32;
                match condition {
                    Condition::True => {
                        // BRA
                        Ok(InstructionKind::Bra {
                            displacement: disp_byte, // resolved later if 0 or -1
                        })
                    }
                    Condition::False => {
                        // BSR
                        Ok(InstructionKind::Bsr {
                            displacement: disp_byte, // resolved later if 0 or -1
                        })
                    }
                    _ => {
                        // Bcc
                        Ok(InstructionKind::Bcc {
                            condition,
                            displacement: disp_byte, // resolved later if 0 or -1
                        })
                    }
                }
            }
            // MOVEQ: 0111 rrr 0 dddddddd
            0b0111 => {
                // Bit 8 must be 0 for MOVEQ
                if eight_nine == 0 {
                    let dst = DataReg::from_bits(top_reg)?;
                    let data = (opcode & 0xFF) as i8;
                    return Ok(InstructionKind::Moveq { data, dst });
                }
                bail!("Unsupported group 7 instruction");
            }
            // MOVE/MOVEA: 00ss ddd mmm sss nnn
            // size encoding: 01=byte, 11=word, 10=long (different from normal!)
            // ddd=dest reg (top_reg), mmm=dest mode (opmode), sss=src mode (ea_mode), nnn=src reg (ea_reg)
            0b0001..=0b0011 => {
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

    /// Get index extension data (for 68020+ full format or 68000 brief format)
    pub fn index_ext(&self) -> Option<(u16, i32)> {
        match self.data {
            Some(AddressModeData::IndexExt {
                ext_word,
                base_disp,
            }) => Some((ext_word, base_disp)),
            Some(AddressModeData::Short(ext_word)) => {
                // Brief format: 8-bit displacement in low byte
                let disp = (ext_word & 0xFF) as i8 as i32;
                Some((ext_word, disp))
            }
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

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataDir {
    RegToMem, // 0
    MemToReg, // 1
}

impl DataDir {
    pub fn from_bit(bit: u8) -> Result<Self> {
        match bit {
            0 => Ok(DataDir::RegToMem),
            1 => Ok(DataDir::MemToReg),
            _ => bail!("Invalid DataDir bit: {bit}"),
        }
    }
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
