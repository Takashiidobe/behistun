use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_pread64(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let buf_addr = self.data_regs[2] as usize;
        let count = self.data_regs[3] as usize;
        let offset = self.data_regs[4] as i64;
        let host_buf = self
            .memory
            .guest_to_host_mut(buf_addr, count)
            .ok_or_else(|| anyhow!("invalid pread64 buffer"))?;
        Ok(unsafe { libc::pread(fd, host_buf as *mut libc::c_void, count, offset) as i64 })
    }
}
