use anyhow::{anyhow, Result};

use crate::Cpu;

impl Cpu {
    /// syslog(type, buf, len)
    pub(crate) fn sys_syslog(&mut self) -> Result<i64> {
        let logtype = self.data_regs[1] as i32;
        let buf_addr = self.data_regs[2] as usize;
        let len = self.data_regs[3] as i32;
        if buf_addr == 0 || len == 0 {
            return Ok(unsafe {
                libc::syscall(libc::SYS_syslog, logtype, std::ptr::null::<u8>(), len)
            });
        }
        let host_buf = self
            .memory
            .guest_to_host_mut(buf_addr, len as usize)
            .ok_or_else(|| anyhow!("invalid syslog buffer"))?;
        Ok(unsafe { libc::syscall(libc::SYS_syslog, logtype, host_buf, len) })
    }
}
