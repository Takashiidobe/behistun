use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_lgetxattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let name_ptr = self.data_regs[2] as usize;
        let value_ptr = self.data_regs[3] as usize;
        let size = self.data_regs[4] as usize;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let buf_host = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::lgetxattr(
                path.as_ptr() as *const libc::c_char,
                name.as_ptr() as *const libc::c_char,
                buf_host as *mut libc::c_void,
                size,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
