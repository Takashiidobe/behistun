use anyhow::{Result, anyhow};

use crate::{Cpu, cpu::syscall::M68K_TLS_TCB_SIZE, cpu::syscall::TLS_DATA_PAD};

impl Cpu {
    /// uClibc helper: return TLS base (m68k uses syscall number 333 for this)
    pub(crate) fn sys_read_tp(&mut self) -> Result<i64> {
        let mut tp = self.tls_base;
        if tp == 0 {
            // Lazily allocate a TLS block if none was configured.
            // Leave room for the 0x7000-byte TCB gap plus some space for
            // initial TLS data.
            const TLS_SIZE: usize = M68K_TLS_TCB_SIZE + TLS_DATA_PAD;
            let addr = self
                .memory
                .find_free_range(TLS_SIZE)
                .ok_or_else(|| anyhow!("no space for TLS block"))?;
            self.memory.add_segment(crate::memory::MemorySegment {
                vaddr: addr,
                data: crate::memory::MemoryData::Owned(vec![0u8; TLS_SIZE]),
                flags: goblin::elf::program_header::PF_R | goblin::elf::program_header::PF_W,
                align: 0x1000,
            });
            tp = (addr + M68K_TLS_TCB_SIZE) as u32;
            self.tls_base = tp;
        }

        // Returned in D0 only. Don't modify A0 or A6 - A6 is the frame pointer!
        self.data_regs[0] = tp;
        Ok(tp as i64)
    }
}
