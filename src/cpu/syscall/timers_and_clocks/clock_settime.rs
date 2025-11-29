use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// clock_settime(clockid, timespec)
    pub(crate) fn sys_clock_settime(&self) -> Result<i64> {
        let clk_id = self.data_regs[1] as libc::clockid_t;
        let ts_addr = self.data_regs[2] as usize;

        if ts_addr == 0 {
            return Ok(-(libc::EFAULT as i64));
        }

        // m68k uclibc uses 64-bit time_t
        let tv_sec_bytes: [u8; 8] = self.memory.read_data(ts_addr, 8)?.try_into().unwrap();
        let tv_sec = i64::from_be_bytes(tv_sec_bytes) as libc::time_t;
        let tv_nsec = self.memory.read_long(ts_addr + 8)? as i64;

        let ts = libc::timespec { tv_sec, tv_nsec };
        Ok(unsafe { libc::clock_settime(clk_id, &ts) as i64 })
    }
}
