use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_setrlimit(&self) -> Result<i64> {
        let resource = self.data_regs[1] as i32;
        let rlim_addr = self.data_regs[2] as usize;
        let rlim_cur = self.memory.read_long(rlim_addr)? as libc::rlim_t;
        let rlim_max = self.memory.read_long(rlim_addr + 4)? as libc::rlim_t;
        let rlim = libc::rlimit { rlim_cur, rlim_max };
        Ok(unsafe { libc::setrlimit(resource as u32, &rlim) as i64 })
    }
}
