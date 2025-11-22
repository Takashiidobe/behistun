#![allow(dead_code)]
use std::{error::Error, fmt};

use goblin::elf::program_header;

/// Memory data can be either owned (Vec<u8>) or foreign (from shmat)
#[derive(Debug)]
pub enum MemoryData {
    Owned(Vec<u8>),
    Foreign {
        ptr: *mut u8,
        len: usize,
        shmid: i32,
    },
}

impl Clone for MemoryData {
    fn clone(&self) -> Self {
        match self {
            MemoryData::Owned(v) => MemoryData::Owned(v.clone()),
            // Foreign memory can't be safely cloned - would need re-attach
            // For now, panic if we try to clone foreign memory
            MemoryData::Foreign { .. } => {
                panic!(
                    "Cannot clone foreign memory segments - fork not yet supported for shared memory"
                )
            }
        }
    }
}

impl Drop for MemoryData {
    fn drop(&mut self) {
        if let MemoryData::Foreign { ptr, .. } = self {
            // Detach shared memory when segment is dropped
            unsafe {
                libc::shmdt(*ptr as *const libc::c_void);
            }
        }
    }
}

#[derive(Debug)]
pub struct MemorySegment {
    pub vaddr: usize,
    pub data: MemoryData,
    pub flags: u32,
    pub align: usize,
}

impl Clone for MemorySegment {
    fn clone(&self) -> Self {
        MemorySegment {
            vaddr: self.vaddr,
            data: self.data.clone(),
            flags: self.flags,
            align: self.align,
        }
    }
}

impl MemorySegment {
    pub fn len(&self) -> usize {
        match &self.data {
            MemoryData::Owned(v) => v.len(),
            MemoryData::Foreign { len, .. } => *len,
        }
    }

    fn as_slice(&self) -> &[u8] {
        match &self.data {
            MemoryData::Owned(v) => v.as_slice(),
            MemoryData::Foreign { ptr, len, .. } => unsafe {
                std::slice::from_raw_parts(*ptr, *len)
            },
        }
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        match &mut self.data {
            MemoryData::Owned(v) => v.as_mut_slice(),
            MemoryData::Foreign { ptr, len, .. } => unsafe {
                std::slice::from_raw_parts_mut(*ptr, *len)
            },
        }
    }
}

#[derive(Debug, Clone)]
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

        let offset = addr - segment.vaddr;
        let slice = segment.as_mut_slice();
        slice[offset..offset + size].copy_from_slice(data);
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

        let offset = addr - segment.vaddr;
        let slice = segment.as_slice();
        Ok(&slice[offset..offset + size])
    }

    fn segment_containing(&self, start: usize, end: usize) -> Option<&MemorySegment> {
        for segment in &self.segments {
            let seg_start = segment.vaddr;
            let seg_end = seg_start.checked_add(segment.len())?;
            if start >= seg_start && end <= seg_end {
                return Some(segment);
            }
        }
        // Debug: log unmapped accesses
        if (0x80000000..0xffef0000).contains(&start) {
            eprintln!("UNMAPPED ACCESS: {:#x}..{:#x}", start, end);
        }
        None
    }

    fn segment_containing_mut(&mut self, start: usize, end: usize) -> Option<&mut MemorySegment> {
        for segment in &mut self.segments {
            let seg_start = segment.vaddr;
            let seg_end = seg_start.checked_add(segment.len())?;
            if start >= seg_start && end <= seg_end {
                return Some(segment);
            }
        }
        // Debug: log unmapped write accesses
        let var_name = start >= 0x80000000;
        if var_name && start < 0xffef0000 {
            eprintln!("UNMAPPED WRITE: {:#x}..{:#x}", start, end);
        }
        None
    }

    /// Return true if any segment fully covers the given address range.
    pub fn covers_range(&self, addr: usize, size: usize) -> bool {
        let end = match addr.checked_add(size) {
            Some(e) => e,
            None => return false,
        };
        self.segment_containing(addr, end).is_some()
    }

    /// Get a mutable host pointer for a guest address range.
    pub fn guest_to_host_mut(&mut self, addr: usize, size: usize) -> Option<*mut u8> {
        if size == 0 {
            return Some(std::ptr::null_mut());
        }
        let end = addr.checked_add(size)?;
        let segment = self.segment_containing_mut(addr, end)?;
        let offset = addr - segment.vaddr;
        let slice = segment.as_mut_slice();
        Some(slice[offset..].as_mut_ptr())
    }

    /// Get an immutable host pointer for a guest address range.
    pub fn guest_to_host(&self, addr: usize, size: usize) -> Option<*const u8> {
        if size == 0 {
            return Some(std::ptr::null());
        }
        let end = addr.checked_add(size)?;
        let segment = self.segment_containing(addr, end)?;
        let offset = addr - segment.vaddr;
        let slice = segment.as_slice();
        Some(slice[offset..].as_ptr())
    }

    /// Add a new memory segment (for mmap support)
    pub fn add_segment(&mut self, segment: MemorySegment) {
        self.segments.push(segment);
        self.segments.sort_by_key(|s| s.vaddr);
    }

    /// Resize an existing segment identified by its base address.
    /// If the new size is larger, the new bytes are zero-initialized.
    /// Note: Only works for Owned memory segments.
    pub fn resize_segment(&mut self, base: usize, new_size: usize) -> Result<(), MemoryError> {
        let end = base
            .checked_add(new_size)
            .ok_or(MemoryError::AddressOverflow {
                addr: base,
                size: new_size,
            })?;

        let next_start = self
            .segments
            .iter()
            .filter(|s| s.vaddr > base)
            .map(|s| s.vaddr)
            .min();

        let Some(segment) = self.segments.iter_mut().find(|s| s.vaddr == base) else {
            return Err(MemoryError::Unmapped {
                addr: base,
                size: new_size,
            });
        };

        // Basic overlap check: ensure we don't grow into the next segment.
        if let Some(next) = next_start
            && end > next
        {
            return Err(MemoryError::AccessViolation {
                addr: end,
                access: "grow",
            });
        }

        // Only owned segments can be resized
        match &mut segment.data {
            MemoryData::Owned(v) => {
                v.resize(new_size, 0);
                Ok(())
            }
            MemoryData::Foreign { .. } => Err(MemoryError::AccessViolation {
                addr: base,
                access: "resize foreign segment",
            }),
        }
    }

    /// Find the index of a segment containing the given address
    pub fn find_segment_index(&self, addr: usize) -> Option<usize> {
        self.segments.iter().position(|s| {
            let start = s.vaddr;
            let end = start + s.len();
            addr >= start && addr < end
        })
    }

    /// Remove a segment by index
    pub fn remove_segment(&mut self, idx: usize) {
        if idx < self.segments.len() {
            self.segments.remove(idx);
        }
    }

    /// Find a free address range of the given size (page-aligned)
    pub fn find_free_range(&self, size: usize) -> Option<usize> {
        const PAGE_SIZE: usize = 4096;
        // Start looking from a high address in 32-bit space (below 0xC0000000 kernel area)
        let mut candidate: usize = 0x40000000; // Start at 1GB

        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

        // Collect and sort segment ranges
        let mut ranges: Vec<(usize, usize)> = self
            .segments
            .iter()
            .map(|s| (s.vaddr, s.vaddr + s.len()))
            .collect();
        ranges.sort_by_key(|r| r.0);

        for (seg_start, seg_end) in &ranges {
            if candidate + aligned_size <= *seg_start {
                // Found a gap before this segment
                return Some(candidate);
            }
            // Move candidate past this segment
            if *seg_end > candidate {
                candidate = (*seg_end + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
            }
        }

        // Check if there's room after the last segment (but within 32-bit space)
        if candidate + aligned_size <= 0xC0000000 {
            return Some(candidate);
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
