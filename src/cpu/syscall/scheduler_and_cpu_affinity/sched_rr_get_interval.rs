use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_sched_rr_get_interval(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as libc::pid_t;
        let tp_addr = self.data_regs[2] as usize;

        let mut tp: libc::timespec = unsafe { std::mem::zeroed() };

        let result = unsafe { libc::sched_rr_get_interval(pid, &mut tp) };
        if result == 0 {
            self.memory
                .write_data(tp_addr, &(tp.tv_sec as i64).to_be_bytes())?;
            self.memory
                .write_data(tp_addr + 8, &(tp.tv_nsec as u32).to_be_bytes())?;
        }
        Ok(Self::libc_to_kernel(result as i64))
    }
}
