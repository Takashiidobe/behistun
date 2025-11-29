use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// utimensat(dirfd, path, times, flags)
    pub(crate) fn sys_utimensat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let times_addr = self.data_regs[3] as usize;
        let flags = self.data_regs[4] as i32;

        let path_cstr = if path_addr != 0 {
            Some(self.guest_cstring(path_addr)?)
        } else {
            None
        };

        let times_ptr = if times_addr != 0 {
            self.memory
                .guest_to_host(times_addr, std::mem::size_of::<libc::timespec>() * 2)
                .ok_or_else(|| anyhow::anyhow!("invalid timespec buffer"))?
        } else {
            std::ptr::null()
        };

        let path_ptr = path_cstr
            .as_ref()
            .map(|c| c.as_ptr())
            .unwrap_or(std::ptr::null());
        let result = unsafe {
            libc::utimensat(dirfd, path_ptr, times_ptr as *const libc::timespec, flags) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }
}
