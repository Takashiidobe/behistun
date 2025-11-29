use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_sched_getparam(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as libc::pid_t;
        let param_addr = self.data_regs[2] as usize;

        let mut param: libc::sched_param = unsafe { std::mem::zeroed() };

        let result = unsafe { libc::sched_getparam(pid, &mut param) };
        if result == 0 {
            self.memory
                .write_data(param_addr, &(param.sched_priority as u32).to_be_bytes())?;
        }
        Ok(Self::libc_to_kernel(result as i64))
    }
}
