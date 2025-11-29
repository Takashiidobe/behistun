use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_sched_setparam(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as libc::pid_t;
        let param_addr = self.data_regs[2] as usize;

        let priority = self.memory.read_long(param_addr)? as i32;

        let param = libc::sched_param {
            sched_priority: priority,
        };

        let result = unsafe { libc::sched_setparam(pid, &param) };
        Ok(Self::libc_to_kernel(result as i64))
    }
}
