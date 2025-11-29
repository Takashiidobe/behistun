use anyhow::Result;

use crate::Cpu;

impl Cpu {
    /// settimeofday(tv, tz)
    pub(crate) fn sys_settimeofday(&self) -> Result<i64> {
        let tv_addr = self.data_regs[1] as usize;
        if tv_addr == 0 {
            return Ok(unsafe { libc::settimeofday(std::ptr::null(), std::ptr::null()) as i64 });
        }
        // m68k uclibc uses 64-bit time_t
        let tv_sec_bytes: [u8; 8] = self.memory.read_data(tv_addr, 8)?.try_into().unwrap();
        let tv_sec = i64::from_be_bytes(tv_sec_bytes) as libc::time_t;
        let tv_usec = self.memory.read_long(tv_addr + 8)? as libc::suseconds_t;
        let tv = libc::timeval { tv_sec, tv_usec };
        Ok(unsafe { libc::settimeofday(&tv, std::ptr::null()) as i64 })
    }
}
