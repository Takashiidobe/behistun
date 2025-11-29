use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_getxattrat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as libc::c_int;
        let path_ptr = self.data_regs[2] as usize;
        let name_ptr = self.data_regs[3] as usize;
        let value_ptr = self.data_regs[4] as usize;
        let size = self.data_regs[5] as usize;

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
            libc::syscall(
                464, // getxattrat syscall number on x86_64 as well
                dirfd,
                path.as_ptr(),
                name.as_ptr(),
                buf_host,
                size,
                0, // at_flags - always 0 for now
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
