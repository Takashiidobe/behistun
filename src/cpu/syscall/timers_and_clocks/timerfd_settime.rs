use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    /// timerfd_settime(fd, flags, new_value, old_value)
    pub(crate) fn sys_timerfd_settime(&self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let flags = self.data_regs[2] as i32;
        let new_value_addr = self.data_regs[3] as usize;
        let old_value_addr = self.data_regs[4] as usize;

        let new_value_ptr = self
            .memory
            .guest_to_host(new_value_addr, std::mem::size_of::<libc::itimerspec>())
            .ok_or_else(|| anyhow!("invalid new_value pointer"))?;

        let old_value_ptr = if old_value_addr != 0 {
            self.memory
                .guest_to_host(old_value_addr, std::mem::size_of::<libc::itimerspec>())
                .ok_or_else(|| anyhow!("invalid old_value pointer"))?
        } else {
            std::ptr::null()
        };

        let result = unsafe {
            libc::timerfd_settime(
                fd,
                flags,
                new_value_ptr as *const libc::itimerspec,
                old_value_ptr as *mut libc::itimerspec,
            ) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }
}
