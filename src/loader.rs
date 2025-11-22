use std::error::Error;

use goblin::elf::{Elf, program_header};

use crate::memory::{MemoryImage, MemorySegment};

pub fn load_memory_image(elf: &Elf, file_bytes: &[u8]) -> Result<MemoryImage, Box<dyn Error>> {
    let mut segments = Vec::new();

    for ph in &elf.program_headers {
        if ph.p_type != program_header::PT_LOAD {
            continue;
        }

        if ph.p_memsz == 0 {
            continue;
        }

        if ph.p_memsz < ph.p_filesz {
            return Err(format!(
                "Segment memsz ({}) smaller than filesz ({}) at vaddr {:#x}",
                ph.p_memsz, ph.p_filesz, ph.p_vaddr
            )
            .into());
        }

        let offset = to_usize("p_offset", ph.p_offset)?;
        let file_size = to_usize("p_filesz", ph.p_filesz)?;
        let mem_size = to_usize("p_memsz", ph.p_memsz)?;

        if offset
            .checked_add(file_size)
            .is_none_or(|end| end > file_bytes.len())
        {
            return Err(format!(
                "Segment at offset {:#x} with size {:#x} exceeds file ({} bytes)",
                ph.p_offset,
                ph.p_filesz,
                file_bytes.len()
            )
            .into());
        }

        let mut data = vec![0u8; mem_size];
        if file_size > 0 {
            let end = offset + file_size;
            data[..file_size].copy_from_slice(&file_bytes[offset..end]);
        }

        segments.push(MemorySegment {
            vaddr: ph.p_vaddr as usize,
            data,
            flags: ph.p_flags,
            align: ph.p_align as usize,
        });
    }

    segments.sort_by_key(|seg| seg.vaddr);
    Ok(MemoryImage::new(segments))
}

fn to_usize(field: &str, value: u64) -> Result<usize, Box<dyn Error>> {
    usize::try_from(value).map_err(|_| format!("{field} ({value}) does not fit in usize").into())
}
