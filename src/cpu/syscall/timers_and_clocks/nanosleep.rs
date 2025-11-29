use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_nanosleep(&mut self) -> Result<i64> {
        let req_addr = self.data_regs[1] as usize;
        let rem_addr = self.data_regs[2] as usize;

        // m68k uclibc uses 64-bit time_t
        let req_sec_bytes: [u8; 8] = self.memory.read_data(req_addr, 8)?.try_into().unwrap();
        let req_sec = i64::from_be_bytes(req_sec_bytes) as libc::time_t;
        let req_nsec = self.memory.read_long(req_addr + 8)? as i64;

        let req = libc::timespec {
            tv_sec: req_sec,
            tv_nsec: req_nsec,
        };
        let mut rem: libc::timespec = unsafe { std::mem::zeroed() };

        let result = unsafe {
            libc::nanosleep(
                &req,
                if rem_addr != 0 {
                    &mut rem
                } else {
                    std::ptr::null_mut()
                },
            )
        };

        if rem_addr != 0 {
            self.memory
                .write_data(rem_addr, &(rem.tv_sec as i64).to_be_bytes())?;
            self.memory
                .write_data(rem_addr + 8, &(rem.tv_nsec as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }
}
