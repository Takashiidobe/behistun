use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_vmsplice(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let iov_addr = self.data_regs[2] as usize;
        let nr_segs = self.data_regs[3] as usize;
        let flags = self.data_regs[4];

        let iovecs = self.build_iovecs(iov_addr, nr_segs, false)?;

        let result = unsafe { libc::vmsplice(fd, iovecs.as_ptr(), iovecs.len(), flags) };

        Ok(result as i64)
    }
}
