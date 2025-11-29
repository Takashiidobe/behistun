use anyhow::Result;

use crate::Cpu;

impl Cpu {
    pub(crate) fn sys_splice(&mut self) -> Result<i64> {
        let fd_in = self.data_regs[1] as i32;
        let off_in_addr = self.data_regs[2] as usize;
        let fd_out = self.data_regs[3] as i32;
        let off_out_addr = self.data_regs[4] as usize;
        let len = self.data_regs[5] as usize;
        let flags = self.data_regs[6];

        let mut off_in_val = if off_in_addr != 0 {
            let high = self.memory.read_long(off_in_addr)?;
            let low = self.memory.read_long(off_in_addr + 4)?;
            ((high as i64) << 32) | (low as i64)
        } else {
            0
        };

        let mut off_out_val = if off_out_addr != 0 {
            let high = self.memory.read_long(off_out_addr)?;
            let low = self.memory.read_long(off_out_addr + 4)?;
            ((high as i64) << 32) | (low as i64)
        } else {
            0
        };

        let off_in_ptr = if off_in_addr != 0 {
            &mut off_in_val as *mut i64
        } else {
            std::ptr::null_mut()
        };
        let off_out_ptr = if off_out_addr != 0 {
            &mut off_out_val as *mut i64
        } else {
            std::ptr::null_mut()
        };

        let result = unsafe { libc::splice(fd_in, off_in_ptr, fd_out, off_out_ptr, len, flags) };

        if result >= 0 {
            if off_in_addr != 0 {
                let high = (off_in_val >> 32) as u32;
                let low = off_in_val as u32;
                self.memory.write_data(off_in_addr, &high.to_be_bytes())?;
                self.memory
                    .write_data(off_in_addr + 4, &low.to_be_bytes())?;
            }
            if off_out_addr != 0 {
                let high = (off_out_val >> 32) as u32;
                let low = off_out_val as u32;
                self.memory.write_data(off_out_addr, &high.to_be_bytes())?;
                self.memory
                    .write_data(off_out_addr + 4, &low.to_be_bytes())?;
            }
        }

        Ok(result as i64)
    }
}
