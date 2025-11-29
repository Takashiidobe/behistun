use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// mknod(path, mode, dev)
    pub(crate) fn sys_mknod(&self) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let mode = self.data_regs[2] as libc::mode_t;
        let dev = self.data_regs[3] as libc::dev_t;

        let path_cstr = self.guest_cstring(path_addr)?;

        Ok(unsafe { libc::mknod(path_cstr.as_ptr(), mode, dev) as i64 })
    }
}
