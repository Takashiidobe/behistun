use anyhow::{Result, bail};

use crate::Cpu;

impl Cpu {
    /// mmap(addr, length, prot, flags, fd, offset)
    pub(crate) fn sys_mmap(&mut self) -> Result<i64> {
        // old_mmap on m68k uses a single pointer to mmap_arg_struct
        //   struct mmap_arg_struct { void *addr; u32 len; u32 prot; u32 flags; u32 fd; u32 offset; }
        let args_ptr = self.data_regs[1] as usize;
        let addr_req = self.memory.read_long(args_ptr)? as usize;
        let length = self.memory.read_long(args_ptr + 4)? as usize;
        let prot = self.memory.read_long(args_ptr + 8)? as i32;
        let flags = self.memory.read_long(args_ptr + 12)? as i32;
        let fd = self.memory.read_long(args_ptr + 16)? as i32;
        let _offset = self.memory.read_long(args_ptr + 20)? as i64; // bytes (not pages)

        let is_anonymous = (flags & 0x20) != 0 || fd == -1;
        if !is_anonymous {
            bail!("mmap: file-backed mappings not yet supported (fd={fd})");
        }

        let addr = self.alloc_anonymous_mmap(addr_req, length, prot)?;
        Ok(addr as i64)
    }
}
