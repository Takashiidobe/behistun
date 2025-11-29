use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_read(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let buf = self.data_regs[2] as usize;
        let count = self.data_regs[3] as usize;

        let host_ptr = self.guest_mut_ptr(buf, count)?;

        let result = unsafe { libc::read(fd, host_ptr, count) as i64 };
        Ok(Self::libc_to_kernel(result))
    }
}
