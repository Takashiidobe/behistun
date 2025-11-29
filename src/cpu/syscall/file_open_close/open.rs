use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_open(&self) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let m68k_flags = self.data_regs[2] as i32;
        let mode = self.data_regs[3];

        let path_cstr = self.guest_cstring(path_addr)?;
        let flags = Self::translate_open_flags(m68k_flags);
        let result = unsafe { libc::open(path_cstr.as_ptr(), flags, mode) as i64 };
        Ok(Self::libc_to_kernel(result))
    }
}
