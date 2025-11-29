use anyhow::{Result, anyhow};

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_setxattrat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as libc::c_int;
        let path_ptr = self.data_regs[2] as usize;
        let name_ptr = self.data_regs[3] as usize;
        let value_ptr = self.data_regs[4] as usize;
        let size = self.data_regs[5] as usize;
        let flags = self.data_regs[6] as i32;

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
