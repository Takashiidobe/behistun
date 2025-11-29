use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// setitimer(which, new, old)
    pub(crate) fn sys_setitimer(&mut self) -> Result<i64> {
        let which = self.data_regs[1] as i32;
        let new_addr = self.data_regs[2] as usize;
        let old_addr = self.data_regs[3] as usize;

        let new_val = if new_addr != 0 {
            Some(self.read_itimerval(new_addr)?)
        } else {
            None
        };

        let mut old_val: libc::itimerval = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::syscall(
                libc::SYS_setitimer,
                which,
                new_val.as_ref().map_or(std::ptr::null(), |v| v as *const _),
                if old_addr != 0 {
                    &mut old_val as *mut _
                } else {
                    std::ptr::null_mut::<libc::itimerval>()
                },
            )
        };

        if result == 0 && old_addr != 0 {
            self.write_itimerval(old_addr, &old_val)?;
        }
        Ok(result)
    }
}
