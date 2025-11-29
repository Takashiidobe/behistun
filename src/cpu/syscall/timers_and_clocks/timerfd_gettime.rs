use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    /// timerfd_gettime(fd, curr_value)
    pub(crate) fn sys_timerfd_gettime(&self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let curr_value_addr = self.data_regs[2] as usize;

        let curr_value_ptr = self
            .memory
            .guest_to_host(curr_value_addr, std::mem::size_of::<libc::itimerspec>())
            .ok_or_else(|| anyhow!("invalid curr_value pointer"))?;

        let result =
            unsafe { libc::timerfd_gettime(fd, curr_value_ptr as *mut libc::itimerspec) as i64 };
        Ok(Self::libc_to_kernel(result))
    }
}
