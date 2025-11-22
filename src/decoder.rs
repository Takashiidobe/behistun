use crate::memory::MemoryImage;
use anyhow::Result;
use anyhow::bail;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum InstructionKind {
    Reset,
    Nop,
    Illegal,
    Rte,
    Rts,
    Rtr,
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
}
// <ea>,Dn
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct EaToDn {
    size: Size,
    dst: DataReg,        // Dn
    src: AddressingMode, // <ea>
}
// Dn,<ea>
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct DnToEa {
    size: Size,
    src: DataReg,        // Dn
    dst: AddressingMode, // <ea>
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Add {
    EaToDn(EaToDn),
    DnToEa(DnToEa),
}
// Dy,Dx
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Dn {
    size: Size,
    src: DataReg, // Dy
    dst: DataReg, // Dx
}
// -(Ay),-(Ax)
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct PreDec {
    size: Size,
    src: AddrReg, // Ay
    dst: AddrReg, // Ax
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Addx {
    Dn(Dn),
    PreDec(PreDec),
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
    pub fn to_bytes(&self) -> Vec<u8> {
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
            InstructionKind::Jsr { mode } => {
                let mode = self.resolve_ea(mode, start + 2, None)?;
                if let Some(data) = mode.data {
                    bytes.extend(data.to_bytes());
                }
                InstructionKind::Jsr { mode }
            }
            InstructionKind::Jmp { mode } => {
                let mode = self.resolve_ea(mode, start + 2, None)?;
                if let Some(data) = mode.data {
                    bytes.extend(data.to_bytes());
                }
                InstructionKind::Jmp { mode }
            }
            InstructionKind::Adda {
                addr_reg,
                size,
                mode,
            } => {
                let mode = self.resolve_ea(mode, start + 2, None)?;
                if let Some(data) = mode.data {
                    bytes.extend(data.to_bytes());
                }
                InstructionKind::Adda {
                    addr_reg,
                    size,
                    mode,
                }
            }
            InstructionKind::Add(add) => match add {
                Add::EaToDn(EaToDn { size, dst, src }) => {
                    let src = self.resolve_ea(src, start + 2, None)?;
                    if let Some(data) = src.data {
                        bytes.extend(data.to_bytes());
                    }
                    InstructionKind::Add(Add::EaToDn(EaToDn { size, src, dst }))
                }
                Add::DnToEa(DnToEa { size, src, dst }) => {
                    let dst = self.resolve_ea(dst, start + 2, None)?;
                    if let Some(data) = dst.data {
                        bytes.extend(data.to_bytes());
                    }
                    InstructionKind::Add(Add::DnToEa(DnToEa { size, src, dst }))
                }
            },
            InstructionKind::Addx(_) => instr_kind,
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
        let nine_twelve = bit_range(opcode, 9, 12); // 9..12
        let eight_nine = bit_range(opcode, 8, 9); // 8..9
        let seven_nine = bit_range(opcode, 7, 9); // 7..9
        let opmode = bit_range(opcode, 6, 9); //  6..9
        let six_seven = bit_range(opcode, 6, 7); // 6..7
        let size_bits = bit_range(opcode, 6, 8); // 6..8
        let ea_bits = bit_range(opcode, 0, 6); // 0..6
        let ea_mode = bit_range(opcode, 3, 6); // 3..6
        let ea_reg = bit_range(opcode, 0, 3); // 0..3
        match group {
            0b0100 => {
                // Jsr/Jmp
                if nine_twelve == 0b111 && seven_nine == 0b01 {
                    // Jsr
                    if six_seven == 0b0 {
                        return Ok(InstructionKind::Jsr {
                            mode: effective_address(ea_bits)?,
                        });
                    } else {
                        // Jmp
                        return Ok(InstructionKind::Jmp {
                            mode: effective_address(ea_bits)?,
                        });
                    }
                }
                bail!("Unsupported");
            }
            0b1101 => {
                // Add/Addx/Adda
                match opmode {
                    // Adda <ea>,An
                    0b011 | 0b111 => Ok(InstructionKind::Adda {
                        addr_reg: AddrReg::from_bits(nine_twelve)?,
                        size: Size::from_wl_bit(eight_nine)?,
                        mode: effective_address(ea_bits)?,
                    }),
                    // Add <ea>, Dn
                    0b000..=0b010 => Ok(InstructionKind::Add(Add::EaToDn(EaToDn {
                        size: Size::from_wl_bit(eight_nine)?,
                        dst: DataReg::from_bits(nine_twelve)?,
                        src: effective_address(ea_bits)?,
                    }))),
                    0b100..=0b110 => match ea_mode {
                        // Addx Dn, Dn
                        0b000 => Ok(InstructionKind::Addx(Addx::Dn(Dn {
                            size: Size::from_size_bits(size_bits)?,
                            src: DataReg::from_bits(nine_twelve)?,
                            dst: DataReg::from_bits(ea_reg)?,
                        }))),
                        // Addx -(An), -(An)
                        0b001 => Ok(InstructionKind::Addx(Addx::PreDec(PreDec {
                            size: Size::from_size_bits(size_bits)?,
                            src: AddrReg::from_bits(ea_reg)?,
                            dst: AddrReg::from_bits(nine_twelve)?,
                        }))),
                        // Add Dn,<ea>
                        _ => Ok(InstructionKind::Add(Add::DnToEa(DnToEa {
                            size: Size::from_size_bits(size_bits)?,
                            src: DataReg::from_bits(nine_twelve)?,
                            dst: effective_address(ea_bits)?,
                        }))),
                    },
                    _ => bail!("Unsupported opmode: {:#05b}", opmode),
                }
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
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct AddressingMode {
    ea: EffectiveAddress,
    data: Option<AddressModeData>,
}

impl From<EffectiveAddress> for AddressingMode {
    fn from(value: EffectiveAddress) -> Self {
        Self {
            ea: value,
            data: None,
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

fn effective_address(bits: u8) -> Result<AddressingMode> {
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

#[allow(unused)]
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
            _ => bail!("Illegal size field in ADD/ADDX"),
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

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub enum RightOrLeft {
    Right, // R b0
    Left,  // L b1
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub enum Mode {
    DataReg,     // Dn  b0
    AddrPreDecr, // -(An) b1
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub enum Rotation {
    Immediate, // 0
    Register,  // 1
}
