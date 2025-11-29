use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// readv(fd, iov, iovcnt)
    pub(crate) fn sys_readv(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let iov_addr = self.data_regs[2] as usize;
        let iovcnt = self.data_regs[3] as usize;
        let iovecs = self.build_iovecs(iov_addr, iovcnt, true)?;
        Ok(unsafe { libc::readv(fd, iovecs.as_ptr(), iovecs.len() as i32) as i64 })
    }
}
