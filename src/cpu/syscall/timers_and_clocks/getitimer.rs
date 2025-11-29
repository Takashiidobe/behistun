use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// getitimer(which, curr)
    pub(crate) fn sys_getitimer(&mut self) -> Result<i64> {
        let which = self.data_regs[1] as i32;
        let curr_addr = self.data_regs[2] as usize;
        let mut curr: libc::itimerval = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::syscall(libc::SYS_getitimer, which, &mut curr as *mut _) };
        if result == 0 && curr_addr != 0 {
            self.write_itimerval(curr_addr, &curr)?;
        }
        Ok(result)
    }
}
