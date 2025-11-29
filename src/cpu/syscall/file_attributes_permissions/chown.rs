use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// chown/lchown/fchownat(path, owner, group)
    pub(crate) fn sys_chown(&self, syscall_num: u32) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let owner = self.data_regs[2] as libc::uid_t;
        let group = self.data_regs[3] as libc::gid_t;

        let path_cstr = self.guest_cstring(path_addr)?;

        Ok(unsafe { libc::syscall(syscall_num as i64, path_cstr.as_ptr(), owner, group) })
    }
}
