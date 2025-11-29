use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// clock_gettime(clockid, timespec)
    pub(crate) fn sys_clock_gettime(&mut self) -> Result<i64> {
        let clk_id = self.data_regs[1] as libc::clockid_t;
        let ts_addr = self.data_regs[2] as usize;
        let mut ts: libc::timespec = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::clock_gettime(clk_id, &mut ts) };
        if result == 0 && ts_addr != 0 {
            // m68k uclibc uses 64-bit time_t
            self.memory
                .write_data(ts_addr, &(ts.tv_sec as i64).to_be_bytes())?;
            self.memory
                .write_data(ts_addr + 8, &(ts.tv_nsec as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }
}
