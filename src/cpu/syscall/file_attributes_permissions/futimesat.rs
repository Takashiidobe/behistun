use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_futimesat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let times_addr = self.data_regs[3] as usize;

        let path_cstr = self.guest_cstring(path_addr)?;
        let times_ptr = if times_addr != 0 {
            self.memory
                .guest_to_host(times_addr, std::mem::size_of::<libc::timeval>() * 2)
                .ok_or_else(|| anyhow!("invalid timeval buffer"))?
        } else {
            std::ptr::null()
        };

        let result = unsafe {
            libc::syscall(
                libc::SYS_futimesat,
                dirfd,
                path_cstr.as_ptr(),
                times_ptr as *const libc::timeval,
            )
        };
        Ok(Self::libc_to_kernel(result))
    }
}
