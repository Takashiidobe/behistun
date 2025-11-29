use crate::Cpu;
use anyhow::Result;

impl Cpu {
    pub(crate) fn sys_removexattrat(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as libc::c_int;
        let path_ptr = self.data_regs[2] as usize;
        let name_ptr = self.data_regs[3] as usize;
        let atflags = self.data_regs[4] as libc::c_int;

        let path = self.read_c_string(path_ptr)?;
        let name = self.read_c_string(name_ptr)?;
        let res = unsafe {
            libc::syscall(
                466, // removexattrat syscall number on x86_64 as well
                dirfd,
                path.as_ptr(),
                name.as_ptr(),
                atflags,
            )
        };
        Ok(Self::libc_to_kernel(res as i64))
    }
}
