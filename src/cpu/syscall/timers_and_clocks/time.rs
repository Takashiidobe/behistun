use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// time(tloc) - tloc can be NULL
    pub(crate) fn sys_time(&mut self) -> Result<i64> {
        let tloc = self.data_regs[1] as usize;

        if tloc == 0 {
            // NULL pointer - just return time
            Ok(unsafe { libc::time(std::ptr::null_mut()) })
        } else {
            // Need to write result to guest memory
            let mut t: libc::time_t = 0;
            let result = unsafe { libc::time(&mut t) };
            if result != -1 {
                // m68k uclibc uses 64-bit time_t
                self.memory.write_data(tloc, &(t as i64).to_be_bytes())?;
            }
            Ok(result)
        }
    }
}
