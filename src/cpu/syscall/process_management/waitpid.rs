use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// waitpid(pid, status, options) - implemented via wait4 with NULL rusage
    pub(crate) fn sys_waitpid(&mut self) -> Result<i64> {
        // m68k ABI: D1=pid, D2=status*, D3=options
        let pid = self.data_regs[1] as i32;
        let status_addr = self.data_regs[2] as usize;
        let options = self.data_regs[3] as i32;

        let mut status: i32 = 0;
        let result = unsafe {
            libc::wait4(
                pid,
                if status_addr != 0 {
                    &mut status
                } else {
                    std::ptr::null_mut()
                },
                options,
                std::ptr::null_mut(),
            )
        };

        if result > 0 && status_addr != 0 {
            self.memory
                .write_data(status_addr, &(status as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }
}
