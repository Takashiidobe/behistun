use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_fstatat64(&mut self) -> Result<i64> {
        let dirfd = self.data_regs[1] as i32;
        let path_addr = self.data_regs[2] as usize;
        let buf_addr = self.data_regs[3] as usize;
        let flags = self.data_regs[4] as i32;

        let path_cstr = self.guest_cstring(path_addr)?;
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::syscall(
                libc::SYS_newfstatat,
                dirfd,
                path_cstr.as_ptr(),
                &mut stat,
                flags,
            )
        };
        if result == 0 {
            self.write_stat(buf_addr, &stat)?;
        }
        Ok(result)
    }
}
