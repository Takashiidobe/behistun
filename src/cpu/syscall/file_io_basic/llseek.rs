use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_llseek(&mut self) -> Result<i64> {
        let fd = self.data_regs[1] as i32;
        let offset_high = self.data_regs[2];
        let offset_low = self.data_regs[3];
        let result_addr = self.data_regs[4] as usize;
        let whence = self.data_regs[5] as i32;

        let offset = ((offset_high as i64) << 32) | (offset_low as i64);
        let result = unsafe { libc::lseek(fd, offset, whence) };

        if result >= 0 && result_addr != 0 {
            self.memory
                .write_data(result_addr, &((result >> 32) as u32).to_be_bytes())?;
            self.memory
                .write_data(result_addr + 4, &(result as u32).to_be_bytes())?;
            Ok(0)
        } else {
            Ok(result)
        }
    }
}
