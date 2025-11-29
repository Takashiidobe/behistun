use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_mq_unlink(&self) -> Result<i64> {
        let name_addr = self.data_regs[1] as usize;
        let name_cstr = self.guest_cstring(name_addr)?;

        let result = unsafe { libc::syscall(241, name_cstr.as_ptr()) as i64 };
        Ok(Self::libc_to_kernel(result))
    }
}
