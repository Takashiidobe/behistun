use anyhow::{anyhow, Result};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_sethostname(&self) -> Result<i64> {
        let name_addr = self.data_regs[1] as usize;
        let len = self.data_regs[2] as usize;
        let host_ptr = self
            .memory
            .guest_to_host(name_addr, len)
            .ok_or_else(|| anyhow!("invalid hostname buffer"))?;
        Ok(unsafe { libc::sethostname(host_ptr as *const i8, len) as i64 })
    }
}
