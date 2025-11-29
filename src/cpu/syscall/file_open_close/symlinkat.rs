use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// symlinkat(target, newdirfd, linkpath)
    pub(crate) fn sys_symlinkat(&self) -> Result<i64> {
        let target_addr = self.data_regs[1] as usize;
        let newdirfd = self.data_regs[2] as i32;
        let linkpath_addr = self.data_regs[3] as usize;

        let target_cstr = self.guest_cstring(target_addr)?;
        let linkpath_cstr = self.guest_cstring(linkpath_addr)?;
        let result = unsafe {
            libc::symlinkat(target_cstr.as_ptr(), newdirfd, linkpath_cstr.as_ptr()) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }
}
