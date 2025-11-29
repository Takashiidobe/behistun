use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    /// getrandom(buf, buflen, flags)
    pub(crate) fn sys_getrandom(&mut self) -> Result<i64> {
        let buf_addr = self.data_regs[1] as usize;
        let buflen = self.data_regs[2] as usize;
        let flags = self.data_regs[3];

        let buf_ptr = self
            .memory
            .guest_to_host_mut(buf_addr, buflen)
            .ok_or_else(|| anyhow!("invalid buffer for getrandom"))?;

        let result = unsafe { libc::syscall(libc::SYS_getrandom, buf_ptr, buflen, flags) };
        Ok(Self::libc_to_kernel(result))
    }
}
