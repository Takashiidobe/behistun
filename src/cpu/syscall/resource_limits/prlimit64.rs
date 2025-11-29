use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_prlimit64(&mut self) -> Result<i64> {
        let pid = self.data_regs[1] as libc::pid_t;
        let resource = self.data_regs[2] as i32;
        let new_limit_addr = self.data_regs[3] as usize;
        let old_limit_addr = self.data_regs[4] as usize;

        let new_limit_ptr = if new_limit_addr != 0 {
            let cur_hi = self.memory.read_long(new_limit_addr)? as u64;
            let cur_lo = self.memory.read_long(new_limit_addr + 4)? as u64;
            let max_hi = self.memory.read_long(new_limit_addr + 8)? as u64;
            let max_lo = self.memory.read_long(new_limit_addr + 12)? as u64;
            let rlim_cur = (cur_hi << 32) | cur_lo;
            let rlim_max = (max_hi << 32) | max_lo;
            Some(libc::rlimit64 { rlim_cur, rlim_max })
        } else {
            None
        };

        let mut old_limit: libc::rlimit64 = unsafe { std::mem::zeroed() };

        let result = unsafe {
            libc::prlimit64(
                pid,
                resource as u32,
                new_limit_ptr
                    .as_ref()
                    .map(|l| l as *const _)
                    .unwrap_or(std::ptr::null()),
                if old_limit_addr != 0 {
                    &mut old_limit
                } else {
                    std::ptr::null_mut()
                },
            )
        };

        if result == 0 && old_limit_addr != 0 {
            let cur_hi = (old_limit.rlim_cur >> 32) as u32;
            let cur_lo = old_limit.rlim_cur as u32;
            let max_hi = (old_limit.rlim_max >> 32) as u32;
            let max_lo = old_limit.rlim_max as u32;
            self.memory
                .write_data(old_limit_addr, &cur_hi.to_be_bytes())?;
            self.memory
                .write_data(old_limit_addr + 4, &cur_lo.to_be_bytes())?;
            self.memory
                .write_data(old_limit_addr + 8, &max_hi.to_be_bytes())?;
            self.memory
                .write_data(old_limit_addr + 12, &max_lo.to_be_bytes())?;
        }

        Ok(result as i64)
    }
}
