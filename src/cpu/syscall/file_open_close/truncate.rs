use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// truncate(path, length)
    pub(crate) fn sys_truncate(&self) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let length = self.data_regs[2] as i64;
        let path = self.guest_cstring(path_addr)?;
        Ok(unsafe { libc::truncate(path.as_ptr(), length) as i64 })
    }
}
