use crate::memory::MemoryImage;
use anyhow::Result;
use anyhow::bail;

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum InstructionKind {
    Reset,
    Nop,
    Illegal,
    Rte,
    Rts,
    Rtr,
    Jsr(AddressingMode),
    Jmp(AddressingMode),
    TrapV,
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum InterpretKind {
    Reset,
    Nop,
    Illegal,
    Rte,
    Rts,
    Rtr,
    Jsr(AddressModeData),
    Jmp(AddressModeData),
    TrapV,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub address: usize,
    pub opcode: u16,
    pub bytes: Vec<u8>,
    pub kind: InterpretKind,
}

impl Instruction {
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
}

#[derive(Debug, Clone)]
pub enum AddressModeData {
    Short(u16),
    Long(u32),
    Imm(ImmediateValue),
    None,
}

#[derive(Debug, Clone, Copy)]
pub enum ImmediateValue {
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

    fn resolve_addressing_mode(
        &self,
        mode: AddressingMode,
        offset: usize,
        immediate_size: Option<Size>,
    ) -> Result<(AddressModeData, Vec<u8>)> {
        match mode {
            AddressingMode::Dr(_)
            | AddressingMode::Ar(_)
            | AddressingMode::Addr(_)
            | AddressingMode::AddrPostIncr(_)
            | AddressingMode::AddrPreDecr(_) => Ok((AddressModeData::None, vec![])),
            AddressingMode::AddrDisplace(_) | AddressingMode::PCDisplace => {
                let word = self.memory.read_word(offset)?;
                Ok((AddressModeData::Short(word), word.to_be_bytes().to_vec()))
            }
            AddressingMode::AddrIndex(_) | AddressingMode::PCIndex => {
                let word = self.memory.read_word(offset)?;
                Ok((AddressModeData::Short(word), word.to_be_bytes().to_vec()))
            }
            AddressingMode::Immediate => {
                let size = immediate_size.unwrap_or(Size::Word);
                match size {
                    Size::Byte => {
                        let byte = self.memory.read_byte(offset)?;
                        let bytes = byte.to_be_bytes().to_vec();
                        let value = ImmediateValue::Byte(byte);
                        Ok((AddressModeData::Imm(value), bytes))
                    }
                    Size::Word => {
                        let word = self.memory.read_word(offset)?;
                        let bytes = word.to_be_bytes().to_vec();
                        let value = ImmediateValue::Word(word);
                        Ok((AddressModeData::Imm(value), bytes))
                    }
                    Size::Long => {
                        let long = self.memory.read_long(offset)?;
                        let bytes = long.to_be_bytes().to_vec();
                        let value = ImmediateValue::Long(long);
                        Ok((AddressModeData::Imm(value), bytes))
                    }
                }
            }
            AddressingMode::AbsShort => {
                let word = self.memory.read_word(offset)?;
                Ok((AddressModeData::Short(word), word.to_be_bytes().to_vec()))
            }
            AddressingMode::AbsLong => {
                let long = self.memory.read_long(offset)?;
                Ok((AddressModeData::Long(long), long.to_be_bytes().to_vec()))
            }
        }
    }

    fn decode_instruction(&self, start: usize) -> Result<Instruction> {
        let opcode = self.memory.read_word(start)?;
        let instr_kind = Self::get_op_kind(opcode)?;
        let mut bytes = opcode.to_be_bytes().to_vec();
        let kind = match &instr_kind {
            InstructionKind::Jsr(mode) => {
                let (address_mode_data, extra_bytes) =
                    self.resolve_addressing_mode(mode.clone(), start + 2, None)?;
                bytes.extend(extra_bytes);
                InterpretKind::Jsr(address_mode_data)
            }
            InstructionKind::Jmp(mode) => {
                let (address_mode_data, extra_bytes) =
                    self.resolve_addressing_mode(mode.clone(), start + 2, None)?;
                bytes.extend(extra_bytes);
                InterpretKind::Jmp(address_mode_data)
            }
            InstructionKind::Reset => InterpretKind::Reset,
            InstructionKind::Nop => InterpretKind::Nop,
            InstructionKind::Illegal => InterpretKind::Illegal,
            InstructionKind::Rte => InterpretKind::Rte,
            InstructionKind::Rts => InterpretKind::Rts,
            InstructionKind::Rtr => InterpretKind::Rtr,
            InstructionKind::TrapV => InterpretKind::TrapV,
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

        let nibble = bit_range(opcode, 12, 16);
        let next_three = bit_range(opcode, 9, 12);
        let next_two = bit_range(opcode, 7, 9);
        let next_one = bit_range(opcode, 6, 7);
        let bottom = bit_range(opcode, 0, 6);
        match nibble {
            0b0100 => {
                // JSR/JMP
                if next_three == 0b111 && next_two == 0b01 {
                    if next_one == 0b0 {
                        return Ok(InstructionKind::Jsr(m_xn(bottom)));
                    } else {
                        return Ok(InstructionKind::Jmp(m_xn(bottom)));
                    }
                }
                bail!("Unsupported");
            }
            _ => bail!("Unsupported nibble: {:#06b}", nibble),
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

#[derive(Debug, PartialEq, Clone)]
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

impl From<u8> for DataReg {
    fn from(value: u8) -> Self {
        match value {
            0b000 => DataReg::D0,
            0b001 => DataReg::D1,
            0b010 => DataReg::D2,
            0b011 => DataReg::D3,
            0b100 => DataReg::D4,
            0b101 => DataReg::D5,
            0b110 => DataReg::D6,
            0b111 => DataReg::D7,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
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

impl From<u8> for AddrReg {
    fn from(value: u8) -> Self {
        match value {
            0b000 => AddrReg::A0,
            0b001 => AddrReg::A1,
            0b010 => AddrReg::A2,
            0b011 => AddrReg::A3,
            0b100 => AddrReg::A4,
            0b101 => AddrReg::A5,
            0b110 => AddrReg::A6,
            0b111 => AddrReg::A7,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AddressingMode {
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

fn m_xn(bits: u8) -> AddressingMode {
    let m = bit_range_u8(bits, 3, 6);
    let xn = bit_range_u8(bits, 0, 3);

    match (m, xn) {
        (0b000, _) => AddressingMode::Dr(DataReg::from(xn)),
        (0b001, _) => AddressingMode::Ar(AddrReg::from(xn)),
        (0b010, _) => AddressingMode::Addr(AddrReg::from(xn)),
        (0b011, _) => AddressingMode::AddrPostIncr(AddrReg::from(xn)),
        (0b100, _) => AddressingMode::AddrPreDecr(AddrReg::from(xn)),
        (0b101, _) => AddressingMode::AddrDisplace(AddrReg::from(xn)),
        (0b110, _) => AddressingMode::AddrIndex(AddrReg::from(xn)),
        (0b111, 0b010) => AddressingMode::PCDisplace,
        (0b111, 0b011) => AddressingMode::PCIndex,
        (0b111, 0b000) => AddressingMode::AbsShort,
        (0b111, 0b001) => AddressingMode::AbsLong,
        (0b111, 0b100) => AddressingMode::Immediate,
        _ => unimplemented!("m: {m:#05b}, xn: {xn:#05b}"),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Size {
    Byte, // .b b00 | / | b01
    Word, // .w b01 | 0 | b11
    Long, // .l b10 | 1 | b10
}

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

pub enum DataDir {
    RegToMem, // 0 1
    MemToReg, // 1 0
}

pub enum DnEa {
    DnEa, // Dn, Ea -> Dn 0
    EaDn, // Ea, Dn -> Ea 1
}

pub enum RightOrLeft {
    Right, // R b0
    Left,  // L b1
}

pub enum Mode {
    DataReg,     // Dn  b0
    AddrPreDecr, // -(An) b1
}

pub enum Rotation {
    Immediate, // 0
    Register,  // 1
}
