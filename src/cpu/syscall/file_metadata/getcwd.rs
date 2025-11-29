use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    /// getcwd(buf, size)
    pub(crate) fn sys_getcwd(&mut self) -> Result<i64> {
        let buf_addr = self.data_regs[1] as usize;
        let size = self.data_regs[2] as usize;
        let host_buf = self
            .memory
            .guest_to_host_mut(buf_addr, size)
            .ok_or_else(|| anyhow!("invalid getcwd buffer"))?;

        let result = unsafe { libc::getcwd(host_buf as *mut i8, size) };

        if result.is_null() {
            Ok(Self::libc_to_kernel(-1))
        } else {
            // getcwd syscall returns the length of the string (including null terminator)
            let len = unsafe { libc::strlen(result) } + 1;
            Ok(len as i64)
        }
    }
}
