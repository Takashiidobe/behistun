use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// wait4(pid, status, options, rusage)
    pub(crate) fn sys_wait4(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as i32;
        let status_addr = self.data_regs[2] as usize;
        let options = self.data_regs[3] as i32;
        let rusage_addr = self.data_regs[4] as usize;

        let mut status: i32 = 0;
        let mut rusage: libc::rusage = unsafe { std::mem::zeroed() };

        let result = unsafe {
            libc::wait4(
                pid,
                if status_addr != 0 {
                    &mut status
                } else {
                    std::ptr::null_mut()
                },
                options,
                if rusage_addr != 0 {
                    &mut rusage
                } else {
                    std::ptr::null_mut()
                },
            )
        };

        if result > 0 {
            if status_addr != 0 {
                self.memory
                    .write_data(status_addr, &(status as u32).to_be_bytes())?;
            }
            if rusage_addr != 0 {
                self.memory
                    .write_data(rusage_addr, &(rusage.ru_utime.tv_sec as u32).to_be_bytes())?;
                self.memory.write_data(
                    rusage_addr + 4,
                    &(rusage.ru_utime.tv_usec as u32).to_be_bytes(),
                )?;
                self.memory.write_data(
                    rusage_addr + 8,
                    &(rusage.ru_stime.tv_sec as u32).to_be_bytes(),
                )?;
                self.memory.write_data(
                    rusage_addr + 12,
                    &(rusage.ru_stime.tv_usec as u32).to_be_bytes(),
                )?;
            }
        }
        Ok(result as i64)
    }
}
