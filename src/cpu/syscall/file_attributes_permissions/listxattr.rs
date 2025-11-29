use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_listxattr(&mut self) -> Result<i64> {
        let (path_ptr, list_ptr, size): (usize, usize, usize) = self.get_args();

        let path = self.read_c_string(path_ptr)?;
        let buf_host = if list_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host_mut(list_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr list buffer"))?
        } else {
            std::ptr::null_mut()
        };

        let res = unsafe {
            libc::listxattr(
                path.as_ptr() as *const libc::c_char,
                buf_host as *mut libc::c_char,
                size,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
