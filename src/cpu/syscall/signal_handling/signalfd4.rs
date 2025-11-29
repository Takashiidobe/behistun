use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_signalfd4(&self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let mask_addr = self.data_regs[2] as usize;
        let sizemask = self.data_regs[3] as usize;
        let flags = self.data_regs[4] as i32;

        let mask_ptr = self
            .memory
            .guest_to_host(mask_addr, sizemask)
            .ok_or_else(|| anyhow!("invalid sigset_t buffer"))?;

        let result = unsafe {
            libc::syscall(
                libc::SYS_signalfd4,
                fd,
                mask_ptr as *const libc::sigset_t,
                sizemask,
                flags,
            )
        };
        Ok(Self::libc_to_kernel(result))
    }
}
