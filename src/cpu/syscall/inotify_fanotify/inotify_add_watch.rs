use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// inotify_add_watch(fd, path, mask)
    pub(crate) fn sys_inotify_add_watch(&self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let mask = self.data_regs[3];

        let path_cstr = self.guest_cstring(path_addr)?;
        let result = unsafe { libc::inotify_add_watch(fd, path_cstr.as_ptr(), mask) as i64 };
        Ok(Self::libc_to_kernel(result))
    }
}
