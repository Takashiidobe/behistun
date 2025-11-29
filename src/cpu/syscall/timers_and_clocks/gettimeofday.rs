use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// gettimeofday(tv, tz)
    pub(crate) fn sys_gettimeofday(&mut self) -> Result<i64> {
        let tv_addr = self.data_regs[1] as usize;
        let mut tv: libc::timeval = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::gettimeofday(&mut tv, std::ptr::null_mut()) };
        if result == 0 && tv_addr != 0 {
            // m68k uclibc uses 64-bit time_t
            self.memory
                .write_data(tv_addr, &(tv.tv_sec as i64).to_be_bytes())?;
            self.memory
                .write_data(tv_addr + 8, &(tv.tv_usec as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }
}
