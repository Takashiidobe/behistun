use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// fchownat(dirfd, path, owner, group, flags)
    pub(crate) fn sys_fchownat(&self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let owner = self.data_regs[3] as libc::uid_t;
        let group = self.data_regs[4] as libc::gid_t;
        let flags = self.data_regs[5] as i32;

        let path_cstr = self.guest_cstring(path_addr)?;
        let result =
            unsafe { libc::fchownat(dirfd, path_cstr.as_ptr(), owner, group, flags) as i64 };
        Ok(Self::libc_to_kernel(result))
    }
}
