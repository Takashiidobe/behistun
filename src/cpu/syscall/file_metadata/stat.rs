use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// stat/lstat(path, buf)
    pub(crate) fn sys_stat(&mut self, syscall_num: u32) -> Result<i64> {
        let path_addr = self.data_regs[1] as usize;
        let buf_addr = self.data_regs[2] as usize;
        let path = self.guest_cstring(path_addr)?;
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::syscall(syscall_num as i64, path.as_ptr(), &mut stat) };
        if result == 0 {
            self.write_stat(buf_addr, &stat)?;
        }
        Ok(result)
    }
}
