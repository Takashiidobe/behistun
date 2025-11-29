use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_listxattrat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as libc::c_int;
        let path_ptr = self.data_regs[2] as usize;
        let args_ptr = self.data_regs[3] as usize;
        let atflags = self.data_regs[4] as libc::c_int;

        let (value_ptr, size, _flags) = self.read_xattr_args(args_ptr)?;
        let path = self.read_c_string(path_ptr)?;
        let buf_host = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::syscall(
                465, // listxattrat syscall number on x86_64 as well
                dirfd,
                path.as_ptr(),
                buf_host,
                size,
                atflags,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }

    fn read_xattr_args(&self, addr: usize) -> Result<(usize, usize, u32)> {
        let value_ptr = self.memory.read_long(addr)? as usize;
        let size = self.memory.read_long(addr + 4)? as usize;
        let flags = self.memory.read_long(addr + 8)?;
        Ok((value_ptr, size, flags))
    }
}
