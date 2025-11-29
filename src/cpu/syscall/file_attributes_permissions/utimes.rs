use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    /// utimes(filename, times)
    pub(crate) fn sys_utimes(&self) -> Result<i64> {
        let filename_addr = self.data_regs[1] as usize;
        let times_addr = self.data_regs[2] as usize;

        let filename_cstr = self.guest_cstring(filename_addr)?;

        let times_ptr = if times_addr != 0 {
            // times is array of 2 timevals (each is 2 longs: tv_sec, tv_usec)
            self.memory
                .guest_to_host(times_addr, std::mem::size_of::<libc::timeval>() * 2)
                .ok_or_else(|| anyhow!("invalid timeval buffer"))?
        } else {
            std::ptr::null()
        };

        let result = unsafe {
            libc::utimes(filename_cstr.as_ptr(), times_ptr as *const libc::timeval) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }
}
