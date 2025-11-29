use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_memfd_create(&self) -> Result<i64> {
        let name_addr = self.data_regs[1] as usize;
        let flags = self.data_regs[2];

        let name_cstr = self.guest_cstring(name_addr)?;
        let result =
            unsafe { libc::syscall(libc::SYS_memfd_create, name_cstr.as_ptr(), flags) as i64 };
        Ok(Self::libc_to_kernel(result))
    }
}
