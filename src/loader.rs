use anyhow::{Result, bail};

use goblin::elf::{Elf, program_header};

use crate::memory::{MemoryImage, MemorySegment};

pub fn load_memory_image(elf: &Elf, file_bytes: &[u8]) -> Result<MemoryImage> {
    let mut segments = Vec::new();

    for ph in &elf.program_headers {
        if ph.p_type != program_header::PT_LOAD || ph.p_memsz == 0 {
            continue;
        }

        if ph.p_memsz < ph.p_filesz {
            bail!(
                "Segment memsz ({}) smaller than filesz ({}) at vaddr {:#x}",
                ph.p_memsz,
                ph.p_filesz,
                ph.p_vaddr
            );
        }

        let offset = ph.p_offset as usize;
        let file_size = ph.p_filesz as usize;
        let mem_size = ph.p_memsz as usize;

        if offset
            .checked_add(file_size)
            .is_none_or(|end| end > file_bytes.len())
        {
            bail!(
                "Segment at offset {:#x} with size {:#x} exceeds file ({} bytes)",
                ph.p_offset,
                ph.p_filesz,
                file_bytes.len()
            );
        }

        // Align segment start down to at least 4KB (common for ELF PT_LOAD)
        let align = ph.p_align.max(0x1000) as usize;
        let seg_start = (ph.p_vaddr as usize) & !(align - 1);
        let pad = ph.p_vaddr as usize - seg_start;

        let mut data = vec![0u8; pad + mem_size];
        if file_size > 0 {
            let end = offset + file_size;
            data[pad..pad + file_size].copy_from_slice(&file_bytes[offset..end]);
        }

        // Some static binaries (e.g., uClibc-built) self-patch code/PLT areas; allow writes.
        let mut flags = ph.p_flags;
        if (flags & program_header::PF_X) != 0 {
            flags |= program_header::PF_W;
        }

        segments.push(MemorySegment {
            vaddr: seg_start,
            data: crate::memory::MemoryData::Owned(data),
            flags,
            align: ph.p_align as usize,
        });
    }

    segments.sort_by_key(|seg| seg.vaddr);

    // Add a null page at address 0 to handle buggy library code that writes to NULL
    // This is a workaround for uclibc's __tunable_get_val which has a code path
    // that writes to a NULL pointer
    segments.insert(
        0,
        MemorySegment {
            vaddr: 0,
            data: crate::memory::MemoryData::Owned(vec![0u8; 4096]),
            flags: program_header::PF_R | program_header::PF_W,
            align: 0x1000,
        },
    );

    // Place a stack segment high in the 32-bit address space so the heap
    // (brk) can grow upward from the program data/BSS.
    let stack_size = 1024 * 1024; // 1MB stack
    // Leave a small guard gap below the top of user space to mimic Linux.
    let stack_top: usize = 0xfffff000;
    let stack_base = stack_top - stack_size;

    segments.push(MemorySegment {
        vaddr: stack_base,
        data: crate::memory::MemoryData::Owned(vec![0u8; stack_size]),
        flags: program_header::PF_R | program_header::PF_W, // Read + Write
        align: 0x1000,
    });

    Ok(MemoryImage::new(segments))
}
