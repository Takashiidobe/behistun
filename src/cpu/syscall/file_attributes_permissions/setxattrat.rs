use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_setxattrat(&mut self) -> Result<i64> {
        let (dirfd, path_ptr, name_ptr, value_ptr, size, flags): (
            libc::c_int,
            usize,
            usize,
            usize,
            usize,
            i32,
        ) = self.get_args();

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let value = if value_ptr != 0 && size > 0 {
            let host = self
                .memory
                .guest_to_host(value_ptr, size)
                .ok_or_else(|| anyhow!("invalid xattr value buffer"))?;
            Some(unsafe { std::slice::from_raw_parts(host, size) })
        } else {
            None
        };

        let res = unsafe {
            libc::syscall(
                463, // setxattrat syscall number on x86_64 as well
                dirfd,
                path.as_ptr(),
                name.as_ptr(),
                value
                    .map(|v| v.as_ptr() as *const libc::c_void)
                    .unwrap_or(std::ptr::null()),
                size,
                flags,
                0, // at_flags - always 0 for now
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
