use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// openat(dirfd, path, flags, mode)
    pub(crate) fn sys_openat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let m68k_flags = self.data_regs[3] as i32;
        let mode = self.data_regs[4];

        let path_cstr = self.guest_cstring(path_addr)?;
        let flags = Self::translate_open_flags(m68k_flags);
        let result = unsafe { libc::openat(dirfd, path_cstr.as_ptr(), flags, mode) as i64 };
        Ok(Self::libc_to_kernel(result))
    }
}
