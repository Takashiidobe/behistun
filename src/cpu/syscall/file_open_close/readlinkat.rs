use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    /// readlinkat(dirfd, path, buf, bufsiz)
    pub(crate) fn sys_readlinkat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let buf_addr = self.data_regs[3] as usize;
        let bufsiz = self.data_regs[4] as usize;

        let path_cstr = self.guest_cstring(path_addr)?;
        let host_buf = self
            .memory
            .guest_to_host_mut(buf_addr, bufsiz)
            .ok_or_else(|| anyhow!("invalid buffer for readlinkat"))?;
        let result = unsafe {
            libc::readlinkat(dirfd, path_cstr.as_ptr(), host_buf as *mut i8, bufsiz) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }
}
