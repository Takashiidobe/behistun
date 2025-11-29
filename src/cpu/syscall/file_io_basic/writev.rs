use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// writev(fd, iov, iovcnt)
    pub(crate) fn sys_writev(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let iov_addr = self.data_regs[2] as usize;
        let iovcnt = self.data_regs[3] as usize;
        let iovecs = self.build_iovecs(iov_addr, iovcnt, false)?;
        Ok(unsafe { libc::writev(fd, iovecs.as_ptr(), iovecs.len() as i32) as i64 })
    }
}
