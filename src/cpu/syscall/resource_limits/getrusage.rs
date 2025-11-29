use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_getrusage(&mut self) -> Result<i64> {
        let who = self.data_regs[1] as i32;
        let usage_addr = self.data_regs[2] as usize;
        let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::getrusage(who, &mut usage) };
        if result == 0 && usage_addr != 0 {
            self.memory
                .write_data(usage_addr, &(usage.ru_utime.tv_sec as i64).to_be_bytes())?;
            self.memory.write_data(
                usage_addr + 8,
                &(usage.ru_utime.tv_usec as u32).to_be_bytes(),
            )?;
            self.memory.write_data(
                usage_addr + 12,
                &(usage.ru_stime.tv_sec as i64).to_be_bytes(),
            )?;
            self.memory.write_data(
                usage_addr + 20,
                &(usage.ru_stime.tv_usec as u32).to_be_bytes(),
            )?;
        }
        Ok(result as i64)
    }
}
