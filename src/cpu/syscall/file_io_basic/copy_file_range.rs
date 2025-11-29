use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    /// copy_file_range(fd_in, off_in, fd_out, off_out, len, flags)
    pub(crate) fn sys_copy_file_range(&mut self) -> Result<i64> {
        let fd_in = self.data_regs[1] as i32;
        let off_in_addr = self.data_regs[2] as usize;
        let fd_out = self.data_regs[3] as i32;
        let off_out_addr = self.data_regs[4] as usize;
        let len = self.data_regs[5] as usize;

        let off_in_ptr = if off_in_addr != 0 {
            self.memory
                .guest_to_host_mut(off_in_addr, 8)
                .ok_or_else(|| anyhow!("invalid off_in pointer"))? as *mut i64
        } else {
            std::ptr::null_mut()
        };

        let off_out_ptr = if off_out_addr != 0 {
            self.memory
                .guest_to_host_mut(off_out_addr, 8)
                .ok_or_else(|| anyhow!("invalid off_out pointer"))? as *mut i64
        } else {
            std::ptr::null_mut()
        };

        let result = unsafe {
            libc::syscall(
                libc::SYS_copy_file_range,
                fd_in,
                off_in_ptr,
                fd_out,
                off_out_ptr,
                len,
                0,
            )
        };
        Ok(Self::libc_to_kernel(result))
    }
}
