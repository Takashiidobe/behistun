use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_landlock_restrict_self(&mut self) -> Result<i64> {
        let ruleset_fd = self.data_regs[1] as i32;
        let flags = self.data_regs[2];

        let result = unsafe { libc::syscall(446, ruleset_fd, flags) };

        Ok(Self::libc_to_kernel(result as i64))
    }
}
