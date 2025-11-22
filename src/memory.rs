use std::{error::Error, fmt};

use goblin::elf::program_header;

#[derive(Debug)]
pub struct MemorySegment {
    pub vaddr: usize,
    pub data: Vec<u8>,
    pub flags: u32,
    pub align: usize,
}

impl MemorySegment {
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

#[derive(Debug)]
pub struct MemoryImage {
    segments: Vec<MemorySegment>,
}

impl MemoryImage {
    pub fn new(segments: Vec<MemorySegment>) -> Self {
        Self { segments }
    }

    pub fn segments(&self) -> &[MemorySegment] {
        &self.segments
    }

    pub fn fetch_instruction(&self, addr: usize, size: usize) -> Result<&[u8], MemoryError> {
        self.read_range(addr, size, program_header::PF_X, "execute")
    }

    pub fn read_data(&self, addr: usize, size: usize) -> Result<&[u8], MemoryError> {
        self.read_range(addr, size, program_header::PF_R, "read")
    }

    pub fn read_byte(&self, addr: usize) -> Result<u8, MemoryError> {
        let bytes: [u8; 1] = self
            .read_range(addr, 1, program_header::PF_R, "read")?
            .try_into()
            .unwrap();

        Ok(u8::from_be_bytes(bytes))
    }

    pub fn read_word(&self, addr: usize) -> Result<u16, MemoryError> {
        let bytes: [u8; 2] = self
            .read_range(addr, 2, program_header::PF_R, "read")?
            .try_into()
            .unwrap();

        Ok(u16::from_be_bytes(bytes))
    }

    pub fn read_long(&self, addr: usize) -> Result<u32, MemoryError> {
        let bytes: [u8; 4] = self
            .read_range(addr, 4, program_header::PF_R, "read")?
            .try_into()
            .unwrap();

        Ok(u32::from_be_bytes(bytes))
    }

    pub fn write_data(&mut self, addr: usize, data: &[u8]) -> Result<(), MemoryError> {
        let size = data.len();
        if size == 0 {
            return Ok(());
        }
        let end = addr
            .checked_add(size)
            .ok_or(MemoryError::AddressOverflow { addr, size })?;

        let segment = self
            .segment_containing_mut(addr, end)
            .ok_or(MemoryError::Unmapped { addr, size })?;

        if (segment.flags & program_header::PF_W) == 0 {
            return Err(MemoryError::AccessViolation {
                addr,
                access: "write",
            });
        }

        let offset = usize::try_from(addr - segment.vaddr)
            .map_err(|_| MemoryError::AddressNotRepresentable { addr })?;
        segment.data[offset..offset + size].copy_from_slice(data);
        Ok(())
    }

    fn read_range(
        &self,
        addr: usize,
        size: usize,
        required_flag: u32,
        access: &'static str,
    ) -> Result<&[u8], MemoryError> {
        let size_usize = size;
        let end = addr
            .checked_add(size_usize)
            .ok_or(MemoryError::AddressOverflow { addr, size })?;

        let segment = self
            .segment_containing(addr, end)
            .ok_or(MemoryError::Unmapped { addr, size })?;

        if required_flag != 0 && (segment.flags & required_flag) == 0 {
            return Err(MemoryError::AccessViolation { addr, access });
        }

        let offset = usize::try_from(addr - segment.vaddr)
            .map_err(|_| MemoryError::AddressNotRepresentable { addr })?;
        Ok(&segment.data[offset..offset + size])
    }

    fn segment_containing(&self, start: usize, end: usize) -> Option<&MemorySegment> {
        for segment in &self.segments {
            let seg_start = segment.vaddr;
            let seg_end = seg_start.checked_add(segment.data.len())?;
            if start >= seg_start && end <= seg_end {
                return Some(segment);
            }
        }
        None
    }

    fn segment_containing_mut(&mut self, start: usize, end: usize) -> Option<&mut MemorySegment> {
        for segment in &mut self.segments {
            let seg_start = segment.vaddr;
            let seg_end = seg_start.checked_add(segment.data.len())?;
            if start >= seg_start && end <= seg_end {
                return Some(segment);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub enum MemoryError {
    AddressOverflow { addr: usize, size: usize },
    AddressNotRepresentable { addr: usize },
    Unmapped { addr: usize, size: usize },
    AccessViolation { addr: usize, access: &'static str },
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::AddressOverflow { addr, size } => {
                write!(f, "address overflow at {addr:#x} (size {size})")
            }
            MemoryError::AddressNotRepresentable { addr } => {
                write!(f, "address {addr:#x} cannot be represented on this host")
            }
            MemoryError::Unmapped { addr, size } => {
                let range_end = addr.saturating_add(*size);
                write!(f, "no segment covers range {addr:#x}..{range_end:#x}")
            }
            MemoryError::AccessViolation { addr, access } => {
                write!(f, "segment at {addr:#x} missing permission to {access}")
            }
        }
    }
}

impl Error for MemoryError {}
