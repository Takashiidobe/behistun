use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// unlinkat(dirfd, path, flags)
    pub(crate) fn sys_unlinkat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let flags = self.data_regs[3] as i32;

        let path_cstr = self.guest_cstring(path_addr)?;
        let result = unsafe { libc::unlinkat(dirfd, path_cstr.as_ptr(), flags) as i64 };
        Ok(Self::libc_to_kernel(result))
    }
}
