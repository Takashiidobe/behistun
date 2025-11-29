use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// preadv(fd, iov, iovcnt, pos_l, pos_h)
    pub(crate) fn sys_preadv(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let iov_addr = self.data_regs[2] as usize;
        let iovcnt = self.data_regs[3] as usize;
        let off_lo = self.data_regs[4] as i64;
        let off_hi = self.data_regs[5] as i64;
        let offset = (off_hi << 32) | off_lo;
        let iovecs = self.build_iovecs(iov_addr, iovcnt, true)?;
        Ok(unsafe { libc::preadv(fd, iovecs.as_ptr(), iovecs.len() as i32, offset) as i64 })
    }
}
