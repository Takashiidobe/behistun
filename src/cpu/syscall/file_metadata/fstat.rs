use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// fstat(fd, buf)
    pub(crate) fn sys_fstat(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let buf_addr = self.data_regs[2] as usize;
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::fstat(fd, &mut stat) };
        if result == 0 {
            self.write_stat(buf_addr, &stat)?;
        }
        Ok(result as i64)
    }
}
