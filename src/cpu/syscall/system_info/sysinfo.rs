use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_sysinfo(&mut self) -> Result<i64> {
        let info_addr = self.data_regs[1] as usize;
        let mut info: libc::sysinfo = unsafe { std::mem::zeroed() };
        let result = unsafe { libc::sysinfo(&mut info) };
        if result == 0 {
            self.memory
                .write_data(info_addr, &(info.uptime as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 4, &(info.loads[0] as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 8, &(info.loads[1] as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 12, &(info.loads[2] as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 16, &(info.totalram as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 20, &(info.freeram as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 24, &(info.sharedram as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 28, &(info.bufferram as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 32, &(info.totalswap as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 36, &(info.freeswap as u32).to_be_bytes())?;
            self.memory
                .write_data(info_addr + 40, &(info.procs as u16).to_be_bytes())?;
        }
        Ok(result as i64)
    }
}
