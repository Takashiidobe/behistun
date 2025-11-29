use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_getrlimit(&mut self) -> Result<i64> {
        let resource = self.data_regs[1] as i32;
        let rlim_addr = self.data_regs[2] as usize;
        let mut rlim: libc::rlimit = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::getrlimit(resource as u32, &mut rlim) };
        if result == 0 && rlim_addr != 0 {
            self.memory
                .write_data(rlim_addr, &(rlim.rlim_cur as u32).to_be_bytes())?;
            self.memory
                .write_data(rlim_addr + 4, &(rlim.rlim_max as u32).to_be_bytes())?;
        }
        Ok(result as i64)
    }
}
