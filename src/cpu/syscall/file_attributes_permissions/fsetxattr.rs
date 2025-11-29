use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_fsetxattr(&mut self) -> Result<i64> {
        let (fd, name_ptr, value_ptr, size, flags): (libc::c_int, usize, usize, usize, i32) =
            self.get_args();

        let name = self.read_c_string(name_ptr)?;
        let value = if value_ptr != 0 && size > 0 {
            self.memory
                .guest_to_host(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr value buffer"))?
        } else {
            std::ptr::null()
        };

        let res = unsafe {
            libc::fsetxattr(
                fd,
                name.as_ptr() as *const libc::c_char,
                value as *const libc::c_void,
                size,
                flags,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
