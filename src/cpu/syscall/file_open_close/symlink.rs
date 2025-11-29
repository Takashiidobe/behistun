use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// symlink(target, linkpath)
    pub(crate) fn sys_symlink(&self) -> Result<i64> {
        let target_addr = self.data_regs[1] as usize;
        let linkpath_addr = self.data_regs[2] as usize;
        let target = self.guest_cstring(target_addr)?;
        let linkpath = self.guest_cstring(linkpath_addr)?;
        Ok(unsafe { libc::symlink(target.as_ptr(), linkpath.as_ptr()) as i64 })
    }
}
