use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// linkat(olddirfd, oldpath, newdirfd, newpath, flags)
    pub(crate) fn sys_linkat(&self) -> Result<i64> {
        let olddirfd = self.data_regs[1] as i32;
        let oldpath_addr = self.data_regs[2] as usize;
        let newdirfd = self.data_regs[3] as i32;
        let newpath_addr = self.data_regs[4] as usize;
        let flags = self.data_regs[5] as i32;

        let oldpath_cstr = self.guest_cstring(oldpath_addr)?;
        let newpath_cstr = self.guest_cstring(newpath_addr)?;
        let result = unsafe {
            libc::linkat(
                olddirfd,
                oldpath_cstr.as_ptr(),
                newdirfd,
                newpath_cstr.as_ptr(),
                flags,
            ) as i64
        };
        Ok(Self::libc_to_kernel(result))
    }
}
