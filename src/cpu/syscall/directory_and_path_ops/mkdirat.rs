use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// mkdirat(dirfd, path, mode)
    pub(crate) fn sys_mkdirat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let mode = self.data_regs[3] as libc::mode_t;

        let path_cstr = self.guest_cstring(path_addr)?;
        let result = unsafe { libc::mkdirat(dirfd, path_cstr.as_ptr(), mode) as i64 };
        Ok(Self::libc_to_kernel(result))
    }
}
