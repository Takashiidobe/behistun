use anyhow::{Result, bail};

use crate::Cpu;

impl Cpu {
    /// mmap2(addr, length, prot, flags, fd, pgoffset)
    /// pgoffset is in 4096-byte pages (common across architectures)
    pub(crate) fn sys_mmap2(&mut self) -> Result<i64> {
        let addr = self.data_regs[1] as usize;
        let length = self.data_regs[2] as usize;
        let prot = self.data_regs[3] as i32;
        let flags = self.data_regs[4] as i32;
        let fd = self.data_regs[5] as i32;
        let _pgoffset = self.data_regs[6] as usize; // m68k passes 6th arg on stack - data_regs[6] used

        let is_anonymous = (flags & 0x20) != 0 || fd == -1;
        if !is_anonymous {
            bail!("mmap2: file-backed mappings not yet supported (fd={fd})");
        }

        let mapped = self.alloc_anonymous_mmap(addr, length, prot)?;
        Ok(mapped as i64)
    }
}
