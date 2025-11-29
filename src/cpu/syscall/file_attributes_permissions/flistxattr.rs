use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_flistxattr(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as libc::c_int;
        let list_ptr = self.data_regs[2] as usize;
        let size = self.data_regs[3] as usize;

        let buf_host = if list_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(list_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr list buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe { libc::flistxattr(fd, buf_host as *mut libc::c_char, size) };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
