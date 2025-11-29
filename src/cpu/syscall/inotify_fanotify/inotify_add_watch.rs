use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// inotify_add_watch(fd, path, mask)
    pub(crate) fn sys_inotify_add_watch(&self) -> Result<i64> {
        let (fd, path_addr, mask) = self.get_args();

        let path_cstr = self.guest_cstring(path_addr)?;
        let result = unsafe { libc::inotify_add_watch(fd, path_cstr.as_ptr(), mask) as i64 };
        Ok(Self::libc_to_kernel(result))
    }
}
