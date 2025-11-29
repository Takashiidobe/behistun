use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// fchmodat2(dirfd, path, mode, flags)
    /// Extended version of fchmodat (Linux 6.6+) that properly supports AT_SYMLINK_NOFOLLOW
    pub(crate) fn sys_fchmodat2(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let mode = self.data_regs[3] as libc::mode_t;
        let flags = self.data_regs[4] as i32;

        let path_cstr = self.guest_cstring(path_addr)?;

        // Call fchmodat2 via syscall (no libc wrapper yet)
        let result = unsafe {
            libc::syscall(
                452, // SYS_fchmodat2
                dirfd,
                path_cstr.as_ptr(),
                mode,
                flags,
            ) as i64
        };

        Ok(Self::libc_to_kernel(result))
    }
}
