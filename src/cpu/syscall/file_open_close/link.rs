use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// link(oldpath, newpath)
    pub(crate) fn sys_link(&self) -> Result<i64> {
        let old_addr = self.data_regs[1] as usize;
        let new_addr = self.data_regs[2] as usize;

        let old_cstr = self.guest_cstring(old_addr)?;
        let new_cstr = self.guest_cstring(new_addr)?;

        Ok(unsafe { libc::link(old_cstr.as_ptr(), new_cstr.as_ptr()) as i64 })
    }
}
