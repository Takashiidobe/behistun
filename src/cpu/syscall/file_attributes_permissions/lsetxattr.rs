use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_lsetxattr(&mut self) -> Result<i64> {
        let path_ptr = self.data_regs[1] as usize;
        let name_ptr = self.data_regs[2] as usize;
        let value_ptr = self.data_regs[3] as usize;
        let size = self.data_regs[4] as usize;
        let flags = self.data_regs[5] as i32;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let value = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr value buffer"))?
        } else {
            std::ptr::null()
        };

        let res = unsafe {
            libc::lsetxattr(
                path.as_ptr() as *const libc::c_char,
                name.as_ptr() as *const libc::c_char,
                value as *const libc::c_void,
                size,
                flags,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
